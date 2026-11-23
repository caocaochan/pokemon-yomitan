//! Structured-content entries for every category.

use crate::db::*;
use crate::images::Img;
use crate::legacy::Legacy;
use crate::pinyin::{Pinyin, variants};
use crate::table::*;
use crate::yomitan::Dict;
use regex::Regex;
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum V {
    S,
    T,
}
impl V {
    fn lang(self) -> u32 {
        if self == V::S { HANS } else { HANT }
    }
    fn other(self) -> u32 {
        if self == V::S { HANT } else { HANS }
    }
}
fn l(v: V, s: &'static str, t: &'static str) -> &'static str {
    if v == V::S { s } else { t }
}

// ---- structured-content helpers -----------------------------------------------------------

fn node(tag: &str, pk: &str, content: Value) -> Value {
    if pk.is_empty() {
        json!({"tag": tag, "content": content})
    } else {
        json!({"tag": tag, "data": {"pk": pk}, "content": content})
    }
}
fn div(pk: &str, c: impl Into<Value>) -> Value {
    node("div", pk, c.into())
}
fn span(pk: &str, c: impl Into<Value>) -> Value {
    node("span", pk, c.into())
}
fn lang_span(pk: &str, lang: &str, c: &str) -> Value {
    json!({"tag": "span", "lang": lang, "data": {"pk": pk}, "content": c})
}
fn details(summary: &str, body: Vec<Value>) -> Value {
    json!({"tag": "details", "data": {"pk": "more"}, "content": [
        {"tag": "summary", "content": summary}, {"tag": "div", "content": body}
    ]})
}
fn img(i: &Img, size: f64, pk: &str) -> Value {
    let mut v = json!({
        "tag": "img", "path": i.zip_path, "width": size, "height": size, "sizeUnits": "em",
        "collapsible": false, "collapsed": false, "background": false, "appearance": "auto",
        "data": {"pk": pk}
    });
    if i.pixelated {
        v["imageRendering"] = json!("pixelated");
    }
    v
}
fn zh_text(v: V, c: &str) -> Value {
    json!({"tag": "div", "lang": if v == V::S { "zh-CN" } else { "zh-TW" }, "data": {"pk": "zh"}, "content": c})
}
fn en_text(c: &str) -> Value {
    json!({"tag": "div", "lang": "en", "data": {"pk": "en"}, "content": c})
}
/// two-row table: header labels, then values
fn grid(heads: Vec<String>, vals: Vec<String>, pk: &str) -> Value {
    let th: Vec<Value> = heads.into_iter().map(|h| json!({"tag": "th", "content": h})).collect();
    let td: Vec<Value> = vals.into_iter().map(|h| json!({"tag": "td", "content": h})).collect();
    json!({"tag": "table", "data": {"pk": pk}, "content": [
        {"tag": "tr", "content": th}, {"tag": "tr", "content": td}
    ]})
}
/// key/value rows
fn facts(rows: Vec<(String, Value)>) -> Value {
    let trs: Vec<Value> = rows
        .into_iter()
        .map(|(k, v)| json!({"tag": "tr", "content": [{"tag": "th", "content": k}, {"tag": "td", "content": v}]}))
        .collect();
    json!({"tag": "table", "data": {"pk": "facts"}, "content": trs})
}

// ---- text helpers ---------------------------------------------------------------------------

fn clean_zh(s: &str) -> String {
    s.chars().filter(|c| !matches!(c, '\n' | '\r' | '\u{c}' | '\u{ad}' | '\u{3000}')).collect::<String>().trim().to_string()
}
fn clean_en(s: &str) -> String {
    s.replace("-\n", "-").replace('\u{ad}', "").split_whitespace().collect::<Vec<_>>().join(" ")
}

pub struct Ctx<'a> {
    pub db: &'a Db,
    pub py: &'a Pinyin,
    pub poke_img: HashMap<u32, Img>,
    pub item_img: HashMap<String, Img>,
    pub legacy: HashMap<u32, Vec<Legacy>>,
    markup: Regex,
}

impl<'a> Ctx<'a> {
    pub fn new(db: &'a Db, py: &'a Pinyin, poke_img: HashMap<u32, Img>, item_img: HashMap<String, Img>, legacy: HashMap<u32, Vec<Legacy>>) -> Self {
        Ctx { db, py, poke_img, item_img, legacy, markup: Regex::new(r"\[([^\]]*)\]\{[a-z]+:([a-z0-9-]+)(?::[a-z0-9-]+)?\}").unwrap() }
    }

