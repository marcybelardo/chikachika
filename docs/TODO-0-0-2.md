# `0.0.2` Milestone

**Status:** Planned — implementation pending

`0.0.2` develops the completed vertical slice into a usable overlay composition workspace. The central canvas, left widget list, and right properties inspector are the agreed layout direction. Milestone scope was confirmed on 2026-09-08, and the dependent design contracts were accepted on 2026-09-17. Implementation remains pending.

This document tracks milestone scope and completion. It does not replace Feature Decision Records (FDRs) for user-visible behavior or Architecture Decision Records (ADRs) for architectural rationale. [FDR-003](fdr/FDR-003-multi-widget-composition-workspace.md), [FDR-004](fdr/FDR-004-bundled-offline-text-fonts.md), and [FDR-005](fdr/FDR-005-singleton-native-settings-window.md) govern the accepted 0.0.2 behavior. [ADR-006](adr/ADR-006-ordered-authoritative-widget-model.md), [ADR-007](adr/ADR-007-version-2-overlay-document-persistence.md), [ADR-008](adr/ADR-008-coordinator-owned-workspace-history.md), and [ADR-009](adr/ADR-009-secondary-native-settings-viewport-lifecycle.md) govern its architecture. These records resolve design; their Accepted status does not claim runtime implementation.

## Outcome

A streamer can compose an overlay from multiple text widgets, easily identify and select each widget, arrange their stacking order and position on a central canvas, and edit the selected widget through a dedicated inspector. The overlay saves locally and remains available at its stable, live browser-source URL. Application settings open separately from the editing workspace.

## Product Requirements

### Workspace layout

- [ ] A menu bar provides File, Edit, View, and Help actions appropriate to the supported features; menus contain working actions rather than placeholders.
- [ ] Overlay switching remains compact, with create, rename, and confirmed deletion accessible without competing with the widget list.
- [ ] The left sidebar lists widgets in the selected overlay and provides an Add widget action.
- [ ] The center gives the preview canvas the primary editing area, preserves its aspect ratio, and fits the complete canvas to the available space.
- [ ] The right sidebar displays the selected widget's editable properties; with no widget selected, it displays overlay information and canvas dimensions.
- [ ] Sidebars resize while the canvas and property controls remain usable at a documented minimum window size.
- [ ] Save state, server state, and recoverable errors remain visible in a compact status area.
- [ ] Copy URL and Open output remain readily accessible and available only when the selected output is ready.

### Widget identity and selection

- [ ] Each widget row has a type indicator and editable name, with a useful default derived from its content where appropriate.
- [ ] Clicking a widget row selects its canvas object; clicking a canvas object selects and reveals its row. Both update the same inspector selection.
- [ ] Hover and selection have visually distinct canvas boundaries, and the selected row is clearly distinguishable without relying solely on color.
- [ ] Canvas selection chooses the frontmost widget at the pointer; obscured widgets remain selectable through the list.
- [ ] The top list row represents the frontmost widget, matching native preview and browser output stacking order.
- [ ] Clicking empty canvas space clears widget selection. Switching overlays or deleting a selected widget cannot leave an inspector targeting a stale widget.
- [ ] Selection outlines, hover guides, and other editing controls never appear in browser/OBS output.
- [ ] The initial workflow supports one selected widget at a time.

### Multiple text widgets and editing

- [ ] An overlay supports zero or more independently identified text widgets.
- [ ] A user can add, name, duplicate, and delete a text widget. Duplication creates a new stable identity.
- [ ] A user can change widget stacking order through explicit forward/backward actions, reflected immediately in the list and both renderers.
- [ ] A user can move the selected widget by dragging and by editing its coordinates; dragging preserves the grab offset and respects the defined canvas bounds.
- [ ] The inspector supports multiline content, font size, RGBA color, alignment, and position for the selected text widget.
- [ ] Core actions have documented keyboard shortcuts that respect text-field focus; editing text must not accidentally delete or manipulate its widget.
- [ ] Session-local undo/redo supports widget creation, duplication, deletion, naming, content, styling, movement, and layer ordering; a completed drag is one undo action.
- [ ] Undo/redo restores valid widget selection and publishes the restored document state to connected browser sources. A new edit after undo invalidates redo history.
- [ ] Closing the editor or quitting with unsaved document changes offers Save, Discard, and Cancel; a failed save keeps the editor open with the work intact.

