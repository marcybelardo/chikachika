# Chikachika

Chikachika is a local-first desktop editor for creating stream overlays. Build
an overlay in the native workspace, copy its stable browser-source URL, and use
it in OBS. Changes appear in the browser output while you edit.

The project is pre-release and currently runs from source on macOS and Linux.

## What it does

- Creates, names, edits, saves, and restores local overlays.
- Provides a fixed-canvas editor for a text widget, including content, size,
  color, alignment, and position.
- Serves each overlay at a stable, transparent browser-source URL on the local
  computer.
- Publishes edits live without requiring an OBS source refresh.
- Keeps overlay documents and application settings local.

The current editor supports one optional text widget per overlay. The planned
[`0.0.2` milestone](docs/TODO-0-0-2.md) expands this into a multi-widget
composition workspace.

## Run from source

Install the [stable Rust toolchain](https://www.rust-lang.org/tools/install),
then clone and run the project:

```sh
git clone https://github.com/marcybelardo/chikachika.git
cd chikachika
cargo run
```

NixOS users can enter the included development environment first with
`nix develop`. Other Linux distributions may need native GUI and OpenSSL
development packages. See [Getting started](docs/user/getting-started.md) for
platform prerequisites and saved-data locations.

## Use with OBS

1. Create or select an overlay and save it.
2. Wait for the local server to report that it is ready.
3. Copy the selected overlay's URL.
4. Add that URL to OBS as a Browser Source.

See the [user guide](docs/user/README.md) for the complete overlay workflow,
OBS setup, port settings, and troubleshooting.

## Development

Run the checks used by continuous integration:

```sh
cargo fmt --all -- --check
cargo test --locked --all-targets
node --test tests/browser_overlay.test.mjs
python3 scripts/check_docs.py
python3 -m unittest discover -s tests -v
```

The complete check suite requires Node.js 22 or newer and Python 3 in addition
to Rust. CI runs the Rust checks on Ubuntu and macOS.

## Project documentation

- [Contributor and agent guidance](AGENTS.md)
- [Current architecture](docs/architecture/INDEX.md)
- [Architecture decisions](docs/adr/INDEX.md)
- [Feature decisions](docs/fdr/INDEX.md)
- [Canonical terminology](docs/GLOSSARY.md)
- [`0.0.1` milestone evidence](docs/TODO-0-0-1.md)
- [`0.0.2` milestone plan](docs/TODO-0-0-2.md)
- [Release process](docs/RELEASING.md)