    fn prose(&self, s: &str, chance: Option<u32>) -> String {
        let s = self.markup.replace_all(s, |c: &regex::Captures| {
            if c[1].is_empty() { c[2].replace('-', " ") } else { c[1].to_string() }
        });
        let s = match chance {
            Some(n) => s.replace("$effect_chance", &n.to_string()),
            None => s.to_string(),
        };
        clean_en(&s)
    }

    /// zh name in the variant, falling back to the other script, then English.
    fn zh(&self, m: &LangText, id: u32, v: V) -> String {
        get(m, id, v.lang()).or_else(|| get(m, id, v.other())).or_else(|| get(m, id, EN)).unwrap_or("").to_string()
    }
    /// "电 Electric"
    fn bi(&self, m: &LangText, id: u32, v: V) -> String {
        let zh = self.zh(m, id, v);
        match get(m, id, EN) {
            Some(en) if en != zh => format!("{zh} {en}"),
            _ => zh,
        }
    }
    fn type_badge(&self, t: u32, v: V) -> Value {
        let ident = self.db.type_ident.get(&t).cloned().unwrap_or_default();
        json!({"tag": "span", "data": {"pk": "type", "type": ident}, "content": self.bi(&self.db.type_name, t, v)})
    }
    fn class_badge(&self, c: u32, v: V) -> Value {
        let ident = match c { 1 => "status", 2 => "physical", _ => "special" };
        let zh = self.zh(&self.db.class_name, c, v);
        let en = get(&self.db.class_name, c, EN).unwrap_or("");
        let en: String = en.chars().take(1).flat_map(char::to_uppercase).chain(en.chars().skip(1)).collect();
        json!({"tag": "span", "data": {"pk": "class", "class": ident}, "content": format!("{zh} {en}")})
    }

    /// Names line: English · Japanese (+ romaji) (+ other-script Chinese)
    fn names_line(&self, m: &LangText, id: u32, v: V, roma: bool) -> Value {
        let mut parts = Vec::new();
        if let Some(en) = get(m, id, EN) {
            parts.push(lang_span("en-name", "en", en));
        }
        if let Some(ja) = get(m, id, JA).or_else(|| get(m, id, JA_KANA)) {
            parts.push(lang_span("ja-name", "ja", ja));
            if let Some(kana) = get(m, id, JA_KANA)
                && kana != ja
            {
                parts.push(lang_span("ja-kana", "ja", &format!("（{kana}）")));
            }
        }
        if roma
            && let Some(r) = get(m, id, JA_ROMA)
            && Some(r) != get(m, id, EN)
        {
            parts.push(lang_span("roma", "ja-Latn", r));
        }
        if let (Some(a), Some(b)) = (get(m, id, v.lang()), get(m, id, v.other()))
            && a != b
        {
            let label = if v == V::S { "繁 " } else { "简 " };
            parts.push(lang_span("alt-script", if v == V::S { "zh-TW" } else { "zh-CN" }, &format!("{label}{b}")));
        }
        div("names", parts)
    }

    /// Best flavor text in `lang` by rank (higher = preferred), plus English from the same key
    /// when available, else the best English.
    fn flavor(&self, rows: Option<&Vec<(u32, u32, String)>>, v: V, rank: &dyn Fn(u32) -> i64) -> (Option<(u32, String)>, Option<(u32, String)>) {
        let Some(rows) = rows else { return (None, None) };
        let best = |lang: u32| rows.iter().filter(|r| r.1 == lang).max_by_key(|r| rank(r.0));
        let zh = best(v.lang()).or_else(|| best(v.other()));
        let en = zh
            .and_then(|z| rows.iter().find(|r| r.1 == EN && r.0 == z.0))
            .or_else(|| best(EN));
        (zh.map(|r| (r.0, clean_zh(&r.2))), en.map(|r| (r.0, clean_en(&r.2))))
    }

    fn vg_rank(&self) -> impl Fn(u32) -> i64 + '_ {
        |vg| *self.db.vg_order.get(&vg).unwrap_or(&0)
    }
}

const DEX_ORDER: &[&str] = &[
    "scarlet", "violet", "sword", "shield", "brilliant-diamond", "shining-pearl", "ultra-sun",
    "ultra-moon", "sun", "moon", "lets-go-pikachu", "lets-go-eevee", "legends-za", "legends-arceus",
];

fn region_label(v: V, r: &str) -> &'static str {
    match r {
        "cn" => l(v, "大陆", "大陸"),
        "tw" => l(v, "台湾", "台灣"),
        _ => "香港",
    }
}
fn short_sources(s: &str) -> String {
    let parts: Vec<&str> = s.split('、').filter(|x| !x.is_empty()).collect();
    if parts.len() > 3 { format!("{}等", parts[..3].join("、")) } else { parts.join("、") }
}

// ---- species ---------------------------------------------------------------------------------

