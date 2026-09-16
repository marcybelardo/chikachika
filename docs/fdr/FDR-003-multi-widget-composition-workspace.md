# FDR-003: Multi-Widget Composition Workspace

**Status:** Accepted
**Date:** 2026-09-17
**Supersedes:** FDR-001

## Overview

The 0.0.2 workspace lets streamers compose a fixed-canvas overlay from multiple text widgets while preserving the local-first, stable-URL, transparent browser-source workflow. It expands editing, selection, stacking, history, and close recovery without adding accounts, network exposure, rich media, or general OBS control.

## User-visible Behavior

- Users create, rename, switch, and explicitly confirm deletion of overlays; stable overlay identity preserves the browser-source URL across rename, edits, saves, and restarts.
- Each overlay contains zero or more independently identified text widgets with editable names, multiline content, position, size, RGBA color, alignment, and bundled font family.
- A left widget list, aspect-preserving fit-to-window center canvas, and right inspector share one widget selection. With no widget selected, the inspector shows overlay information and canvas dimensions.
- Compact overlay lifecycle controls, status/errors, and readiness-gated Copy URL/Open output remain available. The transparent browser output remains final rendering authority and receives supported edits without manual refresh.
- Failures remain visible and non-destructive. Users can recover, retry, or continue with an unaffected overlay where possible. macOS and Linux remain the milestone platform gates.

## Feature Decisions

### 1. Preserve the focused local browser-source workflow

**Decision:** The workspace remains local-first with local saving, fixed explicitly configured canvases, stable loopback browser-source URLs, transparent authoritative browser output, live complete-snapshot updates, readiness-gated exact Copy URL/Open output, explicit overlay deletion confirmation, and visible non-destructive errors. Accounts, cloud storage, collaboration, synchronization, streaming integrations, general OBS control, and LAN/internet exposure remain excluded. macOS and Linux remain required targets.

**Why:** Multiple widgets should deepen the completed workflow without weakening its safety, URL stability, output authority, or bounded product scope.

**Tradeoff:** The editor preview can differ from final browser rendering, and broader sharing, platforms, and integrations remain unsupported.

### 2. Order and name multiple text widgets

**Decision:** An overlay contains zero or more text widgets. Index 0 and the top list row are frontmost. Add and duplicate insert at the front; duplication retains all properties and position, receives a new stable ID, selects the new widget, and uses a copy-derived editable name. Default names use the first nonempty content line or `Text`; names need not be unique. Forward/backward swaps one adjacent layer without wrapping. Rendering draws reverse collection order, and hit-testing checks frontmost first.

**Why:** One explicit ordering rule keeps list, hit-testing, native preview, persistence, and browser output understandable.

**Tradeoff:** Obscured widgets may require list selection, duplicate names can occur, and there are no arbitrary layer jumps.

### 3. Keep one stable-ID selection with deterministic fallback

**Decision:** One shared widget selection is stored by stable ID, never list index. Switching overlays clears it. Add/duplicate selects the new widget. Deleting the selected widget selects the row now at the deleted index, otherwise the preceding last row, otherwise none. Empty canvas clears selection. Selection alone is neither dirty nor an undo action, and selection/history/hover decorations never persist or reach browser output.

**Why:** Stable-ID selection prevents stale inspectors after reorder and gives every lifecycle operation a predictable result.

**Tradeoff:** Switching overlays intentionally loses widget selection, and only one widget can be selected at a time.

### 4. Bound direct manipulation to the fitted canvas

**Decision:** The canvas stays aspect-preserving and fit-to-window. Dragging preserves the initial grab offset. Widget anchor coordinates must remain finite and clamp to the canvas rectangle, but glyph bounds may extend outside it. View > Fit Canvas recomputes the fit rather than introducing a zoom mode. Resizing, zoom/pan, multiselect, snapping, hiding, locking, images, rotation, rich text, and animation are not included.

**Why:** Anchor clamping and fit-to-window provide predictable positioning without introducing a second navigation system.

**Tradeoff:** Large glyphs can extend past the canvas, and advanced layout and media workflows are deferred.

### 5. Use one workspace-wide bounded history

**Decision:** Undo/redo uses one session-only timeline across all overlays with at most 100 completed actions total across undo and redo. It includes widget mutations plus overlay create, rename, and confirmed delete; switching and selection alone are not actions. No-ops add nothing, a new change after Undo invalidates redo, and Save retains history. Restoration uses recorded stable IDs and indices atomically and publishes fresh revisions.

**Why:** Users need lifecycle and edits to undo in the order they occurred, including across overlays.

