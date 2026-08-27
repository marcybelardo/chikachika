# Chikachika

Chikachika is a pre-release local-first desktop application for creating and using overlays. The current repository contains the initial native GUI and loopback health-server slice; the overlay editor and browser-source workflow are not implemented yet.

## Status

**Current status: pre-release implementation (target: `0.0.1`).**

Implemented in the current baseline:

- A native eframe/egui window that opens with a readiness view.
- A local web server started by the application in the same process.
- A `GET /ping` health endpoint that returns the plain-text response `pong`.
- Automated Rust and documentation checks.

Not implemented yet:

- Overlay creation, editing, rendering, or management.
- Browser-source overlay routes, browser assets, or live SSE updates.
- Local overlay persistence or a documented persistence location.
- Stable overlay URLs. The current server has only the health endpoint; do not configure an OBS Browser Source from this baseline.
- OBS connection instructions or end-to-end OBS verification on macOS and Linux.
- Idle CPU or memory measurements for a representative release/development build.

The intended product scope and completion requirements are tracked in [`docs/TODO-0-0-1.md`](docs/TODO-0-0-1.md). Planned behavior should not be read as implemented behavior. See the [current architecture inventory](docs/architecture/INDEX.md) for the implemented component boundaries and limitations.

## Setup

1. Install [Rust](https://www.rust-lang.org/tools/install) with the stable toolchain:

   ```sh
   rustup toolchain install stable
   rustup default stable
   ```

2. Clone the repository and enter it:

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

The application starts the web server before opening the GUI and prints its address, normally:

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

There are no overlay or OBS URLs to use yet. Closing the GUI shuts down the local server.

## Test and checks

Run the same checks used by CI from the repository root:

```sh
cargo fmt --all -- --check
cargo test --locked --all-targets
python3 scripts/check_docs.py
python3 -m unittest discover -s tests -v
```

The Rust job runs formatting and locked all-target tests on Ubuntu and macOS. The documentation job validates decision-record indexes and links, then runs the Python test suite. For a quick Rust-only test run, `cargo test` is also supported, but the locked all-target command above is the CI-equivalent check.

## Current architecture

The application is one native process. `src/main.rs` starts the loopback server, then runs the eframe/egui GUI; when the GUI exits, it shuts down the server. The server runs on a dedicated current-thread Tokio runtime and currently exposes only `GET /ping`. The [architecture inventory](docs/architecture/INDEX.md) is the authoritative current-state reference.

## Pending documentation and validation

The following documentation must wait for the corresponding implementation and verification work:

- OBS setup and browser-source instructions, including macOS and Linux end-to-end checks.
- The actual overlay URL contract and any persistence path or data-format guidance.
- Release-oriented idle CPU and memory measurements, including the build and environment used for those measurements.
