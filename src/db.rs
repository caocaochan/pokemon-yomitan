//! Loads the PokeAPI tables the renderers need.

use crate::table::*;
use std::collections::HashMap;
use std::path::Path;

pub struct Species {
    pub id: u32,
    pub generation: u32,
    pub evolves_from: Option<u32>,
    pub chain: u32,
    pub legendary: bool,
    pub mythical: bool,
    pub baby: bool,
    pub gender_rate: i64,
    pub capture_rate: u32,
    pub hatch_counter: u32,
    pub order: i64,
}

pub struct Pokemon {
    pub id: u32,
    pub species: u32,
    pub height: u32,
    pub weight: u32,
    pub is_default: bool,
}

pub struct Move {
    pub id: u32,
    pub generation: u32,
    pub type_id: u32,
    pub power: Option<u32>,
    pub pp: Option<u32>,
    pub accuracy: Option<u32>,
    pub priority: i64,
    pub class: u32,
    pub effect: u32,
    pub effect_chance: Option<u32>,
}

pub struct Item {
    pub id: u32,
    pub ident: String,
    pub category: u32,
    pub cost: u32,
}

pub struct Nature {
    pub id: u32,
    pub dec: u32,
    pub inc: u32,
    pub hates: u32,
    pub likes: u32,
}

pub struct Evo {
    pub trigger: u32,
    pub item: Option<u32>,
    pub min_level: Option<u32>,
    pub gender: Option<u32>,
    pub location: Option<u32>,
    pub held_item: Option<u32>,
    pub time_of_day: String,
    pub known_move: Option<u32>,
    pub known_move_type: Option<u32>,
    pub happiness: bool,
    pub beauty: bool,
    pub affection: bool,
    pub phys: Option<i64>,
    pub party_species: Option<u32>,
    pub party_type: Option<u32>,
    pub trade_species: Option<u32>,
    pub rain: bool,
    pub upside_down: bool,
    pub used_move: Option<u32>,
    pub move_count: Option<u32>,
    pub steps: Option<u32>,
    pub damage: Option<u32>,
}

/// (key, language, text) — key is a version id (species) or version group id.
pub type Flavor = HashMap<u32, Vec<(u32, u32, String)>>;

pub struct Db {
    pub species: Vec<Species>,
    pub species_name: LangText,
    pub genus: LangText,
    pub species_flavor: Flavor,
    pub version_ident: HashMap<u32, String>,
    pub version_name: LangText,
    pub vg_order: HashMap<u32, i64>,
    pub pokemon: Vec<Pokemon>,
    pub default_form: HashMap<u32, u32>,
    pub form_name: LangText,
    pub form_pokemon_name: LangText,
    pub poke_types: HashMap<u32, Vec<u32>>,
    pub poke_abilities: HashMap<u32, Vec<(u32, bool)>>,
    pub poke_stats: HashMap<u32, [u32; 6]>,
    pub stat_name: LangText,
    pub egg_groups: HashMap<u32, Vec<u32>>,
    pub egg_group_name: LangText,
    pub evo: HashMap<u32, Vec<Evo>>,
    pub evo_trigger_name: LangText,
    pub location_name: LangText,
    pub moves: Vec<Move>,
    pub move_name: LangText,
    pub move_flavor: Flavor,
    pub effect_short: HashMap<u32, String>,
    pub effect_long: HashMap<u32, String>,
    pub class_name: LangText,
    pub abilities: Vec<(u32, u32)>, // (id, generation)
    pub ability_name: LangText,
    pub ability_flavor: Flavor,
    pub ability_short: HashMap<u32, String>,
    pub ability_long: HashMap<u32, String>,
    pub items: Vec<Item>,
    pub item_name: LangText,
    pub item_flavor: Flavor,
    pub item_short: HashMap<u32, String>,
    pub item_long: HashMap<u32, String>,
    pub category_name: LangText,
    pub types: Vec<(u32, String)>,
    pub type_ident: HashMap<u32, String>,
    pub type_name: LangText,
    pub efficacy: HashMap<(u32, u32), u32>,
    pub natures: Vec<Nature>,
    pub nature_name: LangText,
    pub flavor_name: LangText,
}

fn flavor(t: &Table, id: &str, key: &str) -> Flavor {
    let mut m: Flavor = HashMap::new();
    for r in t.iter() {
        m.entry(r.u(id)).or_default().push((r.u(key), r.u("language_id"), r.s("flavor_text").to_string()));
    }
    m
}

fn en_prose(t: &Table, id: &str) -> (HashMap<u32, String>, HashMap<u32, String>) {
    let (mut s, mut l) = (HashMap::new(), HashMap::new());
    for r in t.iter().filter(|r| r.u("local_language_id") == EN) {
        if !r.s("short_effect").is_empty() {
            s.insert(r.u(id), r.s("short_effect").to_string());
        }
        if !r.s("effect").is_empty() {
            l.insert(r.u(id), r.s("effect").to_string());
        }
    }
    (s, l)
}

