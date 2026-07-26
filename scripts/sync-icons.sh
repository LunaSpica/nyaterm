#!/usr/bin/env bash
#
# Vendor the bundled icon assets described by scripts/icons.manifest.
#
# The build stays offline: this script is run by hand when the manifest or a
# pinned version changes, and its output is committed. It exists so the
# provenance of every icon is reproducible and reviewable rather than folklore.
#
#   bash scripts/sync-icons.sh            # sync everything
#   bash scripts/sync-icons.sh --check    # verify the tree matches, change nothing
#
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

ASSET_DIR="crates/nyaterm-app/assets"
MANIFEST="scripts/icons.manifest"
LEGACY_ICON_DIR="temp/nyaterm-tauri/public/icons"

# Pinned so a re-run reproduces the committed tree.
MD_VERSION="0.14.15"        # @material-design-icons/svg   Apache-2.0
SI_VERSION="16.27.1"        # simple-icons                 CC0-1.0
FA_VERSION="7.3.1"          # @fortawesome/fontawesome-free CC BY 4.0
VSC_VERSION="0.0.46-24"     # @vscode/codicons             CC BY 4.0
RMX_VERSION="4.9.1"         # remixicon                    Apache-2.0

# img() rasterizes at 2x an asset's *intrinsic* size and the result is cached for
# the process lifetime, so full-color logos are re-declared at this size. The
# viewBox is left alone, so the artwork is unchanged.
COLOR_INTRINSIC_PX=32

CHECK_ONLY=0
[[ "${1:-}" == "--check" ]] && CHECK_ONLY=1

SCRATCH="$(mktemp -d)"
trap 'rm -rf "$SCRATCH"' EXIT

failures=0
fail() {
  printf 'sync-icons: %s\n' "$*" >&2
  failures=$((failures + 1))
}

require() {
  command -v "$1" >/dev/null 2>&1 || {
    printf 'sync-icons: %s is required\n' "$1" >&2
    exit 1
  }
}
require curl
require tar
require perl
require base64

# --- upstream fetch ---------------------------------------------------------

fetch_pkg() {
  # $1 npm package, $2 version, $3 scratch subdir
  local pkg="$1" version="$2" dest="$SCRATCH/$3"
  local tarball="${pkg##*/}-$version.tgz"
  [[ -d "$dest" ]] && return 0
  mkdir -p "$dest"
  printf 'sync-icons: fetching %s@%s\n' "$pkg" "$version" >&2
  curl -fsSL "https://registry.npmjs.org/$pkg/-/$tarball" |
    tar -xz -C "$dest" --strip-components=1
}

pkg_root() {
  case "$1" in
    md) fetch_pkg "@material-design-icons/svg" "$MD_VERSION" md; printf '%s/md' "$SCRATCH" ;;
    si) fetch_pkg "simple-icons" "$SI_VERSION" si; printf '%s/si/icons' "$SCRATCH" ;;
    fa) fetch_pkg "@fortawesome/fontawesome-free" "$FA_VERSION" fa; printf '%s/fa/svgs' "$SCRATCH" ;;
    vsc) fetch_pkg "@vscode/codicons" "$VSC_VERSION" vsc; printf '%s/vsc/src/icons' "$SCRATCH" ;;
    rmx) fetch_pkg "remixicon" "$RMX_VERSION" rmx; printf '%s/rmx/icons' "$SCRATCH" ;;
    *) return 1 ;;
  esac
}

# --- transforms -------------------------------------------------------------

# Upstream icon-set files are already optimized and correctly sized; copying them
# byte-for-byte keeps the vendored tree diffable against upstream.
copy_verbatim() {
  cp "$1" "$2"
}

