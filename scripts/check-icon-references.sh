#!/usr/bin/env bash
#
# Verify that every icon path the UI references is actually bundled, and that it
# is painted with the matching element.
#
# GPUI fails silently here: a missing asset makes `svg()`/`img()` draw nothing,
# with no error and no log line. Painting a full-color logo through `svg()` is
# just as quiet — it renders as a flat silhouette.
#
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

ASSET_DIR="crates/nyaterm-app/assets"
SRC_DIRS=(crates/nyaterm-desktop/src crates/nyaterm-ui/src)

failures=0
fail() {
  printf 'icon-reference: %s\n' "$*" >&2
  failures=$((failures + 1))
}

# 1. Every referenced asset exists.
referenced="$(grep -rEoh '"(icons|color)/[A-Za-z0-9._/-]+\.(svg|png)"' "${SRC_DIRS[@]}" |
  tr -d '"' | sort -u)"

if [[ -z "$referenced" ]]; then
  fail "no icon references found; the search pattern is probably stale"
fi

while IFS= read -r asset; do
  [[ -z "$asset" ]] && continue
  [[ -f "$ASSET_DIR/$asset" ]] || fail "$asset is referenced but not bundled"
done <<< "$referenced"

# 2. The prefix and the element agree. `icons/**` is an alpha mask, `color/**` is
#    a raster; swapping them is silent but wrong.
while IFS= read -r hit; do
  [[ -z "$hit" ]] && continue
  fail "svg() is given a full-color asset (use color_icon): $hit"
done <<< "$(grep -rn '\.path("color/' "${SRC_DIRS[@]}" || true)"

while IFS= read -r hit; do
  [[ -z "$hit" ]] && continue
  fail "img() is given a monochrome asset (use mono_icon): $hit"
done <<< "$(grep -rn 'img("icons/' "${SRC_DIRS[@]}" || true)"

# 3. Every svg() declares its own color. GPUI resolves an element's style from
#    Style::default(), so `text.color` is None unless the element sets it: a
#    parent's `.text_color(..)` reaches text but never an svg() child, and
#    `Svg::paint` skips a glyph with no color entirely. The result is an
#    invisible icon that still occupies layout — no error, no log line.
#
#    Needs to follow the builder chain across newlines, which grep cannot do.
while IFS= read -r hit; do
  [[ -z "$hit" ]] && continue
  fail "svg() has no text_color, so it is never painted: $hit"
done <<< "$(python3 - "${SRC_DIRS[@]}" <<'PY' || true
import pathlib, re, sys

# The color has to be on the svg's own chain, not nested inside a closure: an
# icon whose only color lives in `.group_hover(.., |s| s.text_color(..))` is
# invisible until the pointer is over it. So both markers are counted at depth 0.
def chain_flags(source, start):
    index, depth = start, 0
    has_path = has_color = False
    while index < len(source):
        char = source[index]
        if char == "(":
            if depth == 0:
                if source.startswith(".path(", index - 5):
                    has_path = True
                elif source.startswith(".text_color(", index - 11):
                    has_color = True
            depth += 1
        elif char == ")":
            if depth == 0:
                break
            depth -= 1
        elif depth == 0 and char in ",;":
            break
        index += 1
    return has_path, has_color


for root in (pathlib.Path(arg) for arg in sys.argv[1:]):
    for path in sorted(root.rglob("*.rs")):
        source = path.read_text(encoding="utf-8", errors="ignore")
        for match in re.finditer(r"\bsvg\(\)", source):
            has_path, has_color = chain_flags(source, match.end())
            if has_path and not has_color:
                line = source.count("\n", 0, match.start()) + 1
                print(f"{path.as_posix()}:{line}")
PY
)"

# 4. Nothing bundled is orphaned. A warning, not a failure: an asset reached only
#    through a lookup table is still referenced by literal, but a future
#    indirection could legitimately hide one.
while IFS= read -r asset; do
  [[ -z "$asset" ]] && continue
  if ! grep -qxF "$asset" <<< "$referenced"; then
    printf 'icon-reference: warning: %s is bundled but never referenced\n' "$asset" >&2
  fi
done <<< "$(cd "$ASSET_DIR" && find icons color -type f \( -name '*.svg' -o -name '*.png' \) 2>/dev/null | sort)"

if (( failures )); then
  printf 'icon-reference: %d problem(s)\n' "$failures" >&2
  exit 1
fi

printf 'icon-reference: all referenced icons are bundled and correctly painted\n'
