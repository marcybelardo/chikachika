# Chikachika

Chikachika is a local-first desktop editor for creating stream overlays. Build
an overlay in the native workspace, copy its stable browser-source URL, and use
it in OBS. The project is pre-release and currently runs from source on macOS
and Linux.

## Current source checkpoints

Issue #22 provides an ordered collection of zero or more text widgets per
overlay. The native editor can add, name, edit, duplicate, delete, and move
widgets one adjacent layer step at a time. Stable IDs and durable widget
properties survive format-2 save/load; selection and delivery revisions do not.
Format-1 overlay documents are not migrated: an incompatible source blocks
startup and is never silently replaced. Server settings use a separate
format-1 `settings.json` file.

Issue #23 adds a three-panel editor: a resizable frontmost-first widget list, a
fit-to-space canvas that preserves its aspect ratio, and a resizable inspector.
The initial window size is 1280×800 logical pixels, with a 1024×640 minimum.
List, canvas, and inspector use one coordinator-owned selection. Headless
tests exercise frontmost overlap selection, empty-canvas clearing, row reveal,
and fast clicks on a separate caption. Native macOS QA confirmed the caption
click updates its selected row and inspector.
Each row has a text type indicator and a Rename action. Overlay switching and
lifecycle controls stay in a compact top strip.

The issue24 in-window menu contract, whose implementation verification is in
progress, covers File > Create Overlay, Save, and guarded Quit; Edit > Undo,
Redo, Add Text, Duplicate, Delete, Forward, and Backward; View > Fit Canvas;
and Help > User Documentation. Undo/redo and other document commands follow
the focus and pending-edit rules in FDR-006. The default macOS native
application menu, including About, Hide, and Quit, is disabled; there is no
placeholder Settings action while #25 remains deferred. The compact status area shows save/server state, errors, and
readiness-gated Copy URL and Open output actions. Its Local server settings
section is collapsed by default; the separate native Settings window is #25
work.

Headless egui tests cover shared selection, fast canvas clicks, overlapping
widgets, divider resizing, supported menus, readiness gating, and canvas fit at
1024×640 and 1280×800. Native macOS review confirmed light, dark, small, and
overlapping text; clear selected and hover outlines; disabled controls; error
visibility; and a quick canvas click updating the row and inspector. The
1024×640 QA variant encountered an occupied test port, so output readiness was
checked at the larger size only.

## Remaining 0.0.2 work

Issue #24's history, focus-safe shortcuts, and guarded in-window close/quit
behavior are in implementation verification; named runtime tests and native
macOS/Linux checks are recorded as pending in the [issue24 evidence
checkpoint](docs/measurements/issue24-runtime-evidence.md). The separate native
Settings window remains #25 work. Bundled font assets and fidelity remain #26
work; font IDs are persisted and selectable, but no font files are bundled yet.
macOS/Linux OBS and resource verification remain #27 work. The v0.0.1 GitHub
Release is a published tagged-source prerelease with no uploaded assets; it is
not a binary package or a release of current 0.0.2 work.

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
browser client consumes complete `widgets`-array snapshots, validates the
whole snapshot before mutation, reconciles DOM nodes by stable widget ID, and
keeps reverse DOM order so model index 0 paints frontmost.
`Number.MAX_SAFE_INTEGER` is the largest accepted browser delivery revision.

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
- [`0.0.2` milestone and issue22/issue23 checkpoints](docs/TODO-0-0-2.md)
- [Release process](docs/RELEASING.md)
- [Issue24 native close validation](docs/measurements/issue24-native-close-validation.md)