impl Db {
    pub fn load(dir: &Path) -> Db {
        let t = |n: &str| Table::load(dir, n);
        let flag = |r: &Row, c: &str| r.s(c) == "1";

        let species = t("pokemon_species")
            .iter()
            .map(|r| Species {
                id: r.u("id"),
                generation: r.u("generation_id"),
                evolves_from: r.opt("evolves_from_species_id"),
                chain: r.u("evolution_chain_id"),
                legendary: flag(&r, "is_legendary"),
                mythical: flag(&r, "is_mythical"),
                baby: flag(&r, "is_baby"),
                gender_rate: r.i("gender_rate").unwrap_or(-1),
                capture_rate: r.u("capture_rate"),
                hatch_counter: r.u("hatch_counter"),
                order: r.i("order").unwrap_or(0),
            })
            .collect();
        let sn = t("pokemon_species_names");
        let pokemon = t("pokemon")
            .iter()
            .map(|r| Pokemon {
                id: r.u("id"),
                species: r.u("species_id"),
                height: r.u("height"),
                weight: r.u("weight"),
                is_default: flag(&r, "is_default"),
            })
            .collect();
        let mut default_form = HashMap::new();
        for r in t("pokemon_forms").iter() {
            if flag(&r, "is_default") {
                default_form.insert(r.u("pokemon_id"), r.u("id"));
            }
        }
        let fnames = t("pokemon_form_names");
        let mut poke_types: HashMap<u32, Vec<(u32, u32)>> = HashMap::new();
        for r in t("pokemon_types").iter() {
            poke_types.entry(r.u("pokemon_id")).or_default().push((r.u("slot"), r.u("type_id")));
        }
        let poke_types = poke_types
            .into_iter()
            .map(|(k, mut v)| {
                v.sort();
                (k, v.into_iter().map(|x| x.1).collect())
            })
            .collect();
        let mut pa: HashMap<u32, Vec<(u32, u32, bool)>> = HashMap::new();
        for r in t("pokemon_abilities").iter() {
            pa.entry(r.u("pokemon_id")).or_default().push((r.u("slot"), r.u("ability_id"), flag(&r, "is_hidden")));
        }
        let poke_abilities = pa
            .into_iter()
            .map(|(k, mut v)| {
                v.sort();
                (k, v.into_iter().map(|x| (x.1, x.2)).collect())
            })
            .collect();
        let mut poke_stats: HashMap<u32, [u32; 6]> = HashMap::new();
        for r in t("pokemon_stats").iter() {
            let s = r.u("stat_id");
            if (1..=6).contains(&s) {
                poke_stats.entry(r.u("pokemon_id")).or_default()[s as usize - 1] = r.u("base_stat");
            }
        }
        let mut egg_groups: HashMap<u32, Vec<u32>> = HashMap::new();
        for r in t("pokemon_egg_groups").iter() {
            egg_groups.entry(r.u("species_id")).or_default().push(r.u("egg_group_id"));
        }
        let mut evo: HashMap<u32, Vec<(bool, Evo)>> = HashMap::new();
        for r in t("pokemon_evolution").iter() {
            let nz = |c: &str| r.opt(c).filter(|&x| x > 0);
            evo.entry(r.u("evolved_species_id")).or_default().push((
                flag(&r, "is_default"),
                Evo {
                    trigger: r.u("evolution_trigger_id"),
                    item: nz("trigger_item_id"),
                    min_level: nz("minimum_level"),
                    gender: nz("gender_id"),
                    location: nz("location_id"),
                    held_item: nz("held_item_id"),
                    time_of_day: r.s("time_of_day").to_string(),
                    known_move: nz("known_move_id"),
                    known_move_type: nz("known_move_type_id"),
                    happiness: nz("minimum_happiness").is_some(),
                    beauty: nz("minimum_beauty").is_some(),
                    affection: nz("minimum_affection").is_some(),
                    phys: r.i("relative_physical_stats"),
                    party_species: nz("party_species_id"),
                    party_type: nz("party_type_id"),
                    trade_species: nz("trade_species_id"),
                    rain: flag(&r, "needs_overworld_rain"),
                    upside_down: flag(&r, "turn_upside_down"),
                    used_move: nz("used_move_id"),
                    move_count: nz("minimum_move_count"),
                    steps: nz("minimum_steps"),
                    damage: nz("minimum_damage_taken"),
                },
            ));
        }
        // keep default rows when a species has any, otherwise all rows
        let evo = evo
            .into_iter()
            .map(|(k, v)| {
                let any_default = v.iter().any(|x| x.0);
                (k, v.into_iter().filter(|x| x.0 || !any_default).map(|x| x.1).collect())
            })
            .collect();

        let moves = t("moves")
            .iter()
            .filter(|r| r.u("id") < 10000)
            .map(|r| Move {
                id: r.u("id"),
                generation: r.u("generation_id"),
                type_id: r.u("type_id"),
                power: r.opt("power"),
                pp: r.opt("pp"),
                accuracy: r.opt("accuracy"),
                priority: r.i("priority").unwrap_or(0),
                class: r.u("damage_class_id"),
                effect: r.u("effect_id"),
                effect_chance: r.opt("effect_chance"),
            })
            .collect();
        let (effect_short, effect_long) = en_prose(&t("move_effect_prose"), "move_effect_id");
        let (ability_short, ability_long) = en_prose(&t("ability_prose"), "ability_id");
        let (item_short, item_long) = en_prose(&t("item_prose"), "item_id");
        let items = t("items")
            .iter()
            .map(|r| Item { id: r.u("id"), ident: r.s("identifier").to_string(), category: r.u("category_id"), cost: r.u("cost") })
            .collect();
        let types: Vec<(u32, String)> =
            t("types").iter().map(|r| (r.u("id"), r.s("identifier").to_string())).collect();
        let type_ident = types.iter().cloned().collect();
        let efficacy = t("type_efficacy")
            .iter()
            .map(|r| ((r.u("damage_type_id"), r.u("target_type_id")), r.u("damage_factor")))
            .collect();
        let natures = t("natures")
            .iter()
            .map(|r| Nature {
                id: r.u("id"),
                dec: r.u("decreased_stat_id"),
                inc: r.u("increased_stat_id"),
                hates: r.u("hates_flavor_id"),
                likes: r.u("likes_flavor_id"),
            })
            .collect();
        let versions = t("versions");
        Db {
            species,
            species_name: lang_text(&sn, "pokemon_species_id", "local_language_id", "name"),
            genus: lang_text(&sn, "pokemon_species_id", "local_language_id", "genus"),
            species_flavor: flavor(&t("pokemon_species_flavor_text"), "species_id", "version_id"),
            version_ident: versions.iter().map(|r| (r.u("id"), r.s("identifier").to_string())).collect(),
            version_name: lang_text(&t("version_names"), "version_id", "local_language_id", "name"),
            vg_order: t("version_groups").iter().map(|r| (r.u("id"), r.i("order").unwrap_or(0))).collect(),
            pokemon,
            default_form,
            form_name: lang_text(&fnames, "pokemon_form_id", "local_language_id", "form_name"),
            form_pokemon_name: lang_text(&fnames, "pokemon_form_id", "local_language_id", "pokemon_name"),
            poke_types,
            poke_abilities,
            poke_stats,
            stat_name: lang_text(&t("stat_names"), "stat_id", "local_language_id", "name"),
            egg_groups,
            egg_group_name: lang_text(&t("egg_group_prose"), "egg_group_id", "local_language_id", "name"),
            evo,
            evo_trigger_name: lang_text(&t("evolution_trigger_prose"), "evolution_trigger_id", "local_language_id", "name"),
            location_name: lang_text(&t("location_names"), "location_id", "local_language_id", "name"),
            moves,
            move_name: lang_text(&t("move_names"), "move_id", "local_language_id", "name"),
            move_flavor: flavor(&t("move_flavor_text"), "move_id", "version_group_id"),
            effect_short,
            effect_long,
            class_name: lang_text(&t("move_damage_class_prose"), "move_damage_class_id", "local_language_id", "name"),
            abilities: t("abilities")
                .iter()
                .filter(|r| r.s("is_main_series") == "1")
                .map(|r| (r.u("id"), r.u("generation_id")))
                .collect(),
            ability_name: lang_text(&t("ability_names"), "ability_id", "local_language_id", "name"),
            ability_flavor: flavor(&t("ability_flavor_text"), "ability_id", "version_group_id"),
            ability_short,
            ability_long,
            items,
            item_name: lang_text(&t("item_names"), "item_id", "local_language_id", "name"),
            item_flavor: flavor(&t("item_flavor_text"), "item_id", "version_group_id"),
            item_short,
            item_long,
            category_name: lang_text(&t("item_category_prose"), "item_category_id", "local_language_id", "name"),
            types,
            type_ident,
            type_name: lang_text(&t("type_names"), "type_id", "local_language_id", "name"),
            efficacy,
            natures,
            nature_name: lang_text(&t("nature_names"), "nature_id", "local_language_id", "name"),
            flavor_name: lang_text(&t("contest_type_names"), "contest_type_id", "local_language_id", "flavor"),
        }
    }
}
