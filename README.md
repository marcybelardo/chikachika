# Chikachika

Chikachika is a pre-release local-first desktop application for creating and using overlays. The repository contains a native overlay workspace, a framework-independent overlay model, local persistence, an embedded browser renderer, and in-memory per-overlay HTTP/SSE hosting infrastructure.

## Status

**Current status: pre-release implementation (target: `0.0.1`).**

Implemented in the current target:

- A native eframe/egui overlay workspace and readiness view.
- A local web server started by the application in the same process.
- A `GET /ping` health endpoint that returns the plain-text response `pong`.
- Automated Rust and documentation checks.
- A framework-independent overlay model with stable UUID identities, fixed canvas validation, and zero-or-one text-widget support.
- Versioned JSON persistence for overlay documents in the platform app-local data directory, including non-destructive malformed-data handling and safe replacement.
- A compile-time embedded transparent browser renderer for the current overlay model.
- Server-side hosting for registered overlays at stable `/overlay/{overlay-id}` HTML routes with bounded `/overlay/{overlay-id}/events` SSE snapshots.
- Application-state wiring for overlay collection, selection, lifecycle actions, dirty/save state, visible errors, and server shutdown coordination.
- Native one-widget editing with multiline text, font size, RGBA color, alignment, fixed-canvas preview, bounded drag movement, and live publication through the shared hosting hub.
- Exact selected-overlay URL display, clipboard copy, and browser-open actions gated on server readiness and hub/workspace consistency.
- Versioned application settings in a separate platform config-local `settings.json`, with validated loopback port selection, visible malformed/conflict errors, and restart-bound port changes.

### Native overlay workspace

The native workspace uses the shared model, persistence store, and hosting hub rather than maintaining a second document representation. It provides the overlay collection and lifecycle actions (create, name, select, rename, and explicitly confirm deletion), plus a one-widget text editor and fixed-canvas preview. Its startup and readiness contract is:

- Resolve the platform app-local store and restore valid saved overlays before presenting a usable workspace.
- Treat a missing store as an empty workspace; do not silently replace malformed or unsupported data with an empty file.
- Keep persistence and server readiness state visible while startup is incomplete or fails; do not expose a browser-source URL until the server is ready and an overlay is selected and registered.
- Save the complete overlay collection through the versioned store after workspace changes. A successful save clears the pending change; a failed save keeps the in-memory change and dirty state, preserves the prior file, and displays a recoverable error.
- Keep startup, load, save, and shutdown failures visible and non-destructive so a user can recover without losing in-memory work or persisted data; malformed startup sources require repair and restart.
- Preserve each overlay's stable identity and browser-source URL across renames and application restarts.

Still pending for `0.0.1`:

- OBS setup instructions and end-to-end OBS verification on macOS and Linux, including target-platform validation.
- Idle CPU and memory measurements for a representative release/development build.

The intended product scope and completion requirements are tracked in [`docs/TODO-0-0-1.md`](docs/TODO-0-0-1.md). See the [current architecture inventory](docs/architecture/INDEX.md) for implemented component boundaries and operational contracts.

## Setup

### NixOS/Linux

This repository includes a Linux-only Nix flake for development. It provides stable Rust, Cargo, rustfmt, rust-analyzer, Clippy, Node.js 22, Python 3, and the native GUI/OpenSSL libraries needed to build and run the application:

```sh
nix develop
```

