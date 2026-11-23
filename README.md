# pokeyomi: PokéAPI → Chinese Yomitan dictionary

Builds `dist/pokemon-zh.zip`, a single Yomitan (format 3) dictionary named **宝可梦 (PokéAPI)**. Design notes are in [DESIGN.md](DESIGN.md).

```powershell
cargo build --release
.\target\release\pokeyomi.exe fetch   # pinned PokeAPI CSVs + sprites, CC-CEDICT, Unihan, Yomitan schemas, 52poke pages → cache/
.\target\release\pokeyomi.exe build   # → dist/pokemon-zh.zip (+ build/pinyin_review.tsv)
```

Run both commands from the project root. `fetch` skips files already in `cache/`. `build` runs offline and takes a few seconds.

## Releasing (and how Yomitan finds updates)

Run the **release** workflow: Actions → release → Run workflow, or `gh workflow run release.yml`. It builds on Ubuntu with `POKEYOMI_REPO` set, so the dictionary's `index.json` gets:

- `isUpdatable: true`
- `indexUrl`: `https://github.com/caocaochan/pokemon-yomitan/releases/latest/download/index.json`
- `downloadUrl`: `https://github.com/caocaochan/pokemon-yomitan/releases/latest/download/pokemon-zh.zip`
- `revision`: the UTC build time as `YYYY.M.D.HHMM`

The workflow publishes `pokemon-zh.zip` and `index.json` as the latest release, tagged `v<revision>`. When the user clicks *Check for updates*, Yomitan fetches `indexUrl` and compares revisions with `compareRevisions`. Dot-separated integers with the same number of parts compare numerically, so every later build counts as newer. Keep the title and the asset names unchanged, or installed copies stop matching.

Local builds without `POKEYOMI_REPO` leave out the update fields. To pick up newer source data, bump `POKEAPI_SHA` / `SPRITES_SHA` in `src/fetch.rs`, then run the workflow.

## Contents (PokeAPI `de6760b`, sprites `c0542f7`)

| Category | Entries | Notes |
|---|---|---|
| Species | 1025 | artwork (256 px WebP), stats, abilities, evolution tree, Pokédex text zh + en, forms, egg groups |
| Legacy/regional names | 494 headword rows (351 names, 229 species) | from 52poke; banner shows region and publisher |
| Moves | 920 | type, class, power/accuracy/PP, zh + en flavor, English effect |
| Abilities | 314 | zh + en flavor, English effect, Pokémon with the ability |
| Items | 2174 | sprite, category, price, zh + en flavor, English effect |
| Types / natures | 21 / 25 | type matchups; nature stat and flavor changes |

Every entry has a Simplified headword. It also has a Traditional headword when the two differ, and that headword shows Traditional text. Half-width and middle-dot variants are also included (`属性:空`, `卡璞·鸣鸣`, `卡璞鸣鸣`). Readings are tone-marked pinyin.

## Using it in Yomitan

1. Settings → Dictionaries → **Import from URL** → `https://github.com/caocaochan/pokemon-yomitan/releases/latest/download/pokemon-zh.zip`, or download the zip from [Releases](https://github.com/caocaochan/pokemon-yomitan/releases/latest). The profile language must be Chinese.
2. Give it a **lower priority than your general dictionary** (e.g. CC-CEDICT). About 210 move names and 100 ability names are ordinary words (冲浪, 加速, 火), so they always show a Pokémon entry as well.

## Pinyin

Readings are generated as follows:
1. CC-CEDICT phrase segmentation.
2. A small table of corrected single-character defaults (`src/pinyin.rs`, `CHAR_DEFAULTS`).
3. Unihan `kMandarin`.
4. Hand fixes in `data/pinyin_overrides.tsv`.

Species readings are reused when a species name appears inside an item or move name. Each build writes `build/pinyin_review.tsv`, which lists every name where a character with several readings was resolved by default. To correct a reading, add `term<TAB>reading` to the overrides file and rebuild.

## Licence

PokeAPI code is BSD-3-Clause. Pokémon names, text and artwork © Nintendo / Creatures / GAME FREAK / The Pokémon Company. Legacy names come from 52poke (CC BY-NC-SA 3.0). CC-CEDICT is CC BY-SA 4.0. The resulting dictionary is for personal, non-commercial use.
