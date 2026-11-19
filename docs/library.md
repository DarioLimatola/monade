# The library (written by the crawler)

The crawler (`monade-tools crawl run`, or the button Crawl in Specchio) reads the seed
authors of `<data folder>/biblioteca/seeds.toml` (private; created from
`tools/seeds.example.toml` on the first run), follows their links to other authors, and writes everything under `<data folder>/biblioteca/`
(the data folder is `MONADE_DATA`, default `D:\monade-data` or `C:\monade-data`).

## The walk along the links

The starting authors are only the beginning. For each author taken, the crawler
records its links on Wikidata (P737 "influenced by", and the other direction: whom it
influenced, at most `--max-links` 16) and adds those people to `frontier.jsonl` with
their distance from the start. Every run first finishes the starting authors not yet
resolved, then goes on from the frontier, nearest first (breadth first), up to
`--depth` (default 3); it never starts again from the seeds. Authors reached by a link
must pass the same gate; their languages come from Wikidata (P1412 languages written,
P103 native language) restricted to it, de, en. Raising `--depth` in a later run goes
deeper from where the walk stopped.

The interface (Specchio, `crawl status`) shows counts only; `crawl status --names`
lists the authors on request.

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
- An author reached by a link is taken only if it wrote in it, de or en.
- At most `--max-works` works per author (default 8) and `--max-pages` chapters per
  work (default 200); a time budget (`--minutes`, default 30); a ceiling on the texts on
  disk (`--max-gb`, default 64): at the ceiling the crawl stops.

## Files

| File | One line per |
|---|---|
| `texts/<lang>/<lang>-<work>.jsonl` | paragraph: `page`, `i`, `text` (Unicode NFC; verses keep their line breaks) |
| `seeds.toml` | starting author: `search`, `lang`, optional `qid` (private) |
| `authors.jsonl` | author taken: `qid`, `label`, `died`, `lang`, `depth` |
| `frontier.jsonl` | author to visit: `qid`, `label`, `depth`, `via`, `rel` |
| `works.jsonl` | work taken: `work_id`, `work_qid`, `author_qid`, `title`, `lang`, `url`, `pages`, `paragraphs`, `chars`, `blake3`, `licence`, `path` |
| `edges.jsonl` | edge of the reference graph: `from`, `rel` (`influenced_by`, `influenced`, `wrote`), `to`, labels |
| `refused.jsonl` | refusal: `what`, `why` (recorded once) |
| `lessons.jsonl` | work queued for the mind (read by the gate, step 6b) |
| `seeds.lock.json` | which Wikidata item each seed became: check it; pin a wrong one with `qid` in the seeds |
| `card.json` | the dataset card of the texts and tables (`docs/data-cards.md`) |
| `crawl.log` | what happened, readable |
| `cache/` | raw answers (not part of the card) |

JSON Lines rather than Parquet for v0: readable, one dependency less, and read in Rust
with `serde_json`, already used. Parquet comes when the data grows (tables of stars).
