"""Tests of the store (compression, index, search) and of the dump mirror. No network.

Run: uv run --project tools python -m unittest discover -s tools/tests
"""

import gzip
import io
import json
import tempfile
import unittest
import urllib.parse
from pathlib import Path

from monade_tools import crawl, dump, store
from monade_tools.net import Fetcher

from test_crawl import FakeWeb

ROWS = [
    {"page": "Faust I/Nacht", "i": 0, "text": "Habe nun, ach! Philosophie, Juristerei und Medizin."},
    {"page": "Faust I/Nacht", "i": 1, "text": "Da steh ich nun, ich armer Tor! Und bin so klug als wie zuvor."},
    {"page": "Faust I/Nacht", "i": 2, "text": "Heiße Magister, heiße Doktor gar."},
]


class Store(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)

    def tearDown(self):
        self.tmp.cleanup()

    def test_compressed_round_trip(self):
        p = self.root / "texts/de/de-Q75.jsonl.zst"
        self.assertEqual(store.write_texts(p, ROWS), (3, sum(len(r["text"]) for r in ROWS)))
        self.assertEqual(list(store.read_texts(p)), ROWS)
        self.assertEqual(p.read_bytes()[:4], b"\x28\xb5\x2f\xfd")  # the zstd magic number
        # Nothing written: no file left behind.
        empty = self.root / "texts/de/none.jsonl.zst"
        self.assertEqual(store.write_texts(empty, []), (0, 0))
        self.assertFalse(empty.exists())

    def test_index_and_search(self):
        store.write_texts(self.root / "texts/de/de-Q75.jsonl.zst", ROWS)
        idx = store.Index(self.root)
        self.assertEqual(idx.add("de-Q75", "texts/de/de-Q75.jsonl.zst"), 3)
        self.assertEqual(idx.add("de-Q75", "texts/de/de-Q75.jsonl.zst"), 0)  # once
        hits = idx.search("Tor")
        self.assertEqual([(h["i"], h["text"]) for h in hits], [(1, ROWS[1]["text"])])
        self.assertEqual(len(idx.search("heiße")), 1)  # case folded
        self.assertEqual(len(idx.search("Medizín")), 1)  # accents folded
        self.assertEqual(len(idx.search("Magister OR Medizin")), 2)
        self.assertEqual(len(idx.search('"armer Tor"')), 1)
        self.assertEqual(idx.search("Gretchen"), [])
        with self.assertRaises(ValueError):
            idx.search('"unclosed')
        idx.close()

    def test_pack_migrates_an_older_library(self):
        lib = crawl.Library(self.root)
        old = self.root / "texts/de/de-Q75.jsonl"
        old.parent.mkdir(parents=True)
        old.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in ROWS) * 50, encoding="utf-8")
        lib.append("works.jsonl", {"work_id": "de-Q75", "lang": "de", "path": "texts/de/de-Q75.jsonl", "blake3": "x", "paragraphs": 150, "chars": 1})
        lib.append("lessons.jsonl", {"work_id": "de-Q75", "lang": "de", "path": "texts/de/de-Q75.jsonl"})
        c = crawl.pack(self.root)
        self.assertEqual(c["packed"], 1)
        self.assertLess(c["after"] * 3, c["before"])  # at least three times smaller
        self.assertEqual(c["indexed"], 150)
        self.assertFalse(old.exists())
        self.assertEqual(lib.rows("works.jsonl")[0]["path"], "texts/de/de-Q75.jsonl.zst")
        self.assertEqual(lib.rows("lessons.jsonl")[0]["path"], "texts/de/de-Q75.jsonl.zst")
        self.assertEqual(crawl.pack(self.root)["packed"], 0)  # twice is harmless


def bulk(docs):
    out = io.BytesIO()
    with gzip.open(out, "wt", encoding="utf-8") as f:
        for i, d in enumerate(docs):
            f.write(json.dumps({"index": {"_id": str(i)}}) + "\n")
            f.write(json.dumps(d, ensure_ascii=False) + "\n")
    return out.getvalue()


