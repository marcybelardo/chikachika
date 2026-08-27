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
| Browser projection and client | [Runtime and process](#runtime-and-process) | `src/browser.rs` owns the model-to-browser representation and compile-time embedded HTML/CSS/JavaScript assets; `assets/browser/overlay.js` consumes complete named `snapshot` updates with safe DOM APIs. |
| Overlay hosting hub | [Runtime and process](#runtime-and-process) | `src/server.rs` owns the cloneable application-facing `OverlayHub`, current model snapshots, per-overlay latest-value channels, and synchronous publish/remove/subscribe operations. |
| Loopback web server | [Runtime and process](#runtime-and-process) | `src/server.rs` owns the dedicated Tokio thread, loopback listener, `/ping`, `/overlay/{id}`, and `/overlay/{id}/events` routes, SSE keepalives, and shutdown handle. |

## Runtime and process

The current implementation is small enough to inventory in this index. `src/main.rs` creates one `OverlayHub`, starts the loopback server with that shared state before entering `gui::run()`, retains the `ServerHandle`, and calls its consuming `shutdown()` after the GUI exits. The GUI runs on the main thread through eframe/egui and remains a readiness view; it does not yet create or register overlays. The framework-independent overlay model in `src/model.rs` owns the current document shape, stable UUID identities, supported text-widget properties, and monotonic revision metadata. The persistence adapter in `src/persistence.rs` owns conversion to and from the versioned JSON envelope, platform app-local path resolution, and safe replacement of complete snapshots, but application startup does not yet load or register persisted overlays.

The browser adapter in `src/browser.rs` projects model snapshots into one serializable complete browser representation and transparent self-contained HTML, and owns compile-time embedded assets. The client in `assets/browser/overlay.js` opens a same-origin EventSource at the current overlay path plus `/events`, listens for named `snapshot` events, ignores stale/duplicate revisions, and safely replaces the optional text widget through controlled DOM properties. `src/server.rs` owns `OverlayHub`: a synchronized map of current `Overlay` values and per-overlay Tokio watch senders. It provides synchronous registration, revision-checked publication, removal, model snapshot lookup, and subscribe-before-read receiver creation. The server runs on a dedicated current-thread Tokio runtime with I/O and time enabled, binds the normal endpoint to `127.0.0.1:51737`, exposes `GET /ping` as plain-text `pong`, serves registered overlays at `GET /overlay/{id}`, and streams complete bounded latest-value snapshots with SSE keepalives at `GET /overlay/{id}/events`. Overlay routes remain unavailable until an overlay is registered; all production binding is loopback-only.

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

Current limitations: the native GUI does not yet manage overlays or consume the model/persistence adapters; application startup does not restore persisted overlays or register them with the hub; stable URL copy/open controls, GUI-driven live browser updates, OBS validation, and platform resource measurements remain unimplemented or unverified.
