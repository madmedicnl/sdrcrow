#!/usr/bin/env python3
"""Build the quick-start PDFs from their Markdown, branded as SDRCrow.

    tools/build-quickstart-pdfs.py            # everything under docs/
    tools/build-quickstart-pdfs.py cb ft8-11m # only those documents

Every `<name>.<lang>.md` in `docs/` gets a `<name>.<lang>.pdf` beside it: a
cover with the emblem, the title, the language and the source file, then the
document itself on a light page with the program's blue and gold as marks. The
stylesheet is `docs/quickstart-pdf.css`; it is embedded in the page, so the PDF
is one file with no side dependencies.

Needs `weasyprint` (WeasyPrint's CLI) and `markdown` — the Python module:

    uv tool install weasyprint
    python3 -m pip --user install markdown     # or the distribution's package

Both are development-only: nothing in the program or the build of it uses them.
"""

import html
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

import markdown

ROOT = pathlib.Path(__file__).resolve().parent.parent
DOCS = ROOT / "docs"
REPO = "https://github.com/madmedicnl/sdrcrow"

# The documents that carry a PDF, and what the running head calls each one.
KINDS = {
    "cb-quickstart": "CB · 11 m",
    "ft8-11m-quickstart": "FT8 · FT4 · 11 m",
    "hfweer-quickstart": "HF weather",
    "listening-quickstart": "Listening",
    "pi-zero-2w-swl": "Pi Zero 2 W · SWL",
    "qo100-quickstart": "QO-100",
    "sstv-11m-quickstart": "SSTV · 11 m",
}

LANGUAGES = {
    "en": (
        "English",
        "Quick start",
        "Software-defined radio for 11 m CB, shortwave listening and decoding.",
    ),
    "fr": (
        "Français",
        "Démarrage rapide",
        "Radio définie par logiciel pour la bande 11 m, l'écoute OC et le décodage.",
    ),
    "it": (
        "Italiano",
        "Guida rapida",
        "Radio definita dal software per 11 m, ascolto delle onde corte e decodifica.",
    ),
    "nl": (
        "Nederlands",
        "Snel aan de slag",
        "Software-defined radio voor 11 m CB, kortegolf luisteren en decoderen.",
    ),
    "tr": (
        "Türkçe",
        "Hızlı başlangıç",
        "11 m CB, kısa dalga dinleme ve çözümleme için yazılım tanımlı radyo.",
    ),
}

TITLE = re.compile(r"^#\s+(.+?)\s*$", re.MULTILINE)
LANG_OF = re.compile(r"^(?P<stem>.+)\.(?P<lang>[a-z]{2}(?:-[A-Z]{2})?)\.md$")


def links_to_github(text: str) -> str:
    """Point the document's own relative links at the repository."""
    def swap(match: re.Match) -> str:
        target = match.group(2)
        if target.startswith(("http://", "https://", "#", "mailto:")):
            return match.group(0)
        if target.endswith(".md"):
            return f"{match.group(1)}({REPO}/blob/main/docs/{target}{match.group(3)})"
        return match.group(0)

    return re.sub(r"(\]\()([^)\s]+)(\s*\))", swap, text)


def standfirst(body_md: str) -> str:
    """The document's own opening paragraph, for the cover."""
    for block in body_md.split("\n\n"):
        block = block.strip()
        if not block or block[0] in "#>|!-*":
            continue
        text = re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", block)
        text = re.sub(r"[`*_]", "", text)
        text = " ".join(text.split())
        if len(text) > 230:
            text = text[:227].rsplit(" ", 1)[0] + "…"
        return text
    return ""


def split_title(title: str) -> tuple[str, str]:
    """`CB quick-start (SDRCrow — 11 m)` -> the name, and the tail in brackets."""
    match = re.match(r"^(.*?)\s*\((.+)\)\s*$", title)
    if match:
        return match.group(1), match.group(2)
    return title, ""


