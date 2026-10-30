#!/usr/bin/env python3
"""Renders docs/guide/*.md into the project site's guide pages (an index and one page per chapter).

usage: build_guide.py <guide source dir> <site guide dir> [<pictures dir>]

The source dir holds `contents.json` (groups of chapters: file, title, description) and one Markdown file per
chapter. With a pictures dir, every PNG in it is converted to a WebP in `<site guide dir>/img` (needs ImageMagick).
Only the Markdown the guide uses is understood: headings, paragraphs, bullet and numbered lists, tables,
fenced code, block quotes, images on a line of their own, and `code`, **bold**, *italic* and [links](url) inline.
"""
import html
import json
import re
import subprocess
import sys
from pathlib import Path

ACCENT = "#ffc27a"
BG = "#1f3b4d"
BG_DEEP = "#152b39"
CSS_VERSION = "20261008"

STYLE = """
      :root { --bg: %(bg)s; --bg-deep: %(deep)s; --accent: %(accent)s; }
      .doc h2 { margin: 3rem 0 1rem; }
      .doc h3 { margin: 2rem 0 0.75rem; color: var(--ink-strong); font-size: 1.1rem; }
      .doc p, .doc ul, .doc ol, .doc pre { margin: 0 0 1rem; }
      .doc ul, .doc ol { padding-left: 1.4rem; list-style: disc; }
      .doc ol { list-style: decimal; }
      .doc li { margin: 0.3rem 0; }
      .doc .tablewrap { overflow-x: auto; margin: 0 0 1.25rem; }
      .doc table { border-collapse: collapse; font-size: 0.9rem; min-width: 100%%; }
      .doc th, .doc td { text-align: left; vertical-align: top; padding: 0.45rem 0.6rem; border-bottom: 1px solid var(--rule); }
      .doc th { color: var(--ink-strong); font-weight: 600; background: var(--panel); }
      .doc pre.code { overflow-x: auto; }
      .doc pre.code code { background: none; padding: 0; font-size: 0.85rem; }
      .doc figure.shot { margin: 1.25rem 0 1.75rem; }
      .doc figure.shot img { display: block; max-width: 100%%; height: auto; border-radius: 10px; border: 1px solid var(--rule); box-shadow: 0 10px 30px rgba(0,0,0,0.35); }
      .doc figure.shot a { border: 0; }
      .doc blockquote { margin: 0 0 1rem; padding: 0.75rem 1rem; border-left: 3px solid var(--accent); background: var(--panel); border-radius: 0 8px 8px 0; }
      .lede p + p { margin-top: 0.75rem; }
      .pager { display: grid; grid-template-columns: 1fr auto 1fr; align-items: baseline; gap: 1rem; margin: 3rem 0 0; padding: 1.25rem 0; border-top: 1px solid var(--rule); font-size: 0.95rem; }
      .pager a { border: 0; color: var(--ink-soft); }
      .pager a:hover { color: var(--ink-strong); }
      .pager .contents { justify-self: center; color: var(--ink-faint); font-weight: 400; }
      .pager .next { justify-self: end; text-align: right; }
      .toc { display: grid; grid-template-columns: repeat(auto-fit, minmax(22rem, 1fr)); gap: 1.75rem 2.5rem; margin: 2rem 0 1rem; }
      .toc h2 { margin: 0 0 0.5rem; font-family: var(--sans); font-size: 0.75rem; font-weight: 600; letter-spacing: 0.14em; text-transform: uppercase; color: var(--accent); }
      .toc ol { list-style: none; padding: 0; margin: 0; }
      .toc li { margin: 0; }
      .toc a { display: grid; grid-template-columns: 2rem 1fr; column-gap: 0.25rem; padding: 0.55rem 0.6rem; margin: 0 -0.6rem; border: 0; border-radius: 8px; }
      .toc a:hover { background: var(--panel); }
      .toc .n { grid-row: span 2; font-family: var(--mono); font-size: 0.85rem; color: var(--ink-faint); padding-top: 0.15rem; }
      .toc .t { color: var(--ink-strong); font-weight: 600; }
      .toc .d { font-weight: 400; font-size: 0.88rem; line-height: 1.45; color: var(--ink-soft); }
""" % {"bg": BG, "deep": BG_DEEP, "accent": ACCENT}

