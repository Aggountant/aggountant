#!/usr/bin/env bash
# Keeps the user guide in step with itself and with the product:
#   1. every docs/user/fi/ page has an en/ page at the same path and vice
#      versa, and each pair has the same heading levels in the same order;
#   2. every relative link in docs/user/ resolves, including its #anchor;
#   3. the site (site/, published at https://aggountant.dev) builds from the
#      guide, and every internal link and #anchor resolves on the site too;
#   4. the practice month (docs/user/practice/run-practice-month.sh) imports
#      its material and runs the tutorial's commands in a throwaway home.
#
# The site build needs Node.js and npm (see site/package.json for the version).
#
# Usage: check-user-guide.sh [path-to-aggountant]
# Without a path, the aggountant binary is built with cargo first.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
cd "$ROOT"

printf -- '-- language pairs, headings and links --\n'
python3 -I - docs/user <<'PY'
import os, re, sys, unicodedata
from urllib.parse import unquote

guide = sys.argv[1]
errors = []

def md_files(base):
    for dirpath, _, names in os.walk(base):
        for name in names:
            if name.endswith(".md"):
                yield os.path.join(dirpath, name)

FENCE = re.compile(r"^\s*(```|~~~)")
HEADING = re.compile(r"^(#{1,6})\s+(.*?)\s*#*\s*$")
CODE_SPAN = re.compile(r"`+[^`]*`+")
LINK = re.compile(r"\]\(\s*<?([^)\s>]+)>?(?:\s+\"[^\"]*\")?\s*\)")

def prose_lines(path):
    """Yields (line number, line) outside fenced code blocks."""
    fenced = False
    with open(path, encoding="utf-8") as f:
        for n, line in enumerate(f, 1):
            if FENCE.match(line):
                fenced = not fenced
                continue
            if not fenced:
                yield n, line.rstrip("\n")

def headings(path):
    return [(len(m.group(1)), m.group(2))
            for _, line in prose_lines(path)
            if (m := HEADING.match(line))]

def slug(text):
    # GitHub's heading anchor: rendered text, lower-cased, punctuation dropped,
    # spaces to hyphens.
    text = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", text)
    text = re.sub(r"[`*]", "", text)
    text = text.lower()
    text = "".join(c for c in text
                   if c in " -_" or unicodedata.category(c)[0] in "LNM")
    return text.replace(" ", "-")

anchor_cache = {}
def anchors(path):
    if path not in anchor_cache:
        seen, result = {}, set()
        for _, text in headings(path):
            s = slug(text)
            n = seen.get(s, 0)
            seen[s] = n + 1
            result.add(s if n == 0 else f"{s}-{n}")
        anchor_cache[path] = result
    return anchor_cache[path]

# 1. Language pairs and heading structure.
pages = {lang: {os.path.relpath(p, os.path.join(guide, lang))
                for p in md_files(os.path.join(guide, lang))}
         for lang in ("fi", "en")}
for lang, other in (("fi", "en"), ("en", "fi")):
    for rel in sorted(pages[lang] - pages[other]):
        errors.append(f"{guide}/{lang}/{rel}: no {other}/ counterpart at {guide}/{other}/{rel}")
for rel in sorted(pages["fi"] & pages["en"]):
    fi_path, en_path = (os.path.join(guide, lang, rel) for lang in ("fi", "en"))
    fi_levels = [level for level, _ in headings(fi_path)]
    en_levels = [level for level, _ in headings(en_path)]
    if fi_levels != en_levels:
        errors.append(f"{en_path}: heading levels {en_levels} differ from {fi_path} {fi_levels}")

# 2. Relative links and anchors.
for path in sorted(md_files(guide)):
    for n, line in prose_lines(path):
        for target in LINK.findall(CODE_SPAN.sub("", line)):
            if re.match(r"^[a-z][a-z0-9+.-]*:", target, re.I):
                continue  # external URL or mailto
            file_part, _, anchor = target.partition("#")
            dest = (os.path.normpath(os.path.join(os.path.dirname(path), unquote(file_part)))
                    if file_part else path)
            where = f"{path}:{n}: link {target}"
            if not os.path.exists(dest):
                errors.append(f"{where}: {dest} does not exist")
            elif anchor and dest.endswith(".md") and unquote(anchor) not in anchors(dest):
                errors.append(f"{where}: no heading with anchor #{anchor} in {dest}")

for e in errors:
    print(e, file=sys.stderr)
if errors:
    print(f"check-user-guide: {len(errors)} problem(s) in {guide}", file=sys.stderr)
    sys.exit(1)
print(f"{len(pages['fi'])} page pairs and their links are in step")
PY

printf -- '-- site build --\n'
if ! command -v npm >/dev/null 2>&1; then
  printf 'error: npm is required to build the user-guide site in site/; install Node.js 22.22 or newer\n' >&2
  exit 1
fi
# The build log is long; show it only when the build fails.
if ! site_log="$(cd site && npm ci --no-audit --no-fund --loglevel=error 2>&1 && npm run --silent build 2>&1)"; then
  printf '%s\n' "$site_log" >&2
  printf 'check-user-guide: the site in site/ does not build\n' >&2
  exit 1
fi
printf 'site build ok\n'

printf -- '-- practice month --\n'
if [[ $# -gt 0 ]]; then
  bin="$1"
else
  cargo build --locked --bin aggountant
  target_dir="$(cargo metadata --format-version 1 --no-deps | jq -er '.target_directory')"
  bin="$target_dir/debug/aggountant"
fi
docs/user/practice/run-practice-month.sh "$bin" >/dev/null
printf 'practice month ok\n'