**Tradeoff:** Whole-workspace snapshots have memory cost; 100 bounds actions, not bytes, so issue #27 must measure a representative workload.

### 6. Group editing gestures and respect text focus

**Decision:** Same-field text edits group until focus loss, selection change, Save, or another command; Enter in multiline content inserts a newline. A drag or continuous numeric/color interaction commits once per completed gesture. Escape cancels a drag and restores its before-state without history. Text focus retains text-native undo and suppresses widget Delete/Duplicate shortcuts. Save commits pending text first; menu document Undo/Redo commits pending edits before applying history. Undo/redo or close resolves pending edits first.

**Why:** Grouping follows user intent and prevents typing or dragging from flooding history or accidentally destroying widgets.

**Tradeoff:** Text-native and document history have distinct focus-sensitive behavior that requires careful UI communication and testing.

### 7. Define content-based dirty state and recoverable close

**Decision:** Dirty state compares persistent document content with the last successful whole-collection save baseline, excluding revisions and selection. Undo/redo back to that content is clean even if an action was evicted. Initial successful load is clean; Save is not an action. Failed saves preserve source, work, history, dirty state, and visible errors. Editor close/quit with dirty content offers Save, Discard, Cancel: Save closes only on success, Discard writes nothing, Cancel continues editing, and failed Save stays open. Main editor close also owns Settings and server shutdown.

**Why:** Content equality reflects whether durable work differs from disk and makes close recovery honest.

**Tradeoff:** Implementations must retain a saved-content baseline independent of history position and delivery revisions.

### 8. Use a concrete dark workspace appearance

**Decision:** The workspace uses one dark neutral theme, initial size 1280×800 logical pixels, and minimum size 1024×640. Targets are an 8-point base spacing unit, 12-point panel padding, 14-point body text, clear headings, neutral dark panels, a lighter checkerboard for transparent output, cyan selection outline plus row emphasis, a distinct thinner hover outline, and readable muted disabled controls with icon/text error indications. Initial left/right widths are 220/280 logical pixels with minima 180/260; resizing reserves a usable center, and the inspector scrolls vertically.

**Why:** Concrete values make readability and minimum-size verification actionable rather than subjective.

**Tradeoff:** These are planned verification targets, not measured results, and user-selectable themes are outside 0.0.2.

### 9. Keep delivery revisions separate from durable content

**Decision:** Loaded and new overlays start running-session revision/high-water at 0. Initial publication may use 0; subsequent accepted mutations and restorations strictly increase it. Deleted IDs retain session high-water marks, restoration reuses their same IDs/URLs, deleted streams close, and restored routes become available. Restarts do not preserve revision counters; a reconnected client receives the current complete snapshot without replay.

**Why:** Fresh monotonic session revisions prevent restored content from appearing stale while keeping delivery metadata out of saved documents.

**Tradeoff:** A page connected to a deleted stream is not promised automatic recovery beyond the existing retry contract until dependent work validates it.

### 10. Provide only meaningful menus and shortcuts

**Decision:** Menus expose working actions from the matrix below and no placeholders. The primary modifier is Cmd on macOS and Ctrl on Linux. Layer and Add Text may remain menu-only; no arbitrary shortcuts are added merely to fill menus.

**Why:** A small predictable command surface supports keyboard work without conflicting with text entry.

**Tradeoff:** Some actions require menus, and focus-sensitive suppression must be tested on both platforms.

## Menu and Shortcut Matrix

| Menu | Action | Shortcut / focus rule |
|---|---|---|
| File | Create Overlay | Menu-only |
| File | Save | Primary+S; commits pending text transaction first |
| File | Settings | Menu-only; opens/focuses singleton native window |
| File | Quit | Platform-standard invocation; uses Save/Discard/Cancel when dirty |
| Edit | Undo | Primary+Z; text-native while text field has focus, otherwise document history |
| Edit | Redo | Primary+Shift+Z; text-native while supported by focused field, otherwise document history |
| Edit | Add Text | Menu-only |
| Edit | Duplicate | Primary+D; suppressed during text entry |
| Edit | Delete | Delete/Backspace outside text entry only; requires selected widget |
| Edit | Forward | Menu-only; one adjacent layer, no wrap |
| Edit | Backward | Menu-only; one adjacent layer, no wrap |
| View | Fit Canvas | Menu-only; recomputes fit, not a new zoom mode |
| Help | User Documentation | Opens the existing user documentation |

## Selection and Ordering Scenarios

