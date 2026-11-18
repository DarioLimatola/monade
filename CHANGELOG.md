# Changelog

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
