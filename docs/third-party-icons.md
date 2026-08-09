# Third-party icon assets

Everything under `crates/nyaterm-app/assets/` is vendored from an upstream icon
set or carried over from the pre-GPUI NyaTerm release. The manifest at
`scripts/icons.manifest` maps fetched assets to exact upstream sources. Project
assets carried over from NyaTerm itself are the committed canonical copies and
use the manifest's `keep` source; they no longer depend on a local legacy source
checkout. Pinned upstream versions live at the top of `scripts/sync-icons.sh`.

Assets are committed rather than fetched at build time, so the build stays
offline. `bash scripts/sync-icons.sh --check` verifies fetched assets against
the manifest and verifies that canonical project assets are present.

## Sources

| Set | Used for | Version | License |
|---|---|---|---|
| [Material Design Icons](https://github.com/marella/material-design-icons) (`@material-design-icons/svg`) | The bulk of the UI chrome — activity bar, menus, file explorer, session and transfer controls, file-kind icons | 0.14.15 | Apache-2.0 |
| [Simple Icons](https://github.com/simple-icons/simple-icons) | Monochrome brand marks (Docker, Kubernetes, Postgres, distro wordmarks, search engines) | 16.27.1 | CC0-1.0 |
| [VS Code Codicons](https://github.com/microsoft/vscode-codicons) | Window title-bar controls only. Material has no thin-stroke equivalent, and the pre-GPUI UI used this same set. | 0.0.46-24 | CC BY 4.0 |
| [Font Awesome Free](https://github.com/FortAwesome/Font-Awesome) | The default server glyph, plus the AWS and Yahoo marks that Simple Icons no longer ships | 7.3.1 | CC BY 4.0 |
| [Remix Icon](https://github.com/Remix-Design/RemixIcon) | The OpenAI mark, likewise absent from Simple Icons | 4.9.1 | Apache-2.0 |
| [Lucide](https://github.com/lucide-icons/lucide) (`lucide-static`) | File explorer controls that use Lucide in the Tauri UI | 0.575.0 | ISC |
| NyaTerm | `icons/logo.svg`, and the full-color OS/distro logos and importer brand marks under `color/` | — | Project assets, carried over from the Tauri release |

CC BY 4.0 requires attribution; this file is that attribution. Apache-2.0
requires the license and notice be carried; both sets are unmodified copies, and
their upstream repositories above hold the canonical texts.

## Layout, and why the prefix matters

```
crates/nyaterm-app/assets/
├── icons/**   monochrome — painted with svg() through mono_icon()
└── color/**   full color — painted with img() through color_icon()
```

The split is not cosmetic. GPUI's `svg()` rasterizes to an **alpha mask** and
keeps only the coverage channel, so an asset's own paints are discarded and the
color comes entirely from `.text_color()`. That is what lets one server glyph
serve seven theme hues — and it is also why a multi-color distro logo cannot go
through it, which is how the migration ended up with fourteen distros sharing one
tinted Tux.

`img()` keeps the rasterized pixels, so a logo survives intact, but it cannot be
tinted and it is decoded asynchronously.

Two consequences are enforced by tests in `crates/nyaterm-app/src/assets.rs`:

- **No raster payloads under `icons/`.** GPUI builds resvg without the
  `raster-images` feature, so an SVG wrapping a base64 PNG renders as nothing at
  all — no error, no log line. The importer brand logos arrived in exactly that
  shape and are extracted to real `.png` files under `color/brand/`.
- **Full-color assets stay small.** `img()` rasterizes at twice an asset's
  *intrinsic* size, not its display size, and the decoded image is never evicted.
  The vendored logos are re-declared at 32px (their `viewBox` untouched), which
  is the difference between ~0.5 MB and ~50 MB resident for a set of 16px icons.

`bash scripts/check-icon-references.sh` checks the other direction: that every
path the UI references is bundled, and that the element matches the prefix.
