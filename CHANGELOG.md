# Changelog

## 0.6.5 — 2026-11-19

- The first learning test on the stars (step 5c, module `hr`): the HR relation by
  counting only (tallies of thermometer codes per colour bin), with a baseline, a
  control with shuffled colours, and the exact median as a reference. Pre-registered
  thresholds and results in `prereg/hr-relation.md`: FAIL against the threshold
  (learned 69 % and 62 % of the baseline, threshold 60 %), control PASS; the tallies
  reach the exact median, so the shortfall is colour alone, not the counters. The
  first run exposed the recency of saturated tallies (catalog order): teachers now come
  in a shuffled order.
- Two voices: `stars --second CAT` compares two catalogs star by star (Hipparcos
  number) and learns the HR relation from both; gzip catalogs are read directly (new
  dependency `flate2`, MIT or Apache-2.0). HYG v4.1 against HYGLike v3.2 (AT-HYG, Gaia
  DR3 distances): 107 729 stars in both; Gaia teaches better (1.08 against 1.17 mag).
- `stars` also writes the sky as a sparse Monade on disk (`stars.sky`).
- Specchio, room Map: a second view, **Tree of the sky**: the octal tree of all the
  Monadi of the sky, walked digit by digit from the root (16 777 pc) to the cells
  (0.008 pc): the 8 parts and the 64 parts below them with their stars, the dense
  Monade each region would be, and the stars themselves once few enough.
- Crawler: a library written by v0.6.0 now gets its links followed (`linked.jsonl`;
  edges recorded once); pages that are not texts are skipped (`Opera:`, `Autore:`, …);
  a work under 250 characters is an index or a list of versions, not a text; at most
  24 works per author by default.

## 0.6.4 — 2026-11-19

- Stars v2 (step 5c, first part): every star is one exact Morton code of 63 bits, an
  octal address of 21 digits in a cube of 16 777 pc around the Sun with cells of
  0.008 pc (largest error 0.006 pc on HYG v4.1, against 3.8 pc before). Only the stars
  are stored, sorted (8 bytes a star: 875 kB for 109 401 stars); a region is a prefix,
  so its stars are one contiguous run, and any region can become a dense 2 MB Monade
  (`Sky::dense`). The Sun is the corner where eight regions meet (`Sky::around_sun`).
  Stars at the catalog's unknown distance (100 000 pc) are left out.
- The map: any radius from 2 to 8 000 pc; colours from B-V (blackbody colours); size
  and light from the apparent or the absolute magnitude; no grid (exact places); turning
  at the same speed on every screen (by time, not by frame); the Hertzsprung–Russell
  diagram of the stars within the radius.

## 0.6.3 — 2026-11-19

- Specchio gains the room **Map** (it replaces the placeholder Memory): the inside of
  the mind in 3D. Sentences typed in the room (at most 8 of 8 words) become Monadi of
  512 bits: words and positional roles generated from their names, bindings (role XOR
  word), sentences (majority), memory (majority of the sentences). Lines are
  references: made of, answers (asking a sentence for a role gives the nearest word,
  with its distance), near (at most 211 bits of 512 apart; chance 256 ± 11.3). A click
  shows a Monade as a cube of 8 × 8 × 8 cells, one per bit, placed by its octal
  address. Nothing is learned yet: the room is ready to show what learning will add.
  New module `map` (JSON at `/api/map?s=...`), page `map.html`; rotation by time, the
  same on every screen.

## 0.6.2 — 2026-11-19

- Texts are stored compressed (Zstandard, `.jsonl.zst`, three to four times smaller)
  and indexed for search (`index.sqlite`, SQLite FTS5, no copy of the text): the new
  module `store`; `monade-tools library search "WORDS"` finds paragraphs;
  `library pack` compresses an older library and rebuilds the index.
- Wikisource from its dump (module `dump`): `monade-tools dump get --lang it|de|en|all`
  downloads the weekly CirrusSearch dump (one resumable download, instead of thousands
  of requests), `dump build` turns it into a local mirror; the crawler then reads the
  chapters from the mirror (Wikidata is still asked for authors, links and works). The
  dump has no paragraph breaks: pages are cut at sentence ends, and works say
  `"source": "dump"`.
- New Python dependency: `zstandard` (BSD-3).

## 0.6.1 — 2026-11-19

