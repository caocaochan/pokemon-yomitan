//! Pinyin generation (CC-CEDICT phrase segmentation + Unihan fallback + manual overrides)
//! and character-level Simplified/Traditional conversion derived from CC-CEDICT.

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::Path;
use std::sync::Mutex;

pub struct Pinyin {
    /// headword (simplified or traditional) -> numbered pinyin, lowercased
    words: HashMap<String, String>,
    max_len: usize,
    /// Unihan kMandarin first value (tone-marked)
    unihan: HashMap<char, String>,
    /// distinct tone-marked readings per char from CC-CEDICT single-char entries
    alts: HashMap<char, BTreeSet<String>>,
    overrides: HashMap<String, String>,
    t2s: HashMap<char, char>,
    s2t: HashMap<char, char>,
    /// (category, term, reading, note) for names that used an ambiguous fallback
    pub review: Mutex<Vec<(String, String, String, String)>>,
    /// Species and legacy names already read (tone-marked); reused as phrases in item/move names.
    names: Mutex<HashMap<String, String>>,
}

/// Stand-alone readings where Unihan's first kMandarin value is wrong for names
/// (地 = "de", 长 = "zhǎng", 勒 = "lēi", 仔 = "zǐ" in Unihan).
const CHAR_DEFAULTS: &[(char, &str)] = &[
    ('地', "dì"),
    ('长', "cháng"),
    ('長', "cháng"),
    ('勒', "lè"),
    ('仔', "zǎi"),
    ('为', "wéi"),
    ('為', "wéi"),
    ('尽', "jìn"),
    ('盡', "jìn"),
];

pub fn is_cjk(c: char) -> bool {
    matches!(c as u32, 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0x20000..=0x3FFFF)
}

/// Full-width ASCII forms → ASCII; ideographic space → space.
pub fn halfwidth(s: &str) -> String {
    s.chars()
        .map(|c| match c as u32 {
            0xFF01..=0xFF5E => char::from_u32(c as u32 - 0xFEE0).unwrap(),
            0x3000 => ' ',
            _ => c,
        })
        .collect()
}

/// Spellings a reader may meet for the same name: as given, half-width, and with the
/// katakana middle dot ・ written as · or dropped.
pub fn variants(s: &str) -> Vec<String> {
    let h = halfwidth(s);
    let mut v = vec![s.to_string(), h.clone()];
    if h.contains('・') {
        v.push(h.replace('・', "·"));
        v.push(h.replace('・', ""));
    }
    let mut seen = std::collections::HashSet::new();
    v.retain(|x| seen.insert(x.clone()));
    v
}

/// "zhong3" -> "zhǒng", "lu:4" -> "lǜ", "zi5" -> "zi".
pub fn mark(syl: &str) -> String {
    let s = syl.to_lowercase().replace("u:", "ü").replace('v', "ü");
    let (base, tone) = match s.chars().last() {
        Some(d @ '1'..='5') => (&s[..s.len() - 1], d.to_digit(10).unwrap()),
        _ => (s.as_str(), 5),
    };
    if tone == 5 || base.is_empty() {
        return base.to_string();
    }
    const MARKS: [(char, [char; 4]); 6] = [
        ('a', ['ā', 'á', 'ǎ', 'à']),
        ('e', ['ē', 'é', 'ě', 'è']),
        ('i', ['ī', 'í', 'ǐ', 'ì']),
        ('o', ['ō', 'ó', 'ǒ', 'ò']),
        ('u', ['ū', 'ú', 'ǔ', 'ù']),
        ('ü', ['ǖ', 'ǘ', 'ǚ', 'ǜ']),
    ];
    let chars: Vec<char> = base.chars().collect();
    let target = if let Some(p) = chars.iter().position(|&c| c == 'a' || c == 'e') {
        p
    } else if let Some(p) = base.find("ou").map(|b| base[..b].chars().count()) {
        p
    } else if let Some(p) = chars.iter().rposition(|c| "aeiouü".contains(*c)) {
        p
    } else {
        return base.to_string();
    };
    chars
        .iter()
        .enumerate()
        .map(|(i, &c)| {
            if i == target {
                MARKS.iter().find(|(v, _)| *v == c).map(|(_, m)| m[tone as usize - 1]).unwrap_or(c)
            } else {
                c
            }
        })
        .collect()
}

