# Changelog

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
