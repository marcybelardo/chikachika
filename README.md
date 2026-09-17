# Chikachika

Chikachika is a local-first desktop editor for creating stream overlays. Build
an overlay in the native workspace, copy its stable browser-source URL, and use
it in OBS. The project is pre-release and currently runs from source on macOS
and Linux.

## Current issue22 checkpoint

The current source checkpoint has an ordered collection of zero or more text
widgets per overlay. The native editor can add, name, edit, duplicate, delete,
and move widgets forward or backward; widget IDs remain stable across those
operations. The model keeps index 0 frontmost, while the native preview paints
back-to-front. A selected widget is owned by the application coordinator, not
by the GUI.

Overlay documents use format 2 in `overlays.json`. Format 2 stores the complete
ordered collection and durable widget properties, but not selection, delivery
revisions, or history. There is no format-1 overlay migration: an incompatible
source blocks startup and is never silently replaced. Server settings remain a
separate format-1 `settings.json` file.

This checkpoint does not make the 0.0.2 milestone complete. The broader
workspace layout and overlap behavior (#23), history and close recovery (#24),
the native Settings window (#25), bundled font assets and fidelity (#26), and
macOS/Linux OBS validation (#27) remain incomplete. Font IDs are persisted and
shown by the editor, but no font files are bundled yet.

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
3. Copy the selected overlay's exact URL.
4. Add that URL to OBS as a Browser Source.

The browser output is transparent and is the final rendering authority. The
browser checkpoint consumes complete `widgets`-array snapshots, validates the
whole snapshot before mutation, reconciles DOM nodes by stable widget ID, and
keeps reverse DOM order so model index 0 paints frontmost. `Number.MAX_SAFE_INTEGER`
is the largest accepted browser delivery revision.

See the [user guide](docs/user/README.md) for the overlay workflow, OBS setup,
path locations, backup recovery, port settings, and troubleshooting.

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
- [`0.0.2` milestone and issue22 checkpoint](docs/TODO-0-0-2.md)
- [Release process](docs/RELEASING.md)
