# FDR-006: Guarded Editor Quit

**Status:** Accepted
**Date:** 2026-10-06
**Supersedes:** FDR-003

## Overview

This decision changes only how users request the existing workspace close flow. The application provides guarded in-window quit actions and removes the default macOS application menu. All other workspace behavior from FDR-003 remains in force as summarized in the retained-decision matrix below. Runtime and native verification is tracked separately; acceptance of this decision is not evidence that a build or native scenario passed.

## User-visible Behavior

- **File > Quit** in the editor and **Cmd+Q** on macOS request the same guarded close as closing the editor window. Pending edits are resolved before dirty state is evaluated.
- For dirty document content, Save closes only after a successful save; Discard closes without saving document changes; Cancel keeps the editor open and editable. A failed Save keeps the editor open with the work intact.
- Native Linux window-manager close uses the same guarded close flow.
- The default macOS native application menu is disabled; its default About, Hide, and Quit items are absent. The in-window File menu remains the supported place for File > Quit.
- OS shutdown, macOS Force Quit, process termination, crashes, and power loss are outside the guarded-close guarantee.

## Feature Decisions

### 1. Route supported quit requests through guarded close

**Decision:** File > Quit and macOS Cmd+Q use the editor's close-request path and the same Save/Discard/Cancel behavior as a dirty window close. Native Linux window-manager close also uses that guarded path. Resolve a pending document interaction before checking whether content is dirty. Do not promise interception of OS shutdown, Force Quit, process termination, crashes, or power loss.

**Why:** Every supported in-application quit action should preserve the same recoverable document boundary instead of allowing a menu or keyboard action to bypass it.

**Tradeoff:** Supported quit actions may require an explicit choice, while forceful or system-level termination cannot be made recoverable by this feature.

### 2. Remove the default macOS application menu

**Decision:** Disable the default macOS native application menu, including its default About, Hide, and Quit items. Do not add a placeholder Settings menu action while the separate Settings window remains deferred to issue #25. Keep the editor's in-window File menu and its supported File > Quit action.

**Why:** A native default Quit item could bypass the editor's guarded close flow; removing it leaves one clear, guarded application quit command.

**Tradeoff:** The standard macOS About/Hide/Quit menu items are not provided; the application does not claim to intercept system shutdown or Force Quit.

### 3. Retain the local-first overlay workflow

**Decision:** Preserve FDR-003's local authoring and saving, fixed canvases, stable overlay identity and loopback browser-source URLs, transparent authoritative browser output, live complete-snapshot delivery, readiness-gated URL actions, explicit overlay deletion confirmation, visible non-destructive failures, and macOS/Linux target scope. Accounts, cloud, collaboration, synchronization, streaming integrations, general OBS control, and LAN/internet exposure remain excluded.

**Why:** Guarded quit changes only the close boundary and must not broaden or weaken the product's existing workflow.

**Tradeoff:** The native preview may differ from browser rendering, and broader sharing, platforms, and integrations remain unsupported.

### 4. Retain ordered text-widget composition and selection

**Decision:** Preserve zero or more stable-ID text widgets per overlay. Index zero and the top list row are frontmost. Add/duplicate insert at the front; duplicate retains properties and position, gets a new ID and copy-derived editable name, and becomes selected. A default name uses the first nonempty content line or `Text`; names need not be unique. Forward/backward swaps one adjacent layer without wrapping. Selection is one shared stable-ID selection; overlay switch clears it, selected-widget deletion falls back to the same index, then last, then none, and empty-canvas selection clears it. Selection, history, and editor decorations do not persist or appear in browser output.

**Why:** A single ordering and selection contract keeps the list, inspector, canvas, persistence, and browser output aligned.

**Tradeoff:** Only one widget is selected at once; an obscured widget may need to be selected from the list; arbitrary layer jumps are not provided.

### 5. Retain fitted-canvas manipulation and workspace boundaries

**Decision:** Preserve the aspect-preserving fit-to-window canvas, initial 1280×800 logical size, 1024×640 minimum, fixed explicit overlay dimensions, drag grab offset, finite/clamped anchor coordinates, and View > Fit Canvas as a fit recomputation rather than zoom. Keep the dark neutral three-region workspace, 8-point spacing, 12-point panel padding, 14-point body text, 220/280 initial side widths with 180/260 minima, usable center reservation, vertical inspector scrolling, and distinct selection/hover/disabled/error treatments. These are the predecessor's targets; existing issue23 review is historical and does not verify issue24. Resizing, zoom/pan, multiselect, snapping, hiding, locking, images, rotation, rich text, and animation remain excluded.

**Why:** Guarded quit does not change composition geometry or introduce another navigation model.

**Tradeoff:** Glyphs may extend beyond the canvas, and advanced layout/media workflows remain deferred.

### 6. Retain one bounded, session-only workspace history

