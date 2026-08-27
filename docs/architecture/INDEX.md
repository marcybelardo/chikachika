# Architecture Inventory

This directory inventories the architecture that currently exists in the repository: components, boundaries, ownership, lifecycle, persistence, interfaces, and operational contracts. It is not a history, rationale document, roadmap, or feature specification.

Use [the architecture inventory skill](../../.agents/skills/architecture-inventory/SKILL.md) when current architecture changes or needs auditing. Architectural rationale belongs in [ADRs](../adr/INDEX.md), feature behavior belongs in [FDRs](../fdr/INDEX.md), and canonical definitions belong in the [glossary](../GLOSSARY.md).

## Inventory

| Area | Document | Owns |
|---|---|---|
| Native application runtime | [Runtime and process](#runtime-and-process) | `src/main.rs` wires the application coordinator, GUI, and loopback server into one process. |
| Native GUI | [Runtime and process](#runtime-and-process) | `src/gui.rs` owns the eframe application, native window lifecycle, overlay workspace presentation, and readiness/error status. |
| Overlay domain model | [Runtime and process](#runtime-and-process) | `src/model.rs` owns the framework-independent overlay document, stable identities, supported text-widget state, and revision metadata. |
| Local persistence | [Runtime and process](#runtime-and-process) | `src/persistence.rs` owns the versioned JSON envelope, platform app-local path resolution, validated load, and safe snapshot replacement. |
| Browser projection and client | [Runtime and process](#runtime-and-process) | `src/browser.rs` owns the model-to-browser representation and compile-time embedded HTML/CSS/JavaScript assets; `assets/browser/overlay.js` consumes complete named `snapshot` updates with safe DOM APIs. |
| Overlay hosting hub | [Runtime and process](#runtime-and-process) | `src/server.rs` owns the cloneable application-facing `OverlayHub`, current model snapshots, per-overlay latest-value channels, and synchronous publish/remove/subscribe operations. |
| Loopback web server | [Runtime and process](#runtime-and-process) | `src/server.rs` owns the dedicated Tokio thread, loopback listener, `/ping`, `/overlay/{id}`, and `/overlay/{id}/events` routes, SSE keepalives, and shutdown handle. |

## Runtime and process

The current implementation is small enough to inventory in this index. `src/main.rs` wires the application coordinator, shared `OverlayHub`, loopback server, and native GUI into one process. The coordinator restores and validates the complete persisted snapshot before presenting the workspace, owns the overlay collection, selected-overlay state, dirty state, and latest user-visible error, and registers restored or newly created overlays with the hub. The GUI runs on the main thread through eframe/egui, presents the overlay list and lifecycle controls, and reports persistence and server readiness state. The framework-independent overlay model in `src/model.rs` owns the current document shape, stable UUID identities, supported text-widget properties, and monotonic revision metadata. The persistence adapter in `src/persistence.rs` owns conversion to and from the versioned JSON envelope, platform app-local path resolution, and safe replacement of complete snapshots. The coordinator keeps failed loads and saves non-destructive: malformed or unsupported data does not replace its source, and a failed save keeps in-memory changes dirty while exposing the error.

The browser adapter in `src/browser.rs` projects model snapshots into one serializable complete browser representation and transparent self-contained HTML, and owns compile-time embedded assets. The client in `assets/browser/overlay.js` opens a same-origin EventSource at the current overlay path plus `/events`, listens for named `snapshot` events, ignores stale/duplicate revisions, and safely replaces the optional text widget through controlled DOM properties. `src/server.rs` owns `OverlayHub`: a synchronized map of current `Overlay` values and per-overlay Tokio watch senders. It provides synchronous registration, revision-checked publication, removal, model snapshot lookup, and subscribe-before-read receiver creation. The server runs on a dedicated current-thread Tokio runtime with I/O and time enabled, binds the normal endpoint to `127.0.0.1:51737`, exposes `GET /ping` as plain-text `pong`, serves registered overlays at `GET /overlay/{id}`, and streams complete bounded latest-value snapshots with SSE keepalives at `GET /overlay/{id}/events`. Overlay routes remain unavailable until an overlay is registered; all production binding is loopback-only.

## Issue #4 application state

The issue #4 workspace boundary is implemented across the application coordinator and native GUI. The coordinator owns the application overlay collection, selected-overlay state, dirty state, and latest user-visible error while using the existing model, persistence store, and hosting hub as adapters rather than maintaining duplicate document state:

- Startup restores and validates the complete versioned app-local snapshot before presenting a usable workspace. A missing file starts an empty collection; malformed or unsupported data blocks restoration without replacing the source file.
- The workspace lists, selects, creates, renames, and deletes overlays only after explicit confirmation. Mutations preserve stable overlay identities and publish registered changes through the shared hub.
- Saving writes a complete collection snapshot. A successful save clears dirty state; a failed save keeps the in-memory change and dirty state, preserves the previous source, and records a visible recoverable error.
- Server startup and persistence failures remain visible and non-destructive. The workspace exposes a browser-source URL only after the server has reported readiness and an overlay is selected and registered; normal shutdown coordinates with the server thread.

This boundary deliberately leaves native text-widget editing to issue #5 and browser-source copy/open controls plus configurable-port UX to issue #8. OBS end-to-end validation on macOS and Linux, broader platform validation, and representative idle CPU/memory measurements remain pending.

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
