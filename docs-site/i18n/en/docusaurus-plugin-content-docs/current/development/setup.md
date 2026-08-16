---
sidebar_position: 2
---

# Development Setup

The NyaTerm application is a Cargo workspace. Node.js and pnpm are only required for the Docusaurus documentation site in this repository.

## Application prerequisites

### Rust and Git

Install the latest stable Rust toolchain:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Windows users can install from [rustup.rs](https://rustup.rs/). Every platform also needs Git and its native compiler toolchain.

### Platform dependencies

#### Windows

Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the **Desktop development with C++** workload.

#### macOS

Install Xcode and its command-line tools:

```bash
xcode-select --install
```

GPUI uses Metal on macOS, so a working macOS SDK is required.

#### Linux (Ubuntu / Debian)

Install Rust crate build tools, font/window-system development libraries, and the Vulkan loader:

```bash
sudo apt update
sudo apt install build-essential clang pkg-config cmake \
  libfontconfig1-dev libfreetype6-dev libssl-dev libudev-dev \
  libwayland-dev libx11-dev libx11-xcb-dev \
  libxcb-cursor-dev libxcb-icccm4-dev libxcb-image0-dev \
  libxcb-keysyms1-dev libxcb-randr0-dev libxcb-render0-dev \
  libxcb-shape0-dev libxcb-xfixes0-dev libxcb-xinerama0-dev \
  libxkbcommon-dev libxkbcommon-x11-dev libzstd-dev \
  libvulkan1 mesa-vulkan-drivers
```

Running the desktop application also requires a working Vulkan driver and an X11 or Wayland session.

## Clone the repository

```bash
git clone https://github.com/nyakang/nyaterm.git
cd nyaterm
```

Cargo manages application dependencies; there is no root JavaScript dependency installation step.

## Run the application

```bash
cargo run -p nyaterm-app --bin nyaterm
```

The first build compiles GPUI and vendored dependencies, so it takes noticeably longer than subsequent incremental builds.

## Common checks

Prefer checks scoped to the affected crate while iterating:

```bash
cargo check -p nyaterm-app
cargo test -p <crate-name>
```

Run the relevant workspace checks before review:

```bash
cargo check --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
```

`cargo fmt --all` writes formatting changes; use it only when you intend to apply them.

## Release-profile build

```bash
cargo build -p nyaterm-app --bin nyaterm --release
```

The native binary is written to `target/release/nyaterm`, or `target/release/nyaterm.exe` on Windows. This command only builds the application binary; it does not produce `.msi`, `.dmg`, `.deb`, or AppImage installers.

## Documentation development

When editing `docs-site`, also install Node.js 18+ and [pnpm](https://pnpm.io/):

```bash
pnpm --dir docs-site install
pnpm --dir docs-site start:zh
```

Start the English documentation server with:

```bash
pnpm --dir docs-site start:en
```

Build every locale with:

```bash
pnpm --dir docs-site build
```

The documentation build checks pages and sidebars; Markdown-link problems are reported according to the site configuration. English and Chinese page completeness still requires manual review because there is no automatic translation-completeness check.

## Development conventions

- Read the root `AGENTS.md` and `CONTRIBUTING.md` first.
- UI state and views live in `nyaterm-desktop`; shared controls live in `nyaterm-ui`.
- Transport, terminal, and core crates stay independent of GPUI.
- New UI text updates both locale files under `crates/nyaterm-desktop/src/i18n/locales/`.
- Never use real credentials in tests, logs, or diagnostic data.
