"""Tests of the crawler with recorded answers: no network is used.

Run: uv run --project tools python -m unittest discover -s tools/tests
"""

import json
import tempfile
import unittest
import urllib.parse
from pathlib import Path

from monade_tools import cards, crawl, extract, store, wiki
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
            de, la = "http://www.wikidata.org/entity/Q188", "http://www.wikidata.org/entity/Q397"
            if "wdt:P737 wd:" in text:  # whom an author influenced
                if "wd:Q5879" in text:
                    return json.dumps(sparql([{"x": uri("Q7"), "xLabel": "Thing"}])).encode()
                return json.dumps(sparql([])).encode()
            if "wdt:P31 wd:Q5 ." in text and "VALUES" in text:  # persons
                rows = []
                if "wd:Q5879" in text:
                    for infl, label in (("Q1394", "Spinoza"), ("Q9312", "Kant")):
                        rows.append({"p": uri("Q5879"), "pLabel": "Goethe", "died": "1832-03-22T00:00:00Z", "lang": de, "infl": uri(infl), "inflLabel": label})
                if "wd:Q155" in text:
                    rows.append({"p": uri("Q155"), "pLabel": "Italo Calvino", "died": "1985-09-19T00:00:00Z"})
                if "wd:Q1394" in text:
                    rows.append({"p": uri("Q1394"), "pLabel": "Spinoza", "died": "1677-02-21T00:00:00Z", "lang": la})
                if "wd:Q9312" in text:
                    rows.append({"p": uri("Q9312"), "pLabel": "Kant", "died": "1804-02-12T00:00:00Z", "lang": de, "infl": uri("Q8"), "inflLabel": "Hume"})
                return json.dumps(sparql(rows)).encode()
            if "wd:Q5879" in text:  # works of Goethe
                return json.dumps(
                    sparql(
                        [
                            {"work": uri("Q75"), "workLabel": "Faust", "site": "https://de.wikisource.org/", "title": "Faust I"},
                            {"work": uri("Q76"), "workLabel": "Leer", "site": "https://de.wikisource.org/", "title": "Leeres Werk"},
                            {"work": uri("Q75"), "workLabel": "Faust", "site": "https://en.wikisource.org/", "title": "Faust (translation)"},
                        ]
                    )
                ).encode()
            return json.dumps(sparql([])).encode()  # works of the others: none
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

    def test_full_crawl_follows_links_then_resumes(self):
        web = FakeWeb()
        root = self.dir / "biblioteca"
        f = Fetcher(root / "cache", pause=0, opener=web)
        counts = crawl.run(self.seeds, root, fetch=f, year=2026, max_depth=1)
        lib = crawl.Library(root)
        authors = {a["label"]: a for a in lib.rows("authors.jsonl")}
        # Goethe from the seeds; Kant reached by a link (depth 1).
        self.assertEqual(set(authors), {"Goethe", "Kant"})
        self.assertEqual(authors["Kant"]["depth"], 1)
        self.assertEqual(authors["Kant"]["lang"], ["de"])
        self.assertEqual(counts["works"], 1)  # Faust I; the empty work and the translation are not taken
        works = lib.rows("works.jsonl")
        self.assertEqual(works[0]["work_id"], "de-Q75")
        self.assertEqual(works[0]["pages"], 3)
        self.assertTrue(works[0]["path"].endswith(".jsonl.zst"))
        self.assertEqual(works[0]["source"], "api")
        texts = list(store.read_texts(root / works[0]["path"]))
        self.assertEqual({t["page"] for t in texts}, {"Faust I/Zueignung", "Faust I/Nacht"})  # index page skipped
        why = {r.get("label", r["what"]): r["why"] for r in lib.rows("refused.jsonl")}
        self.assertEqual(why["Italo Calvino"], "protected: died 1985")
        self.assertEqual(why["Spinoza"], "did not write in it, de or en")  # Latin and Dutch
        self.assertEqual(why["Thing"], "not a person on Wikidata")
        self.assertEqual(why["de-Q76"], "no text found")
        # Depth 1: Kant's own link (Hume) is in the frontier at depth 2, not visited.
        frontier = {x["label"]: x["depth"] for x in lib.rows("frontier.jsonl")}
        self.assertEqual(frontier["Hume"], 2)
        self.assertNotIn("Hume", authors)
        edges = lib.rows("edges.jsonl")
        self.assertIn({"from": "Q5879", "rel": "influenced_by", "to": "Q1394", "from_label": "Goethe", "to_label": "Spinoza"}, edges)
        self.assertIn({"from": "Q5879", "rel": "influenced", "to": "Q7", "from_label": "Goethe", "to_label": "Thing"}, edges)
        self.assertEqual(len(lib.rows("lessons.jsonl")), 1)
        lock = json.loads((root / "seeds.lock.json").read_text(encoding="utf-8"))
        self.assertEqual(lock["Calvino"]["qid"], "Q155")  # Q1 is not a person
        self.assertEqual(cards.problems(cards.read(root / "card.json"), root), [])
        st = crawl.status(root)
        self.assertIn("2 taken", st)
        self.assertNotIn("Goethe", st)  # no names unless asked
        self.assertIn("Goethe", crawl.status(root, names=True))
        # A second run with a deeper limit goes on from the frontier, not from the seeds.
        before = len(web.calls)
        refused = len(lib.rows("refused.jsonl"))
        crawl.run(self.seeds, root, fetch=Fetcher(root / "cache", pause=0, opener=web), year=2026, max_depth=2)
        new_calls = web.calls[before:]
        self.assertFalse(any("wbsearchentities" in c for c in new_calls))  # seeds not searched again
        self.assertTrue(any("Q8" in urllib.parse.unquote(c) for c in new_calls))  # Hume asked
        self.assertEqual(len(lib.rows("works.jsonl")), 1)
        self.assertEqual(sum(1 for a in lib.rows("authors.jsonl") if a["label"] == "Goethe"), 1)
        self.assertEqual(len(lib.rows("refused.jsonl")), refused + 1)  # Hume: not a person here

    def test_old_library_gets_its_links(self):
        # A library of v0.6.0: authors taken, no frontier. The next run follows their links.
        web = FakeWeb()
        root = self.dir / "old"
        lib = crawl.Library(root)
        lib.append("authors.jsonl", {"qid": "Q5879", "label": "Goethe", "died": 1832, "lang": ["de"]})
        lib.append("edges.jsonl", {"from": "Q5879", "rel": "influenced_by", "to": "Q1394", "from_label": "Goethe", "to_label": "Spinoza"})
        (root / "seeds.lock.json").write_text(json.dumps({"Goethe": {"qid": "Q5879"}, "Calvino": {"qid": "Q155"}}), encoding="utf-8")
        lib.refuse({"what": "Q155", "why": "protected: died 1985"})
        crawl.run(self.seeds, root, fetch=Fetcher(root / "cache", pause=0, opener=web), year=2026, max_depth=1)
        self.assertEqual([a["label"] for a in lib.rows("authors.jsonl")], ["Goethe", "Kant"])  # Goethe not twice
        self.assertIn("Q5879", {r["qid"] for r in lib.rows("linked.jsonl")})
        keys = [(e["from"], e["rel"], e["to"]) for e in lib.rows("edges.jsonl")]
        self.assertEqual(len(keys), len(set(keys)))  # no edge twice

    def test_not_texts_are_skipped(self):
        self.assertTrue("Opera:La coscienza di Zeno".startswith(crawl.NOT_WORKS))
        self.assertFalse("Allgemeines Deutsches Kommersbuch:217".startswith(crawl.NOT_WORKS))

    def test_ceiling_on_disk_stops(self):
        root = self.dir / "b3"
        counts = crawl.run(self.seeds, root, fetch=Fetcher(root / "cache", pause=0, opener=FakeWeb()), max_gb=0, year=2026)
        self.assertEqual(counts["works"], 0)

    def test_seeds_live_in_the_data_folder(self):
        example = Path(__file__).resolve().parents[1] / "seeds.example.toml"
        root = self.dir / "b4"
        path = crawl.seeds_file(root, example)
        self.assertEqual(path, root / "seeds.toml")
        self.assertEqual(crawl.read_seeds(path), crawl.read_seeds(example))
        path.write_text('[[author]]\nsearch = "X"\nlang = ["en"]\n', encoding="utf-8")
        self.assertEqual(crawl.read_seeds(crawl.seeds_file(root, example))[0]["search"], "X")  # never overwritten

    def test_time_budget_stops(self):
        root = self.dir / "b2"
        counts = crawl.run(self.seeds, root, fetch=Fetcher(root / "cache", pause=0, opener=FakeWeb()), minutes=0, year=2026)
        self.assertEqual(counts["works"], 0)

    def test_example_seeds_are_valid(self):
        seeds = crawl.read_seeds(Path(__file__).resolve().parents[1] / "seeds.example.toml")
        self.assertTrue(seeds)
        self.assertTrue(all(set(s["lang"]) <= {"it", "de", "en"} for s in seeds))


if __name__ == "__main__":
    unittest.main()