def cover(
    stem: str,
    lang: str,
    title: str,
    source: str,
    lead: str,
    contents: list[dict],
) -> str:
    head, tail = split_title(title)
    toc = "".join(
        f'<li><a href="#{item["id"]}">{html.escape(item["name"])}</a></li>'
        for item in contents
    )
    name, kicker, tagline = LANGUAGES[lang]
    logo = (ROOT / "packaging" / "icons" / "sdrcrow-256.png").as_uri()
    return f"""
<div class="cover">
  <div class="band"></div>
  <img class="mark" src="{logo}" alt="SDRCrow">
  <div class="wordmark">
    <div class="name">SDR<em>CROW</em></div>
    <div class="tagline">{html.escape(tagline)}</div>
  </div>
  <div class="titles">
    <div class="kicker">{html.escape(kicker)} · {html.escape(KINDS[stem])}</div>
    <h1 class="doctitle">{html.escape(head)}</h1>
    {f'<div class="subtitle">{html.escape(tail)}</div>' if tail else ''}
    <div class="rule"></div>
    <div class="lang">{html.escape(name)}</div>
    <p class="standfirst">{html.escape(lead)}</p>
    {f'<ul class="toc">{toc}</ul>' if toc else ''}
  </div>
  <div class="foot">
    <span class="repo">{REPO.replace("https://", "")}</span> · {html.escape(source)}
  </div>
</div>
"""


def build(md_path: pathlib.Path) -> pathlib.Path:
    match = LANG_OF.match(md_path.name)
    if not match:
        raise SystemExit(f"{md_path.name}: no language in the file name")
    stem, lang = match.group("stem"), match.group("lang")
    if stem not in KINDS:
        raise SystemExit(f"{md_path.name}: {stem} is not in KINDS")
    if lang not in LANGUAGES:
        raise SystemExit(f"{md_path.name}: {lang} is not in LANGUAGES")

    text = md_path.read_text(encoding="utf-8")
    heading = TITLE.search(text)
    title = heading.group(1) if heading else stem
    body_md = TITLE.sub("", text, count=1) if heading else text
    body_md = links_to_github(body_md)

    converter = markdown.Markdown(
        extensions=["extra", "sane_lists", "attr_list", "toc"],
        output_format="html5",
    )
    body = converter.convert(body_md)
    contents = [t for t in converter.toc_tokens if t["level"] == 2]
    style = (DOCS / "quickstart-pdf.css").read_text(encoding="utf-8")
    page = f"""<!DOCTYPE html>
<html lang="{lang}">
<head>
<meta charset="utf-8">
<title>{html.escape(title)} — SDRCrow</title>
<style>
{style}
@page {{
    @top-right {{ content: "{KINDS[stem]}"; }}
    @bottom-left {{ content: "{md_path.name}"; }}
}}
</style>
</head>
<body>
{cover(stem, lang, title, md_path.name, standfirst(body_md), contents)}
<div class="body">
{body}
</div>
</body>
</html>
"""
    work = DOCS / ".pdfbuild"
    work.mkdir(exist_ok=True)
    page_path = work / f"{stem}.{lang}.html"
    page_path.write_text(page, encoding="utf-8")
    out = md_path.with_suffix(".pdf")
    subprocess.run(
        ["weasyprint", "--quiet", "-u", str(DOCS), str(page_path), str(out)],
        check=True,
    )
    return out


def main(argv: list[str]) -> int:
    wanted = [a.rstrip("/") for a in argv[1:]]
    docs = sorted(
        p
        for p in DOCS.glob("*.md")
        if (m := LANG_OF.match(p.name))
        and m.group("stem") in KINDS
        and (not wanted or any(m.group("stem").startswith(w) for w in wanted))
    )
    if not docs:
        print("no quick-start Markdown found", file=sys.stderr)
        return 1
    work = DOCS / ".pdfbuild"
    try:
        for path in docs:
            out = build(path)
            print(f"{path.name} -> {out.name}")
    finally:
        shutil.rmtree(work, ignore_errors=True)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
