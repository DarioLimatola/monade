"""Tests of the dataset cards. Run: uv run --project tools python -m unittest discover -s tools/tests"""

import tempfile
import unittest
from pathlib import Path

from monade_tools import cards


class Cards(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        (self.root / "a.txt").write_text("Considerate la vostra semenza", encoding="utf-8")
        (self.root / "sub").mkdir()
        (self.root / "sub" / "b.txt").write_text("fatti non foste", encoding="utf-8")

    def tearDown(self):
        self.tmp.cleanup()

    def card(self):
        return cards.new_card(
            name="test",
            description="two files",
            source="here",
            url="https://example.org",
            licence="public domain",
            non_commercial=False,
            languages=["it"],
            split="train",
            root=self.root,
            files=list(self.root.rglob("*.txt")),
            today="2026-11-19",
        )

    def test_blake3_known_value(self):
        p = self.root / "empty"
        p.write_bytes(b"")
        # Official BLAKE3 test vector for the empty input.
        self.assertEqual(
            cards.file_hash(p),
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262",
        )

    def test_round_trip_and_check(self):
        c = self.card()
        self.assertEqual([f["path"] for f in c["files"]], ["a.txt", "sub/b.txt"])
        out = self.root / "card.json"
        cards.write(c, out)
        self.assertEqual(cards.read(out), c)
        self.assertEqual(cards.problems(c, self.root), [])

    def test_changes_are_found(self):
        c = self.card()
        (self.root / "a.txt").write_text("changed", encoding="utf-8")
        self.assertIn("file changed: a.txt", cards.problems(c, self.root))
        c["licence"] = ""
        self.assertTrue(any("licence" in p for p in cards.problems(c)))

    def test_bad_split_refused(self):
        with self.assertRaises(ValueError):
            cards.new_card(
                name="x", description="", source="", url="", licence="CC0",
                non_commercial=False, languages=[], split="secret",
                root=self.root, files=list(self.root.rglob("*.txt")),
            )


if __name__ == "__main__":
    unittest.main()
