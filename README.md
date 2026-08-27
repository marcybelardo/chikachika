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
- Versioned JSON persistence in the platform app-local data directory, including non-destructive malformed-data handling and safe replacement.
- A compile-time embedded transparent browser renderer for the current overlay model.
- Server-side hosting for registered overlays at stable `/overlay/{overlay-id}` HTML routes with bounded `/overlay/{overlay-id}/events` SSE snapshots.
- Issue #4 application-state wiring for overlay collection, selection, lifecycle actions, dirty/save state, visible errors, and server shutdown coordination.

### Native overlay workspace (issue #4)

The native workspace uses the shared model, persistence store, and hosting hub rather than maintaining a second document representation. It provides the overlay collection and lifecycle actions (create, name, select, rename, and explicitly confirm deletion). Its startup and readiness contract is:

- Resolve the platform app-local store and restore valid saved overlays before presenting a usable workspace.
- Treat a missing store as an empty workspace; do not silently replace malformed or unsupported data with an empty file.
- Keep persistence and server readiness state visible while startup is incomplete or fails; do not expose a browser-source URL until the server is ready and an overlay is selected and registered.
- Save the complete overlay collection through the versioned store after workspace changes. A successful save clears the pending change; a failed save keeps the in-memory change and dirty state, preserves the prior file, and displays a recoverable error.
- Keep startup, load, save, and shutdown failures visible and non-destructive so a user can retry or recover without losing in-memory work or persisted data.
- Preserve each overlay's stable identity and browser-source URL across renames and application restarts.

Still pending beyond issue #4:

- Text-widget selection, movement, content editing, and supported styling in the native workspace (**issue #5**).
- User-facing browser-source URL copy/open actions and configurable-port UX (**issue #8**); the current normal-run default remains loopback `127.0.0.1:51737`.
- OBS setup instructions and end-to-end OBS verification on macOS and Linux, including target-platform validation.
- Idle CPU and memory measurements for a representative release/development build.

The intended product scope and completion requirements are tracked in [`docs/TODO-0-0-1.md`](docs/TODO-0-0-1.md). See the [current architecture inventory](docs/architecture/INDEX.md) for implemented component boundaries and operational contracts.

## Setup

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

3. On Ubuntu/Linux, install the native libraries used by the GUI build (these are the dependencies installed by CI):

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

The server is loopback-only and binds to `127.0.0.1:51737` during a normal application run. While the application is open, check its health endpoint from another terminal:

```sh
curl --fail --show-error http://127.0.0.1:51737/ping
```

The expected response is:

```text
pong
```

The workspace restores saved overlays before presenting the editable collection, keeps selection and save/readiness status visible, and coordinates graceful local-server shutdown when the window closes. A selected overlay's browser-source URL is available only after the server reports readiness and the overlay is registered.

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

The application is one native process. `src/main.rs` wires the application coordinator, shared `OverlayHub`, loopback server, and eframe/egui GUI; when the GUI exits, it coordinates graceful server shutdown and joins the dedicated server thread. The application coordinator owns the overlay collection, selected-overlay state, dirty state, and latest user-visible error while the framework-independent model in `src/model.rs` remains the authoritative overlay document. The persistence adapter in `src/persistence.rs` stores versioned JSON in the platform app-local data directory, and `src/browser.rs` projects the model into a serializable complete browser snapshot and self-contained transparent HTML with compile-time embedded assets. The server runs on a dedicated current-thread Tokio runtime and exposes `GET /ping`, registered-overlay HTML at `GET /overlay/{id}`, and bounded named-SSE snapshots at `GET /overlay/{id}/events`. The [architecture inventory](docs/architecture/INDEX.md) is the authoritative current-state reference.

## Pending documentation and validation

The following documentation and validation remain pending until their corresponding implementation or verification work is complete:

- Native text-widget selection, movement, content editing, supported styling, and editor-driven browser publication (**issue #5**).
- Browser-source URL copy/open controls and configurable-port UX (**issue #8**).
- OBS setup and browser-source instructions, including macOS and Linux end-to-end checks and target-platform validation.
- Release-oriented idle CPU and memory measurements, including the build and environment used for those measurements.
