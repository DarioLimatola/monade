"""Wikisource from its dump: one download instead of thousands of requests.

Wikimedia publishes, about once a week, the search index of every wiki
(`dumps.wikimedia.org/other/cirrussearch/<date>/<wiki>-<date>-cirrussearch-content.json.gz`):
every page of the main namespace with its rendered plain text (chapters transcluded from
scans included). One file per language, a few GB at most, downloaded once, politely
(one connection, resumable).

`build` turns it into a local mirror, `<data>/dumps/<lang>wikisource.sqlite`:

    pages(title PRIMARY KEY, item, z)   z = the page text, Zstandard-compressed

The crawler then reads chapters from the mirror instead of the Wikisource API (Wikidata
is still asked for authors, links and works). The price: the plain text of the dump has
no paragraph breaks, so a page is cut into chunks at sentence ends (`chunks`), and
verses lose their line breaks. The API keeps both; a work taken from the mirror says
so (`"source": "dump"` in works.jsonl).
"""

from __future__ import annotations

import gzip
import json
import re
import sqlite3
import urllib.request
from collections.abc import Iterator
from pathlib import Path
from typing import Callable

import zstandard

from . import extract
from .net import user_agent

BASE = "https://dumps.wikimedia.org/other/cirrussearch/"
LANGS = ("it", "de", "en")
#: Longest chunk of a page (characters), cut at a sentence end when possible.
CHUNK = 1200


def wiki(lang: str) -> str:
    return f"{lang}wikisource"


def dates(index_html: str) -> list[str]:
    """The runs (YYYYMMDD) in the listing of BASE, newest first."""
    return sorted(set(re.findall(r'href="(\d{8})/"', index_html)), reverse=True)


def dump_name(lang: str, date: str) -> str:
    return f"{wiki(lang)}-{date}-cirrussearch-content.json.gz"


def _open(url: str, start: int):
    headers = {"User-Agent": user_agent()}
    if start:
        headers["Range"] = f"bytes={start}-"
    return urllib.request.urlopen(urllib.request.Request(url, headers=headers), timeout=60)


def download(
    url: str,
    dest: Path,
    opener: Callable[[str, int], object] = _open,
    progress: Callable[[int], None] | None = None,
) -> Path:
    """Downloads `url` to `dest`, resuming a `.part` file left by an earlier attempt."""
    if dest.is_file():
        return dest
    part = dest.with_name(dest.name + ".part")
    dest.parent.mkdir(parents=True, exist_ok=True)
    start = part.stat().st_size if part.is_file() else 0
    with opener(url, start) as r:
        status = getattr(r, "status", 200)
        mode = "ab" if start and status == 206 else "wb"  # 200: the server ignored Range
        done = start if mode == "ab" else 0
        with open(part, mode) as f:
            while True:
                block = r.read(1 << 20)
                if not block:
                    break
                f.write(block)
                done += len(block)
                if progress:
                    progress(done)
    part.replace(dest)
    return dest


def pages(path: Path) -> Iterator[dict]:
    """The pages of the main namespace in a cirrussearch dump (Elasticsearch bulk
    format: an action line, then the document)."""
    with gzip.open(path, "rt", encoding="utf-8") as f:
        for line in f:
            doc = json.loads(line)
            if "index" in doc or "create" in doc:
                continue
            if doc.get("namespace", 0) != 0 or not doc.get("title"):
                continue
            yield doc


def mirror_path(dumps: Path, lang: str) -> Path:
    return dumps / f"{wiki(lang)}.sqlite"


def build(dump_file: Path, out: Path, progress: Callable[[int], None] | None = None) -> int:
    """Builds the mirror `out` from a dump file. Returns the number of pages."""
    tmp = out.with_name(out.name + ".tmp")
    if tmp.exists():
        tmp.unlink()
    db = sqlite3.connect(tmp)
    db.execute("CREATE TABLE pages (title TEXT PRIMARY KEY, item TEXT, z BLOB NOT NULL)")
    comp = zstandard.ZstdCompressor(level=10)
    n = 0
    with db:
        for doc in pages(dump_file):
            text = extract.normalise(doc.get("text") or "")
            db.execute(
                "INSERT OR REPLACE INTO pages VALUES (?, ?, ?)",
                (doc["title"], doc.get("wikibase_item"), comp.compress(text.encode("utf-8"))),
            )
            n += 1
            if progress and n % 10000 == 0:
                progress(n)
    db.close()
    tmp.replace(out)
    return n


def chunks(text: str, size: int = CHUNK) -> list[str]:
    """Cuts a page without paragraph breaks into chunks of at most `size` characters,
    at sentence ends when possible."""
    text = text.strip()
    out: list[str] = []
    while len(text) > size:
        window = text[:size]
        cut = max(window.rfind(m) for m in (". ", "! ", "? ", "; ", ": "))
        cut = cut + 1 if cut > size // 3 else (window.rfind(" ") if window.rfind(" ") > 0 else size)
        out.append(text[:cut].strip())
        text = text[cut:].strip()
    if text:
        out.append(text)
    return out


class Mirror:
    """A local mirror of one Wikisource, read by the crawler."""

    def __init__(self, path: Path):
        self.db = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
        self.dec = zstandard.ZstdDecompressor()

    def subpages(self, title: str) -> list[str]:
        """Pages under `title/` (the chapters), in title order, as the API gives them."""
        lo = title + "/"
        hi = title + "0"  # "0" follows "/" in code point order
        return [r[0] for r in self.db.execute(
            "SELECT title FROM pages WHERE title >= ? AND title < ? ORDER BY title", (lo, hi))]

    def paragraphs(self, title: str) -> list[str]:
        row = self.db.execute("SELECT z FROM pages WHERE title = ?", (title,)).fetchone()
        if row is None:
            return []
        return chunks(self.dec.decompress(row[0]).decode("utf-8"))

    def count(self) -> int:
        return self.db.execute("SELECT count(*) FROM pages").fetchone()[0]

    def close(self) -> None:
        self.db.close()


def mirrors(dumps: Path) -> dict[str, Mirror]:
    """The mirrors present in `dumps`, by language."""
    return {lang: Mirror(mirror_path(dumps, lang)) for lang in LANGS if mirror_path(dumps, lang).is_file()}
