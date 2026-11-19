"""Polite fetching: one request at a time per host, a pause between requests, an honest
User-Agent, a cache on disk (so a run can stop and resume without asking twice).

Wikimedia APIs (Wikidata, Wikisource) are used as their API etiquette asks: serial
requests, a descriptive User-Agent with a contact, `maxlag` on the MediaWiki API.
robots.txt is checked for every other site.
"""

from __future__ import annotations

import hashlib
import json
import os
import time
import urllib.error
import urllib.parse
import urllib.request
import urllib.robotparser
from pathlib import Path
from typing import Callable

from . import __version__

#: Hosts reached only through their documented APIs (API etiquette instead of robots.txt).
API_HOSTS = (
    "www.wikidata.org",
    "query.wikidata.org",
    "it.wikisource.org",
    "de.wikisource.org",
    "en.wikisource.org",
)


def user_agent() -> str:
    """The User-Agent: name, version, and a contact (MONADE_CONTACT, or the project)."""
    contact = os.environ.get("MONADE_CONTACT", "https://github.com/DarioLimatola/monade")
    return f"monade-crawler/{__version__} ({contact}) python-urllib"


class FetchError(Exception):
    """A request that failed or was refused (robots.txt)."""


class Fetcher:
    """Fetches URLs politely, with a cache in `cache_dir`.

    `get` returns the body as text. `opener` can be replaced in tests.
    """

    def __init__(
        self,
        cache_dir: Path,
        pause: float = 1.0,
        opener: Callable[[str, dict], bytes] | None = None,
        offline: bool = False,
    ):
        self.cache_dir = cache_dir
        self.pause = pause
        self.opener = opener or self._urlopen
        self.offline = offline
        self.last: dict[str, float] = {}
        self.robots: dict[str, urllib.robotparser.RobotFileParser] = {}
        self.requests = 0
        self.cache_hits = 0

    def _cache_path(self, url: str) -> Path:
        h = hashlib.sha256(url.encode("utf-8")).hexdigest()
        return self.cache_dir / h[:2] / f"{h}.bin"

    @staticmethod
    def _urlopen(url: str, headers: dict) -> bytes:
        req = urllib.request.Request(url, headers=headers)
        with urllib.request.urlopen(req, timeout=60) as r:
            return r.read()

    def _allowed(self, url: str) -> bool:
        parts = urllib.parse.urlsplit(url)
        if parts.hostname in API_HOSTS:
            return True
        base = f"{parts.scheme}://{parts.netloc}"
        rp = self.robots.get(base)
        if rp is None:
            rp = urllib.robotparser.RobotFileParser()
            try:
                body = self.opener(base + "/robots.txt", {"User-Agent": user_agent()})
                rp.parse(body.decode("utf-8", "replace").splitlines())
            except Exception:
                rp.parse([])  # no robots.txt: allowed
            self.robots[base] = rp
        return rp.can_fetch(user_agent(), url)

    def get(self, url: str) -> str:
        """The body of `url` (from the cache if already fetched)."""
        path = self._cache_path(url)
        if path.is_file():
            self.cache_hits += 1
            return path.read_bytes().decode("utf-8")
        if self.offline:
            raise FetchError(f"offline and not cached: {url}")
        if not self._allowed(url):
            raise FetchError(f"refused by robots.txt: {url}")
        host = urllib.parse.urlsplit(url).hostname or ""
        wait = self.last.get(host, 0.0) + self.pause - time.monotonic()
        if wait > 0:
            time.sleep(wait)
        headers = {"User-Agent": user_agent(), "Accept-Encoding": "identity"}
        if host == "query.wikidata.org":
            headers["Accept"] = "application/sparql-results+json"
        tries = 0
        while True:
            try:
                body = self.opener(url, headers)
                break
            except urllib.error.HTTPError as e:
                tries += 1
                # 429 (too many requests) and 503 (maxlag): wait and retry, at most 4 times.
                if e.code in (429, 503) and tries <= 4:
                    retry = e.headers.get("Retry-After") if e.headers else None
                    time.sleep(float(retry) if retry and retry.isdigit() else 5.0 * tries)
                    continue
                raise FetchError(f"HTTP {e.code} for {url}") from e
            except (urllib.error.URLError, TimeoutError, OSError) as e:
                raise FetchError(f"{e} for {url}") from e
            finally:
                self.last[host] = time.monotonic()
        self.requests += 1
        path.parent.mkdir(parents=True, exist_ok=True)
        tmp = path.with_suffix(".tmp")
        tmp.write_bytes(body)
        tmp.replace(path)  # a crash never leaves half a file in the cache
        return body.decode("utf-8")

    def get_json(self, url: str) -> dict:
        """The body of `url`, parsed as JSON."""
        return json.loads(self.get(url))
