# Architecture Inventory

This directory inventories the architecture that currently exists in the repository: components, boundaries, ownership, lifecycle, persistence, interfaces, and operational contracts. It is not a history, rationale document, roadmap, or feature specification.

Use [the architecture inventory skill](../../.agents/skills/architecture-inventory/SKILL.md) when current architecture changes or needs auditing. Architectural rationale belongs in [ADRs](../adr/INDEX.md), feature behavior belongs in [FDRs](../fdr/INDEX.md), and canonical definitions belong in the [glossary](../GLOSSARY.md).

## Inventory

| Area | Document | Owns |
|---|---|---|
| Native application runtime | [Runtime and process](#runtime-and-process) | `src/main.rs` wires the GUI and loopback server into one process. |
| Native GUI | [Runtime and process](#runtime-and-process) | `src/gui.rs` owns the eframe application and native window lifecycle. |
| Overlay domain model | [Runtime and process](#runtime-and-process) | `src/model.rs` owns the framework-independent overlay document, stable identities, supported text-widget state, and revision metadata. |
| Local persistence | [Runtime and process](#runtime-and-process) | `src/persistence.rs` owns the versioned JSON envelope, platform app-local path resolution, validated load, and safe snapshot replacement. |
| Browser projection | [Runtime and process](#runtime-and-process) | `src/browser.rs` owns the model-to-browser representation and compile-time embedded HTML/CSS/JavaScript assets; it does not own HTTP hosting or live transport. |
| Loopback web server | [Runtime and process](#runtime-and-process) | `src/server.rs` owns the dedicated Tokio thread, loopback listener, `/ping` route, and shutdown handle. |

## Runtime and process

The current implementation is small enough to inventory in this index. `src/main.rs` starts `server::start()` before entering `gui::run()`, retains the `ServerHandle`, and calls its consuming `shutdown()` after the GUI exits. The GUI runs on the main thread through eframe/egui; the framework-independent overlay model in `src/model.rs` owns the current document shape, stable UUID identities, supported text-widget properties, and monotonic revision metadata. The persistence adapter in `src/persistence.rs` owns conversion to and from the versioned JSON envelope, platform app-local path resolution, and safe replacement of complete snapshots. The browser adapter in `src/browser.rs` projects model snapshots into transparent self-contained HTML and owns compile-time embedded assets, but does not host routes or live updates. The server runs on a dedicated current-thread Tokio runtime and binds the normal application endpoint to `127.0.0.1:51737`. The server exposes `GET /ping` as a plain-text `pong` health response and is restricted to loopback binding.

Authoritative sources:

- [`src/main.rs`](../../src/main.rs)
- [`src/gui.rs`](../../src/gui.rs)
- [`src/model.rs`](../../src/model.rs)
- [`src/persistence.rs`](../../src/persistence.rs)
- [`src/browser.rs`](../../src/browser.rs)
- [`src/server.rs`](../../src/server.rs)
- [`assets/browser/index.html`](../../assets/browser/index.html)
- [`ADR-001`](../adr/ADR-001-one-native-process-gui-and-server.md)
- [`ADR-002`](../adr/ADR-002-shared-overlay-model-and-stable-ids.md)
- [`ADR-003`](../adr/ADR-003-versioned-app-local-json-persistence.md)
- [`ADR-004`](../adr/ADR-004-loopback-sse-browser-delivery.md)

Current limitations: the native GUI does not yet manage overlays or consume the model/persistence adapters; the server does not yet host overlay routes, browser output, or SSE delivery; stable URL controls, live browser updates, OBS validation, and platform resource measurements remain unimplemented or unverified.