SCRIPT = """
      (() => {
        const bar = document.querySelector(".sitebar");
        const title = document.querySelector(".hero h1");
        new IntersectionObserver(([e]) => bar.classList.toggle("past", !e.isIntersecting && e.boundingClientRect.top < 0),
          { rootMargin: `-${bar.offsetHeight}px 0px 0px 0px` }).observe(title);
        const list = bar.querySelector(".sections");
        const links = new Map([...list.querySelectorAll("a")].map((a) => [a.hash.slice(1), a]));
        const visible = new Set();
        const observer = new IntersectionObserver((entries) => {
          for (const e of entries) e.isIntersecting ? visible.add(e.target.id) : visible.delete(e.target.id);
          const current = [...links.keys()].find((id) => visible.has(id));
          for (const [id, a] of links) {
            if (id !== current) { a.removeAttribute("aria-current"); continue; }
            a.setAttribute("aria-current", "true");
            if (list.scrollWidth > list.clientWidth) list.scrollTo({ left: a.offsetLeft - 16, behavior: "smooth" });
          }
        }, { rootMargin: "-20% 0px -60% 0px" });
        for (const id of links.keys()) { const s = document.getElementById(id); if (s) observer.observe(s); }
      })();
"""


def slug(text):
    s = re.sub(r"<[^>]+>", "", text)
    s = html.unescape(s).lower()
    s = re.sub(r"[^a-z0-9]+", "-", s).strip("-")
    return s or "section"


def inline(text):
    parts = re.split(r"(`[^`]+`)", text)
    out = []
    for p in parts:
        if p.startswith("`") and p.endswith("`") and len(p) > 1:
            out.append("<code>%s</code>" % html.escape(p[1:-1], quote=False))
            continue
        p = html.escape(p, quote=False)
        p = re.sub(r"\*\*(.+?)\*\*", r"<strong>\1</strong>", p)
        p = re.sub(r"(?<![\w*])\*(?!\s)(.+?)(?<!\s)\*(?![\w*])", r"<em>\1</em>", p)
        p = re.sub(
            r"\[([^\]]+)\]\(([^)\s]+)\)",
            lambda m: '<a href="%s">%s</a>' % (m.group(2).replace("&amp;", "&amp;"), m.group(1)),
            p,
        )
        out.append(p)
    return "".join(out)


def cells(line):
    line = line.strip()
    if line.startswith("|"):
        line = line[1:]
    if line.endswith("|"):
        line = line[:-1]
    return [c.strip() for c in re.split(r"(?<!\\)\|", line)]


