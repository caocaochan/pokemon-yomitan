//! Yomitan format-3 dictionary writer with schema validation.

use serde_json::{Value, json};
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;

const BANK_SIZE: usize = 2000;

#[derive(Default)]
pub struct Dict {
    pub terms: Vec<Value>,
    pub tags: Vec<Value>,
    /// (path in zip, source file)
    pub media: Vec<(String, PathBuf)>,
}

impl Dict {
    #[allow(clippy::too_many_arguments)]
    pub fn term(
        &mut self,
        term: &str,
        reading: &str,
        def_tags: &str,
        score: i64,
        content: Value,
        sequence: i64,
    ) {
        self.terms.push(json!([
            term,
            reading,
            def_tags,
            "",
            score,
            [{"type": "structured-content", "content": content}],
            sequence,
            ""
        ]));
    }

    pub fn tag(&mut self, name: &str, category: &str, order: i64, notes: &str) {
        self.tags.push(json!([name, category, order, notes, 0]));
    }

    pub fn write(&self, out: &Path, index: &Value, styles: &str, schema_dir: &Path) -> Result<(), String> {
        let banks: Vec<Value> = self.terms.chunks(BANK_SIZE).map(|c| Value::Array(c.to_vec())).collect();
        let tag_bank = Value::Array(self.tags.clone());

        // --- validation -------------------------------------------------------------
        let load = |n: &str| -> Value {
            serde_json::from_slice(&fs::read(schema_dir.join(n)).expect("schema")).expect("schema json")
        };
        let check = |schema: &Value, inst: &Value, what: &str| -> Result<(), String> {
            let v = jsonschema::validator_for(schema).map_err(|e| e.to_string())?;
            let errs: Vec<String> = v.iter_errors(inst).take(5).map(|e| format!("{e} at {}", e.instance_path())).collect();
            if errs.is_empty() { Ok(()) } else { Err(format!("{what}: {}", errs.join("\n"))) }
        };
        check(&load("dictionary-index-schema.json"), index, "index.json")?;
        check(&load("dictionary-tag-bank-v3-schema.json"), &tag_bank, "tag_bank")?;
        let term_schema = load("dictionary-term-bank-v3-schema.json");
        for (i, b) in banks.iter().enumerate() {
            check(&term_schema, b, &format!("term_bank_{}", i + 1))?;
        }
        let media: HashSet<&str> = self.media.iter().map(|(p, _)| p.as_str()).collect();
        let mut missing = Vec::new();
        for t in &self.terms {
            collect_img_paths(&t[5], &mut |p| {
                if !media.contains(p) {
                    missing.push(p.to_string());
                }
            });
        }
        if !missing.is_empty() {
            return Err(format!("{} image paths not in zip, e.g. {}", missing.len(), missing[0]));
        }
        let declared: HashSet<&str> = self.tags.iter().map(|t| t[0].as_str().unwrap()).collect();
        for t in &self.terms {
            for tag in t[2].as_str().unwrap().split_whitespace() {
                if !declared.contains(tag) {
                    return Err(format!("undeclared tag {tag}"));
                }
            }
        }

        // --- zip ------------------------------------------------------------------------
        fs::create_dir_all(out.parent().unwrap()).unwrap();
        let f = fs::File::create(out).map_err(|e| e.to_string())?;
        let mut z = zip::ZipWriter::new(f);
        let deflate = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let store = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        let mut put = |name: &str, bytes: &[u8], opt: SimpleFileOptions| -> Result<(), String> {
            z.start_file(name, opt).map_err(|e| e.to_string())?;
            z.write_all(bytes).map_err(|e| e.to_string())
        };
        put("index.json", &serde_json::to_vec(index).unwrap(), deflate)?;
        put("styles.css", styles.as_bytes(), deflate)?;
        put("tag_bank_1.json", &serde_json::to_vec(&tag_bank).unwrap(), deflate)?;
        for (i, b) in banks.iter().enumerate() {
            put(&format!("term_bank_{}.json", i + 1), &serde_json::to_vec(b).unwrap(), deflate)?;
        }
        for (p, src) in &self.media {
            put(p, &fs::read(src).map_err(|e| e.to_string())?, store)?;
        }
        z.finish().map_err(|e| e.to_string())?;
        Ok(())
    }
}

fn collect_img_paths(v: &Value, f: &mut dyn FnMut(&str)) {
    match v {
        Value::Object(m) => {
            if m.get("tag").and_then(Value::as_str) == Some("img")
                && let Some(p) = m.get("path").and_then(Value::as_str)
            {
                f(p);
            }
            for x in m.values() {
                collect_img_paths(x, f);
            }
        }
        Value::Array(a) => a.iter().for_each(|x| collect_img_paths(x, f)),
        _ => {}
    }
}
