# CONTEXT — for every helper who works on Monade

Read this first. Update it at the end of every session (section "Sessions").
Helpers change; this file stays.

## Guiding principle (the Demiurge's)

**Simplicity, elegance and recursion.** Little code, clear, repeating the same at
every scale; no superfluous abstraction; the same rule at every level.

In this code: a Monade of any size is `8^k` words, and the same word operations
(XOR, AND, popcount, Hamming, majority) work on all of them. Point 0 and the
specifications are the same thing — a readable append-only log — read and written
by the same `record` module.

## Who decides

- **The Demiurge** (the owner) designs and decides. Only he writes point 0 and the
  specifications, through `monade-run`.
- The mind may propose, never write foundations.
- The design lives in the claude.ai project "Tesseract": `claude/tesseract-structure-v5.md`
  is the source of truth; the council is in `claude/agents/consiglio.md`.
  Anything not confirmed there is labelled **Proposed**.

## Language

Code, identifiers, comments, documents: English. Conversation with the Demiurge:
Italian, brief and essential; instructions for Windows.

## Rules

1. Every kernel has a slow, obvious reference in `oracle` (tests only); the fast
   one is property-tested against it (`proptest`).
2. No `unsafe` (forbidden by lint). No floating point in `monade-core`: no
   floating-point type even in comments (checked by `cargo all`).
3. `monade-core` has no dependencies (only `proptest` for tests).
4. Errors are values; library code never panics on bad input.
5. Every public item has documentation and a runnable example.
6. Small modules, one job each; tests next to the code.
7. Measure before optimising: no fast path without a before/after number.
8. Before every commit: `cargo all` (fmt, clippy `-D warnings`, no-float, tests in
   debug and release). CI runs the same command on Windows and Linux.

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
    src/oracle.rs     slow references + state_hash() (tests / feature `oracle`)
  monade-run/         CLI, the only writer of point 0 and specs
  xtask/              `cargo all`: every check
fuzz/                 cargo-fuzz target for the log readers (Linux, nightly)
prereg/ ladder/ properties/ worlds/ benches/ docs/   empty for now
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
4. Space: generated vectors from (seed, address) with a cryptographic-quality hash.
5. Ternary unit with homeostasis; rungs 0–4.
6. Test 8 (determinism).
7. Impulse with budget and metrology (test 6).
8. Learning cycle with two levels (test 2).
9. Composition and capacity (test 3).
10. Sleep and quiet (test 5).
11. Calibration (test 4).
12. Atom (test 1).
13. Peirce (test 7).
14. Adapters, then monitor.

## Commands (Windows: PowerShell or cmd, same commands)

```
cargo test                                   all tests
cargo all                                    every check, as CI
cargo run -p monade-run -- show              point 0 and specifications
cargo run -p monade-run -- append-foundation --name N --text T --motive M
cargo run -p monade-run -- set-spec KEY VALUE --motive M
```

## Sessions

- **2026-11-18 · draft 1.** Steps 1–3 written. 122 tests (unit, property, doc)
  green on Linux with `cargo all`; Windows to be confirmed by CI or the Demiurge.
  Next: step 4 (generated space): choose the hash (cryptographic quality, no
  dependency or a pinned one, decided by the Demiurge) and its version 1.