- The crawler follows links: each author taken adds to a persistent frontier
  (`frontier.jsonl`) the people linked to it on Wikidata (influenced by, and
  influenced), with their distance from the start; every run goes on from the
  frontier, nearest first, up to `--depth` (default 3), and never starts again from the
  starting authors. Authors reached by a link get their languages from Wikidata
  (P1412, P103) and pass the same gate (70 years; it, de or en).
- The starting authors are private: `<data>/biblioteca/seeds.toml`, created from
  `tools/seeds.example.toml` on the first run. Specchio and `crawl status` show counts
  only (`crawl status --names` on request).
- A ceiling on the texts on disk: `--max-gb`, default 64.

## 0.6.0 — 2026-11-19

- The crawler v0 (`tools/`, `monade-tools crawl run | status`): from the seed authors
  (`tools/seeds.toml`) to clean texts. Wikidata gives the author, the death year, the
  influences and the works; Wikisource (it, de, en) gives the texts (a work and its
  chapters). The gate takes only authors dead for more than 70 years and only texts in
  the author's own languages; refusals are recorded once with their reason. Polite:
  serial requests, a pause per host, a User-Agent with a contact, `maxlag`, retries,
  robots.txt for other sites; a cache on disk, so every run continues the last one;
  a lock against two crawls at once; a time budget. Output in JSON Lines under
  `<data>/biblioteca/` with a dataset card and a queue of lessons (`docs/library.md`).
- Tests of the crawler with recorded answers (no network): the licence rule, the
  extraction of text from Wikisource HTML, Wikidata parsing, a full crawl and its
  resumption.
- Specchio: buttons Crawl (starts the crawler in the background) and Library.
- `monade-run tools` shows the output of the tools while they work; Specchio collects it.
- Tools pinned to Python 3.13 (`tools/.python-version`), so every package has wheels.

## 0.5.2 — 2026-11-19

- Specchio gains the room **Commands**: buttons for Verify (check), Doctor, Foundations
  and Stars (with the catalog path), the output shown below. A fixed list of commands,
  accepted only as POST with the header `X-Monade: 1`, which other web pages cannot
  send: no site can start a command.
- `Specchio.cmd`: double click to build and open Specchio on Windows.
- `monade-run tools` returns the output of the tools (so Specchio can show it) and,
  when uv is missing, says only how to install it (no full usage text).
- Tools: the data folder defaults to `C:\monade-data` when there is no D: drive; the
  free disk is measured on the nearest existing folder.

## 0.5.1 — 2026-11-19

- Step 5b. Tally weights restricted to 1, 2, 4 or 8 (`tally::WEIGHTS`): more than 3
  bits of weight add no accuracy, and small weights leave room for at least 15 votes
  before a counter saturates. Property tests use only these weights.
- Golden hash of a fixed tally: unit test (`golden_tally`) and a third fingerprint in
  `determinism.golden`; `oracle::counts_hash`.
- New check `memory.capacity_noisy`: how many noisy voices a 512-bit tally holds
  (noise 0, 10%, 20%; 2 to 16 voices); separations predicted by an exact formula
  written before the first run; a control that must fail. 16 checks.
- `tools/`: Python tools run with uv, outside the mind: `monade-tools doctor` and
  dataset cards (`card new`, `card check`, BLAKE3 per file; format in
  `docs/data-cards.md`). `monade-run tools ...` runs them.
- Specchio, the interface (skeleton): `monade-run specchio` serves six rooms on
  127.0.0.1 only (Mind = the report, Worlds = the stars, Foundations; Lessons,
  Blackboard and Memory say which step brings them) and opens the browser.
- Report: glossary gains "weight" and "noisy voice".
- Docs: the design consolidated in `docs/design/structure-v6.md`; product and council
  documents removed from the repository. `runs/` no longer tracked by git.

## 0.5.0 — 2026-11-18

- Step 5, the first learning: counters per bit (`monade_core::tally`). One signed
  counter per bit (`i8`, symmetric, saturating at ±127, never wrapping): a vote adds
  `+w` on the 1-bits and `-w` on the 0-bits; the reading is the sign (a tie reads 0).
  `add`, `add_weighted` (weight 1 to 127), `subtract`, `read`, `uncertain`,
  `certainty_histogram`, `residue` (voice XOR reading), `merge` (two cores count apart),
  `decay` (forgetting: towards 0, never across), `voices`, `is_exact`.
- Slow references `oracle::tally`, `tally_read`, `tally_uncertain` (bit by bit, wide
  integers). Property tests: reading = majority, order-free, merge of halves = all,
  merge commutative, subtract inverts add, decay never flips a sign, saturation never
  wraps, fast = slow, wrong lengths are errors; a 2 MB tally reads its majority.