**Decision:** Preserve one timeline across all overlays containing at most 100 completed actions total across undo and redo. Widget changes and overlay create, rename, and confirmed delete are actions; selection, switching, Settings changes, no-ops, rejected edits, and Save are not. Settings alone are not actions. A new completed edit after Undo clears redo; Save retains history. Record before/after whole-collection content and selection IDs/indices. Restoration preserves valid IDs and recorded null selections; missing targets use recorded index, then last, then none, clearing widget selection when the overlay falls back. Restoration publishes fresh runtime revisions and changes timeline position only after success. Dirty state is equality with the last successfully saved document content, independent of selection, revisions, history position, or eviction. History is not persisted and whole-workspace snapshots are bounded by action count, not bytes.

**Why:** A workspace-wide timeline follows the order in which users change documents and keeps durable dirty state distinct from navigation through history.

**Tradeoff:** History consumes memory proportional to snapshot contents; issue #27 owns representative resource measurement, and no byte-size bound is promised.

### 7. Retain grouped edits and focus-safe shortcuts

**Decision:** Preserve the platform primary modifier (Cmd on macOS, Ctrl on Linux): primary+S saves after committing pending text; primary+Z and primary+Shift+Z use text-native undo/redo in a focused text editor and document history otherwise; primary+D duplicates only outside text entry; Delete/Backspace deletes only a selected widget outside text entry. Multiline Enter inserts a newline. Same-field text edits group until focus loss, selection change, Save, or another command. Numeric text groups until its editor ends; pointer numeric drags and color-slider gestures group through release; a focused color numeric field groups until focus loss. Discrete font/alignment changes are one action. Merely opening/changing/closing a popup without content changes adds no action. Escape cancels a drag and restores its before-state without history. Menu Undo/Redo commits pending edits before history. Settings/dialog text input must not trigger widget shortcuts or document history. Add Text and layer changes remain menu-only. No additional arbitrary shortcuts are introduced.

**Why:** Focus-aware command routing prevents typing from changing or deleting widgets and groups history by user intent.

**Tradeoff:** Text-native and document history have different focus-sensitive behavior that needs platform and real-event verification.

### 8. Retain content-based save and failure behavior

**Decision:** Preserve a clean baseline after successful load and the last successful whole-collection save. Save is not a history action. Failed saves preserve the previous source, current work, history, dirty state, and visible error. Undo/redo back to saved content is clean even when history entries were evicted. A successful Save/Discard close proceeds; Cancel or failed Save leaves the editor open with work intact. Discard does not write document changes. Once document close is accepted, the main editor owns Settings-window close and normal server shutdown; opening or canceling the prompt does not stop the server.

**Why:** Close choices and dirty status should reflect recoverable document content, not history position or runtime delivery state.

**Tradeoff:** The save baseline is independent state that must be maintained alongside history.

### 9. Retain runtime revision and browser-output contracts

**Decision:** Preserve complete current snapshots, stable overlay URLs, frontmost rendering, and runtime-only delivery revisions. Fresh loaded/new overlays start at revision/high-water zero; accepted changed publications and restorations increase revisions; no-op publications do not; deleted IDs retain session high-water so restored routes use a newer revision; deleted streams close; restarts reset delivery counters. Reconnection receives current state, not historical replay. Separate per-overlay streams do not promise synchronized frames. Browser output remains transparent and authoritative.

**Why:** Quit behavior must not alter the existing durable-document and runtime-delivery boundary.

**Tradeoff:** A page attached to a deleted stream is not promised automatic recovery beyond the existing reconnect behavior.

### 10. Retain only meaningful in-window menus

**Decision:** The in-window menu surface retains File > Create Overlay and Save, Edit > Undo/Redo, Add Text, Duplicate, Delete, Forward, Backward, View > Fit Canvas, and Help > User Documentation, with the shortcut/focus rules above. File > Quit is the guarded addition in this record. File > Settings is not shown as a placeholder before issue #25 delivers the singleton native Settings window. No menu action is a nonfunctional placeholder.

**Why:** A small menu surface should expose working actions, with every available quit route guarded.

**Tradeoff:** Settings remains in the existing workspace until its separately accepted issue #25 behavior is implemented and verified.

## Retained Decision and Scenario Matrix

This matrix is the self-contained retained portion of FDR-003, checked against its decisions, menu/shortcut table, and scenario tables. Only the quit surfaces and the default macOS application menu are changed by FDR-006.

