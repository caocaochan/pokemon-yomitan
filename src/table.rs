//! Minimal CSV table access for the PokeAPI dump.

use std::collections::HashMap;
use std::path::Path;

pub const EN: u32 = 9;
pub const HANS: u32 = 12;
pub const HANT: u32 = 4;
pub const JA: u32 = 11;
pub const JA_KANA: u32 = 1;
pub const JA_ROMA: u32 = 2;

pub struct Table {
    cols: HashMap<String, usize>,
    rows: Vec<Vec<String>>,
}

#[derive(Clone, Copy)]
pub struct Row<'a> {
    t: &'a Table,
    r: &'a [String],
}

impl Table {
    pub fn load(dir: &Path, name: &str) -> Table {
        let path = dir.join(format!("{name}.csv"));
        let mut rdr = csv::ReaderBuilder::new()
            .flexible(true)
            .from_path(&path)
            .unwrap_or_else(|e| panic!("open {}: {e}", path.display()));
        let cols = rdr
            .headers()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, h)| (h.trim_start_matches('\u{feff}').to_string(), i))
            .collect();
        let rows = rdr
            .records()
            .map(|r| r.unwrap().iter().map(str::to_string).collect())
            .collect();
        Table { cols, rows }
    }

    pub fn iter(&self) -> impl Iterator<Item = Row<'_>> {
        self.rows.iter().map(move |r| Row { t: self, r })
    }
}

impl<'a> Row<'a> {
    pub fn s(&self, col: &str) -> &'a str {
        let i = *self.t.cols.get(col).unwrap_or_else(|| panic!("no column {col}"));
        self.r.get(i).map(String::as_str).unwrap_or("")
    }
    pub fn u(&self, col: &str) -> u32 {
        self.opt(col).unwrap_or(0)
    }
    pub fn opt(&self, col: &str) -> Option<u32> {
        self.s(col).parse().ok()
    }
    pub fn i(&self, col: &str) -> Option<i64> {
        self.s(col).parse().ok()
    }
}

/// id -> language -> text
pub type LangText = HashMap<u32, HashMap<u32, String>>;

pub fn lang_text(t: &Table, id_col: &str, lang_col: &str, text_col: &str) -> LangText {
    let mut m: LangText = HashMap::new();
    for r in t.iter() {
        let v = r.s(text_col).trim();
        if v.is_empty() {
            continue;
        }
        m.entry(r.u(id_col)).or_default().insert(r.u(lang_col), v.to_string());
    }
    m
}

pub fn get<'a>(m: &'a LangText, id: u32, lang: u32) -> Option<&'a str> {
    m.get(&id).and_then(|x| x.get(&lang)).map(String::as_str)
}