### Bundled fonts

- [ ] The text inspector offers a small bundled collection of font families, available offline without relying on installed system fonts.
- [ ] The native preview and browser output use the same bundled font assets; font selection persists and updates live.
- [ ] Choose the supported families and character coverage, verify redistribution terms, and include required license notices.
- [ ] Verify the selected fonts in the native preview and OBS, documenting any remaining metric or line-break differences.

### Settings window

- [ ] Settings open on demand from the menu in a separate native window and close without closing the editor or stopping the server.
- [ ] Reopening Settings focuses the existing settings surface rather than creating duplicates.
- [ ] Network controls show the running port, saved next-launch port, validation errors, settings location, and restart requirement.
- [ ] Settings persist separately from overlay documents and preserve the existing explicit-port, loopback-only, restart-bound behavior.
- [ ] Server/settings failures remain discoverable from the main workspace even when Settings is closed.
- [ ] Verify the chosen settings-window behavior on macOS and Linux.

### Visual clarity

- [ ] Establish a coherent treatment of spacing, typography, panel headings, control grouping, and selected/hover/disabled/error states.
- [ ] Visually separate the application panels, canvas surroundings, and transparent output area so widget boundaries are understandable.
- [ ] Verify readability and selection visibility with light, dark, small, and overlapping widget content.
- [ ] Review the layout with representative overlays at the minimum window size and a larger desktop size.
- [ ] Deliver one improved, readable appearance; selectable light/dark/system themes are outside this milestone.

### Persistence and live browser output

- [ ] Establish an ordered widget model with stable identities and one authoritative document representation shared by the editor, persistence, and browser projections.
- [ ] Save and restore widget content, names, properties, identities, and stacking order.
- [ ] Explicitly version the changed saved format. No conversion of 0.0.1 documents is required; reject unsupported versions without modifying them and document how to move aside temporary old data to start a fresh workspace.
- [ ] Failed saves preserve the previous source and current dirty work; malformed or unsupported files remain non-destructive errors.
- [ ] Adding, editing, reordering, duplicating, and removing widgets updates connected browser sources without manual refresh.
- [ ] Overlay URLs remain stable across edits, renames, saves, and restarts.
- [ ] Browser output remains transparent and authoritative for final rendering; document any native-preview differences.

### OBS verification

- [ ] Exercise creation, composition, saving, restart, and OBS browser-source use on macOS and Linux.
- [ ] Verify overlapping text, stacking order, transparency, deletion, and live updates on both targets.
- [ ] Verify stable URLs and visible server failures after moving settings out of the workspace.

## Confirmed Scope

The following choices were confirmed on 2026-09-08 and completed as accepted design contracts on 2026-09-17. Runtime implementation and verification remain pending.

| Decision | Scope |
|---|---|
| Widget types and imported assets | Multiple text widgets; images are deferred to 0.0.3. Arbitrary font imports remain deferred. |
| Font selection | Include a small bundled font collection shared by native preview and browser output. |
| Undo/redo and unsaved close | Include session-local widget undo/redo and a warning before closing with unsaved changes. |
| Settings window | Use a separate native window. |
| Existing saved data | No conversion of temporary 0.0.1 saves; unsupported versions remain non-destructive errors. |
| Appearance | Deliver one improved appearance. |

Zoom/pan controls, widget hiding/locking, and more advanced selection remain deferred. Fit-to-window preview is the baseline.

### Resolved design contracts

- [FDR-003](fdr/FDR-003-multi-widget-composition-workspace.md) and [ADR-008](adr/ADR-008-coordinator-owned-workspace-history.md) define one coordinator-owned workspace timeline of 100 completed actions, text/gesture grouping, overlay lifecycle, selection restoration, content-based dirty state, fresh delivery revisions, and close recovery.
- [FDR-004](fdr/FDR-004-bundled-offline-text-fonts.md) selects pinned Noto Sans Regular and JetBrains Mono Regular assets with bounded Latin-focused coverage, SIL OFL 1.1 notice obligations, U+FFFD replacement, and explicit issue #26 binary/runtime verification.
- [FDR-003](fdr/FDR-003-multi-widget-composition-workspace.md) defines a dark neutral workspace at 1280×800 initial and 1024×640 minimum logical size, including panel, spacing, typography, checkerboard, selection, hover, disabled, and error targets.
- [ADR-006](adr/ADR-006-ordered-authoritative-widget-model.md) and [ADR-007](adr/ADR-007-version-2-overlay-document-persistence.md) define ordered authoritative content and format-2 persistence. [FDR-005](fdr/FDR-005-singleton-native-settings-window.md) and [ADR-009](adr/ADR-009-secondary-native-settings-viewport-lifecycle.md) define the singleton Settings lifecycle.