def render(md):
    """Returns (title, body html, [(id, heading)])."""
    lines = md.split("\n")
    i = 0
    title = ""
    out = []
    sections = []
    used = set()

    def unique(base):
        s, n = base, 2
        while s in used:
            s = "%s-%d" % (base, n)
            n += 1
        used.add(s)
        return s

    while i < len(lines):
        line = lines[i]
        if not line.strip():
            i += 1
            continue
        m = re.match(r"^(#{1,3}) +(.*)$", line)
        if m:
            level, text = len(m.group(1)), m.group(2).strip()
            if level == 1:
                title = text
            else:
                h = inline(text)
                sid = unique(slug(h))
                if level == 2:
                    sections.append((sid, re.sub(r"<[^>]+>", "", html.unescape(h))))
                out.append('<h%d id="%s">%s</h%d>' % (level, sid, h, level))
            i += 1
            continue
        if line.startswith("```"):
            lang = line[3:].strip()
            i += 1
            code = []
            while i < len(lines) and not lines[i].startswith("```"):
                code.append(lines[i])
                i += 1
            i += 1
            if lang == "rust":
                # rustdoc's hidden lines ("# " and a lone "#") are compiled by the doctests but not shown.
                code = [c for c in code if c != "#" and not c.startswith("# ")]
            cls = ' class="language-%s"' % lang if lang else ""
            out.append(
                '<pre class="code"><code%s>%s\n</code></pre>' % (cls, html.escape("\n".join(code), quote=False))
            )
            continue
        m = re.match(r"^!\[([^\]]*)\]\(([^)\s]+)\)\s*$", line)
        if m:
            alt, src = m.group(1), m.group(2)
            out.append(
                '<figure class="shot"><a href="%s"><img src="%s" alt="%s" loading="lazy"></a></figure>'
                % (src, src, html.escape(alt))
            )
            i += 1
            continue
        if line.startswith(">"):
            quote = []
            while i < len(lines) and lines[i].startswith(">"):
                quote.append(lines[i][1:].strip())
                i += 1
            out.append("<blockquote><p>%s</p></blockquote>" % inline(" ".join(quote)))
            continue
        if line.lstrip().startswith("|") and i + 1 < len(lines) and re.match(r"^\s*\|[\s:|-]+\|?\s*$", lines[i + 1]):
            head = cells(line)
            i += 2
            rows = []
            while i < len(lines) and lines[i].lstrip().startswith("|"):
                rows.append(cells(lines[i]))
                i += 1
            t = ["<div class=\"tablewrap\"><table>", "<thead>", "<tr>"]
            t += ["<th>%s</th>" % inline(c) for c in head]
            t += ["</tr>", "</thead>", "<tbody>"]
            for r in rows:
                t.append("<tr>")
                t += ["<td>%s</td>" % inline(c) for c in r]
                t.append("</tr>")
            t += ["</tbody>", "</table></div>"]
            out.append("\n".join(t))
            continue
        m = re.match(r"^(\s*)([-*]|\d+\.) +(.*)$", line)
        if m:
            ordered = m.group(2)[0].isdigit()
            items = []
            while i < len(lines):
                mm = re.match(r"^([-*]|\d+\.) +(.*)$", lines[i])
                if mm:
                    items.append([mm.group(2)])
                    i += 1
                elif lines[i].startswith("  ") and lines[i].strip() and items:
                    items[-1].append(lines[i].strip())
                    i += 1
                else:
                    break
            tag = "ol" if ordered else "ul"
            out.append(
                "<%s>\n%s\n</%s>" % (tag, "\n".join("<li>%s</li>" % inline("\n".join(it)) for it in items), tag)
            )
            continue
        para = []
        while i < len(lines) and lines[i].strip() and not re.match(
            r"^(#{1,3} |```|>|!\[|\s*\|)", lines[i]
        ) and not re.match(r"^([-*]|\d+\.) +", lines[i]):
            para.append(lines[i])
            i += 1
        if not para:
            para = [lines[i]]
            i += 1
        out.append("<p>%s</p>" % inline("\n".join(para)))
    return title, "\n".join(out), sections


HEAD = """<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <meta name="description" content="%(desc)s" />
    <title>%(title)s</title>
    <link rel="preconnect" href="https://fonts.googleapis.com" />
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
    <link href="https://fonts.googleapis.com/css?family=Open+Sans:400,600" rel="stylesheet" />
    <link href="/css/project.css?v=%(css)s" media="screen" rel="stylesheet" />
    <style>%(style)s    </style>
    <link rel="icon" href="/favicon.svg" type="image/svg+xml" />
    <link rel="alternate icon" href="/favicon.ico" />
    <link rel="apple-touch-icon" href="/apple-touch-icon.png" />
  </head>
"""


