"""The crawler v0: from the seed authors to clean texts the mind can read.

For each seed author:
1. find the author on Wikidata (or use the pinned id), check it is a person;
2. **licence gate**: take the author only if dead for more than 70 years (public domain
   in the EU on 1 January after 70 full years); otherwise refuse and log why;
3. record who influenced whom (edges of the reference graph);
4. find the author's works that have a Wikisource page in the author's own languages
   (no translations), at most `max_works` per author;
5. fetch each work (main page and its chapters), extract clean paragraphs, write them.

Everything is written under `<data>/biblioteca/`:

    texts/<lang>/<lang>-<work>.jsonl   one paragraph per line: {"page", "i", "text"}
    authors.jsonl  works.jsonl  edges.jsonl  refused.jsonl   the tables
    lessons.jsonl  the queue of works for the mind (read from step 6b)
    seeds.lock.json  which Wikidata item each seed became (check it)
    crawl.log  card.json  cache/

A run can stop at any time (time budget, Ctrl+C, a network error) and the next run
continues: answers are cached, finished works are skipped.
"""

from __future__ import annotations

import datetime as _dt
import json
import time
import tomllib
from pathlib import Path

from . import cards, extract, wiki
from .net import Fetcher, FetchError  # noqa: F401  (Fetcher is used by the CLI)

#: Years after the author's death before the work is free in the EU (then 1 January).
PROTECTION_YEARS = 70


def public_domain(died: int | None, year: int) -> bool:
    """True if an author who died in `died` is in the public domain on 1 January `year`."""
    return died is not None and died + PROTECTION_YEARS < year


