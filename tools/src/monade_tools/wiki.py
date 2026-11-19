"""Wikidata (who wrote what, who influenced whom) and Wikisource (the texts).

Only URL building and parsing of the answers live here; fetching is done by
`net.Fetcher`, so everything can be tested with recorded answers.
"""

from __future__ import annotations

import urllib.parse

WIKIDATA_API = "https://www.wikidata.org/w/api.php"
SPARQL = "https://query.wikidata.org/sparql"


def wikisource_api(lang: str) -> str:
    return f"https://{lang}.wikisource.org/w/api.php"


def search_url(name: str) -> str:
    """Wikidata items whose label matches `name` (first 7)."""
    q = {
        "action": "wbsearchentities",
        "search": name,
        "language": "en",
        "type": "item",
        "limit": "7",
        "format": "json",
    }
    return WIKIDATA_API + "?" + urllib.parse.urlencode(q)


def sparql_url(query: str) -> str:
    return SPARQL + "?" + urllib.parse.urlencode({"query": query, "format": "json"})


def search_ids(answer: dict) -> list[str]:
    """The item ids of a wbsearchentities answer, in order."""
    return [x["id"] for x in answer.get("search", []) if "id" in x]


def person_query(qids: list[str]) -> str:
    """For each candidate: is it a human, its label, death year, influences."""
    values = " ".join(f"wd:{q}" for q in qids)
    return f"""SELECT ?p ?pLabel ?died ?infl ?inflLabel WHERE {{
  VALUES ?p {{ {values} }}
  ?p wdt:P31 wd:Q5 .
  OPTIONAL {{ ?p wdt:P570 ?died . }}
  OPTIONAL {{ ?p wdt:P737 ?infl . }}
  SERVICE wikibase:label {{ bd:serviceParam wikibase:language "it,de,en". }}
}}"""


def works_query(qid: str, langs: list[str]) -> str:
    """Works by `qid` (author of, or notable work) that have a Wikisource page in one
    of `langs`, with the title of that page."""
    sites = " ".join(f"<https://{l}.wikisource.org/>" for l in langs)
    return f"""SELECT DISTINCT ?work ?workLabel ?site ?title WHERE {{
  {{ ?work wdt:P50 wd:{qid} . }} UNION {{ wd:{qid} wdt:P800 ?work . }}
  ?page schema:about ?work ; schema:isPartOf ?site ; schema:name ?title .
  VALUES ?site {{ {sites} }}
  SERVICE wikibase:label {{ bd:serviceParam wikibase:language "it,de,en". }}
}}"""


def _value(row: dict, key: str) -> str:
    return row.get(key, {}).get("value", "")


def _qid(uri: str) -> str:
    return uri.rsplit("/", 1)[-1]


def parse_persons(answer: dict) -> dict[str, dict]:
    """qid -> {label, died (year or None), influences [(qid, label)]}."""
    out: dict[str, dict] = {}
    for row in answer.get("results", {}).get("bindings", []):
        q = _qid(_value(row, "p"))
        p = out.setdefault(q, {"label": _value(row, "pLabel"), "died": None, "influences": []})
        died = _value(row, "died")
        if died:
            # "1832-03-22T00:00:00Z"; a negative year means BCE.
            year = int(died[: died.index("-", 1)])
            p["died"] = year if p["died"] is None else min(p["died"], year)
        infl = _value(row, "infl")
        if infl:
            pair = (_qid(infl), _value(row, "inflLabel"))
            if pair not in p["influences"]:
                p["influences"].append(pair)
    return out


def parse_works(answer: dict) -> list[dict]:
    """[{work, label, lang, title}] sorted by language then title."""
    out = []
    seen = set()
    for row in answer.get("results", {}).get("bindings", []):
        site = _value(row, "site")  # https://de.wikisource.org/
        lang = urllib.parse.urlsplit(site).hostname.split(".")[0] if site else ""
        item = (_qid(_value(row, "work")), lang, _value(row, "title"))
        if item in seen or not item[2]:
            continue
        seen.add(item)
        out.append({"work": item[0], "label": _value(row, "workLabel"), "lang": lang, "title": item[2]})
    return sorted(out, key=lambda w: (w["lang"], w["title"]))


def subpages_url(lang: str, title: str) -> str:
    """The subpages (chapters) of a Wikisource work: `Title/...`."""
    q = {
        "action": "query",
        "list": "allpages",
        "apprefix": title.replace(" ", "_") + "/",
        "apnamespace": "0",
        "aplimit": "max",
        "format": "json",
        "formatversion": "2",
        "maxlag": "5",
    }
    return wikisource_api(lang) + "?" + urllib.parse.urlencode(q)


def parse_subpages(answer: dict) -> list[str]:
    return [p["title"] for p in answer.get("query", {}).get("allpages", [])]


def page_url(lang: str, title: str) -> str:
    """The rendered HTML of one Wikisource page."""
    q = {
        "action": "parse",
        "page": title,
        "prop": "text",
        "redirects": "1",
        "disablelimitreport": "1",
        "disableeditsection": "1",
        "format": "json",
        "formatversion": "2",
        "maxlag": "5",
    }
    return wikisource_api(lang) + "?" + urllib.parse.urlencode(q)


def parse_page(answer: dict) -> str:
    """The HTML of a parse answer ('' if the page does not exist)."""
    return answer.get("parse", {}).get("text", "")


def page_link(lang: str, title: str) -> str:
    return f"https://{lang}.wikisource.org/wiki/" + urllib.parse.quote(title.replace(" ", "_"))