LONG = "Erster Satz des Kapitels. " * 80


class Dump(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self.tmp.name)

    def tearDown(self):
        self.tmp.cleanup()

    def test_dates_newest_first(self):
        html = '<a href="20261109/">20261109/</a> <a href="20261116/">x</a> <a href="../">..</a>'
        self.assertEqual(dump.dates(html), ["20261116", "20261109"])

    def test_chunks_cut_at_sentence_ends(self):
        parts = dump.chunks(LONG, 300)
        self.assertTrue(all(len(p) <= 300 for p in parts))
        self.assertTrue(all(p.endswith(".") for p in parts))
        self.assertEqual(" ".join(parts), LONG.strip())
        self.assertEqual(dump.chunks("kurz"), ["kurz"])
        self.assertEqual(dump.chunks(""), [])

    def test_download_resumes(self):
        data = bytes(range(256)) * 9000

        class Resp:
            def __init__(self, body, status):
                self.body, self.status = io.BytesIO(body), status

            def read(self, n):
                return self.body.read(n)

            def __enter__(self):
                return self

            def __exit__(self, *a):
                return False

        dest = self.dir / "x.json.gz"
        part = self.dir / "x.json.gz.part"
        part.write_bytes(data[:1000])
        asked = []

        def opener(url, start):
            asked.append(start)
            return Resp(data[start:], 206)

        dump.download("u", dest, opener)
        self.assertEqual(asked, [1000])
        self.assertEqual(dest.read_bytes(), data)
        self.assertFalse(part.exists())
        dump.download("u", dest, opener)  # already there: nothing asked
        self.assertEqual(asked, [1000])

    def test_mirror_and_crawl_from_it(self):
        docs = [
            {"namespace": 0, "title": "Faust I", "text": "Inhalt", "wikibase_item": "Q75"},
            {"namespace": 0, "title": "Faust I/Nacht", "text": LONG},
            {"namespace": 0, "title": "Faust I/Zueignung", "text": "Ihr naht euch wieder, schwankende Gestalten."},
            {"namespace": 0, "title": "Faust II", "text": "Anderes Werk."},
            {"namespace": 104, "title": "Seite:Faust.djvu/1", "text": "a scan page"},
        ]
        gz = self.dir / "dewikisource-20261116-cirrussearch-content.json.gz"
        gz.write_bytes(bulk(docs))
        out = dump.mirror_path(self.dir, "de")
        self.assertEqual(dump.build(gz, out), 4)  # namespace 104 left out
        m = dump.Mirror(out)
        self.assertEqual(m.subpages("Faust I"), ["Faust I/Nacht", "Faust I/Zueignung"])  # not Faust II
        self.assertGreater(len(m.paragraphs("Faust I/Nacht")), 1)
        self.assertEqual(m.paragraphs("Nothing"), [])
        # The crawler takes the chapters from the mirror: no Wikisource request.
        seeds = self.dir / "seeds.toml"
        seeds.write_text('[[author]]\nsearch = "Goethe"\nlang = ["de"]\n', encoding="utf-8")
        web = FakeWeb()
        root = self.dir / "biblioteca"
        crawl.run(seeds, root, fetch=Fetcher(root / "cache", pause=0, opener=web), year=2026,
                  max_depth=0, mirrors={"de": m})
        hosts = {urllib.parse.urlsplit(c).hostname for c in web.calls}
        self.assertFalse(any(h.endswith("wikisource.org") for h in hosts), hosts)
        works = crawl.Library(root).rows("works.jsonl")
        self.assertEqual([(w["work_id"], w["source"], w["pages"]) for w in works], [("de-Q75", "dump", 3)])
        idx = store.Index(root)
        self.assertEqual(len(idx.search("Gestalten")), 1)
        idx.close()
        m.close()


if __name__ == "__main__":
    unittest.main()
