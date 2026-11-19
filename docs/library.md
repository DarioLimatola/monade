# The library (written by the crawler)

The crawler (`monade-tools crawl run`, or the button Crawl in Specchio) reads the seed
authors of `tools/seeds.toml` and writes everything under `<data folder>/biblioteca/`
(the data folder is `MONADE_DATA`, default `D:\monade-data` or `C:\monade-data`).

## Sources (v0)

| Source | What | How |
|---|---|---|
| Wikidata | the author's item, death year, influences (P737), works (P50, P800) | `wbsearchentities`, SPARQL |
| Wikisource it / de / en | the texts: a work's page and its chapters (subpages) | MediaWiki API `action=parse`, `list=allpages` |

Requests are serial, with a pause of 1 second per host, a User-Agent with a contact
(`MONADE_CONTACT` to set yours), `maxlag=5`, and retries on 429/503. Every answer is
cached, so a run can stop and the next one continues. Other sites are fetched only if
their robots.txt allows it. Project Gutenberg forbids automated access to its website
(only its mirrors and its harvest endpoint are allowed): not used in v0.

## The gate

- An author is taken only if dead for more than 70 years (public domain in the EU from
  1 January after 70 full years); otherwise refused, with the reason, in `refused.jsonl`.
- Only Wikisource pages in the author's own languages (`lang` in the seeds): no
  translations.
- At most `--max-works` works per author (default 8) and `--max-pages` chapters per
  work (default 200); a time budget (`--minutes`, default 30).

## Files

| File | One line per |
|---|---|
| `texts/<lang>/<lang>-<work>.jsonl` | paragraph: `page`, `i`, `text` (Unicode NFC; verses keep their line breaks) |
| `authors.jsonl` | author taken: `qid`, `label`, `died`, `lang`, `search` |
| `works.jsonl` | work taken: `work_id`, `work_qid`, `author_qid`, `title`, `lang`, `url`, `pages`, `paragraphs`, `chars`, `blake3`, `licence`, `path` |
| `edges.jsonl` | edge of the reference graph: `from`, `rel` (`influenced_by`, `wrote`), `to`, labels |
| `refused.jsonl` | refusal: `what`, `why` (recorded once) |
| `lessons.jsonl` | work queued for the mind (read by the gate, step 6b) |
| `seeds.lock.json` | which Wikidata item each seed became: check it; pin a wrong one with `qid` in the seeds |
| `card.json` | the dataset card of the texts and tables (`docs/data-cards.md`) |
| `crawl.log` | what happened, readable |
| `cache/` | raw answers (not part of the card) |

JSON Lines rather than Parquet for v0: readable, one dependency less, and read in Rust
with `serde_json`, already used. Parquet comes when the data grows (tables of stars).
