"""Tests of the crawler with recorded answers: no network is used.

Run: uv run --project tools python -m unittest discover -s tools/tests
"""

import json
import tempfile
import unittest
import urllib.parse
from pathlib import Path

from monade_tools import cards, crawl, extract, wiki
from monade_tools.net import Fetcher

PAGE_HTML = """
<div class="ws-noexport header_box">Navigation: previous | next</div>
<table><tr><td>Info box</td></tr></table>
<h2>Erster Teil<span class="mw-editsection">[edit]</span></h2>
<p>Habe nun, ach! Philosophie,<sup class="reference">[1]</sup> Juristerei und Medizin.</p>
<span class="pagenum">[12]</span>
<div class="poem"><p>Ihr naht euch wieder,<br>schwankende Gestalten,<br>die früh sich einst</p></div>
<div id="toc">Inhalt 1 2 3</div>
<p style="display:none">hidden</p>
<ol class="references"><li>a footnote</li></ol>
<p>Café und  Straße.</p>
"""


def sparql(rows):
    return {"results": {"bindings": [{k: {"value": v} for k, v in r.items()} for r in rows]}}


def uri(q):
    return f"http://www.wikidata.org/entity/{q}"


class FakeWeb:
    """Answers the crawler's requests from a table, and counts them."""

    def __init__(self):
        self.calls = []

    def __call__(self, url, headers):
        self.calls.append(url)
        assert "monade-crawler" in headers["User-Agent"]
        u = urllib.parse.urlsplit(url)
        q = dict(urllib.parse.parse_qsl(u.query))
        if u.hostname == "www.wikidata.org":
            ids = {"Goethe": ["Q5879"], "Calvino": ["Q1", "Q155"]}.get(q["search"], [])
            return json.dumps({"search": [{"id": i} for i in ids]}).encode()
        if u.hostname == "query.wikidata.org":
            text = q["query"]
            if "wdt:P31 wd:Q5" in text and "Q5879" in text:
                return json.dumps(
                    sparql(
                        [
                            {"p": uri("Q5879"), "pLabel": "Goethe", "died": "1832-03-22T00:00:00Z", "infl": uri("Q1394"), "inflLabel": "Spinoza"},
                            {"p": uri("Q5879"), "pLabel": "Goethe", "died": "1832-03-22T00:00:00Z", "infl": uri("Q9312"), "inflLabel": "Kant"},
                        ]
                    )
                ).encode()
            if "wdt:P31 wd:Q5" in text and "Q155" in text:
                # Q1 is not a person (only Q155 comes back); Q155 died in 1985.
                return json.dumps(sparql([{"p": uri("Q155"), "pLabel": "Italo Calvino", "died": "1985-09-19T00:00:00Z"}])).encode()
            if "wd:Q5879" in text:
                return json.dumps(
                    sparql(
                        [
                            {"work": uri("Q75"), "workLabel": "Faust", "site": "https://de.wikisource.org/", "title": "Faust I"},
                            {"work": uri("Q76"), "workLabel": "Leer", "site": "https://de.wikisource.org/", "title": "Leeres Werk"},
                            {"work": uri("Q75"), "workLabel": "Faust", "site": "https://en.wikisource.org/", "title": "Faust (translation)"},
                        ]
                    )
                ).encode()
        if u.hostname == "de.wikisource.org":
            if q["action"] == "query":
                pages = ["Faust I/Zueignung", "Faust I/Nacht"] if q["apprefix"] == "Faust_I/" else []
                return json.dumps({"query": {"allpages": [{"title": t} for t in pages]}}).encode()
            if q["page"] == "Faust I":
                return json.dumps({"parse": {"text": "<ul><li>Zueignung</li><li>Nacht</li></ul>"}}).encode()
            if q["page"].startswith("Faust I/"):
                return json.dumps({"parse": {"text": PAGE_HTML}}).encode()
            return json.dumps({"parse": {"text": ""}}).encode()
        raise AssertionError(f"unexpected request {url}")


