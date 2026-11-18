# CONTEXT — for every helper who works on Monade

Read this first. Update it at the end of every session (section "Sessions").
Helpers change; this file stays.

## Guiding principle (the author's)

**Simplicity, elegance and recursion.** Little code, clear, repeating the same at
every scale; no superfluous abstraction; the same rule at every level.

In this code: a Monade of any size is `8^k` words, and the same word operations
(XOR, AND, popcount, Hamming, majority) work on all of them. Point 0 and the
specifications are the same thing — a readable append-only log — read and written
by the same `record` module.

## Who decides

- **The author** (the owner) designs and decides. Only he writes point 0 and the
  specifications, through `monade-run`.
- The mind may propose, never write foundations.
- The design lives in the claude.ai project "Tesseract": `claude/tesseract-structure-v5.md`
  together with `claude/monade-parliament.md` (part 2) is the source of truth; a copy
  of part 2 is in `docs/design/`.
  Anything not confirmed there is labelled **Proposed**.
- Wording: where a conventional technical term exists, use it (e.g. **seed**).

## Language

Code, identifiers, comments, documents: English. Conversation with the author:
Italian, brief and essential; instructions for Windows.

## Rules

1. Every kernel has a slow, obvious reference in `oracle` (tests only); the fast
   one is property-tested against it (`proptest`).
2. No `unsafe` (forbidden by lint). No floating point in `monade-core`: no
   floating-point type even in comments (checked by `cargo all`).
3. Reuse well-maintained open-source libraries where possible and efficient (a
   criterion of good programming). `monade-core` keeps few, vetted ones (today only the
   official `blake3`). `monade-run` uses `serde` and `serde_json`.
4. Errors are values; library code never panics on bad input.
5. Every public item has documentation and a runnable example.
6. Small modules, one job each; tests next to the code.
7. Measure before optimising: no fast path without a before/after number.
8. Before every commit: `cargo all` (fmt, clippy `-D warnings`, no-float, tests in
   debug and release). CI runs the same command on Windows and Linux.
9. This repository holds only the engine and its technical design.
10. Rust version pinned in `rust-toolchain.toml` (1.97.0): rustup uses it in this folder
    automatically. Raise it on purpose, in its own commit, after `cargo all` passes.

## Structure

```
crates/
  monade-core/        no dependencies, no floating point
    src/word.rs       Word (u64; views: bits, bytes, Ref), Block (8 words, 64-byte
                      aligned), XOR, AND, popcount, Hamming, majority on any length
    src/record.rs     readable log format shared by origin and specs
    src/origin.rs     point 0: append-only log of foundations, EMPTY for now
    src/specs.rs      specifications: versioned history, locked seed
    src/size.rs       sizes 64 bits → 2 MB (powers of 8), arenas, Ref::ORIGIN
    src/space.rs      generated space (GEN): vectors from id/name + seed, BLAKE3 v1
    src/address.rs    octal address: root + base-8 path (down, up, offset, Morton
                      point, packed 56 bits), Arena::part
    src/oracle.rs     slow references + state_hash() (tests / feature `oracle`)
  monade-run/         CLI, the only writer of point 0 and specs
    src/check.rs      `check`: 9 checks that say in words and numbers what they verified
    src/runs.rs       run folders: runs/YYYYMMDD-HHMMSS/{run.json, results.jsonl}
    src/report.rs     `report`: one self-contained HTML page (history + latest run)
    src/stars.rs      first world: stars of a catalog in four 2 MB Monadi + 3D map
  xtask/              `cargo all`: every check
fuzz/                 cargo-fuzz target for the log readers (Linux, nightly)
runs/                 results of `check` (one folder per run, never overwritten)
docs/                 design copied from the project (structure, part 2)
prereg/ ladder/ properties/ worlds/ benches/   empty for now
.github/workflows/ci.yml   Windows + Linux; nightly fuzzing
```

## Decisions embodied in the code

- **Point 0 as coordinate:** `Word::ZERO`, `Block::ZERO`, and `Ref::ORIGIN` (index 0
  of every arena) hold the zero Monade. v ⊕ 0 = v. Storing the zero Monade is an
  error (`SizeError::Zero`): self-binding gives point 0, and point 0 holds no data.
- **Point 0 as object:** empty. Mechanism only: append-only, ids from 1 never
  reused, no delete or modify function; a change is a new entry that amends an
  old one (chains are linear: an entry can be amended once). Fields: id, name,
  text, version, date, motive, state (active/suspended), amends.
- **Specifications:** a history, never rewritten. Genesis has two **locked**
  entries: `space.seed = 0x4d4f4e4144450001` (ASCII "MONADE" + 0x0001) and
  `space.hash_version = 1`. Nothing can change them: a new seed is a new mind.
  Reading the file is its check (genesis prefix + replay).
- **Files:** `foundations/origin.txt` and `foundations/specs.txt` (folder set by
  `--dir`). A missing file means the start. A hand edit is refused (not canonical).
  A write is refused unless the new text begins with the old one.
- **Sizes:** level 0 word (64 b), 1 block (512 b), 2 symbol (4096 b), 3, 4, 5,
  6 thought (2 MB). Defined recursively (one size = 8 of the one below); any other
  length is `Err`, never a panic.