impl Ctx<'_> {
    fn default_pokemon(&self, sid: u32) -> Option<&Pokemon> {
        self.db.pokemon.iter().find(|p| p.species == sid && p.is_default)
    }

    fn evo_cond(&self, sid: u32, v: V) -> String {
        let db = self.db;
        let Some(rows) = db.evo.get(&sid) else { return String::new() };
        let item = |i: u32| self.zh(&db.item_name, i, v);
        let mv = |m: u32| self.zh(&db.move_name, m, v);
        let sp = |s: u32| self.zh(&db.species_name, s, v);
        let ty = |t: u32| self.zh(&db.type_name, t, v);
        let mut out: Vec<String> = Vec::new();
        for e in rows {
            let mut p: Vec<String> = Vec::new();
            match e.trigger {
                1 => {
                    if let Some(n) = e.min_level {
                        p.push(format!("Lv.{n}"));
                    }
                }
                2 => p.push(l(v, "通信交换", "通信交換").into()),
                3 => {
                    if let Some(i) = e.item {
                        p.push(format!("{}{}", l(v, "使用", "使用"), item(i)));
                    }
                }
                t => p.push(get(&db.evo_trigger_name, t, EN).unwrap_or("?").to_string()),
            }
            if let Some(i) = e.held_item {
                p.push(format!("{}{}", l(v, "携带", "攜帶"), item(i)));
            }
            if let Some(m) = e.known_move {
                p.push(format!("{}{}", l(v, "学会", "學會"), mv(m)));
            }
            if let Some(t) = e.known_move_type {
                p.push(format!("{}{}{}", l(v, "学会", "學會"), ty(t), l(v, "属性招式", "屬性招式")));
            }
            if e.happiness {
                p.push(l(v, "亲密度高", "親密度高").into());
            }
            if e.affection {
                p.push(l(v, "友好度高", "友好度高").into());
            }
            if e.beauty {
                p.push(l(v, "美丽度高", "美麗度高").into());
            }
            match e.time_of_day.as_str() {
                "day" => p.push(l(v, "白天", "白天").into()),
                "night" => p.push(l(v, "夜晚", "夜晚").into()),
                "dusk" => p.push(l(v, "黄昏", "黃昏").into()),
                "full-moon" => p.push(l(v, "满月", "滿月").into()),
                _ => {}
            }
            match e.gender {
                Some(1) => p.push("♀".into()),
                Some(2) => p.push("♂".into()),
                _ => {}
            }
            if let Some(loc) = e.location {
                p.push(format!("{}{}", l(v, "在", "在"), self.zh(&db.location_name, loc, v)));
            }
            if let Some(s) = e.party_species {
                p.push(format!("{}{}", l(v, "同行有", "同行有"), sp(s)));
            }
            if let Some(t) = e.party_type {
                p.push(format!("{}{}", ty(t), l(v, "属性宝可梦同行", "屬性寶可夢同行")));
            }
            if let Some(s) = e.trade_species {
                p.push(format!("{}{}{}", l(v, "与", "與"), sp(s), l(v, "交换", "交換")));
            }
            if e.rain {
                p.push(l(v, "下雨", "下雨").into());
            }
            if e.upside_down {
                p.push(l(v, "倒置主机", "倒置主機").into());
            }
            match e.phys {
                Some(1) => p.push(l(v, "攻击>防御", "攻擊>防禦").into()),
                Some(-1) => p.push(l(v, "攻击<防御", "攻擊<防禦").into()),
                Some(0) => p.push(l(v, "攻击=防御", "攻擊=防禦").into()),
                _ => {}
            }
            if let Some(m) = e.used_move {
                let n = e.move_count.map(|n| format!("{n}{}", l(v, "次", "次"))).unwrap_or_default();
                p.push(format!("{}{}{}", l(v, "使用", "使用"), mv(m), n));
            }
            if let Some(n) = e.steps {
                p.push(format!("{n}{}", l(v, "步", "步")));
            }
            if let Some(n) = e.damage {
                p.push(format!("{}{n}", l(v, "受到伤害≥", "受到傷害≥")));
            }
            if p.is_empty() {
                p.push(l(v, "升级", "升級").into());
            }
            let s = p.join("，");
            if !out.contains(&s) {
                out.push(s);
            }
        }
        out.join(l(v, " 或 ", " 或 "))
    }

    fn evo_tree(&self, sid: u32, v: V) -> Option<Value> {
        let db = self.db;
        let me = db.species.iter().find(|s| s.id == sid)?;
        let chain: Vec<&Species> = db.species.iter().filter(|s| s.chain == me.chain).collect();
        if chain.len() < 2 {
            return None;
        }
        let mut kids: BTreeMap<u32, Vec<&Species>> = BTreeMap::new();
        for s in &chain {
            if let Some(p) = s.evolves_from {
                kids.entry(p).or_default().push(s);
            }
        }
        for k in kids.values_mut() {
            k.sort_by_key(|s| s.order);
        }
        fn li(c: &Ctx, s: &Species, current: u32, v: V, kids: &BTreeMap<u32, Vec<&Species>>, root: bool) -> Value {
            let mut content = vec![span(
                if s.id == current { "evo-cur" } else { "evo-name" },
                c.bi(&c.db.species_name, s.id, v),
            )];
            if !root {
                let cond = c.evo_cond(s.id, v);
                if !cond.is_empty() {
                    content.insert(0, span("evo-cond", format!("（{cond}）")));
                }
                content.insert(0, span("evo-arrow", "→ "));
            }
            if let Some(ch) = kids.get(&s.id) {
                let items: Vec<Value> = ch.iter().map(|k| li(c, k, current, v, kids, false)).collect();
                content.push(node("ul", "evo", items.into()));
            }
            node("li", "", content.into())
        }
        let roots: Vec<&&Species> = chain.iter().filter(|s| s.evolves_from.is_none()).collect();
        let items: Vec<Value> = roots.iter().map(|r| li(self, r, sid, v, &kids, true)).collect();
        Some(node("ul", "evo", items.into()))
    }

    fn species_content(&self, sid: u32, v: V, banner: Option<Value>) -> Value {
        let db = self.db;
        let sp = db.species.iter().find(|s| s.id == sid).unwrap();
        let mut out: Vec<Value> = Vec::new();
        if let Some(b) = banner {
            out.push(b);
        }
        let pk = self.default_pokemon(sid);
        let pid = pk.map(|p| p.id).unwrap_or(sid);

        // header
        let mut names = vec![span("dex", format!("#{sid:04}"))];
        names.push(self.names_line(&db.species_name, sid, v, true));
        let genus_zh = self.zh(&db.genus, sid, v);
        let genus = match get(&db.genus, sid, EN) {
            Some(en) => format!("{genus_zh} · {en}"),
            None => genus_zh,
        };
        names.push(div("genus", genus));
        let types: Vec<Value> = db.poke_types.get(&pid).into_iter().flatten().map(|&t| self.type_badge(t, v)).collect();
        names.push(div("types", types));
        let mut head = Vec::new();
        if let Some(i) = self.poke_img.get(&pid) {
            head.push(img(i, 7.0, "art"));
        }
        head.push(div("head-text", names));
        out.push(div("head", head));

        // facts
        let mut rows: Vec<(String, Value)> = Vec::new();
        if let Some(ab) = db.poke_abilities.get(&pid) {
            let list: Vec<String> = ab
                .iter()
                .map(|&(a, hidden)| {
                    let n = self.bi(&db.ability_name, a, v);
                    if hidden { format!("{n}（{}）", l(v, "隐藏特性", "隱藏特性")) } else { n }
                })
                .collect();
            rows.push((l(v, "特性 Abilities", "特性 Abilities").into(), json!(list.join(" / "))));
        }
        if let Some(p) = pk {
            rows.push((
                l(v, "身高/体重", "身高/體重").into(),
                json!(format!("{:.1} m · {:.1} kg", p.height as f64 / 10.0, p.weight as f64 / 10.0)),
            ));
        }
        if let Some(legs) = self.legacy.get(&sid) {
            let s: Vec<String> = legs
                .iter()
                .map(|lg| {
                    let regions: Vec<String> = lg
                        .uses
                        .iter()
                        .map(|(r, src)| {
                            let src = short_sources(src);
                            if src.is_empty() { region_label(v, r).to_string() } else { format!("{}：{src}", region_label(v, r)) }
                        })
                        .collect();
                    format!("{}（{}）", lg.name, regions.join("；"))
                })
                .collect();
            rows.push((l(v, "旧译名", "舊譯名").into(), json!(s.join("、"))));
        }
        out.push(facts(rows));

        // stats
        if let Some(st) = db.poke_stats.get(&pid) {
            let mut heads: Vec<String> = (1..=6).map(|i| self.zh(&db.stat_name, i, v)).collect();
            heads.push(l(v, "总计", "總計").into());
            let mut vals: Vec<String> = st.iter().map(|x| x.to_string()).collect();
            vals.push(st.iter().sum::<u32>().to_string());
            out.push(grid(heads, vals, "stats"));
        }

        if let Some(e) = self.evo_tree(sid, v) {
            out.push(e);
        }

        // Pokédex text
        let rank = |ver: u32| -> i64 {
            let id = db.version_ident.get(&ver).map(String::as_str).unwrap_or("");
            DEX_ORDER.iter().position(|x| *x == id).map(|p| 100 - p as i64).unwrap_or(0)
        };
        let rows_f = db.species_flavor.get(&sid);
        let (zh, en) = self.flavor(rows_f, v, &rank);
        let vlabel = |ver: u32, lang: u32| get(&db.version_name, ver, lang).unwrap_or("").to_string();
        if let Some((ver, t)) = &zh {
            out.push(div("dex-zh", vec![span("ver", format!("{} ", vlabel(*ver, v.lang()))), zh_text(v, t)]));
        }
        if let Some((ver, t)) = &en {
            out.push(div("dex-en", vec![span("ver", format!("{} ", vlabel(*ver, EN))), en_text(t)]));
        }

        // forms
        let varieties: Vec<&Pokemon> = db.pokemon.iter().filter(|p| p.species == sid && !p.is_default).collect();
        if !varieties.is_empty() {
            let base_types = db.poke_types.get(&pid);
            let items: Vec<Value> = varieties
                .iter()
                .map(|p| {
                    let fid = db.default_form.get(&p.id).copied().unwrap_or(0);
                    let zh = get(&db.form_name, fid, v.lang())
                        .or_else(|| get(&db.form_name, fid, v.other()))
                        .map(str::to_string);
                    let en = get(&db.form_pokemon_name, fid, EN).or_else(|| get(&db.form_name, fid, EN)).unwrap_or("");
                    let label = match zh {
                        Some(z) => format!("{z} {en}"),
                        None => en.to_string(),
                    };
                    // name (and types when they differ from the default form) on hover
                    let t = db.poke_types.get(&p.id);
                    let title = if t != base_types {
                        let types: Vec<String> =
                            t.into_iter().flatten().map(|&ty| self.bi(&db.type_name, ty, v)).collect();
                        format!("{label}（{}）", types.join(" / "))
                    } else {
                        label
                    };
                    match self.poke_img.get(&p.id) {
                        Some(i) => {
                            let mut n = img(i, 3.5, "form-art");
                            n["title"] = json!(title);
                            n
                        }
                        None => json!({"tag": "span", "title": title, "data": {"pk": "form-noart"}, "content": "？"}),
                    }
                })
                .collect();
            out.push(div("forms-title", format!("{} Forms ({})", l(v, "形态", "形態"), varieties.len())));
            out.push(div("forms", items));
        }

        // more
        let mut more: Vec<(String, Value)> = Vec::new();
        if let Some(eg) = db.egg_groups.get(&sid) {
            let s: Vec<String> = eg.iter().map(|&g| self.bi(&db.egg_group_name, g, v)).collect();
            more.push((l(v, "蛋群 Egg groups", "蛋群 Egg groups").into(), json!(s.join(" / "))));
        }
        let gender = if sp.gender_rate < 0 {
            l(v, "无性别 Genderless", "無性別 Genderless").to_string()
        } else {
            let f = sp.gender_rate as f64 * 12.5;
            format!("♂ {}% ♀ {}%", 100.0 - f, f)
        };
        more.push((l(v, "性别 Gender", "性別 Gender").into(), json!(gender)));
        more.push((l(v, "捕获率 Catch rate", "捕獲率 Catch rate").into(), json!(sp.capture_rate.to_string())));
        more.push((l(v, "孵化周期 Egg cycles", "孵化週期 Egg cycles").into(), json!(sp.hatch_counter.to_string())));
        let mut body = vec![facts(more)];
        // other dex texts (deduplicated, grouped by version)
        if let Some(rows) = rows_f {
            let mut seen: Vec<(String, Vec<String>)> = Vec::new();
            let mut sorted: Vec<&(u32, u32, String)> = rows.iter().filter(|r| r.1 == v.lang()).collect();
            sorted.sort_by_key(|r| -rank(r.0));
            for r in sorted {
                let t = clean_zh(&r.2);
                if zh.as_ref().is_some_and(|z| z.1 == t) {
                    continue;
                }
                let name = vlabel(r.0, v.lang());
                match seen.iter_mut().find(|s| s.0 == t) {
                    Some(s) => s.1.push(name),
                    None => seen.push((t, vec![name])),
                }
            }
            for (t, vers) in seen {
                body.push(div("dex-zh", vec![span("ver", format!("{} ", vers.join("/"))), zh_text(v, &t)]));
            }
        }
        out.push(details(l(v, "更多 More", "更多 More"), body));
        Value::Array(out)
    }

    fn species_tags(&self, sid: u32) -> String {
        let sp = self.db.species.iter().find(|s| s.id == sid).unwrap();
        let mut t = vec!["宝可梦".to_string(), format!("第{}世代", sp.generation)];
        if sp.legendary {
            t.push("传说".into());
        }
        if sp.mythical {
            t.push("幻之".into());
        }
        if sp.baby {
            t.push("宝宝".into());
        }
        t.join(" ")
    }
}