Run the commands in the [test and checks](#test-and-checks) section from inside that shell. The flake currently supports `x86_64-linux` and `aarch64-linux`; it does not configure macOS.

### Manual setup

For environments that do not use the Nix shell:

1. Install [Rust](https://www.rust-lang.org/tools/install) with the stable toolchain:

   ```sh
   rustup toolchain install stable
   rustup default stable
   ```

2. Install Node.js 22 or newer for the executable embedded-browser test:

   ```sh
   node --version
   ```

   The test suite requires Node.js 22.

3. Clone the repository and enter it:

   ```sh
   git clone https://github.com/marcybelardo/chikachika.git
   cd chikachika
   ```

4. On Ubuntu/Linux outside the Nix shell, install the native libraries used by CI for the GUI build:

   ```sh
   sudo apt-get update
   sudo apt-get install --no-install-recommends -y \
     libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
     libxkbcommon-dev libssl-dev libgtk-3-dev
   ```

   On macOS, install stable Rust and use a normal desktop development environment. CI validates the project on `macos-latest`; no additional CI-installed packages are currently specified there.

## Run

Start the native application from the repository root:

```sh
cargo run
```

The application starts the web server before opening the GUI and prints its health address, normally:

```text
Chikachika web server listening at http://127.0.0.1:51737/ping
```

The server is loopback-only and binds to `127.0.0.1:51737` when settings are missing, or to the valid persisted port shown in the settings panel. A saved port change takes effect on the next launch rather than rebinding the current server. While the application is open, check its health endpoint from another terminal:

```sh
curl --fail --show-error http://127.0.0.1:51737/ping
```

If a different port is configured, replace `51737` with the running port shown in the settings panel.

The expected response is:

```text
pong
```

The workspace restores saved overlays before presenting the editable collection, keeps selection and save/readiness status visible, and coordinates graceful local-server shutdown when the window closes. A selected overlay's browser-source URL, `Copy URL`, and `Open in browser` actions are available only after the server reports readiness and the overlay is registered. The settings panel shows the platform config path, validates ports from `1` through `65535`, and explains that changes apply after restart. Malformed settings and occupied ports remain visible without changing the copied URL or selecting a fallback port.

## Local overlay data

Chikachika stores the complete versioned overlay snapshot in `overlays.json` under the platform app-local data directory selected by `directories::ProjectDirs`:

| Platform | Path |
|---|---|
| Linux | `$XDG_DATA_HOME/chikachika/overlays.json` when `XDG_DATA_HOME` is an absolute path; otherwise `$HOME/.local/share/chikachika/overlays.json` |
| macOS | `$HOME/Library/Application Support/Chikachika/overlays.json` |

The application creates the parent directory when needed and never falls back to the repository or current working directory. Malformed or unsupported data is left unchanged and blocks workspace startup so it can be backed up or repaired; restart Chikachika after repairing the source file. This file is the overlay document store and does not contain the loopback server settings.

### Application settings

The application stores the loopback server port separately from overlay documents in a versioned `settings.json` envelope under the platform config location selected by `directories::ProjectDirs`.

| Platform | Path |
|---|---|
| Linux | `$XDG_CONFIG_HOME/chikachika/settings.json` when `XDG_CONFIG_HOME` is absolute; otherwise `$HOME/.config/chikachika/settings.json` |
| macOS | `$HOME/Library/Application Support/Chikachika/settings.json` |

Missing settings use `127.0.0.1:51737`. Only ports in the inclusive range `1..=65535` are valid. Malformed or unsupported settings remain unchanged, are shown as an error, and prevent server startup rather than silently falling back. A port change is saved for the next launch and does not live-rebind the current server; no alternate port is selected automatically.

## Test and checks

Run the same checks used by CI from the repository root:

```sh
cargo fmt --all -- --check
cargo test --locked --all-targets
node --test tests/browser_overlay.test.mjs
python3 scripts/check_docs.py
python3 -m unittest discover -s tests -v
```

The Rust job runs formatting and locked all-target tests on Ubuntu and macOS. The documentation job runs the Node embedded-client test, validates decision-record indexes and links, and runs the Python test suite. For a quick Rust-only test run, `cargo test` is also supported, but the locked all-target command above is the CI-equivalent check.

## Current architecture

The application is one native process. `src/main.rs` wires the application coordinator, shared `OverlayHub`, loopback server, and eframe/egui GUI; when the GUI exits, it coordinates graceful server shutdown and joins the dedicated server thread. The application coordinator owns the overlay collection, selected-overlay state, dirty state, latest user-visible error, and readiness address while the framework-independent model in `src/model.rs` remains the authoritative overlay document. The GUI editor in `src/gui.rs` routes one-widget content, style, and position changes through the coordinator, renders a fixed-aspect preview, publishes accepted revisions through the shared hub, gates exact URL actions on readiness, and owns the restart-bound port settings controls. The persistence adapter in `src/persistence.rs` stores versioned overlay JSON in the platform app-local data directory; `src/settings.rs` stores a separate versioned port envelope in the platform config-local directory; and `src/browser.rs` projects the model into a serializable complete browser snapshot and self-contained transparent HTML with compile-time embedded assets. The server runs on a dedicated current-thread Tokio runtime and exposes `GET /ping`, registered-overlay HTML at `GET /overlay/{id}`, and bounded named-SSE snapshots at `GET /overlay/{id}/events`. The [architecture inventory](docs/architecture/INDEX.md) is the authoritative current-state reference.

## Pending documentation and validation

The following documentation and validation remain pending until their corresponding implementation or verification work is complete:

- OBS setup and browser-source instructions, including macOS and Linux end-to-end checks and target-platform validation.
- Release-oriented idle CPU and memory measurements, including the build and environment used for those measurements.
