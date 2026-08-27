# Chikachika

Chikachika is a pre-release local-first desktop application for creating and using overlays. The current repository contains the initial native GUI and loopback server, a framework-independent overlay model, local persistence, an embedded browser renderer, and in-memory per-overlay HTTP/SSE hosting infrastructure. The native overlay editor and user-facing browser-source workflow are not implemented yet.

## Status

**Current status: pre-release implementation (target: `0.0.1`).**

Implemented in the current baseline:

- A native eframe/egui window that opens with a readiness view.
- A local web server started by the application in the same process.
- A `GET /ping` health endpoint that returns the plain-text response `pong`.
- Automated Rust and documentation checks.
- A framework-independent overlay model with stable UUID identities, fixed canvas validation, and zero-or-one text-widget support.
- Versioned JSON persistence in the platform app-local data directory, including non-destructive malformed-data handling and safe replacement.
- A compile-time embedded transparent browser renderer for the current overlay model.

Not implemented yet:

- Overlay creation, editing, or management in the native GUI.
- Native-GUI overlay lifecycle or text editing.
- User-facing stable URL controls, such as copy/open actions, or a GUI workflow that registers overlays with the running server.
- OBS connection instructions or end-to-end OBS verification on macOS and Linux.

The server-side hosting infrastructure is implemented for registered overlays: it provides stable `/overlay/{overlay-id}` HTML routes and bounded `/overlay/{overlay-id}/events` SSE snapshots. The current readiness GUI does not yet create or register overlays, so no usable overlay URL is exposed by the application run.
- Idle CPU or memory measurements for a representative release/development build.

The intended product scope and completion requirements are tracked in [`docs/TODO-0-0-1.md`](docs/TODO-0-0-1.md). Planned behavior should not be read as implemented behavior. See the [current architecture inventory](docs/architecture/INDEX.md) for the implemented component boundaries and limitations.

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

The current GUI does not create overlays or expose browser-source URLs yet. Closing the GUI shuts down the local server.

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

The application is one native process. `src/main.rs` creates an `OverlayHub`, starts the loopback server with that shared state, then runs the eframe/egui GUI; when the GUI exits, it signals the server and shuts down its dedicated thread. The framework-independent model in `src/model.rs` is the authoritative overlay document. The persistence adapter in `src/persistence.rs` stores versioned JSON in the platform app-local data directory, and `src/browser.rs` projects the model into a serializable complete browser snapshot and self-contained transparent HTML with compile-time embedded assets. The server runs on a dedicated current-thread Tokio runtime and exposes `GET /ping`, registered-overlay HTML at `GET /overlay/{id}`, and bounded named-SSE snapshots at `GET /overlay/{id}/events`. The current readiness GUI does not yet create/register overlays or expose their URLs. The [architecture inventory](docs/architecture/INDEX.md) is the authoritative current-state reference.

## Pending documentation and validation

The following documentation must wait for the corresponding implementation and verification work:

- OBS setup and browser-source instructions, including macOS and Linux end-to-end checks.
- The application’s overlay URL contract and any additional data-format guidance beyond the implemented persistence adapter.
- Release-oriented idle CPU and memory measurements, including the build and environment used for those measurements.
