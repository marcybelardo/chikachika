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
- [`src/gui.rs`](../../src/gui.rs)

## Inventory

| Area | Document | Owns |
|---|---|---|
| Native application runtime | [Runtime and process](#runtime-and-process) | `src/main.rs` wires the application coordinator, GUI, and loopback server into one process. |
| Native GUI | [Runtime and process](#runtime-and-process) | `src/gui.rs` owns the eframe application, native window lifecycle, current workspace presentation, and readiness/error status. |
| Overlay domain model | [Runtime and process](#runtime-and-process) | `src/model.rs` owns the framework-independent overlay collection, stable identities, ordered text-widget state, and durable properties. |
| Application coordinator | [Runtime and process](#runtime-and-process) | `src/app.rs::HeadlessCoordinator` owns widget/overlay selection, the saved-content baseline, candidate validation, and hub-facing mutation publication. |
| Local persistence | [Runtime and process](#runtime-and-process) | `src/persistence.rs` owns the version-2 JSON envelope, platform app-local path resolution, validated load, and safe snapshot replacement. |
| Application settings | [Runtime and process](#runtime-and-process) | `src/settings.rs` owns the separate version-1 `settings.json` envelope, platform config-local path resolution, port validation, and next-launch replacement. |
| Browser projection and client | [Runtime and process](#runtime-and-process) | `src/browser.rs` owns model-to-browser projection and embedded HTML/CSS/JavaScript assets; `assets/browser/overlay.js` consumes complete `widgets`-array snapshots and reconciles DOM nodes by widget ID. |
| Overlay hosting hub | [Runtime and process](#runtime-and-process) | `src/server.rs` owns the cloneable application-facing `OverlayHub`, current model snapshots, runtime revisions/high-water marks, and per-overlay latest-value channels. |
| Loopback web server | [Runtime and process](#runtime-and-process) | `src/server.rs` owns the dedicated Tokio thread, loopback listener, `/ping`, `/overlay/{id}`, and `/overlay/{id}/events` routes, SSE keepalives, and shutdown handle. |

## Runtime and process

The current implementation is small enough to inventory in this index. `src/main.rs` wires the application coordinator, shared `OverlayHub`, loopback server, and native GUI into one process. The coordinator restores and validates the complete persisted snapshot before presenting the workspace, owns the overlay collection, selected-overlay and selected-widget state, dirty state, latest user-visible error, and readiness address, and registers restored or newly created overlays with the hub. The GUI runs on the main thread through eframe/egui, presents the current widget selector, inspector, collection preview, lifecycle controls, persistence state, and server readiness state. The GUI does not own a second document or selection store.

The framework-independent overlay model in `src/model.rs` owns the durable ordered collection of zero or more text widgets. Widget and overlay IDs are UUID v4 values; model index 0 is frontmost. Add and duplicate insert at index 0, adjacent forward/backward operations swap order without wrapping, and the model validates IDs, names, finite positions, canvas dimensions, font sizes, and supported font IDs. The model’s equality is durable-content equality; selection, history, and delivery revisions are outside it.

`HeadlessCoordinator` in `src/app.rs` owns selection and the last successfully saved whole-collection `saved_content` baseline. Switching overlays clears widget selection; accepted add/duplicate/delete operations repair selection by stable ID/index rules. A candidate replacement is validated and published before coordinator state is committed, so rejected operations preserve document, selection, baseline, and publication state. A successful save replaces the baseline; a failed save keeps current work dirty and exposes the error.

The persistence adapter in `src/persistence.rs` converts the model to and from a strongly typed JSON envelope with `format_version: 2`. `overlays.json` stores overlay IDs, names, canvas dimensions, ordered widgets, widget IDs, names, multiline content, font IDs, positions, sizes, RGBA colors, and alignment. It omits selection, delivery revisions/high-water marks, pending edits, gestures, hover, and history. The loader checks the envelope version before decoding the format-specific structure, validates the complete collection, and returns no candidate on malformed, unsupported, duplicate, unknown-font, or invalid data. `HeadlessCoordinator::bootstrap_outcome` exposes a blocked result with no editable coordinator for an existing incompatible source.

Save I/O clones and serializes a complete collection before file I/O, creates a temporary file in the destination directory, writes and syncs it, closes it, and replaces the destination. This is safe replacement under ordinary filesystem semantics and same-directory/platform replacement behavior; it is not a claim of power-loss durability or protection from physical I/O failure. Failed writes, syncs, or replacements leave the previous destination and in-memory dirty work available for recovery.

The settings adapter in `src/settings.rs` is separate from overlay documents. It resolves `ProjectDirs::from("", "", "Chikachika").config_local_dir()/settings.json`, stores a version-1 envelope containing the validated loopback port, and applies a changed port only on the next launch. A missing settings file uses `127.0.0.1:51737`; malformed, unsupported, or invalid settings block server startup rather than using a fallback or silently rebinding. The GUI surfaces the exact settings path when path resolution succeeds.

`OverlayHub` in `src/server.rs` owns running-session delivery revisions and retained per-ID high-water marks, not durable model content. Fresh registration starts at revision 0; accepted changed publication increments the revision; no-op publication does not; a deleted ID re-registers above its retained high-water mark; a fresh process resets delivery revisions while persisted identities remain. Revision exhaustion is an explicit atomic error. The hub exposes complete current snapshots through the existing read-only routes and bounded latest-value watch channels.

`src/browser.rs` projects model snapshots with an explicit hub revision and renders initial HTML by traversing model widgets in reverse order, so model index 0 is painted last/frontmost. The embedded browser client in `assets/browser/overlay.js` validates the complete `widgets` array—including unique UUID-v4 IDs, supported font IDs, finite/in-bounds render fields, and safe revisions—before changing the DOM or advancing its revision. It reconciles nodes through a widget-ID map, removes absent nodes, and appends reverse model order so index 0 remains frontmost. `Number.MAX_SAFE_INTEGER` is accepted; larger or unsafe revisions are rejected. Browser output remains transparent, with user content assigned through safe DOM text properties.

## Paths and operational boundary

Overlay documents resolve through `ProjectDirs::from("", "", "Chikachika").data_local_dir()/overlays.json`; settings resolve through the same unqualified project identity and `config_local_dir()/settings.json`. On Linux, `data_local_dir` and `config_local_dir` follow the absolute `XDG_DATA_HOME`/`XDG_CONFIG_HOME` values or the standard `$HOME/.local/share`/`$HOME/.config` locations. On macOS, both are below `$HOME/Library/Application Support/Chikachika`. The application surfaces the settings path in the settings panel and the overlay source path in blocked persistence recovery. Neither adapter falls back to the working directory.

The loopback server runs on a dedicated current-thread Tokio runtime, binds `127.0.0.1` and the validated configured port, and exposes `GET /ping`, `GET /overlay/{id}`, and `GET /overlay/{id}/events`. The event stream sends a complete current snapshot first, then complete replacements on accepted changes, with keepalives and no historical replay. Overlay URLs are built from the ready address and stable overlay ID; they are not based on names or collection indexes.

## Related decisions

- [`ADR-001`](../adr/ADR-001-one-native-process-gui-and-server.md)
- [`ADR-002`](../adr/ADR-002-shared-overlay-model-and-stable-ids.md)
- [`ADR-003`](../adr/ADR-003-versioned-app-local-json-persistence.md)
- [`ADR-004`](../adr/ADR-004-loopback-sse-browser-delivery.md)
- [`ADR-005`](../adr/ADR-005-separate-server-settings-from-overlay-documents.md)
- [`ADR-006`](../adr/ADR-006-ordered-authoritative-widget-model.md)
- [`ADR-007`](../adr/ADR-007-version-2-overlay-document-persistence.md)
- [`ADR-008`](../adr/ADR-008-coordinator-owned-workspace-history.md)
- [`ADR-009`](../adr/ADR-009-secondary-native-settings-viewport-lifecycle.md)
- [`FDR-003`](../fdr/FDR-003-multi-widget-composition-workspace.md)
- [`FDR-004`](../fdr/FDR-004-bundled-offline-text-fonts.md)
- [`FDR-005`](../fdr/FDR-005-singleton-native-settings-window.md)
