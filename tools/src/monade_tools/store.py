"""The store of the texts: compressed on disk, found again through an index.

Texts are JSON Lines compressed with Zstandard (`.jsonl.zst`, one file per work):
plain text compresses three to four times and stays readable with any zstd tool.
The index is one SQLite file (`index.sqlite`, Python's own `sqlite3`):

    para(id, work_id, page, i, path, line)   where each paragraph is
    fts(text)                                full-text index (FTS5, contentless)

The index holds no copy of the text: a hit gives the file and the line, and the
paragraph is read back from the compressed file. The index can always be rebuilt from
the texts (`rebuild`), so it is a convenience, never the source of truth.
"""

from __future__ import annotations

import io
import json
import sqlite3
from collections.abc import Iterable, Iterator
from pathlib import Path

import zstandard

#: Compression level: 10 is a good balance of size and speed for text.
LEVEL = 10

SCHEMA = """
CREATE TABLE IF NOT EXISTS para (
    id INTEGER PRIMARY KEY, work_id TEXT NOT NULL, page TEXT NOT NULL,
    i INTEGER NOT NULL, path TEXT NOT NULL, line INTEGER NOT NULL);
CREATE INDEX IF NOT EXISTS para_work ON para(work_id);
CREATE VIRTUAL TABLE IF NOT EXISTS fts USING fts5(
    text, content='', tokenize='unicode61 remove_diacritics 2');
"""


def write_texts(path: Path, rows: Iterable[dict]) -> tuple[int, int]:
    """Writes rows ({"page", "i", "text"}) to a `.jsonl.zst` file, atomically.
    Returns (paragraphs, characters)."""
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_name(path.name + ".tmp")
    n = chars = 0
    with open(tmp, "wb") as raw:
        with zstandard.ZstdCompressor(level=LEVEL).stream_writer(raw) as z:
            for r in rows:
                z.write((json.dumps(r, ensure_ascii=False) + "\n").encode("utf-8"))
                n += 1
                chars += len(r["text"])
    if n == 0:
        tmp.unlink()
    else:
        tmp.replace(path)
    return n, chars


def read_texts(path: Path) -> Iterator[dict]:
    """The rows of a text file, `.jsonl.zst` or plain `.jsonl` (older libraries)."""
    if path.suffix == ".zst":
        with open(path, "rb") as raw:
            reader = zstandard.ZstdDecompressor().stream_reader(raw)
            for line in io.TextIOWrapper(reader, encoding="utf-8"):
                if line.strip():
                    yield json.loads(line)
    else:
        with open(path, encoding="utf-8") as f:
            for line in f:
                if line.strip():
                    yield json.loads(line)


class Index:
    """The full-text index of a library (`<library>/index.sqlite`)."""

    def __init__(self, root: Path):
        self.root = root
        self.db = sqlite3.connect(root / "index.sqlite")
        self.db.executescript(SCHEMA)

    def close(self) -> None:
        self.db.close()

    def has(self, work_id: str) -> bool:
        return self.db.execute("SELECT 1 FROM para WHERE work_id = ? LIMIT 1", (work_id,)).fetchone() is not None

    def add(self, work_id: str, rel_path: str) -> int:
        """Indexes every paragraph of one work (once). Returns how many."""
        if self.has(work_id):
            return 0
        n = 0
        with self.db:
            for line, r in enumerate(read_texts(self.root / rel_path)):
                cur = self.db.execute(
                    "INSERT INTO para(work_id, page, i, path, line) VALUES (?, ?, ?, ?, ?)",
                    (work_id, r["page"], r["i"], rel_path, line),
                )
                self.db.execute("INSERT INTO fts(rowid, text) VALUES (?, ?)", (cur.lastrowid, r["text"]))
                n += 1
        return n

    def count(self) -> int:
        return self.db.execute("SELECT count(*) FROM para").fetchone()[0]

    def search(self, query: str, limit: int = 10) -> list[dict]:
        """Paragraphs matching `query` (FTS5 syntax: words, "a phrase", a OR b, pre*),
        best first, each with its work, page and text read back from the file."""
        try:
            ids = [r[0] for r in self.db.execute(
                "SELECT rowid FROM fts WHERE fts MATCH ? ORDER BY rank LIMIT ?", (query, limit))]
        except sqlite3.OperationalError as e:
            raise ValueError(f"bad query: {e}") from None
        hits = []
        for pid in ids:
            work_id, page, i, path, line = self.db.execute(
                "SELECT work_id, page, i, path, line FROM para WHERE id = ?", (pid,)).fetchone()
            hits.append({"work_id": work_id, "page": page, "i": i, "path": path, "line": line})
        # Read each file once, in order of lines.
        by_file: dict[str, list[dict]] = {}
        for h in hits:
            by_file.setdefault(h["path"], []).append(h)
        for path, hs in by_file.items():
            wanted = {h["line"]: h for h in hs}
            last = max(wanted)
            for line, r in enumerate(read_texts(self.root / path)):
                if line in wanted:
                    wanted[line]["text"] = r["text"]
                if line >= last:
                    break
        return hits


def rebuild(root: Path, works: list[dict]) -> int:
    """Rebuilds the index from scratch from the texts of `works`. Returns paragraphs."""
    p = root / "index.sqlite"
    if p.exists():
        p.unlink()
    idx = Index(root)
    try:
        return sum(idx.add(w["work_id"], w["path"]) for w in works)
    finally:
        idx.close()
