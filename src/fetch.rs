//! Download pinned source data into `cache/`. Files already present are skipped.

use crate::table::{HANS, Table};
use rayon::prelude::*;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

pub const POKEAPI_SHA: &str = "de6760be38b60a3024f1d9054330ff265f0dc619";
pub const SPRITES_SHA: &str = "c0542f7c51215d9672439012838c99f520596cbd";
pub const YOMITAN_TAG: &str = "26.11.10.0";
const UNIHAN_URL: &str = "https://www.unicode.org/Public/17.0.0/ucd/Unihan.zip";
const CEDICT_URL: &str = "https://www.mdbg.net/chinese/export/cedict/cedict_1_0_ts_utf-8_mdbg.txt.gz";
const UA: &str = "pokeyomi/0.1 (Yomitan dictionary builder)";

pub const CSVS: &[&str] = &[
    "pokemon_species", "pokemon_species_names", "pokemon_species_flavor_text", "pokemon",
    "pokemon_forms", "pokemon_form_names", "pokemon_types", "pokemon_abilities", "pokemon_stats",
    "stats", "stat_names", "pokemon_egg_groups", "egg_group_prose", "pokemon_evolution",
    "evolution_triggers", "evolution_trigger_prose", "moves", "move_names", "move_flavor_text",
    "move_effect_prose", "move_damage_class_prose", "abilities", "ability_names",
    "ability_flavor_text", "ability_prose", "items", "item_names", "item_flavor_text",
    "item_prose", "item_categories", "item_category_prose", "types", "type_names",
    "type_efficacy", "natures", "nature_names", "contest_type_names", "version_groups",
    "versions", "version_names", "location_names",
];

const SCHEMAS: &[&str] = &[
    "dictionary-index-schema.json",
    "dictionary-term-bank-v3-schema.json",
    "dictionary-tag-bank-v3-schema.json",
];

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(120))
        .user_agent(UA)
        .build()
}

/// Ok(Some(bytes)) on 200, Ok(None) on 404, Err otherwise (after retries).
fn get(agent: &ureq::Agent, url: &str) -> Result<Option<Vec<u8>>, String> {
    let mut last = String::new();
    for attempt in 0..5 {
        match agent.get(url).call() {
            Ok(resp) => {
                let mut buf = Vec::new();
                resp.into_reader().read_to_end(&mut buf).map_err(|e| e.to_string())?;
                return Ok(Some(buf));
            }
            Err(ureq::Error::Status(404, _)) => return Ok(None),
            Err(e) => {
                last = e.to_string();
                std::thread::sleep(Duration::from_secs(2u64.pow(attempt)));
            }
        }
    }
    Err(format!("{url}: {last}"))
}

fn save(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let tmp = path.with_extension("part");
    fs::File::create(&tmp).unwrap().write_all(bytes).unwrap();
    fs::rename(&tmp, path).unwrap();
}

fn fetch_to(agent: &ureq::Agent, url: &str, path: &Path) {
    if path.exists() {
        return;
    }
    match get(agent, url) {
        Ok(Some(b)) => save(path, &b),
        Ok(None) => panic!("404: {url}"),
        Err(e) => panic!("{e}"),
    }
}

pub fn run(cache: &Path) {
    let a = agent();
    let csv_dir = cache.join("csv");
    for name in CSVS {
        let url = format!(
            "https://raw.githubusercontent.com/PokeAPI/pokeapi/{POKEAPI_SHA}/data/v2/csv/{name}.csv"
        );
        fetch_to(&a, &url, &csv_dir.join(format!("{name}.csv")));
    }
    println!("csv: {} tables", CSVS.len());

    for s in SCHEMAS {
        let url = format!(
            "https://raw.githubusercontent.com/yomidevs/yomitan/{YOMITAN_TAG}/ext/data/schemas/{s}"
        );
        fetch_to(&a, &url, &cache.join("schemas").join(s));
    }

    let cedict = cache.join("cedict.txt");
    if !cedict.exists() {
        let gz = get(&a, CEDICT_URL).unwrap().expect("cedict 404");
        let mut txt = Vec::new();
        flate2::read::GzDecoder::new(&gz[..]).read_to_end(&mut txt).unwrap();
        save(&cedict, &txt);
    }
    let unihan = cache.join("Unihan_Readings.txt");
    if !unihan.exists() {
        let z = get(&a, UNIHAN_URL).unwrap().expect("unihan 404");
        let mut ar = zip::ZipArchive::new(std::io::Cursor::new(z)).unwrap();
        let mut txt = Vec::new();
        ar.by_name("Unihan_Readings.txt").unwrap().read_to_end(&mut txt).unwrap();
        save(&unihan, &txt);
    }
    println!("cedict + unihan ok");

    sprites(&a, cache, &csv_dir);
    poke52(&a, cache, &csv_dir);
}