fn marked(numbered: &str) -> String {
    numbered.split_whitespace().map(mark).collect::<Vec<_>>().join(" ")
}

impl Pinyin {
    pub fn load(cache: &Path, overrides: &Path) -> Pinyin {
        let mut counts: HashMap<String, HashMap<String, usize>> = HashMap::new();
        let mut alts: HashMap<char, BTreeSet<String>> = HashMap::new();
        let mut t2s_c: HashMap<char, HashMap<char, usize>> = HashMap::new();
        let mut s2t_c: HashMap<char, HashMap<char, usize>> = HashMap::new();
        let text = fs::read_to_string(cache.join("cedict.txt")).expect("cedict.txt");
        for line in text.lines() {
            if line.starts_with('#') {
                continue;
            }
            let Some((head, rest)) = line.split_once(" [") else { continue };
            let Some((py, defs)) = rest.split_once("] ") else { continue };
            let Some((trad, simp)) = head.split_once(' ') else { continue };
            let py = py.to_lowercase();
            // Readings like "xx5" or with punctuation tokens are useless for names.
            if py.contains("xx") {
                continue;
            }
            for hw in [trad, simp] {
                *counts.entry(hw.to_string()).or_default().entry(py.clone()).or_default() += 1;
            }
            let (tc, sc): (Vec<char>, Vec<char>) = (trad.chars().collect(), simp.chars().collect());
            if tc.len() == sc.len() {
                for (t, s) in tc.iter().zip(&sc) {
                    *t2s_c.entry(*t).or_default().entry(*s).or_default() += 1;
                    *s2t_c.entry(*s).or_default().entry(*t).or_default() += 1;
                }
            }
            if sc.len() == 1 && tc.len() == 1 {
                let d = defs.trim_start_matches('/');
                let restricted = ["variant of", "old variant", "used in", "surname", "see ", "erhua"]
                    .iter()
                    .any(|p| d.starts_with(p));
                let m = mark(&py);
                if !restricted && !py.ends_with('5') {
                    alts.entry(sc[0]).or_default().insert(m.clone());
                    alts.entry(tc[0]).or_default().insert(m);
                }
            }
        }
        let pick = |m: HashMap<char, HashMap<char, usize>>| -> HashMap<char, char> {
            m.into_iter()
                .map(|(k, v)| {
                    let best = v.into_iter().max_by_key(|(c, n)| (*n, std::cmp::Reverse(*c))).unwrap().0;
                    (k, best)
                })
                .collect()
        };
        let words: HashMap<String, String> = counts
            .into_iter()
            .map(|(k, v)| {
                let best = v.into_iter().max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0))).unwrap().0;
                (k, best)
            })
            .collect();
        let max_len = words.keys().map(|k| k.chars().count()).max().unwrap_or(1).min(8);

        let mut unihan = HashMap::new();
        let u = fs::read_to_string(cache.join("Unihan_Readings.txt")).expect("Unihan_Readings.txt");
        for line in u.lines() {
            let mut f = line.split('\t');
            let (Some(cp), Some("kMandarin"), Some(val)) = (f.next(), f.next(), f.next()) else { continue };
            let Some(c) = u32::from_str_radix(cp.trim_start_matches("U+"), 16).ok().and_then(char::from_u32)
            else {
                continue;
            };
            unihan.insert(c, val.split_whitespace().next().unwrap_or("").to_string());
        }

        let mut ov = HashMap::new();
        if let Ok(t) = fs::read_to_string(overrides) {
            for line in t.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((k, v)) = line.split_once('\t') {
                    ov.insert(k.trim().to_string(), v.trim().to_string());
                }
            }
        }
        Pinyin {
            words,
            max_len,
            unihan,
            alts,
            overrides: ov,
            t2s: pick(t2s_c),
            s2t: pick(s2t_c),
            review: Mutex::new(Vec::new()),
            names: Mutex::new(HashMap::new()),
        }
    }

    pub fn to_simp(&self, s: &str) -> String {
        s.chars().map(|c| *self.t2s.get(&c).unwrap_or(&c)).collect()
    }
    pub fn to_trad(&self, s: &str) -> String {
        s.chars().map(|c| *self.s2t.get(&c).unwrap_or(&c)).collect()
    }

    /// Tone-marked, space-separated reading. `alt` is an equivalent spelling (the other
    /// script) whose override, if any, also applies.
    pub fn reading(&self, category: &str, term: &str, alt: Option<&str>) -> String {
        let remember = |r: &str| {
            if category == "species" || category == "legacy" {
                let mut n = self.names.lock().unwrap();
                n.insert(halfwidth(term), r.to_string());
                if let Some(a) = alt {
                    n.insert(halfwidth(a), r.to_string());
                }
            }
        };
        if let Some(r) = self.overrides.get(term).or_else(|| alt.and_then(|a| self.overrides.get(a))) {
            remember(r);
            return r.clone();
        }
        let names = self.names.lock().unwrap();
        let norm = halfwidth(term);
        let chars: Vec<char> = norm.chars().collect();
        let n = chars.len();
        // best[i] = (chars covered by multi-char words, -segments) for prefix of length i
        let mut best: Vec<Option<(usize, i64, usize)>> = vec![None; n + 1];
        best[0] = Some((0, 0, 0));
        for i in 0..n {
            let Some((cov, seg, _)) = best[i] else { continue };
            let mut relax = |j: usize, add: usize| {
                let cand = (cov + add, seg - 1, i);
                if best[j].is_none_or(|b| (cand.0, cand.1) > (b.0, b.1)) {
                    best[j] = Some(cand);
                }
            };
            relax(i + 1, 0);
            for l in 2..=self.max_len.min(n - i) {
                let w: String = chars[i..i + l].iter().collect();
                if names.contains_key(&w) || self.words.contains_key(&w) {
                    relax(i + l, l);
                }
            }
        }
        let mut cuts = vec![n];
        let mut j = n;
        while j > 0 {
            j = best[j].unwrap().2;
            cuts.push(j);
        }
        cuts.reverse();

        let mut out: Vec<String> = Vec::new();
        let mut notes: Vec<String> = Vec::new();
        let mut ascii = String::new();
        let flush = |ascii: &mut String, out: &mut Vec<String>| {
            if !ascii.is_empty() {
                out.push(std::mem::take(ascii).to_lowercase());
            }
        };
        for w in cuts.windows(2) {
            let seg: String = chars[w[0]..w[1]].iter().collect();
            if seg.chars().count() > 1 {
                flush(&mut ascii, &mut out);
                out.push(names.get(&seg).cloned().unwrap_or_else(|| marked(&self.words[&seg])));
                continue;
            }
            let c = chars[w[0]];
            if c.is_ascii_alphanumeric() {
                ascii.push(c);
                continue;
            }
            flush(&mut ascii, &mut out);
            if !is_cjk(c) {
                continue; // punctuation such as ・ ： Ⅱ
            }
            let from_default = CHAR_DEFAULTS.iter().find(|(k, _)| *k == c).map(|(_, r)| r.to_string());
            let from_unihan = from_default.or_else(|| self.unihan.get(&c).cloned());
            let from_cedict = self.words.get(&c.to_string()).map(|p| marked(p));
            let r = from_unihan.or(from_cedict).unwrap_or_else(|| c.to_string());
            if let Some(a) = self.alts.get(&c)
                && a.iter().any(|x| x != &r)
            {
                let others: Vec<&str> = a.iter().filter(|x| *x != &r).map(String::as_str).collect();
                notes.push(format!("{c}={r} (also {})", others.join("/")));
            }
            out.push(r);
        }
        flush(&mut ascii, &mut out);
        drop(names);
        let reading = out.join(" ");
        remember(&reading);
        if !notes.is_empty() {
            self.review.lock().unwrap().push((
                category.to_string(),
                term.to_string(),
                reading.clone(),
                notes.join("; "),
            ));
        }
        reading
    }
}

#[cfg(test)]
mod tests {
    use super::mark;
    #[test]
    fn marks() {
        assert_eq!(mark("zhong3"), "zhǒng");
        assert_eq!(mark("lu:4"), "lǜ");
        assert_eq!(mark("zi5"), "zi");
        assert_eq!(mark("gou3"), "gǒu");
        assert_eq!(mark("gui4"), "guì");
        assert_eq!(mark("liu2"), "liú");
        assert_eq!(mark("xue2"), "xué");
        assert_eq!(mark("Pi2"), "pí");
        assert_eq!(mark("er2"), "ér");
        assert_eq!(mark("r5"), "r");
    }
}
