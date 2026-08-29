# `0.0.1` Milestone

**Status:** In progress

`0.0.1` is the first functional vertical slice of the application. It remains an unreleased pre-release target: implementation and architecture may change substantially while this checklist is in progress.

This document tracks milestone scope and completion. User-visible behavior is governed by [FDR-001: Overlay Editing and Local Browser Source](fdr/FDR-001-overlay-editing-and-local-browser-source.md), with the issue #8 URL and port additions recorded in [FDR-002: Browser-Source URL Actions and Port Settings](fdr/FDR-002-browser-source-url-actions-and-port-settings.md). This checklist does not replace Feature Decision Records (FDRs) for feature behavior or Architecture Decision Records (ADRs) for architectural rationale.

## Outcome

A streamer can create a basic text overlay in the desktop application, save it locally, serve it at a stable local URL, and use that URL as a transparent browser source in OBS. Editor changes reach the browser source without requiring a manual refresh.

## Product Requirements

### Application and overlay management (issue #4)

- [ ] The desktop application opens successfully on both required 0.0.1 targets: macOS and Linux. *(Target-platform validation remains pending.)*
- [x] Startup restores and validates the complete versioned app-local overlay snapshot before presenting a usable workspace; a missing store starts an empty collection.
- [x] Malformed or unsupported saved data blocks restoration without replacing the source file, and the failure remains visible and non-destructive; source repair requires an application restart.
- [x] The workspace lists and selects overlays while preserving stable selection state independently of dirty/save state.
- [x] A user can create an overlay, which becomes selected and dirty until successfully saved.
- [x] A user can rename an overlay without changing its stable identity or browser-source URL.
- [x] A user can delete an overlay only after explicit confirmation; selection moves to an unaffected remaining overlay when applicable.
- [x] The workspace owns one shared application overlay collection and publishes registered changes through the existing model and hosting hub rather than maintaining duplicate document state.
- [x] The application presents a usable browser-source URL only after server readiness is known and an overlay is selected and registered.
- [x] Server startup, persistence, and shutdown errors remain visible and non-destructive; normal shutdown coordinates with the server thread.

### Canvas and text editing (issue #5)

- [x] An overlay uses an explicitly configured fixed-size canvas.
- [x] An overlay supports exactly zero or one optional text widget; multiple widgets are deferred.
- [x] A user can select and move the text widget on the canvas; dragging preserves the grab offset and stays within the canvas bounds.
- [x] A user can edit the widget's text content, including multiline content.
- [x] A user can configure the text's font size, RGBA color, and alignment.
- [x] The editor provides a useful fixed-aspect visual preview, while the browser output remains authoritative for final rendering.
- [x] Layering, canvas resizing, rich text, and animation remain deferred beyond 0.0.1.

### Local persistence (issues #3 and #4)

- [x] Overlays and their supported document state are saved locally as one complete versioned snapshot.
- [x] A successful save clears dirty state.
- [x] A failed save preserves the prior source file, keeps the in-memory change and dirty state, and displays a recoverable error.
- [x] Saved overlays are restored after restarting the application.
- [x] Persisted data has an explicit format version so incompatible pre-release changes can be detected and handled deliberately.
- [x] Malformed or unsupported data and persistence errors are visible and non-destructive.

### Application settings and port lifecycle (issue #8)

The following requirements are accepted in [ADR-005](adr/ADR-005-separate-server-settings-from-overlay-documents.md) and [FDR-002](fdr/FDR-002-browser-source-url-actions-and-port-settings.md), but remain unchecked until the issue #8 implementation and verification land.

- [ ] Server settings are persisted separately from overlay documents as a versioned `settings.json` envelope in the platform config location. *(Implementation pending.)*
- [ ] Missing settings use the loopback default `127.0.0.1:51737`.
- [ ] Configured ports accept only the inclusive range `1..=65535`.
- [ ] Malformed or unsupported settings remain unchanged, are shown visibly, and prevent server startup instead of silently falling back.
- [ ] A port change is saved for the next launch and does not live-rebind the current server.
- [ ] No automatic alternate port is selected after a bind conflict.

### Browser-source hosting

- [x] The application serves each overlay at its stable local URL after the issue #4 workspace registers it. *(The server route and workspace registration are implemented.)*
- [x] The browser output has a transparent background and respects the overlay's configured canvas dimensions.
- [ ] Changes made in the editor appear in a connected browser source without a manual page refresh. *(Editor publication, bounded SSE delivery, and client-side application are implemented and covered automatically; connected-browser and OBS validation remain pending in issue #10.)*
- [ ] The application provides a straightforward way to copy the exact selected browser-source URL only after server readiness (**issue #8; implementation pending**).
- [ ] The application provides a straightforward way to open the exact selected browser output for preview or troubleshooting only after server readiness (**issue #8; implementation pending**).
- [x] Server startup and port conflicts are visible and non-destructive.
- [x] The local server binds only to the loopback interface by default.

### OBS verification

- [ ] On macOS, a served overlay is exercised end to end in OBS as a browser source using the displayed URL.
- [ ] On Linux, a served overlay is exercised end to end in OBS as a browser source using the displayed URL.
- [ ] Text content and supported styling render correctly in OBS on both required targets.
- [ ] Transparency works in OBS on both required targets.
- [ ] Live editor changes appear in OBS without recreating or manually refreshing the browser source on both required targets.

## Quality Requirements

- [ ] Automated tests cover the persisted overlay model and its round-trip behavior. *(Round-trip coverage is implemented and passes on both CI targets; milestone completion status has not yet been advanced.)*
- [x] Automated tests cover the browser representation or update protocol where practical.
- [ ] The application handles malformed or unsupported persisted data without silently losing user data. *(The non-destructive load behavior and visible blocked-startup path are implemented and covered automatically; milestone completion status has not yet been advanced.)*
- [ ] Idle CPU and memory behavior are measured in a representative development build and any material concerns are recorded.
- [ ] Setup, run, test, and OBS connection instructions are documented.
- [ ] The glossary, architecture inventory, ADRs, and FDRs are current for the implemented vertical slice. *(Issue #8 decisions and pending target ownership are documented; the final current-state pass remains required after its implementation changes the URL workflow.)*

## Explicitly Out of Scope

- Twitch or other streaming-service integrations
- Chatbots, moderation, analytics, or general stream management
- Cloud accounts, required cloud hosting, collaboration, or synchronization
- Plugin systems or marketplaces
- Rich animation and trigger systems
- General OBS scene control
- LAN or internet exposure of the local overlay server
- Portability beyond macOS and Linux as a 0.0.1 release gate

## Completion Gate

`0.0.1` is complete when every in-scope checkbox is satisfied, the macOS and Linux OBS workflows have each been exercised end to end, relevant tests pass, and current documentation matches the implementation. Any deferred requirement must be explicitly removed from this milestone through an accepted product decision rather than left implicitly incomplete.
