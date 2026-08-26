# ADR-001: Run the GUI and Local Web Server in One Native Process

**Status:** Accepted
**Date:** 2026-08-26
**Supersedes:** None

## Context

The 0.0.1 application needs a native editor and a local browser-source server that share one overlay document. The desktop workflow is local-first, must remain understandable on macOS and Linux, and must report startup and failure states visibly. The browser output is a local delivery surface rather than a second application, while the GUI must remain responsive during HTTP serving and must shut down without leaving a server thread behind.

## Decision

- eframe/egui owns the native GUI event loop on the main thread.
- A dedicated server thread owns the Tokio runtime and the axum HTTP server.
- The server reports its successfully bound address to the GUI before the GUI presents a usable browser-source URL.
- Normal GUI shutdown signals graceful server shutdown and joins the server thread before process exit.
- The dedicated server thread uses a current-thread Tokio runtime because 0.0.1 has low local concurrency.
- A future change from the current-thread runtime requires a superseding ADR justified by measured needs.
- Browser HTML, CSS, and JavaScript assets are compiled into the executable with standard `include_str!` and `include_bytes!` macros.
- The embedded asset set stays small and does not depend on the runtime working directory.

## Rationale

One native process keeps the document ownership and lifecycle visible without IPC, while separate execution contexts prevent HTTP work from blocking the GUI event loop. A dedicated current-thread runtime is proportionate to a low-concurrency local server and leaves a measured path to reconsideration. Compile-time embedding makes the browser output self-contained and avoids fragile launch-directory assumptions.

## Alternatives Considered

- Separate GUI and server processes are rejected because they add IPC, lifecycle coordination, and duplicated failure surfaces for this local slice.
- An embedded Chromium editor is rejected because it adds a heavyweight browser runtime when the native editor and browser output already have separate responsibilities.
- Running the current-thread Tokio runtime under the GUI event loop is rejected because server work could block native event processing.
- A heavier asset framework is rejected because the small 0.0.1 asset set needs no runtime asset packaging or discovery layer.

## Consequences

### Positive

- The native editor, server lifecycle, and browser URL status have one process boundary and an explicit shutdown path.
- The GUI event loop remains isolated from server execution, and the browser bundle works independently of the launch directory.
- Runtime complexity stays proportional to the local concurrency expected in 0.0.1.

### Negative

- A server startup failure or thread-join failure must be surfaced by the native application rather than isolated in another process.
- The current-thread runtime is not a general high-concurrency server design and must be revisited if measurements show that need.
- Compile-time asset embedding increases the executable whenever browser assets change and requires a rebuild for asset updates.

## Related

- **ADRs:** None
- **FDRs:** [FDR-001: Overlay Editing and Local Browser Source](../fdr/FDR-001-overlay-editing-and-local-browser-source.md)
