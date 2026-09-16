# ADR-009: Use a Secondary Native Settings Viewport

**Status:** Accepted
**Date:** 2026-09-17
**Supersedes:** None

## Context

The 0.0.2 workspace removes server settings from the main editing surface while retaining one native process, one GUI event loop, and the existing server-thread lifecycle. The secondary surface must not create another settings owner, rebind the running server, or interfere with main-window shutdown.

## Decision

- Settings uses one singleton secondary native viewport owned by the same eframe/egui application and GUI event loop as the main editor.
- Opening Settings creates the viewport only when absent. Opening it again focuses and reuses the existing viewport rather than creating a duplicate.
- Closing only the Settings viewport discards uncommitted Settings form edits without an additional prompt. It does not close the editor, stop the server, or save settings.
- Saving Settings is explicit, writes only the separate format-1 config-local `settings.json`, and applies the saved port on the next application launch only.
- The Settings viewport shows the running port, saved next-launch port, validation errors, settings path, and restart requirement.
- Port validation, missing-setting default `127.0.0.1:51737`, visible malformed/unsupported settings errors, no fallback endpoint, no alternate port, no live rebind, and readiness-gated exact URL actions remain governed by ADR-005 and FDR-002.
- Settings and overlay-document save boundaries are independent. Saving one never implicitly saves the other.
- Settings/server errors remain discoverable in the main workspace after the secondary viewport closes.
- Closing or quitting the main editor owns application shutdown: it resolves document close flow, closes the Settings viewport, gracefully stops the server, joins the server thread, and exits the single process.
- Native focus, reuse, and close behavior on macOS and Linux must be verified in issues #25 and #27; acceptance of this record does not claim those runtime checks are complete.

## Rationale

A singleton viewport makes Settings visibly separate without introducing another process or GUI runtime. Explicit save and discard-on-close give the form a predictable boundary, while next-launch application preserves the stable running endpoint. Main-window ownership of shutdown keeps ADR-001's lifecycle intact and prevents a secondary window from becoming an independent application root.

## Settings Lifecycle Scenarios

| Scenario ID | Action | Required outcome |
|---|---|---|
| `settings_open` | Open Settings while no Settings viewport exists. | Create one secondary native viewport populated from saved/running state. |
| `settings_reopen` | Invoke Settings while its viewport already exists. | Focus and reuse the singleton; do not create a duplicate or reset form edits. |
| `settings_close_unsaved` | Close Settings with uncommitted form edits. | Discard those form edits without an extra prompt; main editor and server continue, and main-visible errors remain discoverable. |
| `settings_save` | Save a valid changed port. | Persist format-1 settings explicitly for next launch; do not live-rebind or change the running URL. |
| `editor_close_with_settings` | Close the main editor while Settings is open. | Complete the document Save/Discard/Cancel flow; only successful Save or Discard proceeds to close Settings, stop/join the server, and exit. Cancel or failed save leaves the application running. |

## Alternatives Considered

- **Keep Settings embedded in the editor:** Rejected because the accepted workspace dedicates the main panels to composition and inspector behavior.
- **Create a new Settings window on every invocation:** Rejected because duplicate forms could conflict over one settings source.
- **Use another process or GUI event loop:** Rejected because it adds lifecycle and synchronization complexity and contradicts ADR-001.
- **Save automatically on close:** Rejected because closing is explicitly discard behavior and invalid or exploratory edits must not alter next-launch configuration.
- **Live-rebind after save:** Rejected because it would invalidate the running endpoint and conflicts with ADR-005/FDR-002.

## Consequences

### Positive

- Settings has a separate, reusable native surface without changing process or server ownership.
- Explicit save and discard-on-close prevent accidental configuration writes.
- The running endpoint and readiness-gated URLs remain stable for the launch.

### Negative

- Secondary viewport focus and close behavior needs platform-specific validation.
- Users must reopen Settings after discarding form changes and restart after saving a port change.
- Main shutdown must coordinate another viewport while still honoring failed-save and Cancel behavior.

## Related

- **ADRs:** [ADR-001: Run the GUI and Local Web Server in One Native Process](ADR-001-one-native-process-gui-and-server.md), [ADR-005: Separate Versioned Server Settings from Overlay Documents](ADR-005-separate-server-settings-from-overlay-documents.md)
- **FDRs:** [FDR-002: Browser-Source URL Actions and Port Settings](../fdr/FDR-002-browser-source-url-actions-and-port-settings.md), [FDR-005: Singleton Native Settings Window](../fdr/FDR-005-singleton-native-settings-window.md)