class Crawl(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self.tmp.name)
        self.seeds = self.dir / "seeds.toml"
        self.seeds.write_text(
            '[[author]]\nsearch = "Goethe"\nlang = ["de"]\n\n[[author]]\nsearch = "Calvino"\nlang = ["it"]\n',
            encoding="utf-8",
        )

    def tearDown(self):
        self.tmp.cleanup()

    def test_public_domain_rule(self):
        self.assertTrue(crawl.public_domain(1955, 2026))  # Thomas Mann, free since 1 January 2026
        self.assertFalse(crawl.public_domain(1956, 2026))
        self.assertFalse(crawl.public_domain(1985, 2026))  # Calvino
        self.assertFalse(crawl.public_domain(None, 2026))  # no death date: may be alive

    def test_extract_keeps_text_drops_apparatus(self):
        p = extract.paragraphs(PAGE_HTML)
        joined = "\n".join(p)
        self.assertIn("Erster Teil", p)
        self.assertIn("Habe nun, ach! Philosophie, Juristerei und Medizin.", p)
        self.assertIn("Ihr naht euch wieder,\nschwankende Gestalten,\ndie früh sich einst", p)
        self.assertIn("Café und Straße.", p)  # NFC, spaces collapsed
        for gone in ("[edit]", "[1]", "[12]", "Navigation", "Info box", "Inhalt", "hidden", "footnote"):
            self.assertNotIn(gone, joined)
        self.assertEqual(extract.paragraphs("  "), [])

    def test_wikidata_parsing(self):
        persons = wiki.parse_persons(
            sparql([{"p": uri("Q1"), "pLabel": "A", "died": "-0399-01-01T00:00:00Z"}])
        )
        self.assertEqual(persons["Q1"]["died"], -399)
        works = wiki.parse_works(
            sparql(
                [
                    {"work": uri("Q2"), "workLabel": "B", "site": "https://it.wikisource.org/", "title": "Zeta"},
                    {"work": uri("Q3"), "workLabel": "C", "site": "https://it.wikisource.org/", "title": "Alfa"},
                    {"work": uri("Q3"), "workLabel": "C", "site": "https://it.wikisource.org/", "title": "Alfa"},
                ]
            )
        )
        self.assertEqual([w["title"] for w in works], ["Alfa", "Zeta"])
        self.assertEqual(works[0]["lang"], "it")

    def test_full_crawl_then_resume(self):
        web = FakeWeb()
        root = self.dir / "biblioteca"
        f = Fetcher(root / "cache", pause=0, opener=web)
        counts = crawl.run(self.seeds, root, fetch=f, year=2026)
        self.assertEqual(counts["authors"], 1)
        self.assertEqual(counts["works"], 1)  # Faust I; the empty work and the translation are not taken
        lib = crawl.Library(root)
        works = lib.rows("works.jsonl")
        self.assertEqual(works[0]["work_id"], "de-Q75")
        self.assertEqual(works[0]["pages"], 3)
        texts = [json.loads(l) for l in (root / works[0]["path"]).read_text(encoding="utf-8").splitlines()]
        self.assertEqual({t["page"] for t in texts}, {"Faust I/Zueignung", "Faust I/Nacht"})  # index page skipped
        refused = lib.rows("refused.jsonl")
        self.assertTrue(any("1985" in r["why"] for r in refused))  # Calvino
        self.assertTrue(any(r["why"] == "no text found" for r in refused))
        edges = lib.rows("edges.jsonl")
        self.assertIn({"from": "Q5879", "rel": "influenced_by", "to": "Q1394", "from_label": "Goethe", "to_label": "Spinoza"}, edges)
        self.assertEqual(len(lib.rows("lessons.jsonl")), 1)
        lock = json.loads((root / "seeds.lock.json").read_text(encoding="utf-8"))
        self.assertEqual(lock["Calvino"]["qid"], "Q155")  # Q1 is not a person
        card = cards.read(root / "card.json")
        self.assertEqual(cards.problems(card, root), [])
        self.assertIn("Goethe", crawl.status(root))
        # A second run: nothing new, nothing asked twice, no duplicate rows.
        before = len(web.calls)
        again = crawl.run(self.seeds, root, fetch=Fetcher(root / "cache", pause=0, opener=web), year=2026)
        self.assertEqual(again["works"], 0)
        self.assertEqual(len(web.calls), before)
        self.assertEqual(len(lib.rows("works.jsonl")), 1)
        self.assertEqual(len(lib.rows("authors.jsonl")), 1)
        self.assertEqual(len(lib.rows("refused.jsonl")), len(refused))  # refusals recorded once

    def test_time_budget_stops(self):
        root = self.dir / "b2"
        counts = crawl.run(self.seeds, root, fetch=Fetcher(root / "cache", pause=0, opener=FakeWeb()), minutes=0, year=2026)
        self.assertEqual(counts["works"], 0)

    def test_seeds_of_the_project_are_valid(self):
        seeds = crawl.read_seeds(Path(__file__).resolve().parents[1] / "seeds.toml")
        self.assertEqual(len(seeds), 10)
        self.assertTrue(all(set(s["lang"]) <= {"it", "de", "en"} for s in seeds))


if __name__ == "__main__":
    unittest.main()