- `monade-run check`: `memory.reinforcement`, `memory.capacity`, `memory.residue`,
  thresholds written before the first run, each with a control that must fail. 15 checks.
- Report: charts of categories (series side by side, values printed); glossary and
  "what the Monade is today" updated.
## 0.4.2 — 2026-11-18

- First world, the stars: `monade-run stars --catalog FILE` reads an HYG-like CSV
  (positions read as thousandths of a parsec, no floating point) and puts the stars
  into four 2 MB Monadi, cubes of 256³ cells around the Sun (half-widths 16, 64, 256,
  1024 pc), one bit per cell, the cell found by its octal address. Six more layers
  per scale with the same addresses (naked eye; five colour classes).
- `stars.html`: a self-contained 3D map (rotate, zoom, scale, naked-eye filter,
  names of the brightest stars). The points are read back from the Monadi.
- Measures: stars inside, cells lit, stars sharing a cell, mean and largest distance
  to the cell centre, lit cells per octant.

## 0.4.1 — 2026-11-18

- Rust version pinned: `rust-toolchain.toml` (1.97.0, with clippy and rustfmt); CI uses it.
- `as_chunks` instead of `chunks_exact` (a clippy rule of newer Rust).
- v0.4.0 confirmed on Windows: the generated space has the same fingerprint as on Linux.

## 0.4.0 — 2026-11-18

- Step 4, the generated space (`monade_core::space`, Logos `GEN`): basic vectors
  computed from an id or a name and the seed, never stored. Hash version 1: BLAKE3
  keyed with a key derived from the seed (context "monade space v1"); input = domain
  byte (0 id, 1 name) + payload; extendable output read in 64-byte blocks, so any
  512-bit block is computed alone and a smaller size is the beginning of a larger one.
  `Space::from_specs`, `generate`, `named`, `block`, `part` (by octal address).
- Golden fingerprints of the genesis space fixed in the tests: changing them is a new mind.
- Slow reference `oracle::generate` (byte by byte from the start).
- Dependency: `blake3` (feature `pure`: Rust only), optimised even in debug builds.
- `monade-run check`: `space.generated` (fingerprint, distance of generated pairs,
  random access, speed) and `space.sentence` ("the dog sleeps" in one block, asked
  role by role). 12 checks.

## 0.3.1 — 2026-11-18

- Report in full detail: what the Monade is today; for every check its claim, why it
  matters, how it works, what was measured, what was expected, all the numbers, and
  histograms of the distributions (distance of random vectors; members vs decoys of a
  bundle of 3 and 8 voices); a glossary of the words used.
- Run files gain `why`, `how` and `charts` (older runs still load).

## 0.3.0 — 2026-11-18

- `monade_core::address`: the octal address. A root (which arena) and a path of
  base-8 digits, one per level; down = append a digit, up = drop it. The path is the
  Morton order of the cube (2 MB = 256³, one point per bit). Packed form: 56 bits
  (digits 0–23, depth 24–27, root 28–55). Offsets, points, packing, `Arena::part`.
- Slow references in `oracle` (`address_offset` by recursion, `morton_point` by
  de-interleaving); property tests on every size; never a panic.
- `monade-run check`: 10th check, `address.octal`.
- Design copy refreshed: octal address, the three levels (bits, integer GPU, continuous).
- Removed two documents that are not part of the technical design.

## 0.2.0 — 2026-11-18

- `monade-run check`: 9 checks that say, in words and numbers, what they verified
  (determinism with golden hashes, XOR laws, fast = slow, distance of random vectors,
  recall from a bundle of 3 and 8 voices, tie gives 0, sizes, arenas, foundations).
- Run folders `runs/YYYYMMDD-HHMMSS/` with `run.json` and `results.jsonl`; never overwritten.
- `monade-run report`: one self-contained HTML page (piano roll of every check across
  runs, the latest run in full). Opens offline with a double click.
- Design of council session 9 copied into `docs/design/`.
- Reuse of well-maintained open-source libraries: `serde`, `serde_json` in `monade-run`.
- Determinism confirmed on Linux and Windows: the same golden state hashes.

## 0.1.0 — 2026-11-18

- Steps 1–3: word and block, point 0 and specifications with the locked seed,
  sizes and arenas, oracle, `cargo all`, CI on Windows and Linux, fuzzing.