def page(title, here, sections, eyebrow, h1, main, pager):
    nav = "\n".join('          <li><a href="#%s">%s</a></li>' % (i, html.escape(t)) for i, t in sections)
    return (
        HEAD
        % {
            "desc": html.escape("Babbage user guide: %s." % h1, quote=True),
            "title": html.escape(title),
            "css": CSS_VERSION,
            "style": STYLE,
        }
        + """  <body>
    <nav class="sitebar" aria-label="Site">
      <div class="sitebar-in">
        <div class="where">
          <a class="home" href="/" aria-label="Jason Rimmer, home"><span class="full">Jason Rimmer</span><span class="short" aria-hidden="true">JR</span></a>
          <span class="here" aria-hidden="true">%(here)s</span>
        </div>
        <ul class="sections" aria-label="On this page">
%(nav)s
        </ul>
      </div>
    </nav>
    <div class="page">
      <header class="hero">
        <p class="eyebrow">%(eyebrow)s</p>
        <h1>%(h1)s</h1>
      </header>
      <main class="doc">
%(main)s
      </main>
      %(pager)s
      <footer class="footer">
        <span>&copy; 2026 <a href="/">Jason Rimmer</a></span>
        <span><a href="../">Babbage</a></span>
      </footer>
    </div>
    <script>%(script)s    </script>
  </body>
</html>
"""
        % {
            "here": html.escape(here),
            "nav": nav,
            "eyebrow": eyebrow,
            "h1": html.escape(h1),
            "main": main,
            "pager": pager,
            "script": SCRIPT,
        }
    )


def convert_pictures(pictures, dst):
    img = dst / "img"
    img.mkdir(parents=True, exist_ok=True)
    for png in sorted(pictures.glob("*.png")):
        out = img / (png.stem + ".webp")
        subprocess.run(
            ["magick", str(png), "-quality", "86", "-define", "webp:method=6", str(out)], check=True
        )


def main():
    src, dst = Path(sys.argv[1]), Path(sys.argv[2])
    if len(sys.argv) > 3:
        convert_pictures(Path(sys.argv[3]), dst)
    contents = json.loads((src / "contents.json").read_text())
    guide = contents.get("name", "User guide")
    chapters = [c for g in contents["groups"] for c in g["chapters"]]
    chapters.sort(key=lambda c: c["file"])
    dst.mkdir(parents=True, exist_ok=True)
    seen = set()
    for n, c in enumerate(chapters):
        name = c["file"]
        title, body, sections = render((src / (name + ".md")).read_text())
        seen.add(name + ".html")
        prev = chapters[n - 1] if n else None
        nxt = chapters[n + 1] if n + 1 < len(chapters) else None
        pager = '<nav class="pager">%s<a class="contents" href="index.html">Contents</a>%s</nav>' % (
            '<a class="prev" href="%s.html">← %s</a>' % (prev["file"], html.escape(prev["title"]))
            if prev
            else "<span></span>",
            '<a class="next" href="%s.html">%s →</a>' % (nxt["file"], html.escape(nxt["title"]))
            if nxt
            else "<span></span>",
        )
        # The first paragraphs, up to the first heading, are the lede.
        (dst / (name + ".html")).write_text(
            page(
                "%s · Babbage %s" % (c["title"], guide.lower()),
                c["title"],
                sections,
                '<a href="../">Babbage</a> · <a href="index.html">%s</a>' % html.escape(guide),
                title or c["title"],
                body,
                pager,
            )
        )
    toc = []
    for g in contents["groups"]:
        items = "".join(
            '<li><a href="%s.html"><span class="n">%s</span><span class="t">%s</span><span class="d">%s</span></a></li>'
            % (c["file"], c["file"][:2], html.escape(c["title"]), html.escape(c["description"]))
            for c in g["chapters"]
        )
        toc.append("<div><h2>%s</h2><ol>%s</ol></div>" % (html.escape(g["title"]), items))
    intro = "<p>%s</p>" % html.escape(contents["intro"])
    index = page(
        "%s · Babbage %s" % (guide, guide.lower()),
        guide,
        [],
        '<a href="../">Babbage</a> · <a href="index.html">Contents</a>',
        guide,
        intro + '\n      <nav class="toc" aria-label="Chapters">%s</nav>' % "".join(toc),
        '<nav class="pager"><span></span><a class="contents" href="index.html">Contents</a><a class="next" href="%s.html">%s →</a></nav>'
        % (chapters[0]["file"], html.escape(chapters[0]["title"])),
    )
    (dst / "index.html").write_text(index)
    seen.add("index.html")
    for old in dst.glob("*.html"):
        if old.name not in seen:
            print("note: %s is not in contents.json" % old.name, file=sys.stderr)
    print("wrote %d chapters and the contents page" % len(chapters))


if __name__ == "__main__":
    main()
