# PokéAPI → Chinese Yomitan dictionaries: design

Status: v1 implemented (see README.md). Stats, egg groups and damage classes are shown inside entries, not as headwords. §4 changed to a single dictionary and §4a added legacy names (user decisions 2026-11-23).
Checked on 2026-11-23 against Yomitan 26.11.10.0 (master), PokeAPI data `de6760be38b6` and sprites `c0542f7c5121`.

## 1. Goals

- Look up an official Chinese Pokémon term (species, move, ability, item, type, nature) in Yomitan and get a compact bilingual entry: Chinese first, English beside it, with an image where one exists.
- Support both Simplified and Traditional text. Yomitan does not convert between the two, so both forms are separate headwords.
- Make the build reproducible and offline: pin the source data commits, cache downloads, rebuild without touching the network.

## 2. Findings that shape the design

All of these were measured on the pinned data.

| Finding | Evidence | Consequence |
|---|---|---|
| Chinese **names** are nearly complete | zh-Hans/zh-Hant: species 1025/1025, moves 938/938, abilities 314/374 (the other 60 are Colosseum/XD-only), items 2174/2221, types 21/21, natures 25/25 | Names are the headwords. |
| Chinese **descriptions** exist only as in-game flavor text | Species flavor text 1020/1025 (Sun/Moon → Legends Z-A; 494, 648, 649, 720, 721 have none). Move 915/915, ability 307/314, item 2175/2221. **Prose effects (`*_prose`) are English only (0 Chinese rows).** | Chinese text = flavor text. Mechanics text = English prose. |
| Location names are mostly missing in Chinese | 89/1097 | Locations are out of scope. |
| No form-specific Chinese Pokémon names | `pokemon_form_names.pokemon_name` is empty for every zh row; only `form_name` (e.g. 阿罗拉的样子) exists | Forms are shown inside the species entry. They get no headwords of their own. |
| Traditional and Simplified often differ | Names that differ: species 671, moves 684, abilities 234, items 1494 | A second headword is needed whenever the two forms differ. |
| **Many move/ability/item names are ordinary words** | Exact CC-CEDICT headword matches: moves **210/938** (撞击, 叫声, 冲浪, 打雷…), abilities **103/314** (加速, 柔软, 免疫…), items 59, species 12 (梦幻, 雷公, 拉普拉斯…). Type names are single common characters (火, 水, 电). | The data ships as **separate dictionaries**, so the noisy ones can be lowered in priority or turned off (§4). |
| Yomitan's Chinese support | `language/zh/chinese.js`: `normalizePinyin` applies NFC, lowercases, and strips spaces and `・:'’-` before comparing readings. The `zh` descriptor has no Simplified↔Traditional preprocessor. | The reading field holds tone-marked pinyin, space-separated for display. |
| PokeAPI has no pinyin, and no public source was found | Bulbapedia's list sits behind Cloudflare (403). 52poke's list has 中/日/英 names but no pinyin. | Pinyin is generated, then corrected by hand (§6). |
| Yomitan structured content can do what we need | Supported tags: `img` (with `width`/`height`/`sizeUnits`/`imageRendering`/`collapsible`), `table`, `details`/`summary`, `ruby`, `a`, `data-*`. A `styles.css` file in the zip is applied. Accepted image types include webp/png/gif/avif. | Custom CSS styles the entries. WebP is used for artwork. |
| Image size | Official artwork is 475×475 PNG, 118–203 KB. Lanczos resize to 256 px + WebP q80 with alpha (yuva420p) gives **11.3–16.1 KB** (6 samples). 1351 Pokémon rows (1025 default + 326 varieties) come to **≈18 MB**. Item sprites are 30×30 PNG, ~0.3 KB each. | 256 px WebP for Pokémon, original PNG for items. |

## 3. Data sources

- **PokeAPI CSV dump** (`PokeAPI/pokeapi/data/v2/csv`, pinned commit). Not the REST API: it would take ~5k requests, and PokeAPI's fair-use policy asks for local caching. The CSVs are the same data the API serves. Language ids: `9`=en, `12`=zh-Hans, `4`=zh-Hant, `11`=ja.
- **PokeAPI sprites** (`PokeAPI/sprites`, pinned commit), downloaded file by file from `raw.githubusercontent.com` into `cache/`, not a full clone (the repo is several GB):
  - `sprites/pokemon/other/official-artwork/{pokemon_id}.png`, falling back to `other/home/{id}.png`, then `sprites/pokemon/{id}.png`
  - `sprites/items/{identifier}.png`
- **CC-CEDICT**, used only to generate pinyin. It is not shipped.

Tables used: `pokemon_species(_names|_flavor_text)`, `pokemon`, `pokemon_types`, `pokemon_abilities`, `pokemon_stats`, `pokemon_forms`, `pokemon_form_names`, `evolution_chains`/`pokemon_evolution`, `pokemon_egg_groups`, `egg_group_prose`, `moves`, `move_names`, `move_flavor_text`, `move_effect_prose`, `move_damage_class_prose`, `abilities`, `ability_names`, `ability_flavor_text`, `ability_prose`, `items`, `item_names`, `item_flavor_text`, `item_prose`, `item_category_prose`, `types`, `type_names`, `natures`, `nature_names`, `stat_names`, `version_names`, `version_groups`.

## 4. One dictionary

**Decision (user, 2026-11-23): everything ships as a single zip, `pokemon-zh.zip`, titled 宝可梦 (PokéAPI).**

| Category | Headwords (Hans + differing Hant) | Images |
|---|---|---|
| Species (+ legacy/regional names, §4a) | ≈1696 + legacy | artwork, ≈18 MB |
| Moves | ≈1622 | none |
| Abilities | ≈548 | none |
| Items | ≈3668 | sprites, <1 MB |
| Types, natures, stats, egg groups, damage classes | small | none |

Accepted trade-off: about 1 move name in 4 is an everyday word (冲浪, 加速, and type names like 火). Lookups of those words will always show a Pokémon entry too. Yomitan only lets you rank or disable whole dictionaries, so the dictionary's priority applies to all categories at once. Two things keep it readable:
- The category tag (宝可梦 / 招式 / 特性 / 道具 / 属性 / 性格) is the first chip on every entry.
- Scores rank species > moves > abilities > items > misc when several categories share a headword (e.g. a species and a move both named 梦幻), via `score`.

## 4a. Legacy and regional names

Source: 52poke (神奇宝贝百科). Each species page has a `{{名字/entry|<region>|<name>|tt=<publishers>|<etymology>|…}}` table. Regions are `cn`, `tw`, `hk`, plus `zh_nin` (the official unified name, already covered by PokeAPI). Examples:
- 妙蛙种子: hk 奇異種子 (群英、曼迪), hk 奇異種籽 (羚邦)
- 皮卡丘: hk 比卡超 (羚邦、無綫…)
- 喵喵: tw 喵貓, hk 喵喵怪
- 波克比: cn 顽皮, hk 小刺蛋, 當吉蛋

Fetching: MediaWiki API, `action=query&prop=revisions&rvprop=content&rvslots=main&redirects=1`, 50 titles per request (the zh-Hans official names), about 21 requests with a 1 s delay. Cached in `cache/52poke/`.

Each legacy name that differs from the official Hans and Hant names becomes an extra headword for that species: same `sequence`, `score` −1, tag `旧译`. The entry gets a banner line, e.g. 旧译名 · 香港（羚邦、無綫…）→ 现名 皮卡丘 Pikachu. Its Simplified/Traditional counterpart, made by character-level conversion with a table derived from CC-CEDICT trad/simp pairs, is added as a headword too when it differs. Names that are not CJK (e.g. hk "Pikachu") are skipped.

Licence: 52poke text is CC BY-NC-SA 3.0. It is credited in `attribution`, and this makes the dictionary non-commercial.

## 5. Term entry format (term_bank_v3)

`[term, reading, definitionTags, rules, score, definitions, sequence, termTags]`

- **term**: the zh-Hans name. If the zh-Hant name differs, a second row with the zh-Hant term, the **same `sequence`**, and definitions rendered from the zh-Hant text (a Traditional headword shows Traditional flavor text).
- **reading**: tone-marked pinyin, one space between syllables (`pí kǎ qiū`). Full-width digits and punctuation in names (`１０％`, `：`, `・`) are kept in the term. An extra ASCII-normalized headword is added when a name contains them.
- **definitionTags**: category plus attributes, e.g. `species gen1 electric`. **termTags**: empty.
- **rules**: `""` (nothing inflects).
- **score**: 0 for all entries. Ranking across dictionaries comes from Yomitan's dictionary priority. Within the species dictionary, default forms rank above alias headwords (score −1).
- **sequence**: `category_base + id` (species 100000+id, moves 200000+id, …). This keeps Yomitan's "merged" display grouping Hans and Hant together.

Tag bank: `species/招式/特性/道具`, `gen1…gen9` (第1世代…), 18 type tags (`fire`: 火 Fire, colored via CSS), `legendary` 传说, `mythical` 幻之, `baby` 宝宝, `physical/special/status` (物理/特殊/变化), and `hidden` 隐藏特性.

### Example: species entry (structured content, abridged)

```json
["皮卡丘", "pí kǎ qiū", "species gen1 electric", "", 0, [{
  "type": "structured-content",
  "content": [
    {"tag": "div", "data": {"pk": "head"}, "content": [
      {"tag": "img", "path": "img/pokemon/25.webp", "width": 7, "height": 7, "sizeUnits": "em", "collapsible": false, "appearance": "auto", "background": false},
      {"tag": "div", "data": {"pk": "names"}, "content": [
        {"tag": "span", "data": {"pk": "dex"}, "content": "#0025"},
        {"tag": "span", "lang": "en", "content": "Pikachu"},
        {"tag": "span", "lang": "ja", "data": {"pk": "ja"}, "content": "ピカチュウ"},
        {"tag": "div", "content": "鼠宝可梦 · Mouse Pokémon"},
        {"tag": "div", "content": [{"tag": "span", "data": {"type": "electric"}, "content": "电 Electric"}]}
      ]}
    ]},
    {"tag": "table", "data": {"pk": "facts"}, "content": "…特性 Abilities / 身高 Height / 体重 Weight…"},
    {"tag": "table", "data": {"pk": "stats"}, "content": "…HP 35 · 攻击 55 · 防御 40 · 特攻 50 · 特防 50 · 速度 90 · 总计 320…"},
    {"tag": "div", "data": {"pk": "evo"}, "content": "皮丘 Pichu → 皮卡丘 Pikachu（雷之石 Thunder Stone）→ 雷丘 Raichu"},
    {"tag": "div", "data": {"pk": "dex-zh"}, "lang": "zh-CN", "content": "皮卡丘们把尾巴贴在一起交换电流，其实是在互相打招呼。"},
    {"tag": "div", "data": {"pk": "dex-en"}, "lang": "en", "content": "…same-version English entry…"},
    {"tag": "details", "content": [{"tag": "summary", "content": "形态 Forms (16)"}, "…small 3em images + 形态名…"]},
    {"tag": "details", "content": [{"tag": "summary", "content": "更多 More"}, "…egg groups, gender ratio, catch rate, hatch steps, other dex versions…"]}
  ]
}], 100025, ""]
```

Popup layout, top to bottom:

1. **Header**: artwork (7em) beside dex number, then the names: Chinese (Hans/Hant), English, and Japanese (kana + romaji); then genus zh · en and type badges zh+en. Moves, abilities and items show zh/en/ja names the same way.
2. **Facts table**: abilities (zh + en, hidden ability marked), height in m, weight in kg.
3. **Base stats**: a single row with zh labels and total.
4. **Evolution chain**: one line with zh+en names and the trigger (level, item, friendship…), built from `pokemon_evolution` using zh names for items, moves and locations. A trigger whose location has no zh name falls back to English.
5. **Pokédex text**: the newest version that has both zh and en text, chosen in the order SV → SwSh → BDSP → USUM → SM → LGPE, with Legends games last because they cover few species. zh line first, en line second.
6. **Collapsed `details`**: forms (artwork + `form_name` in zh/en), then other stats and the other dex versions (duplicate texts removed).

Moves: zh/en/ja name; type badge, damage class, power/accuracy/PP/priority; zh flavor text (newest version group) + en flavor text; **English effect prose** (`$effect_chance` substituted, `[text]{mechanic:x}` markup removed); a collapsed list of the Pokémon that learn it is **left out** (it would make entries large for little benefit).
Abilities: zh/en/ja name; zh + en flavor text; English prose; collapsed list of Pokémon with the ability (zh names).
Items: 30px sprite shown at 2em with `imageRendering: pixelated`; category zh/en; zh + en flavor text; English prose; price.
Misc: types get a 3-row type-effectiveness summary (super effective / not very / no effect, using zh names). Natures show +stat/−stat and liked/disliked flavor.

Text cleanup: join zh flavor-text lines with no separator; join en lines with a space; remove `\f`, soft hyphens and `­`; collapse whitespace; deduplicate identical zh texts across versions.

`styles.css`: type colors keyed on `[data-sc-type=fire]` etc., compact tables, a muted color for Japanese and English secondary text, and `prefers-color-scheme` handling so entries work in both Yomitan themes. Verified in source:
- A structured-content `data: {"pk": "head"}` becomes the attribute `data-sc-pk="head"` (`structured-content-generator.js` `_setElementDataset`).
- Yomitan scopes a dictionary's CSS by itself with `addScopeToCss(styles, '[data-dictionary="<title>"]')` (`display.js`), so the CSS file needs no scoping of its own.
- The importer **rejects an empty `styles.css`** ("No styles found"). Every dictionary that includes the file must put at least one rule in it.

## 6. Pinyin generation

PokeAPI has no pinyin and Pokémon names are coined, so a naive per-character lookup will get some readings wrong. 342/1025 species names contain a character with more than one reading.

Pipeline:

1. **Phrase pass**: forward maximum matching of the name against CC-CEDICT Simplified headwords. Each matched word takes its CEDICT reading, so 种子 → zhǒng zi and 火龙 → huǒ lóng.
2. **Character fallback**: for characters left unmatched, use the most common reading (Unihan `kMandarin` first value). Characters with several readings are recorded.
3. **Overrides**: `data/pinyin_overrides.tsv` (`term<TAB>reading`, by hand). It always wins.
4. **Review report**: `build/pinyin_review.tsv` lists every name where step 2 picked from several readings and no override exists. It is reviewed once, and the fixes are saved as overrides. Expect a few hundred candidates on the first run, most of them trivially right.

Convert to tone marks (CEDICT uses numbers). `u:` becomes `ü`. The neutral tone has no mark. Traditional headwords reuse the Simplified reading. Legacy names are matched against CC-CEDICT under both Simplified and Traditional headwords. Their reading is Mandarin even for Hong Kong names, because Yomitan's zh reading field is pinyin; Jyutping is out of scope.

Confidence: names whose syllables come from CC-CEDICT phrases are reliable. Fallback readings of coined names (e.g. 卡 kǎ vs qiǎ) depend on the override review. There is no canonical source to check them against, so some will stay a judgment call.

## 7. Implementation (Rust)

Rust fits because this is a reusable build tool, and image decode/resize/encode over ~1350 files is the heavy step.

```
PokemonYomitan/
  Cargo.toml
  src/
    main.rs          CLI: `fetch`, `build [--only species,moves,…] [--img-size 256] [--quality 80]`
    fetch.rs         pinned CSV + sprite + CEDICT download into cache/ (skips files already present, checks HTTP status)
    data.rs          typed CSV loaders (serde + csv), `Lang { En, Hans, Hant, Ja }` lookup maps
    pinyin.rs        §6 pipeline
    text.rs          flavor-text cleanup, prose markup stripping
    sc.rs            small builder for structured-content JSON nodes
    render/          species.rs, moves.rs, abilities.rs, items.rs, misc.rs
    images.rs        decode PNG → resize (Lanczos3) → lossy WebP with alpha
    yomitan.rs       index.json, tag_bank, term_bank chunking (10k rows/file), styles.css, zip writer
  data/
    styles.css
    pinyin_overrides.tsv
  cache/             (gitignored) CSVs, sprites, cedict
  dist/              output zips