- **Generated space, hash version 1 (locked):** key = BLAKE3 `derive_key("monade space
  v1", seed as 8 bytes LE)`; vector of an id = BLAKE3 keyed output of `[0] + id (8
  bytes LE)`, of a name = output of `[1] + UTF-8 name`; read as little-endian words;
  block k = output bytes 64k..64k+64 (random access); smaller sizes are prefixes.
  Golden fingerprints in `space.rs` tests: if they change, it is another mind.
- **Octal address, packed 56 bits:** `Address` = root (< 2²⁸, which arena) + up to 8
  base-8 digits (digit 0 = topmost split). A Monade of `8^e` bits (`e = level + 2`)
  is a cube of side `2^e`; each digit is an octant (bit 0 → x, 1 → y, 2 → z), so the
  path is the Morton order. Packed: digits bits 0–23 (top digit highest, unused 0),
  depth 24–27, root 28–55, 56–63 zero; the top size is not packed. `Arena::part`
  returns whole words only (a byte or a bit is an error for now).
- **Majority:** a bit is set when more than half of the inputs are set; a tie gives 0.
- **state_hash:** FNV-1a 64 over little-endian bytes; for comparing runs, not
  cryptographic. (The generated space, step 4, needs a cryptographic-quality hash.)
- Arenas store words contiguously (`Vec<Word>`, 8-byte aligned). 64-byte alignment
  of arena storage is deferred to the fast paths, with measurements.

## Order of writing

1. ✅ Word + oracle + CI + XOR and Hamming invariants.
2. ✅ Point 0 mechanism (empty) and specifications with the locked seed + log
   invariants + fuzzing of the reader.
3. ✅ Sizes, arenas, fast = slow.
3b. ✅ `monade-run check` (readable results) + run folders + `monade-run report` (HTML).
3c. ✅ Octal address (`address.rs`), with its check.
4. ✅ Space: generated vectors from seed + id (Logos `GEN`); hash version 1 = BLAKE3
   (official crate), keyed from the seed, context "monade space v1".
5. Counters per bit: add, read by majority, certainty, residue (test 19).
6. Blackboard: Monade of 8^4 bits, voices bound to keys, consensus, residue (test 19).
7. Ternary unit with homeostasis; rungs 0–4.
8. Test 8 (determinism, across machines).
9. Impulse with budget and metrology (test 6).
10. Learning cycle with two levels (test 2).
11. Composition and capacity, with PERM and MATCH (test 3).
12. Logos v1: 64-bit instruction, interpreter of 8 primitives, budget, depth ≤ 8,
    128-bit content hash, text ↔ binary form.
13. Layers: XOR overlay virtualisation (depth ≤ 8, apply or discard); scars in a
    Bloom filter.
14. Sleep and quiet (test 5): consolidation, subsumption (test 18), compaction,
    compiling upward.
15. Calibration (4), atom (1), Peirce (7), Swadesh (20), communication (21); then
    adapters and monitor (multi-dimensional map with forces, test 17).

## Commands (Windows: PowerShell or cmd, same commands)

```
cargo test                                   all tests
cargo all                                    every check, as CI
cargo run -p monade-run -- show              point 0 and specifications
cargo run -p monade-run -- check             verify; results saved in runs/
cargo run -p monade-run -- report            report.html from every run
cargo run --release -p monade-run -- stars --catalog hygdata_v41.csv   stars.html
cargo run -p monade-run -- append-foundation --name N --text T --motive M
cargo run -p monade-run -- set-spec KEY VALUE --motive M
```

## Sessions

- **2026-11-18 · draft 1.** Steps 1–3 written. 122 tests (unit, property, doc)
  green on Linux with `cargo all`; Windows to be confirmed.
  Next: step 4 (generated space): choose the hash (cryptographic quality, no
  dependency or a pinned one) and its version 1.
- **2026-11-18 · v0.2.0.** Design of council session 9 recorded in `docs/design/`
  (experts as Monadi, blackboard, residue, Logos, mechanisms, hash). Step 3b
  written: `check`,
  run folders, `report`. Golden state hashes fixed in `check.rs`: the same numbers
  must appear on every machine: confirmed on Linux and on Windows (x86_64, AVX2,
  20 threads), all 9 checks passed. The alias is `cargo all`
  (older notes said `cargo ci`).
- **2026-11-18 · v0.3.0.** Octal address (step 3c): `Address`, packed 56 bits,
  Morton = path; slow references and property tests; 10th check `address.octal`.
  Three levels decided (core in bits, integer GPU accelerator, continuous level
  entering only by the sign bridge). Next: step 4 (generated space, BLAKE3).
- **2026-11-18 · v0.3.1.** Detailed report (why, how, numbers, histograms, glossary).
  v0.3.0 confirmed on Windows: 10/10, golden hashes unchanged.
- **2026-11-18 · v0.4.0.** Step 4: the generated space (BLAKE3 v1) and two checks,
  `space.generated` and `space.sentence` ("the dog sleeps" answered role by role,
  6/6, margin ≥ 105 bits). Next: step 5, counters per bit (the first learning).
- **2026-11-18 · v0.4.1.** Rust pinned to 1.97.0 (`rust-toolchain.toml`), after a new
  clippy rule of Rust 1.100 failed on one machine only. v0.4.0 confirmed on Windows:
  12/12, generated-space fingerprint identical to Linux.
- **2026-11-18 · v0.4.2.** First world: the stars (HYG catalog, 119 626 stars) in four
  2 MB Monadi (cubes of 32, 128, 512, 2048 pc; cells of 0.125 to 8 pc), with layers,
  and a 3D map read back from the Monadi. No learning; uses the octal address only.
