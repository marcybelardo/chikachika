# Architecture Inventory

This directory inventories the architecture that currently exists in the repository: components, boundaries, ownership, lifecycle, persistence, interfaces, and operational contracts. It is not a history, rationale document, roadmap, or feature specification.

Use [the architecture inventory skill](../../.agents/skills/architecture-inventory/SKILL.md) when current architecture changes or needs auditing. Architectural rationale belongs in [ADRs](../adr/INDEX.md), feature behavior belongs in [FDRs](../fdr/INDEX.md), and canonical definitions belong in the [glossary](../GLOSSARY.md).

## Inventory

| Area | Document | Owns |
|---|---|---|
| Native application runtime | [Runtime and process](#runtime-and-process) | `src/main.rs` wires the GUI and loopback server into one process. |
| Native GUI | [Runtime and process](#runtime-and-process) | `src/gui.rs` owns the eframe application and native window lifecycle. |
| Overlay domain model | [Runtime and process](#runtime-and-process) | `src/model.rs` owns the framework-independent overlay document, stable identities, supported text-widget state, and revision metadata. |
| Loopback web server | [Runtime and process](#runtime-and-process) | `src/server.rs` owns the dedicated Tokio thread, loopback listener, `/ping` route, and shutdown handle. |

## Runtime and process

The current implementation is small enough to inventory in this index. `src/main.rs` starts `server::start()` before entering `gui::run()`, retains the `ServerHandle`, and calls its consuming `shutdown()` after the GUI exits. The GUI runs on the main thread through eframe/egui; the framework-independent overlay model in `src/model.rs` owns the current document shape, stable UUID identities, supported text-widget properties, and monotonic revision metadata; the server runs on a dedicated current-thread Tokio runtime and binds the normal application endpoint to `127.0.0.1:51737`. The server exposes `GET /ping` as a plain-text `pong` health response and is restricted to loopback binding.

Authoritative sources:

- [`src/main.rs`](../../src/main.rs)
- [`src/gui.rs`](../../src/gui.rs)
- [`src/model.rs`](../../src/model.rs)
- [`src/server.rs`](../../src/server.rs)
- [`ADR-001`](../adr/ADR-001-one-native-process-gui-and-server.md)
- [`ADR-002`](../adr/ADR-002-shared-overlay-model-and-stable-ids.md)
- [`ADR-004`](../adr/ADR-004-loopback-sse-browser-delivery.md)

Current limitations: overlay routes, persistence, browser assets, and SSE delivery are not implemented yet.
