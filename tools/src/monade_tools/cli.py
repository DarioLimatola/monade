"""`monade-tools`: the command line of the Python tools.

    monade-tools doctor
    monade-tools card new --name N --source S --url U --licence L --split train
                          --languages it,en --dir FOLDER [--description D]
                          [--non-commercial] [--notes T]
    monade-tools card check CARD.json
"""

from __future__ import annotations

import argparse
import os
import platform
import shutil
import subprocess
import sys
from pathlib import Path

from . import __version__, cards


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
    a = p.parse_args(argv)
    if a.command == "doctor":
        return doctor()
    if a.action == "new":
        return card_new(a)
    return card_check(a)


if __name__ == "__main__":
    raise SystemExit(main())