# The legacy logos carry editor cruft and, more importantly, declare a 256px
# intrinsic size against a 1024 viewBox. Strip the junk and re-declare the size.
normalize_color_svg() {
  local src="$1" dest="$2"
  perl -0777 -pe '
    s/<\?xml.*?\?>//gs;
    s/<!DOCTYPE.*?>//gs;
    s/<!--.*?-->//gs;
    s{(<svg\b)([^>]*?)(/?>)}{
      my ($open, $attrs, $close) = ($1, $2, $3);
      # Root-only: drop editor metadata and the authored intrinsic size.
      $attrs =~ s/\s+(?:t|p-id|class|version|id|x|y|xml:space|enable-background)="[^"]*"//g;
      $attrs =~ s/\s+(?:width|height)="[^"]*"//g;
      $open . qq{ width="$ENV{COLOR_INTRINSIC_PX}" height="$ENV{COLOR_INTRINSIC_PX}"} . $attrs . $close;
    }es;
    # Child elements carry the same exporter junk; ids and gradients must survive.
    s/\s+(?:t|p-id)="[^"]*"//g;
    s/^\s+//;
    s/\s+$//;
    s/>\s+</></g;
  ' "$src" > "$dest"

  if grep -q '<image' "$dest"; then
    fail "$dest still contains <image>; gpui builds resvg without raster-images"
  fi
  if grep -q '<text' "$dest"; then
    printf 'sync-icons: warning: %s renders <text>, so it depends on system fonts\n' "$dest" >&2
  fi
}

# Pull the raster payload out of an SVG wrapper and write it as a real image.
# The data URI is wrapped across lines inside the xlink:href attribute, so this
# has to read the whole file and strip whitespace rather than match per line.
extract_embedded_png() {
  local src="$1" dest="$2" payload
  payload="$(perl -0777 -ne '
    if (/data:image\/png;base64,([A-Za-z0-9+\/=\s]+?)["\x27]/s) {
      my $data = $1;
      $data =~ s/\s+//g;
      print $data;
    }
  ' "$src")"
  if [[ -z "$payload" ]]; then
    fail "$src has no base64 PNG payload to extract"
    return 0
  fi
  printf '%s' "$payload" | base64 -d > "$dest"
}

# --- driver -----------------------------------------------------------------

export COLOR_INTRINSIC_PX

emit() {
  local dest_rel="$1" spec="$2"
  local dest="$ASSET_DIR/$dest_rel"
  local staged="$SCRATCH/staged/$dest_rel"
  mkdir -p "$(dirname "$staged")"

  case "$spec" in
    keep)
      [[ -f "$dest" ]] || fail "$dest_rel is marked 'keep' but does not exist"
      return 0
      ;;
    local-svg:*)
      local src="$LEGACY_ICON_DIR/${spec#local-svg:}"
      [[ -f "$src" ]] || { fail "missing legacy asset $src"; return 0; }
      normalize_color_svg "$src" "$staged"
      ;;
    local-png:*)
      local src="$LEGACY_ICON_DIR/${spec#local-png:}"
      [[ -f "$src" ]] || { fail "missing legacy asset $src"; return 0; }
      extract_embedded_png "$src" "$staged"
      ;;
    *:*)
      local kind="${spec%%:*}" rel="${spec#*:}" root
      root="$(pkg_root "$kind")" || { fail "unknown source kind '$kind' for $dest_rel"; return 0; }
      [[ -f "$root/$rel" ]] || { fail "missing upstream $kind:$rel for $dest_rel"; return 0; }
      copy_verbatim "$root/$rel" "$staged"
      ;;
    *)
      fail "unparseable source spec '$spec' for $dest_rel"
      return 0
      ;;
  esac

  if (( CHECK_ONLY )); then
    if ! cmp -s "$staged" "$dest"; then
      fail "$dest_rel is out of date; run bash scripts/sync-icons.sh"
    fi
  else
    mkdir -p "$(dirname "$dest")"
    cp "$staged" "$dest"
  fi
}

count=0
while IFS=$'\t' read -r dest_rel spec; do
  # Tolerate a CRLF checkout; a stray \r would otherwise turn every source spec
  # into an unknown one.
  dest_rel="${dest_rel%$'\r'}"
  spec="${spec%$'\r'}"
  [[ -z "${dest_rel// }" ]] && continue
  [[ "$dest_rel" == \#* ]] && continue
  if [[ -z "${spec:-}" ]]; then
    fail "manifest line for '$dest_rel' has no tab-separated source spec"
    continue
  fi
  emit "$dest_rel" "$spec"
  count=$((count + 1))
done < "$MANIFEST"

if (( failures )); then
  printf 'sync-icons: %d problem(s) across %d manifest entries\n' "$failures" "$count" >&2
  exit 1
fi

if (( CHECK_ONLY )); then
  printf 'sync-icons: %d assets match the manifest\n' "$count"
else
  printf 'sync-icons: synced %d assets\n' "$count"
fi
