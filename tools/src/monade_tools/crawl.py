"""The crawler: from the starting authors, along their links, to clean texts the mind can read.

The starting authors (the seeds) are only the beginning. Each author taken adds to a
**frontier** the people linked to it on Wikidata (who influenced it, and whom it
influenced); every run goes on from the frontier, nearest links first, and never
starts again from the seeds. For each author reached:

1. find it on Wikidata (seeds by name or pinned id; frontier by id), check it is a person;
2. **licence gate**: take it only if dead for more than 70 years (public domain in the
   EU on 1 January after 70 full years), and only if it wrote in a language of the
   library (it, de, en); otherwise refuse once and log why;
3. record the links (edges of the reference graph) and extend the frontier;
4. find its works with a Wikisource page in its own languages (no translations), at
   most `max_works` per author;
5. fetch each work (main page and its chapters), extract clean paragraphs, write them.

Limits: a time budget per run, a depth of links, and a ceiling on the size of the
library on disk (`max_gb`, default 64).

Everything is written under `<data>/biblioteca/`:

    seeds.toml   the starting authors (private: kept in the data folder, not in the repo)
    texts/<lang>/<lang>-<work>.jsonl.zst   one paragraph per line: {"page", "i", "text"}, Zstandard
    index.sqlite   full-text index of the paragraphs (rebuilt by `library pack`)
    authors.jsonl  works.jsonl  edges.jsonl  refused.jsonl  frontier.jsonl   the tables
    lessons.jsonl  the queue of works for the mind (read from step 6b)
    seeds.lock.json  which Wikidata item each seed became (check it)
    crawl.log  card.json  cache/

A run can stop at any time (time budget, Ctrl+C, a network error) and the next run
continues: answers are cached, finished works are skipped.
"""

from __future__ import annotations

import datetime as _dt
import json
import shutil
import time
import tomllib
from pathlib import Path

from . import cards, extract, store, wiki
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


def seeds_file(root: Path, example: Path) -> Path:
    """The starting authors: `<library>/seeds.toml` (private, in the data folder). The
    first time, it is created from the example in the repository, to be edited."""
    path = root / "seeds.toml"
    if not path.is_file():
        root.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(example, path)
    return path


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


def library_bytes(lib: Library) -> int:
    """Size of the texts on disk."""
    texts = lib.root / "texts"
    if not texts.is_dir():
        return 0
    return sum(p.stat().st_size for p in texts.rglob("*") if p.suffix in (".jsonl", ".zst"))


def crawl_work(fetch: Fetcher, lib: Library, author: str, w: dict, max_pages: int, mirror=None) -> dict | None:
    """Takes one work (main page + chapters), from the local mirror of the dump if there
    is one for its language, else from the Wikisource API; writes its paragraphs
    compressed and adds them to the index."""
    lang, title = w["lang"], w["title"]
    work_id = f"{lang}-{w['work']}"
    if mirror is not None:
        sub = mirror.subpages(title)
        read = mirror.paragraphs
    else:
        sub = wiki.parse_subpages(fetch.get_json(wiki.subpages_url(lang, title)))

        def read(page: str) -> list[str]:
            return extract.paragraphs(wiki.parse_page(fetch.get_json(wiki.page_url(lang, page))))

    pages = [title] + sub[:max_pages]
    out_path = lib.path(f"texts/{lang}/{work_id}.jsonl.zst")

    def rows():
        for page in pages:
            paras = read(page)
            if page == title and sub and extract.is_index_page(paras):
                continue  # the main page only lists the chapters
            for i, text in enumerate(paras):
                yield {"page": page, "i": i, "text": text}

    n_par, n_chars = store.write_texts(out_path, rows())
    if n_par == 0:
        lib.refuse({"what": work_id, "title": title, "why": "no text found"})
        return None
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
        "source": "dump" if mirror is not None else "api",
    }
    lib.append("works.jsonl", row)
    idx = store.Index(lib.root)
    try:
        idx.add(work_id, row["path"])
    finally:
        idx.close()
    lib.append("lessons.jsonl", {"work_id": work_id, "lang": lang, "path": row["path"]})
    return row


