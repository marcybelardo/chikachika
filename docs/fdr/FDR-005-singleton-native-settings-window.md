# FDR-005: Singleton Native Settings Window

**Status:** Accepted
**Date:** 2026-09-17
**Supersedes:** None

## Overview

The 0.0.2 workspace moves server Settings into a separate singleton native window while preserving FDR-002's exact URL readiness, port validation, error, and restart contracts. This supplements rather than supersedes FDR-002.

## User-visible Behavior

- File > Settings opens one secondary native Settings window; invoking it again focuses and reuses the existing window.
- Closing Settings discards unsaved form edits without another prompt and leaves the main editor and server running.
- Settings shows running port, saved next-launch port, validation errors, settings path, and restart requirement.
- Saving is explicit and affects the next launch only. It does not live-rebind the current server or save overlay documents.
- Settings/server errors remain discoverable from the main workspace after Settings closes.
- Closing the main editor owns secondary-window and server shutdown after the document Save/Discard/Cancel flow succeeds.

## Feature Decisions

### 1. Open one reusable secondary native window

**Decision:** Settings appears in a singleton secondary native window in the same application. Reopening focuses/reuses it rather than creating duplicates.

**Why:** Separation leaves the main workspace focused on composition while one settings form prevents conflicting edits.

**Tradeoff:** Native focus and reuse behavior differs by platform and must be verified on macOS and Linux under issues #25 and #27.

### 2. Discard unsaved form edits on Settings close

**Decision:** Closing only Settings discards uncommitted Settings form edits without an extra prompt. It neither closes the editor nor stops the server. Reopening starts from current saved/running state.

**Why:** The explicit Save action is the single commit boundary, and dismissing a small configuration form should be predictable.

**Tradeoff:** Accidentally closed form edits cannot be recovered and receive no additional warning.

### 3. Preserve explicit next-launch port behavior

**Decision:** The window shows and preserves FDR-002's contracts: valid ports are `1..=65535`; missing settings mean `127.0.0.1:51737`; malformed or unsupported settings and bind conflicts remain visible; no alternate/fallback port is chosen; Save applies on next launch and never live-rebinds the running server.

**Why:** Moving controls must not change the endpoint users configured in OBS or weaken visible failure behavior.

**Tradeoff:** A saved port change requires restart, and invalid settings can prevent server startup until repaired.

### 4. Keep settings and document saves independent

**Decision:** Settings Save writes only format-1 config-local `settings.json`; document Save writes only the overlay collection. Neither implies the other. Closing Settings is not document close, while closing the editor uses document Save/Discard/Cancel and closes Settings only when shutdown proceeds.

**Why:** Configuration and user-created content have different ownership, locations, and recovery needs.

**Tradeoff:** Users may need two explicit saves when changing both configuration and overlay content.

### 5. Keep errors and URL actions honest outside the window

**Decision:** Server/settings failures remain discoverable in the main workspace after Settings closes. Copy/Open use the exact selected URL only after server readiness and overlay registration, unchanged from FDR-002.

**Why:** Closing a secondary surface must not hide a blocking server condition or enable an unready URL.

**Tradeoff:** Some settings status is intentionally duplicated in compact form in the workspace.

## Settings Close and Lifecycle Scenarios

| Scenario ID | Action | Required outcome |
|---|---|---|
| `settings_open` | Choose File > Settings with no Settings window. | Open one secondary native window showing running/saved state, path, errors, and restart requirement. |
| `settings_reopen` | Choose Settings while it is already open. | Focus/reuse the existing window; preserve its current form edits and create no duplicate. |
| `settings_close_unsaved` | Close Settings with uncommitted edits. | Discard form edits without another prompt; editor and server continue; errors remain available in the workspace. |
| `settings_save_next_launch` | Save a valid new port. | Persist format-1 settings for next launch; running port and exact current URLs remain unchanged. |
| `settings_invalid` | Enter an invalid port or load malformed/unsupported settings. | Show visible validation/source errors; do not overwrite the source, start a fallback, choose an alternate, or live-rebind. |
| `editor_close_cancel` | Close editor while Settings is open, then choose Cancel or encounter failed document Save. | Leave editor, Settings, server, and work running. |
| `editor_close_complete` | Close editor and successfully Save or choose Discard. | Close Settings, stop and join server, and exit; Discard writes no document changes. |

## Open Questions

None for the Settings-window behavior covered here. Native focus and close behavior remains required implementation verification under issues #25 and #27.

## Related

- **ADRs:** [ADR-005: Separate Versioned Server Settings from Overlay Documents](../adr/ADR-005-separate-server-settings-from-overlay-documents.md), [ADR-009: Use a Secondary Native Settings Viewport](../adr/ADR-009-secondary-native-settings-viewport-lifecycle.md)
- **FDRs:** [FDR-002: Browser-Source URL Actions and Port Settings](FDR-002-browser-source-url-actions-and-port-settings.md), [FDR-003: Multi-Widget Composition Workspace](FDR-003-multi-widget-composition-workspace.md)