/// Each target is a list of candidate URLs tried in order; a `.missing` marker records
/// that none exist so reruns don't hit the network again.
fn sprites(a: &ureq::Agent, cache: &Path, csv_dir: &Path) {
    let base = format!("https://raw.githubusercontent.com/PokeAPI/sprites/{SPRITES_SHA}/sprites");
    let mut jobs: Vec<(PathBuf, Vec<String>)> = Vec::new();
    for r in Table::load(csv_dir, "pokemon").iter() {
        let id = r.u("id");
        jobs.push((
            cache.join(format!("sprites/pokemon/{id}.png")),
            vec![
                format!("{base}/pokemon/other/official-artwork/{id}.png"),
                format!("{base}/pokemon/other/home/{id}.png"),
                format!("{base}/pokemon/{id}.png"),
            ],
        ));
    }
    let names = Table::load(csv_dir, "item_names");
    let zh_items: std::collections::HashSet<u32> =
        names.iter().filter(|r| r.u("local_language_id") == HANS).map(|r| r.u("item_id")).collect();
    for r in Table::load(csv_dir, "items").iter() {
        if !zh_items.contains(&r.u("id")) {
            continue;
        }
        let ident = r.s("identifier");
        jobs.push((
            cache.join(format!("sprites/items/{ident}.png")),
            vec![format!("{base}/items/{ident}.png")],
        ));
    }
    jobs.retain(|(p, _)| !p.exists() && !p.with_extension("missing").exists());
    let total = jobs.len();
    let done = AtomicUsize::new(0);
    let pool = rayon::ThreadPoolBuilder::new().num_threads(8).build().unwrap();
    pool.install(|| {
        jobs.par_iter().for_each(|(path, urls)| {
            let mut found = false;
            for u in urls {
                if let Some(b) = get(a, u).unwrap_or_else(|e| panic!("{e}")) {
                    save(path, &b);
                    found = true;
                    break;
                }
            }
            if !found {
                save(&path.with_extension("missing"), b"");
            }
            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
            if n % 250 == 0 || n == total {
                println!("sprites {n}/{total}");
            }
        });
    });
}

/// 52poke species pages (wikitext), 50 titles per API call.
fn poke52(a: &ureq::Agent, cache: &Path, csv_dir: &Path) {
    let dir = cache.join("52poke");
    let mut titles: Vec<(u32, String)> = Table::load(csv_dir, "pokemon_species_names")
        .iter()
        .filter(|r| r.u("local_language_id") == HANS)
        .map(|r| (r.u("pokemon_species_id"), r.s("name").to_string()))
        .collect();
    titles.sort();
    for (i, chunk) in titles.chunks(50).enumerate() {
        let path = dir.join(format!("batch_{i:02}.json"));
        if path.exists() {
            continue;
        }
        let t: Vec<&str> = chunk.iter().map(|(_, n)| n.as_str()).collect();
        let url = wiki_url(&t);
        let b = get(a, &url).unwrap().expect("52poke 404");
        save(&path, &b);
        std::thread::sleep(Duration::from_secs(1));
    }
    println!("52poke: {} batches", titles.len().div_ceil(50));
}

pub fn wiki_url(titles: &[&str]) -> String {
    let joined = titles.join("|");
    let mut enc = String::new();
    for b in joined.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => enc.push(b as char),
            _ => enc.push_str(&format!("%{b:02X}")),
        }
    }
    format!(
        "https://wiki.52poke.com/api.php?action=query&prop=revisions&rvprop=content&rvslots=main\
         &format=json&formatversion=2&redirects=1&titles={enc}"
    )
}

/// Second pass for titles whose page lacked a name table (disambiguation pages):
/// retry with the "（宝可梦）" suffix.
pub fn poke52_retry(cache: &Path, titles: &[String]) {
    if titles.is_empty() {
        return;
    }
    let a = agent();
    let dir = cache.join("52poke");
    for (i, chunk) in titles.chunks(50).enumerate() {
        let path = dir.join(format!("retry_{i:02}.json"));
        if path.exists() {
            continue;
        }
        let t: Vec<String> = chunk.iter().map(|n| format!("{n}（宝可梦）")).collect();
        let t: Vec<&str> = t.iter().map(String::as_str).collect();
        let b = get(&a, &wiki_url(&t)).unwrap().expect("52poke 404");
        save(&path, &b);
        std::thread::sleep(Duration::from_secs(1));
    }
}
