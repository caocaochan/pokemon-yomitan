//! Legacy / regional Chinese names from 52poke `{{名字/entry|cn|tw|hk|…}}` tables.

use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct Legacy {
    pub name: String,
    /// (region code, sources) — region in cn/tw/hk
    pub uses: Vec<(String, String)>,
}

struct Batch {
    /// requested/normalized/redirected title -> final title
    map: HashMap<String, String>,
    pages: HashMap<String, String>,
}

fn load_batch(path: &Path) -> Batch {
    let v: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let q = &v["query"];
    let mut map = HashMap::new();
    for key in ["normalized", "redirects"] {
        if let Some(a) = q[key].as_array() {
            for r in a {
                map.insert(r["from"].as_str().unwrap().to_string(), r["to"].as_str().unwrap().to_string());
            }
        }
    }
    let mut pages = HashMap::new();
    if let Some(a) = q["pages"].as_array() {
        for p in a {
            if let Some(c) = p["revisions"][0]["slots"]["main"]["content"].as_str() {
                pages.insert(p["title"].as_str().unwrap().to_string(), c.to_string());
            }
        }
    }
    Batch { map, pages }
}

fn resolve<'a>(batches: &'a [Batch], title: &str) -> Option<&'a str> {
    for b in batches {
        let mut t = title.to_string();
        for _ in 0..3 {
            if let Some(n) = b.map.get(&t) {
                t = n.clone();
            }
        }
        if let Some(c) = b.pages.get(&t) {
            return Some(c);
        }
    }
    None
}

/// Returns species id -> legacy names, plus the titles whose page had no name table.
pub fn load(
    cache: &Path,
    official: &HashMap<u32, (String, String)>,
) -> (HashMap<u32, Vec<Legacy>>, Vec<String>) {
    let dir = cache.join("52poke");
    let mut batches = Vec::new();
    if let Ok(rd) = fs::read_dir(&dir) {
        let mut paths: Vec<_> = rd.filter_map(|e| e.ok()).map(|e| e.path()).collect();
        paths.sort();
        for p in paths {
            if p.extension().is_some_and(|e| e == "json") {
                batches.push(load_batch(&p));
            }
        }
    }
    let entry = Regex::new(r"\{\{名字/entry\|(cn|tw|hk)\|([^|{}\[\]]+)\|([^\n]*)").unwrap();
    let tt = Regex::new(r"tt=([^|}]+)").unwrap();
    let mut out: HashMap<u32, Vec<Legacy>> = HashMap::new();
    let mut missing = Vec::new();
    let mut ids: Vec<_> = official.keys().copied().collect();
    ids.sort();
    for id in ids {
        let (hans, hant) = &official[&id];
        let page = resolve(&batches, hans)
            .filter(|c| c.contains("{{名字/entry"))
            .or_else(|| resolve(&batches, &format!("{hans}（宝可梦）")).filter(|c| c.contains("{{名字/entry")));
        let Some(page) = page else {
            missing.push(hans.clone());
            continue;
        };
        let mut list: Vec<Legacy> = Vec::new();
        for cap in entry.captures_iter(page) {
            let region = cap[1].to_string();
            let name = cap[2].trim().to_string();
            if name == *hans || name == *hant || !name.chars().any(crate::pinyin::is_cjk) {
                continue;
            }
            let src = tt.captures(&cap[3]).map(|m| m[1].trim().to_string()).unwrap_or_default();
            match list.iter_mut().find(|l| l.name == name) {
                Some(l) => l.uses.push((region, src)),
                None => list.push(Legacy { name, uses: vec![(region, src)] }),
            }
        }
        if !list.is_empty() {
            out.insert(id, list);
        }
    }
    (out, missing)
}
