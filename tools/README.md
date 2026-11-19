# tools

The Python tools of the Monade. They live **outside the mind**: they download data,
write dataset cards, run baselines and analyses, and leave files (Parquet, JSON)
that the Rust program reads. Nothing here runs inside the mind.

Managed with [uv](https://docs.astral.sh/uv/): one command installs the right Python
and the packages, the same on every machine.

```
uv run --project tools monade-tools doctor           what the tools see (Python, data folder, disk, GPU)
uv run --project tools monade-tools card new ...     write card.json for a folder of data
uv run --project tools monade-tools card check F     check a card and its files
uv run --project tools monade-tools crawl run       the crawler: from the starting authors (<data>/biblioteca/seeds.toml) along their links
uv run --project tools monade-tools crawl status    what the library holds
uv run --project tools python -m unittest discover -s tools/tests
```

The crawler and its files: `docs/library.md`. In Specchio: the buttons Crawl (in the
background, at most 30 minutes) and Library.

The same through the Rust program: `cargo run -p monade-run -- tools doctor`.

Data lives outside the repository (default `D:\monade-data` on Windows, or `C:\monade-data`
without a D: drive; `~/monade-data` elsewhere; set `MONADE_DATA` to change it). The format of the cards
is in `docs/data-cards.md`.