// ---- moves / abilities / items / types / natures ------------------------------------------------

impl Ctx<'_> {
    fn move_content(&self, m: &Move, v: V) -> Value {
        let db = self.db;
        let mut out = vec![self.names_line(&db.move_name, m.id, v, false)];
        out.push(div("types", vec![self.type_badge(m.type_id, v), self.class_badge(m.class, v)]));
        let dash = |x: Option<u32>| x.map(|n| n.to_string()).unwrap_or_else(|| "—".into());
        let mut heads = vec![l(v, "威力", "威力").to_string(), l(v, "命中", "命中").into(), "PP".into()];
        let mut vals = vec![dash(m.power), dash(m.accuracy), dash(m.pp)];
        if m.priority != 0 {
            heads.push(l(v, "优先度", "優先度").into());
            vals.push(format!("{:+}", m.priority));
        }
        out.push(grid(heads, vals, "move-stats"));
        let rank = self.vg_rank();
        let (zh, en) = self.flavor(db.move_flavor.get(&m.id), v, &rank);
        if let Some((_, t)) = zh {
            out.push(zh_text(v, &t));
        }
        if let Some((_, t)) = en {
            out.push(en_text(&t));
        }
        if let Some(s) = db.effect_short.get(&m.effect) {
            out.push(div("effect", self.prose(s, m.effect_chance)));
        }
        if let Some(s) = db.effect_long.get(&m.effect) {
            let long = self.prose(s, m.effect_chance);
            if Some(&long) != db.effect_short.get(&m.effect).map(|x| self.prose(x, m.effect_chance)).as_ref() {
                out.push(details(l(v, "效果详情 Effect", "效果詳情 Effect"), vec![div("effect", long)]));
            }
        }
        Value::Array(out)
    }

    fn ability_content(&self, id: u32, v: V) -> Value {
        let db = self.db;
        let mut out = vec![self.names_line(&db.ability_name, id, v, false)];
        let rank = self.vg_rank();
        let (zh, en) = self.flavor(db.ability_flavor.get(&id), v, &rank);
        if let Some((_, t)) = zh {
            out.push(zh_text(v, &t));
        }
        if let Some((_, t)) = en {
            out.push(en_text(&t));
        }
        if let Some(s) = db.ability_short.get(&id) {
            out.push(div("effect", self.prose(s, None)));
        }
        let mut body = Vec::new();
        if let Some(s) = db.ability_long.get(&id) {
            body.push(div("effect", self.prose(s, None)));
        }
        // Pokémon with this ability
        let mut seen = HashSet::new();
        let mut list: Vec<(i64, String)> = Vec::new();
        let has = |pid: u32| db.poke_abilities.get(&pid).and_then(|ab| ab.iter().find(|a| a.0 == id)).map(|a| a.1);
        for p in &db.pokemon {
            let Some(hidden) = has(p.id) else { continue };
            if !p.is_default && self.default_pokemon(p.species).is_some_and(|d| has(d.id).is_some()) {
                continue; // the species' default form already lists it
            }
            let mut n = self.zh(&db.species_name, p.species, v);
            if !p.is_default {
                let fid = db.default_form.get(&p.id).copied().unwrap_or(0);
                if let Some(f) = get(&db.form_name, fid, v.lang()).or_else(|| get(&db.form_name, fid, EN)) {
                    n = format!("{n}（{f}）");
                }
            }
            if hidden {
                n.push('*');
            }
            if seen.insert(n.clone()) {
                let order = db.species.iter().find(|s| s.id == p.species).map(|s| s.order).unwrap_or(0);
                list.push((order * 100_000 + p.id as i64, n));
            }
        }
        list.sort();
        if !list.is_empty() {
            let names: Vec<String> = list.into_iter().map(|x| x.1).collect();
            body.push(div(
                "users",
                format!("{}（*{}）：{}", l(v, "拥有此特性的宝可梦", "擁有此特性的寶可夢"), l(v, "隐藏特性", "隱藏特性"), names.join("、")),
            ));
        }
        if !body.is_empty() {
            out.push(details(l(v, "更多 More", "更多 More"), body));
        }
        Value::Array(out)
    }

    fn item_content(&self, it: &Item, v: V) -> Value {
        let db = self.db;
        let mut head = Vec::new();
        if let Some(i) = self.item_img.get(&it.ident) {
            head.push(img(i, 2.5, "item-art"));
        }
        let mut text = vec![self.names_line(&db.item_name, it.id, v, false)];
        let mut meta = Vec::new();
        if let Some(c) = get(&db.category_name, it.category, v.lang()).or_else(|| get(&db.category_name, it.category, EN)) {
            meta.push(span("category", c.to_string()));
        }
        if it.cost > 0 {
            meta.push(span("price", format!("₽{}", it.cost)));
        }
        if !meta.is_empty() {
            text.push(div("meta", meta));
        }
        head.push(div("head-text", text));
        let mut out = vec![div("head", head)];
        let rank = self.vg_rank();
        let (zh, en) = self.flavor(db.item_flavor.get(&it.id), v, &rank);
        if let Some((_, t)) = zh {
            out.push(zh_text(v, &t));
        }
        if let Some((_, t)) = en {
            out.push(en_text(&t));
        }
        if let Some(s) = db.item_short.get(&it.id) {
            out.push(div("effect", self.prose(s, None)));
        }
        if let Some(s) = db.item_long.get(&it.id) {
            let long = self.prose(s, None);
            if Some(&long) != db.item_short.get(&it.id).map(|x| self.prose(x, None)).as_ref() {
                out.push(details(l(v, "效果详情 Effect", "效果詳情 Effect"), vec![div("effect", long)]));
            }
        }
        Value::Array(out)
    }

    fn type_content(&self, t: u32, v: V) -> Value {
        let db = self.db;
        let mut out = vec![div("types", vec![self.type_badge(t, v)]), self.names_line(&db.type_name, t, v, false)];
        let list = |pairs: Vec<u32>| -> Value {
            if pairs.is_empty() {
                return json!("—");
            }
            Value::Array(pairs.into_iter().map(|x| self.type_badge(x, v)).collect())
        };
        let by = |atk: bool, f: u32| -> Vec<u32> {
            let mut r: Vec<u32> = db
                .efficacy
                .iter()
                .filter(|((a, d), x)| **x == f && if atk { *a == t } else { *d == t })
                .map(|((a, d), _)| if atk { *d } else { *a })
                .collect();
            r.sort();
            r
        };
        if db.efficacy.keys().any(|(a, _)| *a == t) {
            out.push(facts(vec![
                (l(v, "攻击 ×2", "攻擊 ×2").into(), list(by(true, 200))),
                (l(v, "攻击 ×½", "攻擊 ×½").into(), list(by(true, 50))),
                (l(v, "攻击 ×0", "攻擊 ×0").into(), list(by(true, 0))),
                (l(v, "防御 ×2", "防禦 ×2").into(), list(by(false, 200))),
                (l(v, "防御 ×½", "防禦 ×½").into(), list(by(false, 50))),
                (l(v, "防御 ×0", "防禦 ×0").into(), list(by(false, 0))),
            ]));
        }
        Value::Array(out)
    }

    fn nature_content(&self, n: &Nature, v: V) -> Value {
        let db = self.db;
        let mut out = vec![self.names_line(&db.nature_name, n.id, v, false)];
        let stat = |s: u32| self.bi(&db.stat_name, s, v);
        let fl = |f: u32| self.bi(&db.flavor_name, f, v);
        let rows = if n.inc == n.dec {
            vec![(l(v, "能力", "能力").to_string(), json!(l(v, "无变化 Neutral", "無變化 Neutral")))]
        } else {
            vec![
                (l(v, "能力", "能力").into(), json!(format!("↑ {}  ↓ {}", stat(n.inc), stat(n.dec)))),
                (l(v, "口味", "口味").into(), json!(format!("{} {}  {} {}", l(v, "喜欢", "喜歡"), fl(n.likes), l(v, "讨厌", "討厭"), fl(n.hates)))),
            ]
        };
        out.push(facts(rows));
        Value::Array(out)
    }
}

