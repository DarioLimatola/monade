# Changelog

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