```

Crates: `csv`, `serde`/`serde_json`, `ureq` (blocking HTTP, rustls), `zip` (deflate; images stored, not compressed again), `image` (PNG decode + resize), `webp` (libwebp-sys, built with MSVC via `cc`; lossy encoding with alpha, which the pure-Rust `image` encoder can't do), `rayon` for parallel image encoding, `jsonschema` to validate output against Yomitan's own schemas (vendored from `ext/data/schemas/`) before zipping.

Fallback if `libwebp-sys` causes build trouble: shell out to the `ffmpeg` already on PATH (`-vf scale=256:256:flags=lanczos -c:v libwebp -quality 80`). That path is verified to keep alpha.

`index.json`:

```json
{"title": "宝可梦 (PokéAPI)", "revision": "2026-11-23.de6760b", "format": 3,
 "sequenced": true, "author": "generated from PokéAPI",
 "url": "https://pokeapi.co/", "sourceLanguage": "zh", "targetLanguage": "zh",
 "description": "Pokémon species, moves, abilities, items, types and natures: zh-Hans/zh-Hant/legacy headwords, Chinese + English + Japanese names, official artwork.",
 "attribution": "Data: PokéAPI (BSD-3-Clause code; data from veekun). Names, text and artwork © Nintendo / Creatures / GAME FREAK / The Pokémon Company. Legacy/regional names: 52poke 神奇宝贝百科 (CC BY-NC-SA 3.0). Pinyin: generated with CC-CEDICT (CC BY-SA 4.0) and Unihan."}
