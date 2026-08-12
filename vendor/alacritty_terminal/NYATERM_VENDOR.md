# NyaTerm vendoring notes

- Upstream: [`alacritty_terminal`](https://github.com/alacritty/alacritty), crates.io 0.26.0.
- Upstream revision: `3aca5eb133ca75cd33340c9032f34de4a50e3830` (from `.cargo_vcs_info.json`).
- License: Apache-2.0; the upstream `LICENSE-APACHE` is retained.

## Local patch

NyaTerm adds a wrapping `u64` epoch to each grid. It advances by the number of rows
rotated only when upward scrolling starts at row zero. Read-only `Term` accessors expose
the stable primary/alternate epochs, screen generations, and RIS reset generation.
Entering the alternate screen advances its generation because Alacritty clears that
screen before swapping it into use. No ANSI, grid rotation, event, or parsing behavior is
changed.

`Grid::history_size()` cannot serve this purpose: once scrollback reaches its configured
limit, Alacritty keeps rotating the ring buffer while the reported history size remains
constant. Presentation metadata keyed to physical lines would consequently stop moving.

The crates.io package's 46 MiB reference-terminal fixtures are omitted. They are upstream
integration fixtures rather than library sources; NyaTerm retains and runs the upstream
unit tests embedded under `src/` and adds focused epoch/generation coverage there.

## Validation

```sh
cargo test --manifest-path vendor/alacritty_terminal/Cargo.toml
cargo test -p nyaterm-terminal
cargo check --workspace
```