| Scenario ID | Action | Required outcome |
|---|---|---|
| `overlap_selection` | Click a point covered by multiple widgets. | Select the frontmost hit by stable ID; an obscured widget remains selectable from the list. |
| `selected_widget_deletion` | Delete the selected widget. | Select the row now at its index, otherwise the preceding last row, otherwise none. |
| `overlay_switch_selection` | Switch overlays. | Clear widget selection; show overlay information/dimensions in the inspector. |
| `duplicate_front` | Duplicate a selected widget. | Retain properties/position, assign a new ID and copy-derived editable name, insert at index 0, and select it. |
| `layer_step` | Move a widget forward or backward. | Swap one adjacent item with no wrapping; list and renderers update consistently. |
| `drag_cancel` | Drag from an offset, then press Escape. | Preserve grab offset during movement, restore pre-drag state, and add no history action. |

## History, Save, and Lifecycle Scenarios

| Scenario ID | Action | Required outcome |
|---|---|---|
| `cross_overlay_undo` | Undo an action performed on another overlay. | Atomically restore the collection and valid recorded overlay/widget IDs exactly, with fresh revisions. |
| `absent_overlay_selection` | Recorded overlay ID is absent. | Select same recorded index, otherwise last, otherwise none; clear widget selection. Recorded null remains null. |
| `absent_widget_selection` | Recorded widget ID is absent in an existing recorded overlay. | Select same recorded index, otherwise last, otherwise none. Recorded null remains null. |
| `undo_overlay_delete` | Undo confirmed overlay deletion. | Restore original ID/URL and before-selection, using fallback only for absent recorded targets. |
| `redo_overlay_delete` | Redo overlay deletion. | Delete again, close streams, and restore after-selection using same-index/last/none; clear widget selection on overlay fallback. |
| `save_edit_undo_clean` | Save, edit, then Undo to saved content. | Dirty becomes false by persistent-content equality regardless of revisions or history eviction. |
| `failed_save` | Save fails. | Preserve previous source, work, history, dirty state, visible error, and open editor. |
| `pending_text_then_undo` | Invoke menu Undo with pending field edits. | Commit pending edits, then apply document history; focused text-native Undo remains local to text. |
| `pending_text_then_save` | Save with pending field edits. | Commit the grouped transaction before whole-collection save; retain history. |
| `redo_invalidation` | Make a new document change after Undo. | Clear redo history. |
| `ordinary_overlay_delete` | Confirm deletion of the selected overlay. | Select overlay at same index, otherwise last, otherwise none; clear widget selection. |
| `editor_close_dirty` | Close/quit with dirty content. | Save closes only on success; Discard does not write; Cancel or failed Save leaves editor running. |
| `restart_revision_initialization` | Save, restart/load, publish, edit, then Undo. | Revision starts at 0; initial publish may be 0; edit is 1; Undo is fresh revision 2. |

## Predecessor Clause Audit

### Retained

- Local-first authoring, local saving, loopback browser output, and OBS use remain the product boundary.
- Overlay create/rename/confirmed-delete behavior and durable identity/URL across rename and restart remain.
- Fixed explicit canvases and authoritative transparent browser output remain.
- Text content, position, font size, color, and alignment remain editable.
- Exact browser output can still be copied/opened when ready, and supported edits update connected output without manual refresh.
- Visible, non-destructive persistence and server failures remain; users can recover, retry, or continue with an unaffected overlay where possible.
- macOS and Linux remain required targets; accounts, cloud, collaboration, synchronization, integrations, general OBS control, and LAN/internet exposure remain excluded.

### Changed

- One optional text widget expands to zero or more ordered text widgets with names, duplication, deletion, and layering.
- The editor becomes a three-region composition workspace with shared selection, history, shortcuts, close recovery, fonts, and concrete visual targets.

### Retired

- The zero-or-one widget limit and deferral of multiple widgets/layering are deliberately retired.
- No local-first, stable-identity, fixed-canvas, browser-authority, live-update, failure-handling, or platform guarantee is retired.

## Open Questions

None for the 0.0.2 workspace behavior covered here. Runtime verification remains assigned to issues #22–#27.

## Related

- **ADRs:** [ADR-006: Use an Ordered Authoritative Widget Model](../adr/ADR-006-ordered-authoritative-widget-model.md), [ADR-007: Persist Version-2 Overlay Documents](../adr/ADR-007-version-2-overlay-document-persistence.md), [ADR-008: Keep Workspace History in the Coordinator](../adr/ADR-008-coordinator-owned-workspace-history.md)
- **FDRs:** [FDR-002: Browser-Source URL Actions and Port Settings](FDR-002-browser-source-url-actions-and-port-settings.md), [FDR-004: Bundled Offline Text Fonts](FDR-004-bundled-offline-text-fonts.md), [FDR-005: Singleton Native Settings Window](FDR-005-singleton-native-settings-window.md)
