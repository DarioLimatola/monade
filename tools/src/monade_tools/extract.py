"""From the HTML of a Wikisource page to clean paragraphs of text.

Removed: navigation boxes, headers and footers of the page, edit links, references
and footnote markers, page numbers, tables of contents, hidden elements. Kept:
paragraphs, verses (one paragraph per stanza, lines joined with newlines), headings.
Text is normalised to Unicode NFC and whitespace is collapsed.
"""

from __future__ import annotations

import re
import unicodedata

from lxml import html as lhtml

#: Elements removed with their content (classes and ids used by Wikisource).
DROP_XPATH = " | ".join(
    [
        "//table",
        "//style",
        "//script",
        "//sup[contains(@class,'reference')]",
        "//ol[contains(@class,'references')]",
        "//*[contains(@class,'ws-noexport')]",
        "//*[contains(@class,'noprint')]",
        "//*[contains(@class,'mw-editsection')]",
        "//*[contains(@class,'pagenum')]",
        "//*[contains(@class,'ws-pagenum')]",
        "//*[contains(@class,'header_box')]",
        "//*[contains(@class,'navbox')]",
        "//*[@id='toc']",
        "//*[contains(@class,'toc')]",
        "//*[contains(@style,'display:none')]",
        "//*[contains(@style,'display: none')]",
    ]
)

BLOCKS = ("p", "h1", "h2", "h3", "h4", "h5", "h6", "dd", "li", "blockquote")


def normalise(text: str) -> str:
    """NFC, no soft hyphens, spaces collapsed (newlines kept inside verse)."""
    text = unicodedata.normalize("NFC", text).replace("­", "")
    lines = [re.sub(r"[ \t ]+", " ", line).strip() for line in text.split("\n")]
    return "\n".join(line for line in lines if line)


def paragraphs(page_html: str) -> list[str]:
    """The paragraphs of a page, in order; empty if there is no text."""
    if not page_html.strip():
        return []
    root = lhtml.fromstring(f"<div>{page_html}</div>")
    for el in root.xpath(DROP_XPATH):
        el.drop_tree()
    out = []
    # Verses: <div class="poem"><p>line<br>line</p></div> -> one paragraph per stanza.
    for br in root.iter("br"):
        br.tail = "\n" + (br.tail or "")
    for el in root.iter(*BLOCKS):
        # Only innermost blocks, so nothing is counted twice.
        if any(child.tag in BLOCKS for child in el.iterdescendants()):
            continue
        text = normalise(el.text_content())
        if len(text) >= 2:
            out.append(text)
    return out


def is_index_page(paras: list[str]) -> bool:
    """A page that is only a list of chapters (no prose of its own)."""
    words = sum(len(p.split()) for p in paras)
    return words < 40