class Library:
    """The folder of the crawler and its tables (JSON Lines, append-only)."""

    def __init__(self, root: Path):
        self.root = root
        self.root.mkdir(parents=True, exist_ok=True)

    def path(self, name: str) -> Path:
        return self.root / name

    def append(self, table: str, row: dict) -> None:
        with open(self.path(table), "a", encoding="utf-8") as f:
            f.write(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n")

    def rows(self, table: str) -> list[dict]:
        p = self.path(table)
        if not p.is_file():
            return []
        with open(p, encoding="utf-8") as f:
            return [json.loads(line) for line in f if line.strip()]

    def log(self, text: str) -> None:
        stamp = _dt.datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        line = f"{stamp}  {text}"
        print(line, flush=True)
        with open(self.path("crawl.log"), "a", encoding="utf-8") as f:
            f.write(line + "\n")

    def done_works(self) -> set[str]:
        """Works already taken, or already found without text (not asked again)."""
        taken = {w["work_id"] for w in self.rows("works.jsonl")}
        empty = {r["what"] for r in self.rows("refused.jsonl") if r["why"] == "no text found"}
        return taken | empty

    def refuse(self, row: dict) -> bool:
        """Records a refusal once; False if it was already recorded."""
        key = (row["what"], row["why"])
        if any((r["what"], r["why"]) == key for r in self.rows("refused.jsonl")):
            return False
        self.append("refused.jsonl", row)
        return True


def read_seeds(path: Path) -> list[dict]:
    """The seed authors: [{search, lang, qid?}]."""
    with open(path, "rb") as f:
        data = tomllib.load(f)
    seeds = data.get("author", [])
    for s in seeds:
        if not s.get("search") or not s.get("lang"):
            raise ValueError(f"every author needs `search` and `lang`: {s}")
    return seeds


def resolve(fetch: Fetcher, seed: dict) -> tuple[str | None, dict]:
    """The Wikidata id of a seed (pinned, or the first human in the search results)
    and what Wikidata says about that person."""
    if seed.get("qid"):
        ids = [seed["qid"]]
    else:
        ids = wiki.search_ids(fetch.get_json(wiki.search_url(seed["search"])))
    if not ids:
        return None, {}
    persons = wiki.parse_persons(fetch.get_json(wiki.sparql_url(wiki.person_query(ids))))
    for q in ids:  # keep the order of the search: the first human wins
        if q in persons:
            return q, persons[q]
    return None, {}


class Budget:
    """Stops the crawl when the time is over."""

    def __init__(self, minutes: float):
        self.deadline = time.monotonic() + minutes * 60

    def over(self) -> bool:
        return time.monotonic() > self.deadline


def crawl_work(fetch: Fetcher, lib: Library, author: str, w: dict, max_pages: int) -> dict | None:
    """Fetches one work (main page + chapters) and writes its paragraphs."""
    lang, title = w["lang"], w["title"]
    work_id = f"{lang}-{w['work']}"
    sub = wiki.parse_subpages(fetch.get_json(wiki.subpages_url(lang, title)))
    pages = [title] + sub[:max_pages]
    out_path = lib.path(f"texts/{lang}/{work_id}.jsonl")
    out_path.parent.mkdir(parents=True, exist_ok=True)
    tmp = out_path.with_suffix(".tmp")
    n_par = n_chars = 0
    with open(tmp, "w", encoding="utf-8") as f:
        for page in pages:
            paras = extract.paragraphs(wiki.parse_page(fetch.get_json(wiki.page_url(lang, page))))
            if page == title and sub and extract.is_index_page(paras):
                continue  # the main page only lists the chapters
            for i, text in enumerate(paras):
                f.write(json.dumps({"page": page, "i": i, "text": text}, ensure_ascii=False) + "\n")
                n_par += 1
                n_chars += len(text)
    if n_par == 0:
        tmp.unlink()
        lib.refuse({"what": work_id, "title": title, "why": "no text found"})
        return None
    tmp.replace(out_path)
    row = {
        "work_id": work_id,
        "work_qid": w["work"],
        "author_qid": author,
        "title": title,
        "label": w["label"],
        "lang": lang,
        "url": wiki.page_link(lang, title),
        "pages": len(pages),
        "paragraphs": n_par,
        "chars": n_chars,
        "blake3": cards.file_hash(out_path),
        "licence": "public domain (EU); transcription: Wikisource, CC BY-SA 4.0",
        "path": out_path.relative_to(lib.root).as_posix(),
    }
    lib.append("works.jsonl", row)
    lib.append("lessons.jsonl", {"work_id": work_id, "lang": lang, "path": row["path"]})
    return row


def run(
    seeds_path: Path,
    root: Path,
    *,
    max_works: int = 8,
    max_pages: int = 200,
    minutes: float = 30,
    fetch: Fetcher | None = None,
    year: int | None = None,
) -> dict:
    """One crawl. Returns counts: authors, works, refused, requests."""
    lib = Library(root)
    fetch = fetch or Fetcher(root / "cache")
    year = year or _dt.date.today().year
    budget = Budget(minutes)
    done = lib.done_works()
    known_authors = {a["qid"] for a in lib.rows("authors.jsonl")}
    lock = {}
    counts = {"authors": 0, "works": 0, "refused": 0}
    lib.log(f"crawl start: {seeds_path}, max {max_works} works per author, {minutes} minutes")
    for seed in read_seeds(seeds_path):
        if budget.over():
            lib.log("time budget over: stopping (the next run continues)")
            break
        try:
            qid, person = resolve(fetch, seed)
        except FetchError as e:
            lib.log(f"{seed['search']}: network error, skipped this time ({e})")
            continue
        if qid is None:
            lib.refuse({"what": seed["search"], "why": "no person found on Wikidata"})
            lib.log(f"{seed['search']}: no person found on Wikidata")
            counts["refused"] += 1
            continue
        lock[seed["search"]] = {"qid": qid, "label": person["label"], "died": person["died"]}
        if not public_domain(person["died"], year):
            why = f"protected: died {person['died']}" if person["died"] else "no death date: may be alive"
            lib.refuse({"what": qid, "label": person["label"], "why": why})
            lib.log(f"{person['label']} ({qid}): refused, {why}")
            counts["refused"] += 1
            continue
        if qid not in known_authors:
            lib.append(
                "authors.jsonl",
                {"qid": qid, "label": person["label"], "died": person["died"], "lang": seed["lang"], "search": seed["search"]},
            )
            for iq, il in person["influences"]:
                lib.append("edges.jsonl", {"from": qid, "rel": "influenced_by", "to": iq, "from_label": person["label"], "to_label": il})
            known_authors.add(qid)
        counts["authors"] += 1
        try:
            works = wiki.parse_works(fetch.get_json(wiki.sparql_url(wiki.works_query(qid, seed["lang"]))))
        except FetchError as e:
            lib.log(f"{person['label']}: works not listed this time ({e})")
            continue
        works = [w for w in works if w["lang"] in seed["lang"]][:max_works]
        lib.log(f"{person['label']} ({qid}, died {person['died']}): {len(works)} works on Wikisource")
        for w in works:
            if budget.over():
                break
            work_id = f"{w['lang']}-{w['work']}"
            if work_id in done:
                continue
            try:
                row = crawl_work(fetch, lib, qid, w, max_pages)
            except FetchError as e:
                lib.log(f"  {w['title']}: network error, retried next run ({e})")
                continue
            if row:
                done.add(work_id)
                counts["works"] += 1
                lib.append("edges.jsonl", {"from": qid, "rel": "wrote", "to": w["work"], "from_label": person["label"], "to_label": w["label"]})
                lib.log(f"  {w['title']} [{w['lang']}]: {row['paragraphs']} paragraphs, {row['chars']} characters")
            else:
                counts["refused"] += 1
                lib.log(f"  {w['title']}: no text found")
    if lock:
        old = {}
        lp = lib.path("seeds.lock.json")
        if lp.is_file():
            old = json.loads(lp.read_text(encoding="utf-8"))
        old.update(lock)
        lp.write_text(json.dumps(old, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_card(lib)
    counts["requests"] = fetch.requests
    counts["cached"] = fetch.cache_hits
    lib.log(
        f"crawl end: {counts['authors']} authors, {counts['works']} new works, {counts['refused']} refused, "
        f"{fetch.requests} requests, {fetch.cache_hits} from cache"
    )
    return counts


def write_card(lib: Library) -> None:
    """The dataset card of the library (texts and tables; not the cache)."""
    files = [p for p in (lib.root / "texts").rglob("*.jsonl")] if (lib.root / "texts").is_dir() else []
    files += [lib.path(t) for t in ("authors.jsonl", "works.jsonl", "edges.jsonl") if lib.path(t).is_file()]
    if not files:
        return
    langs = sorted({w["lang"] for w in lib.rows("works.jsonl")})
    card = cards.new_card(
        name="biblioteca",
        description="Works of the seed authors from Wikisource, with authors and influences from Wikidata.",
        source="Wikisource (texts), Wikidata (authors, works, influences)",
        url="https://wikisource.org",
        licence="texts: public domain in the EU; Wikisource transcriptions: CC BY-SA 4.0; Wikidata: CC0",
        non_commercial=False,
        languages=langs,
        split="train",
        root=lib.root,
        files=files,
        notes="written by monade-tools crawl; the sealed test sets are kept elsewhere",
    )
    cards.write(card, lib.path("card.json"))


def status(root: Path) -> str:
    """A short summary of the library."""
    lib = Library(root)
    works = lib.rows("works.jsonl")
    authors = lib.rows("authors.jsonl")
    refused = lib.rows("refused.jsonl")
    by_lang: dict[str, int] = {}
    for w in works:
        by_lang[w["lang"]] = by_lang.get(w["lang"], 0) + w["chars"]
    lines = [
        f"library    {root}",
        f"authors    {len(authors)}: " + ", ".join(a["label"] for a in authors),
        f"works      {len(works)}, paragraphs {sum(w['paragraphs'] for w in works)}",
        "characters " + (", ".join(f"{k} {v:,}".replace(",", " ") for k, v in sorted(by_lang.items())) or "0"),
        f"refused    {len(refused)}" + ("".join(f"\n  - {r.get('label') or r['what']}: {r['why']}" for r in refused[-10:])),
        f"edges      {len(lib.rows('edges.jsonl'))}",
        f"lessons    {len(lib.rows('lessons.jsonl'))} queued for the mind",
    ]
    lock = lib.path("crawl.lock")
    if lock.is_file():
        lines.append(f"running    since {lock.read_text(encoding='utf-8').strip()}")
    return "\n".join(lines)