def next_targets(lib: Library, seeds: list[dict], max_depth: int):
    """What to visit next: seeds not yet resolved, then the frontier by depth (nearest
    links first), skipping every author already taken or refused."""
    lock = {}
    lp = lib.path("seeds.lock.json")
    if lp.is_file():
        lock = json.loads(lp.read_text(encoding="utf-8"))
    seen = {a["qid"] for a in lib.rows("authors.jsonl")} | {r["what"] for r in lib.rows("refused.jsonl")}
    for s in seeds:
        q = s.get("qid") or lock.get(s["search"], {}).get("qid")
        if q is None or q not in seen:
            yield {"seed": s, "depth": 0}
    frontier = sorted(lib.rows("frontier.jsonl"), key=lambda f: f["depth"])
    queued = set()
    for f in frontier:
        if f["depth"] > max_depth or f["qid"] in seen or f["qid"] in queued:
            continue
        queued.add(f["qid"])
        yield {"qid": f["qid"], "depth": f["depth"], "label": f.get("label", "")}


def run(
    seeds_path: Path,
    root: Path,
    *,
    max_works: int = 8,
    max_pages: int = 200,
    minutes: float = 30,
    max_depth: int = 3,
    max_links: int = 16,
    max_gb: float = 64,
    fetch: Fetcher | None = None,
    year: int | None = None,
    mirrors: dict | None = None,
) -> dict:
    """One crawl. Returns counts: authors, works, refused, requests."""
    lib = Library(root)
    fetch = fetch or Fetcher(root / "cache")
    year = year or _dt.date.today().year
    budget = Budget(minutes)
    max_bytes = int(max_gb * 1024**3)
    done = lib.done_works()
    seeds = read_seeds(seeds_path)
    lock_new: dict[str, dict] = {}
    in_frontier = {f["qid"] for f in lib.rows("frontier.jsonl")}
    counts = {"authors": 0, "works": 0, "refused": 0}
    mirrors = mirrors or {}
    if mirrors:
        lib.log("local mirror of Wikisource for " + ", ".join(sorted(mirrors)) + ": chapters read from the dump")
    lib.log(f"crawl start: max {max_works} works per author, links up to depth {max_depth}, {minutes} minutes, {max_gb} GB")
    tried: set[str] = set()  # targets attempted in this run (a network error is retried next run)

    def key(t: dict) -> str:
        return t["seed"]["search"] if "seed" in t else t["qid"]

    while True:
        # The frontier grows while we read: always take the nearest target not yet tried.
        target = next((t for t in next_targets(lib, seeds, max_depth) if key(t) not in tried), None)
        if target is None:
            lib.log("nothing left to visit within the depth limit")
            break
        tried.add(key(target))
        if budget.over():
            lib.log("time budget over: stopping (the next run continues)")
            break
        if library_bytes(lib) >= max_bytes:
            lib.log(f"library at its ceiling ({max_gb} GB): stopping")
            break
        depth = target["depth"]
        try:
            if "seed" in target:
                seed = target["seed"]
                qid, person = resolve(fetch, seed)
                if qid is None:
                    if lib.refuse({"what": seed["search"], "why": "no person found on Wikidata"}):
                        counts["refused"] += 1
                    lib.log(f"{seed['search']}: no person found on Wikidata")
                    continue
                lock_new[seed["search"]] = {"qid": qid, "label": person["label"], "died": person["died"]}
                langs = list(seed["lang"])
            else:
                qid = target["qid"]
                found = wiki.parse_persons(fetch.get_json(wiki.sparql_url(wiki.person_query([qid]))))
                if qid not in found:
                    if lib.refuse({"what": qid, "label": target.get("label", ""), "why": "not a person on Wikidata"}):
                        counts["refused"] += 1
                    lib.log(f"{target.get('label') or qid}: not a person on Wikidata")
                    continue
                person = found[qid]
                langs = person["langs"]
        except FetchError as e:
            lib.log(f"{target.get('label') or target.get('seed', {}).get('search')}: network error, retried next run ({e})")
            continue
        label = person["label"]
        why = None
        if not public_domain(person["died"], year):
            why = f"protected: died {person['died']}" if person["died"] else "no death date: may be alive"
        elif not langs:
            why = "did not write in it, de or en"
        if why:
            if lib.refuse({"what": qid, "label": label, "why": why}):
                counts["refused"] += 1
            lib.log(f"{label} ({qid}): refused, {why}")
            continue
        lib.append("authors.jsonl", {"qid": qid, "label": label, "died": person["died"], "lang": langs, "depth": depth})
        counts["authors"] += 1
        # Links: who influenced this author, and whom it influenced -> the frontier.
        links = [(q, l, "influenced_by") for q, l in person["influences"]]
        try:
            links += [(q, l, "influenced") for q, l in wiki.parse_influenced(
                fetch.get_json(wiki.sparql_url(wiki.influenced_query(qid, max_links))))]
        except FetchError as e:
            lib.log(f"{label}: influenced not listed this time ({e})")
        for q, l, rel in links[: 2 * max_links]:
            lib.append("edges.jsonl", {"from": qid, "rel": rel, "to": q, "from_label": label, "to_label": l})
            if q not in in_frontier:  # recorded even beyond the depth limit: a later run may go deeper
                lib.append("frontier.jsonl", {"qid": q, "label": l, "depth": depth + 1, "via": qid, "rel": rel})
                in_frontier.add(q)
        try:
            works = wiki.parse_works(fetch.get_json(wiki.sparql_url(wiki.works_query(qid, langs))))
        except FetchError as e:
            lib.log(f"{label}: works not listed this time ({e})")
            continue
        works = [w for w in works if w["lang"] in langs][:max_works]
        lib.log(f"{label} ({qid}, died {person['died']}, depth {depth}): {len(works)} works on Wikisource")
        for w in works:
            if budget.over() or library_bytes(lib) >= max_bytes:
                break
            work_id = f"{w['lang']}-{w['work']}"
            if work_id in done:
                continue
            try:
                row = crawl_work(fetch, lib, qid, w, max_pages, mirrors.get(w["lang"]))
            except FetchError as e:
                lib.log(f"  {w['title']}: network error, retried next run ({e})")
                continue
            if row:
                done.add(work_id)
                counts["works"] += 1
                lib.append("edges.jsonl", {"from": qid, "rel": "wrote", "to": w["work"], "from_label": label, "to_label": w["label"]})
                lib.log(f"  {w['title']} [{w['lang']}]: {row['paragraphs']} paragraphs, {row['chars']} characters")
            else:
                counts["refused"] += 1
                lib.log(f"  {w['title']}: no text found")
    if lock_new:
        old = {}
        lp = lib.path("seeds.lock.json")
        if lp.is_file():
            old = json.loads(lp.read_text(encoding="utf-8"))
        old.update(lock_new)
        lp.write_text(json.dumps(old, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_card(lib)
    counts["requests"] = fetch.requests
    counts["cached"] = fetch.cache_hits
    lib.log(
        f"crawl end: {counts['authors']} authors, {counts['works']} new works, {counts['refused']} refused, "
        f"{len(list(next_targets(lib, seeds, max_depth)))} authors waiting in the frontier, "
        f"{fetch.requests} requests, {fetch.cache_hits} from cache"
    )
    return counts


def write_card(lib: Library) -> None:
    """The dataset card of the library (texts and tables; not the cache)."""
    texts = lib.root / "texts"
    files = sorted(p for p in texts.rglob("*") if p.suffix in (".jsonl", ".zst")) if texts.is_dir() else []
    files += [lib.path(t) for t in ("authors.jsonl", "works.jsonl", "edges.jsonl") if lib.path(t).is_file()]
    if not files:
        return
    langs = sorted({w["lang"] for w in lib.rows("works.jsonl")})
    card = cards.new_card(
        name="biblioteca",
        description="Works of the starting authors and of the authors linked to them, from Wikisource, with authors and influences from Wikidata.",
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


def status(root: Path, names: bool = False) -> str:
    """A short summary of the library (author names only if `names`)."""
    lib = Library(root)
    works = lib.rows("works.jsonl")
    authors = lib.rows("authors.jsonl")
    refused = lib.rows("refused.jsonl")
    frontier = {f["qid"] for f in lib.rows("frontier.jsonl")}
    seen = {a["qid"] for a in authors} | {r["what"] for r in refused}
    by_lang: dict[str, int] = {}
    for w in works:
        by_lang[w["lang"]] = by_lang.get(w["lang"], 0) + w["chars"]
    depths: dict[int, int] = {}
    for a in authors:
        depths[a.get("depth", 0)] = depths.get(a.get("depth", 0), 0) + 1
    size = library_bytes(lib)
    lines = [
        f"library    {root} ({size / 1024**2:.1f} MB of texts)",
        "authors    "
        + f"{len(authors)} taken (by distance from the start: "
        + ", ".join(f"{d}: {n}" for d, n in sorted(depths.items()))
        + ")",
        f"frontier   {len(frontier - seen)} authors waiting",
        f"works      {len(works)}, paragraphs {sum(w['paragraphs'] for w in works)}",
        "characters " + (", ".join(f"{k} {v:,}".replace(",", " ") for k, v in sorted(by_lang.items())) or "0"),
        f"refused    {len(refused)}",
        f"edges      {len(lib.rows('edges.jsonl'))}",
        f"lessons    {len(lib.rows('lessons.jsonl'))} queued for the mind",
    ]
    if lib.path("index.sqlite").is_file():
        idx = store.Index(root)
        lines.append(f"index      {idx.count()} paragraphs searchable")
        idx.close()
    if names:
        lines.append("authors: " + ", ".join(a["label"] for a in authors))
        lines += [f"refused: {r.get('label') or r['what']}: {r['why']}" for r in refused]
    lock = lib.path("crawl.lock")
    if lock.is_file():
        lines.append(f"running    since {lock.read_text(encoding='utf-8').strip()}")
    return "\n".join(lines)


def pack(root: Path) -> dict:
    """Compresses the texts of an older library (plain `.jsonl`) and rebuilds the index.
    Returns counts: packed files, bytes before and after, paragraphs indexed."""
    lib = Library(root)
    works = lib.rows("works.jsonl")
    before = after = packed = 0
    for w in works:
        p = root / w["path"]
        if p.suffix == ".jsonl" and p.is_file():
            new = p.with_name(p.name + ".zst")
            store.write_texts(new, store.read_texts(p))
            before += p.stat().st_size
            after += new.stat().st_size
            p.unlink()
            w["path"] = new.relative_to(root).as_posix()
            w["blake3"] = cards.file_hash(new)
            packed += 1
    if packed:
        for table in ("works.jsonl", "lessons.jsonl"):
            rows = works if table == "works.jsonl" else lib.rows(table)
            if table == "lessons.jsonl":
                paths = {w["work_id"]: w["path"] for w in works}
                for r in rows:
                    r["path"] = paths.get(r["work_id"], r["path"])
            tmp = lib.path(table + ".tmp")
            with open(tmp, "w", encoding="utf-8") as f:
                for r in rows:
                    f.write(json.dumps(r, ensure_ascii=False, sort_keys=True) + "\n")
            tmp.replace(lib.path(table))
        write_card(lib)
    indexed = store.rebuild(root, works)
    return {"packed": packed, "before": before, "after": after, "indexed": indexed}
