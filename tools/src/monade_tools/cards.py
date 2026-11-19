"""Dataset cards: what a set of data is, where it comes from, under which licence.

A card is one JSON file next to the data. Every file of the set is listed with its
size and its BLAKE3 hash, so that a lesson can prove which bytes it learned from.
The format is described in docs/data-cards.md.
"""

from __future__ import annotations

import datetime as _dt
import json
from pathlib import Path

import blake3

#: Version of the card format.
CARD_VERSION = 1

#: Splits a set (or a file of it) may belong to.
SPLITS = ("train", "validation", "test")

#: Fields every card must have, in the order they are written.
FIELDS = (
    "card_version",
    "name",
    "description",
    "source",
    "url",
    "licence",
    "non_commercial",
    "languages",
    "retrieved",
    "split",
    "files",
    "notes",
)


def file_hash(path: Path) -> str:
    """BLAKE3 of a file, as 64 hexadecimal digits, read in 1 MB pieces."""
    h = blake3.blake3()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def describe_files(root: Path, paths: list[Path]) -> list[dict]:
    """Size and hash of each file, with its path relative to `root` (sorted)."""
    out = []
    for p in sorted(paths):
        out.append(
            {
                "path": p.relative_to(root).as_posix(),
                "bytes": p.stat().st_size,
                "blake3": file_hash(p),
            }
        )
    return out


def new_card(
    *,
    name: str,
    description: str,
    source: str,
    url: str,
    licence: str,
    non_commercial: bool,
    languages: list[str],
    split: str,
    root: Path,
    files: list[Path],
    notes: str = "",
    today: str | None = None,
) -> dict:
    """A complete card for the given files (all under `root`)."""
    if split not in SPLITS:
        raise ValueError(f"split must be one of {SPLITS}, not {split!r}")
    if not files:
        raise ValueError("a card describes at least one file")
    return {
        "card_version": CARD_VERSION,
        "name": name,
        "description": description,
        "source": source,
        "url": url,
        "licence": licence,
        "non_commercial": non_commercial,
        "languages": languages,
        "retrieved": today or _dt.date.today().isoformat(),
        "split": split,
        "files": describe_files(root, files),
        "notes": notes,
    }


def problems(card: dict, root: Path | None = None) -> list[str]:
    """What is wrong with a card (empty list: nothing). With `root`, the files are
    checked too: present, same size, same hash."""
    out = [f"missing field {f}" for f in FIELDS if f not in card]
    out += [f"unknown field {f}" for f in card if f not in FIELDS]
    if card.get("card_version") != CARD_VERSION:
        out.append(f"card_version must be {CARD_VERSION}")
    if card.get("split") not in SPLITS:
        out.append(f"split must be one of {SPLITS}")
    if not card.get("licence"):
        out.append("licence is empty: no data without a licence")
    if root is not None:
        for f in card.get("files", []):
            p = root / f.get("path", "")
            if not p.is_file():
                out.append(f"file missing: {f.get('path')}")
            elif p.stat().st_size != f.get("bytes") or file_hash(p) != f.get("blake3"):
                out.append(f"file changed: {f.get('path')}")
    return out


def write(card: dict, path: Path) -> None:
    """Writes a card as readable JSON (fields in the fixed order)."""
    ordered = {k: card[k] for k in FIELDS}
    path.write_text(json.dumps(ordered, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def read(path: Path) -> dict:
    """Reads a card."""
    return json.loads(path.read_text(encoding="utf-8"))