// ---- headword emission ------------------------------------------------------------------------

impl Ctx<'_> {
    /// Emits the Simplified headword and, when different, the Traditional one (with its own
    /// content), plus half-width variants of either.
    #[allow(clippy::too_many_arguments)]
    fn emit(&self, d: &mut Dict, cat: &str, m: &LangText, id: u32, tags: &str, score: i64, seq: i64, content: &dyn Fn(V) -> Value) -> bool {
        let Some(hans) = get(m, id, HANS) else { return false };
        let hant = get(m, id, HANT).unwrap_or(hans);
        let reading = self.py.reading(cat, hans, Some(hant));
        let mut terms: Vec<(String, V)> = vec![(hans.to_string(), V::S)];
        if hant != hans {
            terms.push((hant.to_string(), V::T));
        }
        let mut seen = HashSet::new();
        for (term, v) in terms {
            let c = content(v);
            for t in variants(&term) {
                if seen.insert(t.clone()) {
                    d.term(&t, &reading, tags, score, c.clone(), seq);
                }
            }
        }
        true
    }

    pub fn build(&self, d: &mut Dict) -> HashMap<&'static str, usize> {
        let db = self.db;
        let mut counts: HashMap<&'static str, usize> = HashMap::new();

        for s in &db.species {
            let tags = self.species_tags(s.id);
            if self.emit(d, "species", &db.species_name, s.id, &tags, 50, 100_000 + s.id as i64, &|v| self.species_content(s.id, v, None)) {
                *counts.entry("species").or_default() += 1;
            }
            // legacy names
            let Some(legs) = self.legacy.get(&s.id) else { continue };
            let official_s = get(&db.species_name, s.id, HANS).unwrap_or("");
            let official_t = get(&db.species_name, s.id, HANT).unwrap_or(official_s);
            let mut done: HashSet<String> = [official_s.to_string(), official_t.to_string()].into();
            for lg in legs {
                let trad_region = lg.uses.iter().any(|(r, _)| r != "cn");
                let (simp, trad) = if trad_region {
                    (self.py.to_simp(&lg.name), lg.name.clone())
                } else {
                    (lg.name.clone(), self.py.to_trad(&lg.name))
                };
                let reading = self.py.reading("legacy", &simp, Some(&trad));
                let mut forms = vec![(lg.name.clone(), if trad_region { V::T } else { V::S })];
                forms.push((if trad_region { simp.clone() } else { trad.clone() }, if trad_region { V::S } else { V::T }));
                for (term, v) in forms {
                    for t in variants(&term) {
                        if !done.insert(t.clone()) {
                            continue;
                        }
                        let uses: Vec<String> = lg
                            .uses
                            .iter()
                            .map(|(r, src)| {
                                let src = short_sources(src);
                                if src.is_empty() { region_label(v, r).to_string() } else { format!("{}：{src}", region_label(v, r)) }
                            })
                            .collect();
                        let banner = div(
                            "banner",
                            format!(
                                "{}（{}）→ {} {}",
                                l(v, "旧译名", "舊譯名"),
                                uses.join("；"),
                                l(v, "现名", "現名"),
                                self.bi(&db.species_name, s.id, v)
                            ),
                        );
                        let tags = format!("{} 旧译", self.species_tags(s.id));
                        d.term(&t, &reading, &tags, 45, self.species_content(s.id, v, Some(banner)), 100_000 + s.id as i64);
                        *counts.entry("legacy").or_default() += 1;
                    }
                }
            }
        }
        for m in &db.moves {
            let tags = format!("招式 第{}世代", m.generation);
            if self.emit(d, "move", &db.move_name, m.id, &tags, 40, 200_000 + m.id as i64, &|v| self.move_content(m, v)) {
                *counts.entry("moves").or_default() += 1;
            }
        }
        for &(a, generation) in &db.abilities {
            let tags = format!("特性 第{generation}世代");
            if self.emit(d, "ability", &db.ability_name, a, &tags, 35, 300_000 + a as i64, &|v| self.ability_content(a, v)) {
                *counts.entry("abilities").or_default() += 1;
            }
        }
        for it in &db.items {
            if self.emit(d, "item", &db.item_name, it.id, "道具", 30, 400_000 + it.id as i64, &|v| self.item_content(it, v)) {
                *counts.entry("items").or_default() += 1;
            }
        }
        for (t, _) in &db.types {
            if self.emit(d, "type", &db.type_name, *t, "属性", 20, 500_000 + *t as i64, &|v| self.type_content(*t, v)) {
                *counts.entry("types").or_default() += 1;
            }
        }
        for n in &db.natures {
            if self.emit(d, "nature", &db.nature_name, n.id, "性格", 20, 600_000 + n.id as i64, &|v| self.nature_content(n, v)) {
                *counts.entry("natures").or_default() += 1;
            }
        }
        counts
    }
}

pub fn tags(d: &mut Dict) {
    d.tag("宝可梦", "partOfSpeech", 0, "Pokémon species");
    d.tag("招式", "partOfSpeech", 0, "Move");
    d.tag("特性", "partOfSpeech", 0, "Ability");
    d.tag("道具", "partOfSpeech", 0, "Item");
    d.tag("属性", "partOfSpeech", 0, "Type");
    d.tag("性格", "partOfSpeech", 0, "Nature");
    d.tag("旧译", "archaism", 1, "Legacy or regional name (source: 52poke)");
    d.tag("传说", "name", 2, "Legendary Pokémon");
    d.tag("幻之", "name", 2, "Mythical Pokémon");
    d.tag("宝宝", "name", 2, "Baby Pokémon");
    for g in 1..=9 {
        d.tag(&format!("第{g}世代"), "", 3, &format!("Introduced in Generation {g}"));
    }
}
