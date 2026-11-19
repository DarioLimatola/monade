# Dataset cards (format version 1)

Every set of data the Monade learns from has a **card**: one `card.json` in the
folder of the data. Data lives outside the repository (default `D:\monade-data` on
Windows, `C:\monade-data` without a D: drive, `~/monade-data` elsewhere, or the folder in `MONADE_DATA`). Cards are
written and checked by the tools:

```
uv run --project tools monade-tools card new --name hyg-v4.1 --source "HYG Database" \
    --url https://github.com/astronexus/HYG-Database --licence "CC BY-SA 4.0" \
    --split train --languages "" --dir D:\monade-data\stars\hyg
uv run --project tools monade-tools card check D:\monade-data\stars\hyg\card.json
```

| Field | Meaning |
|---|---|
| `card_version` | 1 |
| `name` | short name with a version, e.g. `hyg-v4.1` |
| `description` | what the set contains |
| `source` | who publishes it |
| `url` | where it was taken from |
| `licence` | its licence, as written by the source (never empty) |
| `non_commercial` | true if the licence forbids commercial use |
| `languages` | language codes (`it`, `en`, `de`), empty for non-linguistic data |
| `retrieved` | date of download, `YYYY-MM-DD` |
| `split` | `train`, `validation` or `test` |
| `files` | every file: `path` (relative, `/`), `bytes`, `blake3` (64 hex digits) |
| `notes` | anything else (changes made, known biases) |

Rules:

- **Test sets are sealed:** their card and hashes are written before any training
  run; the data is kept by the author and opened once per version. The gate of the
  mind (step 6b) refuses to learn from inputs marked `test`.
- A file whose size or hash differs from its card is refused (`card check`).
- Fields are written in the order above, so a card has one canonical form.