| FDR-003 area | Retained contract | Scenario IDs retained |
|---|---|---|
| Local-first scope | Local save, loopback-only output, stable IDs/URLs, explicit deletion confirmation, transparent browser authority, live snapshots, visible non-destructive errors, macOS/Linux scope; no accounts/cloud/collaboration/sync/integrations/general OBS control/LAN exposure. | — |
| Widget collection | Zero or more stable-ID text widgets; index 0 frontmost; duplicate preserves properties/position with new ID; adjacent layer movement does not wrap; browser paint/hit order agrees. | `duplicate_front`, `layer_step` |
| Selection | One shared ID selection; switching clears; deletion uses same-index/last/none fallback; empty canvas clears; selection is not dirty/history and never persists. | `overlap_selection`, `selected_widget_deletion`, `overlay_switch_selection` |
| Canvas | Fixed size, aspect fit, grab-offset drag, finite/clamped anchor, fit recomputation; no zoom/pan or deferred advanced composition features. | `drag_cancel` |
| History | One 100-completed-action workspace timeline; actions include widget edits and overlay create/rename/confirmed-delete; no-ops, selection/switch, rejected changes, Settings and Save add none; redo invalidation, Save retention, selection fallback, fresh revisions, session-only snapshots, content-based dirty state. | `cross_overlay_undo`, `absent_overlay_selection`, `absent_widget_selection`, `undo_overlay_delete`, `redo_overlay_delete`, `save_edit_undo_clean`, `failed_save`, `pending_text_then_undo`, `pending_text_then_save`, `redo_invalidation`, `ordinary_overlay_delete`, `restart_revision_initialization` |
| Close lifecycle | Save/Discard/Cancel outcomes and pending-edit resolution retained; FDR-006 specifies guarded File Quit, Cmd+Q, and Linux native window close. OS shutdown/Force Quit and other process termination are excluded. | `editor_close_dirty` |
| Appearance and sizing | Dark neutral appearance; 1280×800 initial / 1024×640 minimum; 8-point spacing, 12-point panel padding, 14-point body text; initial 220/280 and minimum 180/260 side widths with usable center reservation; vertically scrolling inspector; selection/hover/disabled/error distinctions remain targets. Existing issue23 evidence is historical, not new issue24 QA. | — |
| Deferred Settings and fonts | File > Settings is not a placeholder; the accepted singleton native Settings behavior remains under #25. The font-family IDs `noto-sans` and `jetbrains-mono` do not mean bundled font files are present; asset, license, coverage, and renderer checks remain #26. Combined OBS/platform/resource certification remains #27. | — |
| Browser revision delivery | Complete snapshots, frontmost order, stable IDs, session revisions/high-water, no history replay, transparent authoritative browser output. | `restart_revision_initialization` |
| Shortcut/menu surface | Cmd on macOS, Ctrl on Linux; Save, text/document Undo/Redo, duplicate, delete focus behavior, multiline Enter and menu-only Add/layer actions as in Decision 7. No placeholder Settings action. File > Quit is guarded. | — |

### Retained shortcuts and menu actions

| In-window menu | Action | Shortcut / focus behavior |
|---|---|---|
| File | Create Overlay | Menu-only |
| File | Save | Primary+S; commits pending text first |
| File | Quit | Guarded close path; dirty content offers Save/Discard/Cancel |
| Edit | Undo | Primary+Z; text-native with text focus, document history otherwise |
| Edit | Redo | Primary+Shift+Z; text-native where supported with text focus, document history otherwise |
| Edit | Add Text | Menu-only |
| Edit | Duplicate | Primary+D; suppressed during text entry |
| Edit | Delete | Delete/Backspace outside text entry; requires selected widget |
| Edit | Forward / Backward | Menu-only; one adjacent layer, no wrap |
| View | Fit Canvas | Menu-only; recomputes fit |
| Help | User Documentation | Opens existing user documentation |

File > Settings is intentionally absent until issue #25. The default macOS native application menu, including About/Hide/Quit, is disabled under this record.

## Verification Status

The behavior is accepted; implementation and native evidence remain pending. The planned AC.2–AC.7 test-name mapping and the manual macOS/Linux checklist are recorded in [the issue24 runtime evidence checkpoint](../measurements/issue24-runtime-evidence.md) and [native close validation record](../measurements/issue24-native-close-validation.md). Neither this FDR nor those pending checklists claims a test or native scenario passed.

## Open Questions

None for the approved user-visible choice. OS shutdown and Force Quit remain explicitly outside the guarantee; native behavior still requires the pending platform checks.

## Related

- **ADRs:** [ADR-008: Keep Workspace History in the Coordinator](../adr/ADR-008-coordinator-owned-workspace-history.md); rationale and technical ownership remain there.
- **FDRs:** [FDR-003: Multi-Widget Composition Workspace](FDR-003-multi-widget-composition-workspace.md), [FDR-004: Bundled Offline Text Fonts](FDR-004-bundled-offline-text-fonts.md), [FDR-005: Singleton Native Settings Window](FDR-005-singleton-native-settings-window.md), [FDR-002: Browser-Source URL Actions and Port Settings](FDR-002-browser-source-url-actions-and-port-settings.md).