```

## 8. Verification plan

1. Validate against the schema during the build: every term/tag bank is checked against Yomitan's v3 schemas.
2. Assertions on counts: 1025 species, 938 moves, and so on. Every `img.path` must exist in the zip, and no headword may be empty.
3. Import all five zips into a clean Yomitan profile set to Chinese. Check that import completes and record the IndexedDB size.
4. Look up a sample set covering the edge cases: 皮卡丘 / 妙蛙種子 (Traditional), 属性：空 (full-width colon), 多边兽Ⅱ/２型 variants, 冲浪 (move vs CEDICT word), 火 (type, misc dictionary), 喷火龙 (Mega/Gigantamax forms), 洛托姆 (many forms), 494 比克提尼 (no zh dex text, must fall back cleanly to en only), and an item with no sprite.
5. Look at the entries in light and dark themes, and on a narrow popup width.

## 9. Licensing

PokeAPI's code is BSD-3. The underlying names, text and sprites are copyrighted by The Pokémon Company and its partners. The zips are fine for personal use. A public release would be the user's call, and safer without the artwork. CC-CEDICT is CC BY-SA 4.0. Only derived readings are included, and they are attributed.

## 10. Out of scope for v1 (possible later)

- Learnsets per move or per Pokémon.
- Locations (89/1097 have zh names).
- Shiny artwork, animated Showdown GIFs.
- Cantonese (Jyutping) readings.
- Auto-update (`isUpdatable` + `indexUrl`) once there is a place to host the files.
