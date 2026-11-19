"""`monade-tools`: the command line of the Python tools.

    monade-tools doctor
    monade-tools card new --name N --source S --url U --licence L --split train
                          --languages it,en --dir FOLDER [--description D]
                          [--non-commercial] [--notes T]
    monade-tools card check CARD.json
    monade-tools crawl run [--seeds tools/seeds.toml] [--max-works 8] [--minutes 30]
    monade-tools crawl status
"""

from __future__ import annotations

import argparse
import os
import platform
import shutil
import subprocess
import sys
from pathlib import Path

import time

from . import __version__, cards, crawl


def data_dir() -> Path:
    """The data folder: MONADE_DATA if set; on Windows D:\\monade-data if a D: drive
    exists, else C:\\monade-data; ~/monade-data elsewhere."""
    env = os.environ.get("MONADE_DATA")
    if env:
        return Path(env)
    if platform.system() == "Windows":
        return Path("D:/monade-data") if Path("D:/").exists() else Path("C:/monade-data")
    return Path.home() / "monade-data"


def existing_parent(p: Path) -> Path:
    """The nearest folder above `p` (or `p` itself) that exists."""
    for q in [p, *p.parents]:
        if q.exists():
            return q
    return Path.cwd()


def gpu() -> str:
    """Name and memory of the NVIDIA GPU, if nvidia-smi answers."""
    exe = shutil.which("nvidia-smi")
    if not exe:
        return "none found (nvidia-smi not available)"
    try:
        out = subprocess.run(
            [exe, "--query-gpu=name,memory.total", "--format=csv,noheader"],
            capture_output=True,
            text=True,
            timeout=10,
            check=True,
        )
        return out.stdout.strip() or "none found"
    except (OSError, subprocess.SubprocessError) as e:
        return f"nvidia-smi failed: {e}"


def doctor() -> int:
    """Says what the tools see: Python, system, data folder, disk, GPU."""
    d = data_dir()
    print(f"monade-tools {__version__}")
    print(f"python       {platform.python_version()} ({sys.executable})")
    print(f"system       {platform.system()} {platform.release()} {platform.machine()}")
    print(f"data folder  {d} ({'exists' if d.is_dir() else 'missing: create it, or set MONADE_DATA'})")
    try:
        free = shutil.disk_usage(existing_parent(d)).free
        print(f"free disk    {free // 10**9} GB")
    except OSError as e:
        print(f"free disk    unknown ({e})")
    print(f"gpu          {gpu()}")
    return 0


def card_new(a: argparse.Namespace) -> int:
    root = Path(a.dir)
    files = [p for p in root.rglob("*") if p.is_file() and p.name != "card.json"]
    card = cards.new_card(
        name=a.name,
        description=a.description,
        source=a.source,
        url=a.url,
        licence=a.licence,
        non_commercial=a.non_commercial,
        languages=[x for x in a.languages.split(",") if x],
        split=a.split,
        root=root,
        files=files,
        notes=a.notes,
    )
    out = root / "card.json"
    cards.write(card, out)
    print(f"card written: {out} ({len(card['files'])} files)")
    return 0


def card_check(a: argparse.Namespace) -> int:
    path = Path(a.card)
    found = cards.problems(cards.read(path), path.parent)
    for p in found:
        print(f"problem: {p}")
    print("card ok" if not found else f"{len(found)} problem(s)")
    return 1 if found else 0


def library_dir() -> Path:
    """Where the crawler writes: <data folder>/biblioteca."""
    return data_dir() / "biblioteca"


def crawl_run(a: argparse.Namespace) -> int:
    root = library_dir()
    root.mkdir(parents=True, exist_ok=True)
    lock = root / "crawl.lock"
    # One crawl at a time; a lock older than 12 hours is left by a crash.
    if lock.is_file() and time.time() - lock.stat().st_mtime < 12 * 3600 and not a.force:
        print(f"a crawl is already running (since {lock.read_text(encoding='utf-8').strip()}); --force to ignore")
        return 1
    lock.write_text(time.strftime("%Y-%m-%d %H:%M:%S"), encoding="utf-8")
    try:
        crawl.run(
            Path(a.seeds),
            root,
            max_works=a.max_works,
            max_pages=a.max_pages,
            minutes=a.minutes,
            fetch=crawl.Fetcher(root / "cache", offline=a.offline),
        )
    except KeyboardInterrupt:
        print("stopped: the next run continues from here")
    finally:
        lock.unlink(missing_ok=True)
    return 0


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(prog="monade-tools", description="Tools of the Monade.")
    sub = p.add_subparsers(dest="command", required=True)
    sub.add_parser("doctor", help="what the tools see: Python, data folder, disk, GPU")
    card = sub.add_parser("card", help="dataset cards").add_subparsers(dest="action", required=True)
    new = card.add_parser("new", help="write card.json for every file in a folder")
    new.add_argument("--name", required=True)
    new.add_argument("--description", default="")
    new.add_argument("--source", required=True)
    new.add_argument("--url", required=True)
    new.add_argument("--licence", required=True)
    new.add_argument("--non-commercial", action="store_true")
    new.add_argument("--languages", default="")
    new.add_argument("--split", required=True, choices=cards.SPLITS)
    new.add_argument("--dir", required=True)
    new.add_argument("--notes", default="")
    chk = card.add_parser("check", help="check a card and its files")
    chk.add_argument("card")
    cr = sub.add_parser("crawl", help="the crawler: authors and texts").add_subparsers(dest="action", required=True)
    run = cr.add_parser("run", help="read the seed authors from Wikidata and Wikisource")
    run.add_argument("--seeds", default=str(Path(__file__).resolve().parents[2] / "seeds.toml"))
    run.add_argument("--max-works", type=int, default=8)
    run.add_argument("--max-pages", type=int, default=200)
    run.add_argument("--minutes", type=float, default=30)
    run.add_argument("--offline", action="store_true", help="only what is already in the cache")
    run.add_argument("--force", action="store_true", help="ignore the lock of another crawl")
    cr.add_parser("status", help="what the library holds")
    a = p.parse_args(argv)
    if a.command == "doctor":
        return doctor()
    if a.command == "crawl":
        if a.action == "run":
            return crawl_run(a)
        print(crawl.status(library_dir()))
        return 0
    if a.action == "new":
        return card_new(a)
    return card_check(a)


if __name__ == "__main__":
    raise SystemExit(main())
