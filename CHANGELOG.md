# Changelog

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