These design items are complete; the unchecked requirements below remain the source of truth for implementation completion.

## Quality Requirements

- [ ] Tests cover widget identity, ordered mutations, selection validity, persistence round trips, and non-destructive rejection of unsupported saved formats.
- [ ] Tests cover browser projection and live changes for multiple widgets, including reorder and removal.
- [ ] Undo/redo tests cover grouped edits, deletion restoration, redo invalidation, and dirty state around saving; close-flow checks cover Save, Discard, Cancel, and save failure.
- [ ] Exercise real pointer and keyboard selection workflows, including obscured widgets and text-field focus.
- [ ] Measure representative idle resource use and responsiveness with a documented multi-widget workload; investigate material regressions against the [0.0.1 measurement](measurements/0.0.1-idle-resource-usage.md).
- [ ] Update setup, editing, settings, and troubleshooting guides for the new workspace.
- [ ] Record changed feature and architecture contracts using the owning documentation skills; update indexes, glossary, and current architecture as implementation makes them stale.
- [ ] Verify documentation links and ensure milestone requirements match the accepted decisions and implemented behavior.

## Explicitly Out of Scope

- Image widgets and image imports (deferred to 0.0.3), arbitrary font imports, and a general asset library
- Conversion of 0.0.1 saves and selectable light/dark/system themes
- Multi-selection, grouping, widget locking/hiding, snapping, and alignment tools
- Free canvas resizing, manual zoom/pan, rotation, and rich text
- Animation, trigger systems, and streaming-service integrations
- Plugin systems, marketplaces, cloud accounts, collaboration, and synchronization
- General OBS scene control or LAN/internet exposure
- Platforms beyond macOS and Linux as release gates

## Contract-to-Issue Verification Matrix

| Accepted contract | Dependent issue | Required implementation evidence |
|---|---:|---|
| Ordered model, format-2 persistence, and live complete-snapshot output | #22 | Ordered identity mutations, non-destructive format rejection, round trips, revision initialization, route stability, and connected-output updates |
| Workspace layout, stable-ID selection, ordering, drag bounds, and appearance | #23 | Pointer/list selection including overlap and deletion fallback, panel sizing at 1024×640 and 1280×800, and visual-state review |
| Workspace history, keyboard focus, dirty baseline, save failure, and editor close | #24 | Named history/save scenarios, 100-action capacity, grouping, redo invalidation, text-focus shortcut suppression, and Save/Discard/Cancel |
| Singleton native Settings window | #25 | Reuse/focus, discard-on-close, independent save boundaries, next-launch port behavior, and macOS/Linux native lifecycle checks |
| Exact bundled fonts, coverage, replacement, and licenses | #26 | Pinned-byte lengths and SHA-256, cmap/U+FFFD checks, complete SIL OFL 1.1 notices, decoded data-URL equality, and native/browser rendering |
| Combined OBS/platform and representative-resource verification | #27 | macOS/Linux OBS composition, stable URLs, transparent stacking/live updates, Settings failures, font behavior, and history memory measurement |

The Python documentation tests validate that these accepted contracts and assignments are present; they do not exercise future runtime features.

## Implementation Sequence

1. Record the changed feature/model contracts and settle the dependent design details. **Complete on 2026-09-17; implementation remains pending.**
2. Establish ordered widget state, mutation behavior, and saved-format handling (#22).
3. Build the workspace panels and shared selection behavior (#23).
4. Complete widget editing, layer actions, and agreed recovery/keyboard behavior (#24).
5. Move Settings into a native window (#25).
6. Integrate bundled fonts (#26), then complete combined platform/OBS verification and documentation (#27).

## Completion Gate

`0.0.2` is complete when every in-scope checkbox is satisfied, the design details above are resolved and recorded, the macOS and Linux OBS workflows pass, relevant tests pass, and documentation matches the implementation. Deferred requirements must be explicitly removed through the appropriate product decision rather than left implicitly incomplete. Release publication is tracked separately from implementation completion; publication of 0.0.1 is not a prerequisite for this planning work.
