# Architecture Inventory

This directory inventories the architecture that currently exists in the repository: components, boundaries, ownership, lifecycle, persistence, interfaces, and operational contracts. It is not a history, rationale document, roadmap, or feature specification.

Use [the architecture inventory skill](../../.agents/skills/architecture-inventory/SKILL.md) when current architecture changes or needs auditing. Architectural rationale belongs in [ADRs](../adr/INDEX.md), feature behavior belongs in [FDRs](../fdr/INDEX.md), and canonical definitions belong in the [glossary](../GLOSSARY.md).

## Authoritative sources

- [`src/main.rs`](../../src/main.rs)
- [`src/app.rs`](../../src/app.rs)
- [`src/model.rs`](../../src/model.rs)
- [`src/persistence.rs`](../../src/persistence.rs)
- [`src/settings.rs`](../../src/settings.rs)
- [`src/server.rs`](../../src/server.rs)

## Inventory

| Area | Document | Owns |
|---|---|---|
| Native application runtime | [Runtime and process](#runtime-and-process) | `src/main.rs` wires the application coordinator, GUI, and loopback server into one process. |
| Native GUI | [Runtime and process](#runtime-and-process) | `src/gui.rs` owns the eframe application, native window lifecycle, overlay workspace presentation, and readiness/error status. |
| Overlay domain model | [Runtime and process](#runtime-and-process) | `src/model.rs` owns the framework-independent overlay document, stable identities, supported text-widget state, and revision metadata. |
| Local persistence | [Runtime and process](#runtime-and-process) | `src/persistence.rs` owns the versioned JSON envelope, platform app-local path resolution, validated load, and safe snapshot replacement. |
| Application settings | [Runtime and process](#runtime-and-process) | `src/settings.rs` owns the separate versioned `settings.json` envelope, platform config-local path resolution, port validation, and safe next-launch replacement. |
| Browser projection and client | [Runtime and process](#runtime-and-process) | `src/browser.rs` owns the model-to-browser representation and compile-time embedded HTML/CSS/JavaScript assets; `assets/browser/overlay.js` consumes complete named `snapshot` updates with safe DOM APIs. |
| Overlay hosting hub | [Runtime and process](#runtime-and-process) | `src/server.rs` owns the cloneable application-facing `OverlayHub`, current model snapshots, per-overlay latest-value channels, and synchronous publish/remove/subscribe operations. |
| Loopback web server | [Runtime and process](#runtime-and-process) | `src/server.rs` owns the dedicated Tokio thread, loopback listener, `/ping`, `/overlay/{id}`, and `/overlay/{id}/events` routes, SSE keepalives, and shutdown handle. |

## Runtime and process

The current implementation is small enough to inventory in this index. `src/main.rs` wires the application coordinator, shared `OverlayHub`, loopback server, and native GUI into one process. The coordinator restores and validates the complete persisted snapshot before presenting the workspace, owns the overlay collection, selected-overlay state, dirty state, latest user-visible error, and readiness address, and registers restored or newly created overlays with the hub. The GUI runs on the main thread through eframe/egui, presents the overlay list and lifecycle controls, renders the one-widget editor and fixed-aspect preview, reports persistence and server readiness state, and provides readiness-gated exact URL actions plus restart-bound port settings. The framework-independent overlay model in `src/model.rs` owns the current document shape, stable UUID identities, supported text-widget properties, and monotonic revision metadata. The persistence adapter in `src/persistence.rs` owns conversion to and from the versioned overlay JSON envelope, platform app-local path resolution, and safe replacement of complete snapshots. The settings adapter in `src/settings.rs` owns conversion to and from the separate versioned port envelope, platform config-local path resolution, strict port validation, and safe next-launch replacement. The coordinator keeps failed loads and saves non-destructive: malformed or unsupported data does not replace its source, and a failed save keeps in-memory changes dirty while exposing the error.

The browser adapter in `src/browser.rs` projects model snapshots into one serializable complete browser representation and transparent self-contained HTML, and owns compile-time embedded assets. The client in `assets/browser/overlay.js` opens a same-origin EventSource at the current overlay path plus `/events`, listens for named `snapshot` events, ignores stale/duplicate revisions, and safely replaces the optional text widget through controlled DOM properties. `src/server.rs` owns `OverlayHub`: a synchronized map of current `Overlay` values and per-overlay Tokio watch senders. It provides synchronous registration, revision-checked publication, removal, model snapshot lookup, and subscribe-before-read receiver creation. The server runs on a dedicated current-thread Tokio runtime with I/O and time enabled, binds the normal endpoint to loopback `127.0.0.1:51737` when settings are missing or to the validated persisted port, exposes `GET /ping` as plain-text `pong`, serves registered overlays at `GET /overlay/{id}`, and streams complete bounded latest-value snapshots with SSE keepalives at `GET /overlay/{id}/events`. Overlay routes remain unavailable until an overlay is registered; all production binding is loopback-only.

## Application state and editor boundary

The application coordinator and native GUI jointly implement the workspace and editor boundary. The coordinator owns the application overlay collection, selected-overlay state, dirty state, and latest user-visible error while using the existing model, persistence store, and hosting hub as adapters rather than maintaining duplicate document state:

- Startup restores and validates the complete versioned app-local snapshot before presenting a usable workspace. A missing file starts an empty collection; malformed or unsupported data blocks restoration without replacing the source file and requires source repair plus restart.
- The workspace lists, selects, creates, renames, and deletes overlays only after explicit confirmation. Mutations preserve stable overlay identities and publish registered changes through the shared hub.
- Saving writes a complete collection snapshot. A successful save clears dirty state; a failed save keeps the in-memory change and dirty state, preserves the previous source, and records a visible recoverable error. Linux stores the source below `$XDG_DATA_HOME/chikachika` or `$HOME/.local/share/chikachika`; macOS stores it below `$HOME/Library/Application Support/Chikachika`.
- Server startup and persistence failures remain visible and non-destructive. A server bind failure leaves the loaded workspace saveable without exposing a URL; the workspace exposes a browser-source URL only after the server has reported readiness and an overlay is selected and registered; normal shutdown coordinates with the server thread.
- The native editor exposes the optional text widget's content, font size, RGBA color, alignment, and canvas position. It renders a fixed-aspect preview with browser-equivalent right-bounded text layout, accepts movement only through the widget hit target, preserves drag offset, clamps movement to model-valid canvas bounds, and routes accepted changes through `HeadlessCoordinator::update_overlay` for dirty-state tracking and hub publication.

Current limitation: the editor's native preview uses egui text metrics, so exact glyph metrics and line breaking may differ from the authoritative browser renderer.

## Settings and URL-action boundary

The loopback port is stored separately from overlay documents in the versioned `settings.json` envelope under the platform config-local directory resolved by `directories::ProjectDirs`. Missing settings use `127.0.0.1:51737`; malformed or unsupported settings block server startup without replacement or fallback; a changed port takes effect only after restart. The GUI displays, copies, or opens only the exact selected URL after server readiness and hub/workspace consistency are established. A bind conflict remains visible and does not select an alternate port.

## Related decisions

- [`ADR-001`](../adr/ADR-001-one-native-process-gui-and-server.md)
- [`ADR-002`](../adr/ADR-002-shared-overlay-model-and-stable-ids.md)
- [`ADR-003`](../adr/ADR-003-versioned-app-local-json-persistence.md)
- [`ADR-004`](../adr/ADR-004-loopback-sse-browser-delivery.md)
- [`ADR-005`](../adr/ADR-005-separate-server-settings-from-overlay-documents.md)
