# `0.0.2` Milestone

**Status:** In progress — issue22 checkpoint delivered; issue23 layout review complete, direct canvas-click follow-up pending; issues #24–#27 remain

`0.0.2` develops the completed vertical slice into a usable overlay composition workspace. The central canvas, left widget list, and right properties inspector are the agreed layout. Milestone scope was confirmed on 2026-09-08, and the dependent design contracts were accepted on 2026-09-17. Issue22’s ordered model, format-2 persistence, coordinator selection/baseline behavior, native multi-widget controls, and complete browser array reconciliation are delivered. Issue23’s three-panel layout and pointer-selection paths are covered by headless GUI tests. Corrected-build visual inspection at 1280×800 confirmed near-black and white overlapping text legible on the lighter checkerboard with all panels visible. A separate QA variant initialized at 1024×640 confirmed the panels, controls, fitted canvas, overlapping light/dark text, and status/error area. Its test port was occupied, so server readiness was not checked in that variant. Manual QA found direct canvas text clicks did not visibly switch the inspector to the clicked caption; a fix is under investigation. Issues #24–#27 remain.

This document tracks milestone scope and completion. It does not replace Feature Decision Records (FDRs) for user-visible behavior or Architecture Decision Records (ADRs) for architectural rationale. [FDR-003](fdr/FDR-003-multi-widget-composition-workspace.md), [FDR-004](fdr/FDR-004-bundled-offline-text-fonts.md), and [FDR-005](fdr/FDR-005-singleton-native-settings-window.md) govern the accepted 0.0.2 behavior. [ADR-006](adr/ADR-006-ordered-authoritative-widget-model.md), [ADR-007](adr/ADR-007-version-2-overlay-document-persistence.md), [ADR-008](adr/ADR-008-coordinator-owned-workspace-history.md), and [ADR-009](adr/ADR-009-secondary-native-settings-viewport-lifecycle.md) govern its architecture. These records resolve design; their Accepted status does not claim runtime implementation.

## Outcome

The delivered issue22 checkpoint provides a native ordered collection of zero or more text widgets per overlay. Users can add, name, edit, duplicate, delete, and move widgets one adjacent layer step at a time; stable IDs and durable properties survive format-2 save/load. The coordinator owns selection and the saved-content dirty baseline. The browser client consumes complete `widgets`-array snapshots, validates them atomically, reconciles DOM nodes by stable widget ID, and preserves frontmost paint order; the complete multi-widget browser-output outcome is delivered for this checkpoint.

## Issue22 documentation checkpoint

- **Delivered:** Ordered concrete text-widget collections with stable UUID v4 identities; model index 0 is frontmost. Add/duplicate insert at index 0, forward/backward operations swap adjacent items without wrapping, and native preview paints back-to-front.
- **Delivered:** Coordinator-owned selected widget ID, selection repair for accepted add/duplicate/delete/update operations, overlay-switch selection clearing, and content-based dirty state using the last successful whole-collection save baseline. Selection and runtime revisions are not persisted.
- **Delivered:** Format-2 `overlays.json` persistence containing ordered overlays and durable widget fields. Format 1 is not migrated or converted; malformed, unsupported, duplicate, unknown-font, or invalid data is rejected non-destructively and an incompatible existing store blocks startup without an empty editable replacement.
- **Delivered:** Separate format-1 `settings.json` persistence for the loopback port. It is resolved under the platform config-local directory, saved for the next launch, and does not live-rebind the running server. The GUI surfaces the exact settings path when available.
- **Delivered at the issue22 checkpoint:** Native widget selector, inspector, add/duplicate/delete/forward/backward controls, and collection preview. The issue23 workspace and overlap hit-testing are recorded in the following checkpoint. History, the separate Settings window, bundled-font delivery, and OBS/platform certification remain incomplete.
- **Delivered browser output:** Complete `widgets`-array snapshots are validated atomically before DOM mutation or revision advancement. The client reconciles an ID-to-node map, removes absent nodes, preserves existing node identity, and appends reverse model order so index 0 paints frontmost. `Number.MAX_SAFE_INTEGER` is accepted while unsafe larger revisions are rejected. Connected output receives complete current snapshots through the existing SSE boundary.
- **Recovery:** If an existing overlay store is format 1 or otherwise incompatible, copy it to a separately named backup and move the original source aside yourself before restarting. Chikachika does not migrate, overwrite, delete, or automatically move user data. Start a fresh workspace only after preserving that recovery copy. Apply the same user-managed backup discipline to malformed settings data.
- **Save safety:** Replacement uses a same-directory temporary file and ordinary filesystem replacement semantics. This preserves the previous file when write/sync/replace operations fail, but it is not a power-loss durability guarantee or protection from physical I/O failure.

### Named issue22 evidence

The focused Python contract is `issue22_documentation_checkpoint` in `tests/test_docs.py`; it checks the living documentation claims, exact surfaced paths, explicit recovery guidance, integrated browser guarantees, and remaining issue status. The implementation evidence named by this checkpoint includes `ordered_widget_mutations`, `widget_selection_lifecycle`, `format_two_round_trip_and_transient_omission`, `format_one_rejected_non_destructively`, `blocked_bootstrap_preserves_incompatible_store`, `hub_session_revision_lifecycle`, `browser_multi_widget_projection_and_html`, `multi_widget_editor_scenario`, and `multi_widget_preview_paint_order`. The integrated browser contract is additionally covered by Node `multi_widget_snapshot_reconciliation`, `invalid_snapshot_is_atomic`, and `stale_snapshot_preserves_dom`.

## Issue23 workspace checkpoint

- **Delivered:** Initial window size 1280×800 logical pixels and minimum 1024×640. Resizable left and right sidebars surround an aspect-preserving canvas that fits the available center space. The left list and right inspector stay within the configured width limits.
- **Delivered:** Compact overlay switcher and create/rename/confirmed-delete controls above the workspace. The left widget list has a text type indicator, frontmost-first rows, Add text, and a row Rename action. Rename selects that widget and focuses its existing inspector name field.
- **Headless-tested:** The coordinator maintains one selection across widget rows, canvas hits, and inspector. egui scenarios exercise frontmost overlap selection, obscured-row selection, row reveal, empty-canvas and overlay-switch clearing, and the deletion index fallback. Manual QA found that direct canvas text clicks did not visibly switch the inspector to the clicked caption; a fix is under investigation.
- **Delivered:** File > Create Overlay/Save; Edit > Add Text/Duplicate/Delete/Forward/Backward; View > Fit Canvas; Help > User Documentation. Fit Canvas recomputes the fit; zoom and pan remain out of scope.
- **Delivered:** Compact status area with save/server state and visible errors, readiness-gated Copy URL/Open output, and Local server settings collapsed by default. Port settings remain in this section until the separate native Settings window in #25 is implemented.
- **Headless evidence:** egui tests cover supported menu actions, readiness-gated output actions, row rename focus, frontmost overlap selection, obscured-row selection, empty-canvas clearing, row reveal, overlay switching, deletion fallback, editor guides excluded from browser output, and canvas geometry at 1024×640 and 1280×800.
- **Visual review:** Corrected-build review at 1280×800 confirmed near-black and white overlapping text legible on the lighter checkerboard with all three panels visible. A separate QA variant initialized at 1024×640 confirmed the panels, controls, fitted canvas, overlapping light/dark text, and status/error area. Its test port was occupied, so readiness was not verified there. Manual QA found that direct canvas text clicks did not visibly switch the inspector; that follow-up remains open.

### Named issue23 evidence

The Python contract is `issue23_documentation_checkpoint` in `tests/test_docs.py`. The GUI tests named by this checkpoint are `native_window_and_sidebar_sizes_match_the_workspace_contract`, `workspace_keeps_canvas_inside_the_minimum_window`, `workspace_keeps_canvas_inside_a_larger_desktop_window`, `issue_23_menus_run_their_supported_actions`, `output_actions_wait_for_server_readiness`, `widget_row_rename_focuses_the_existing_inspector_field`, `canvas_click_selects_frontmost_overlap_and_updates_inspector`, `obscured_widget_remains_selectable_from_its_list_row`, `empty_canvas_click_clears_widget_selection`, `canvas_selection_reveals_its_widget_row`, `switching_overlays_by_pointer_clears_widget_selection`, `deleting_selected_widget_by_pointer_selects_the_same_index_fallback`, and `hover_outline_is_distinct_and_editor_guides_do_not_change_browser_output`. The documentation test ties these names to `src/gui.rs`; none of these headless tests substitutes for the manual visual review above.

## Product Requirements

### Workspace layout

- [x] A menu bar provides File, Edit, View, and Help actions for currently supported features, with working menu actions.
- [x] Overlay switching remains compact, with create, rename, and confirmed deletion accessible outside the widget list.
- [x] The left sidebar lists widgets in the selected overlay and provides an Add text action.
- [x] The center canvas preserves its aspect ratio and fits the available space.
- [x] The right sidebar displays selected widget properties; with no widget selected, it displays overlay information and canvas dimensions.
- [x] Resizable sidebars, configured minimum widths, and canvas/property geometry are covered by headless checks at the documented minimum window size.
- [x] Save state, server state, and recoverable errors remain visible in the compact status area.
- [x] Copy URL and Open output are readily accessible and enabled only when the selected output is ready.

**Progress note:** Issue23 layout and headless interaction implementation is covered by named egui tests below. Geometry tests do not replace visual review. Corrected-build inspection confirmed contrast and layout at 1280×800; a separate QA variant initialized at 1024×640 confirmed panel and canvas layout, text contrast, and status/error visibility. Direct canvas-click inspector feedback remains under investigation.

### Widget identity and selection

- [x] Each widget row has a text type indicator, editable name, and Rename affordance that focuses the inspector name field; default names follow the accepted text-widget naming rule.
- [x] Headless egui tests verify that clicking a widget row selects its stable ID and updates the inspector selection.
- [ ] Confirm in manual UI review that clicking a canvas object visibly switches the inspector to that widget and reveals its row. Headless hit-testing passes, but manual QA found the inspector did not visibly switch.
- [x] Hover and selection use distinct canvas boundaries, and the selected row has a marker that does not rely on color alone.
- [x] Headless egui tests choose the frontmost widget hit and confirm obscured widgets remain selectable through the list.
- [x] The top list row represents the frontmost widget, matching native preview and browser output stacking order.
- [x] Switching overlays or deleting a selected widget cannot leave an inspector targeting a stale widget; selection is repaired or cleared by the coordinator.
- [x] Selection outlines, hover guides, and other editing controls never appear in browser/OBS output.
- [x] The initial workflow supports one selected widget at a time.

**Progress note:** Headless egui scenarios cover list/canvas/inspector state synchronization, overlap, row reveal, empty-canvas clearing, overlay switching, deletion fallback, selection markers, and editor-guide isolation from browser output. Manual QA has not confirmed that a direct canvas click visibly switches the inspector; the follow-up remains open.

### Multiple text widgets and editing

- [x] An overlay supports zero or more independently identified text widgets.
- [x] A user can add, name, duplicate, and delete a text widget. Duplication creates a new stable identity.
- [x] A user can change widget stacking order through explicit forward/backward actions, reflected immediately in the native list and preview.
- [x] A user can move the selected widget by dragging and by editing its coordinates; dragging preserves the grab offset and respects the defined canvas bounds.
- [x] The inspector supports multiline content, font size, RGBA color, alignment, position, and the two persisted font-family IDs for the selected text widget.
- [ ] Core actions have documented keyboard shortcuts that respect text-field focus; editing text must not accidentally delete or manipulate its widget.
- [ ] Session-local undo/redo supports widget creation, duplication, deletion, naming, content, styling, movement, and layer ordering; a completed drag is one undo action.
- [ ] Undo/redo restores valid widget selection and publishes the restored document state to connected browser sources. A new edit after undo invalidates redo history.
- [ ] Closing the editor or quitting with unsaved document changes offers Save, Discard, and Cancel; a failed save keeps the editor open with the work intact.

**Progress note:** Native controls and selection-targeted inspector edits are covered by the current headless GUI scenario tests. History, keyboard focus/shortcuts, and close recovery remain #24 work.

### Bundled fonts

- [ ] The text inspector offers a small bundled collection of font families, available offline without relying on installed system fonts.
- [ ] The native preview and browser output use the same bundled font assets; font selection persists and updates live.
- [ ] Choose the supported families and character coverage, verify redistribution terms, and include required license notices.
- [ ] Verify the selected fonts in the native preview and OBS, documenting any remaining metric or line-break differences.

**Progress note:** `noto-sans` and `jetbrains-mono` IDs persist and are selectable, but no font files are bundled or audited; #26 remains incomplete.

### Settings window

- [ ] Settings open on demand from the menu in a separate native window and close without closing the editor or stopping the server.
- [ ] Reopening Settings focuses the existing settings surface rather than creating duplicates.
- [ ] Network controls show the running port, saved next-launch port, validation errors, settings location, and restart requirement.
- [x] Settings persist separately from overlay documents and preserve the existing explicit-port, loopback-only, restart-bound behavior.
- [x] Server/settings failures remain discoverable from the main workspace even when Settings is closed.
- [ ] Verify the chosen settings-window behavior on macOS and Linux.

**Progress note:** Separate format-1 settings persistence, validation, surfaced path, and restart-bound port behavior are delivered in the collapsed Local server settings section in the compact status area. The separate singleton native Settings window remains #25 work.

### Visual clarity

- [ ] Establish a coherent treatment of spacing, typography, panel headings, control grouping, and selected/hover/disabled/error states.
- [ ] Visually separate the application panels, canvas surroundings, and transparent output area so widget boundaries are understandable.
- [ ] Verify readability and selection visibility with light, dark, small, and overlapping widget content.
- [x] Review the layout with representative overlays at the minimum window size and a larger desktop size. The 1280×800 build and separate 1024×640 QA variant both showed all three panels and the fitted canvas.
- [ ] Deliver one improved, readable appearance; selectable light/dark/system themes are outside this milestone.

**Progress note:** Corrected-build visual review confirmed text contrast and panel visibility at 1280×800. A separate QA variant initialized at 1024×640 confirmed the panels, controls, fitted canvas, overlapping light/dark text, and status/error area; server readiness was not verified there because its test port was occupied. Direct canvas-click inspector feedback remains open, so this section does not claim full visual certification.

### Persistence and live browser output

- [x] Establish an ordered widget model with stable identities and one authoritative document representation shared by the editor, persistence, and browser projections.
- [x] Save and restore widget content, names, properties, identities, and stacking order.
- [x] Explicitly version the changed saved format as format 2. No conversion of 0.0.1 documents is required; reject unsupported versions without modifying them and document how to move aside temporary old data to start a fresh workspace.
- [x] Failed saves preserve the previous source and current dirty work; malformed or unsupported files remain non-destructive errors and incompatible startup is blocked.
- [x] Native coordinator mutations update the current registered overlay snapshot without manual refresh at the coordinator/hub boundary.
- [x] Overlay URLs remain stable across edits, renames, saves, and restarts because they use stable overlay IDs.
- [x] Browser output remains transparent and the current served output is the rendering authority; native-preview differences are documented.
- [x] Browser snapshots contain every widget in frontmost-first model order, and the client validates the complete array atomically, reconciles by stable ID, removes absent nodes, and enforces reverse DOM order.

**Progress note:** Format-2 model/persistence, failure safety, coordinator/hub publication, URL identity, transparent multi-widget renderer, and complete browser ID-map reconciliation are covered. Browser revision validation accepts `Number.MAX_SAFE_INTEGER` and rejects unsafe larger values.

### OBS verification

- [ ] Exercise creation, composition, saving, restart, and OBS browser-source use on macOS and Linux.
- [ ] Verify overlapping text, stacking order, transparency, deletion, and live updates on both targets.
- [ ] Verify stable URLs and visible server failures after moving settings out of the workspace.

**Progress note:** Automated/headless checks do not substitute for native OBS validation; #27 remains incomplete.

## Confirmed Scope

The following choices were confirmed on 2026-09-08 and completed as accepted design contracts on 2026-09-17. Runtime implementation status is tracked by the issue22 and issue23 checkpoints above. Layout review has covered both documented sizes; direct canvas-click inspector feedback and the remaining #24–#27 work are still pending.

| Decision | Scope |
|---|---|
| Widget types and imported assets | Multiple text widgets; images are deferred to 0.0.3. Arbitrary font imports remain deferred. |
| Font selection | Include a small bundled font collection shared by native preview and browser output. |
| Undo/redo and unsaved close | Include session-local widget undo/redo and a warning before closing with unsaved changes. |
| Settings window | Use a separate native window. |
| Existing saved data | No conversion of temporary 0.0.1 saves; unsupported versions remain non-destructive errors. |
| Appearance | Deliver one improved appearance. |

Zoom/pan controls, widget hiding/locking, and more advanced selection remain deferred.

### Resolved design contracts

- [FDR-003](fdr/FDR-003-multi-widget-composition-workspace.md) and [ADR-008](adr/ADR-008-coordinator-owned-workspace-history.md) define one coordinator-owned workspace timeline of 100 completed actions, text/gesture grouping, overlay lifecycle, selection restoration, content-based dirty state, fresh delivery revisions, and close recovery. The timeline and close-flow portions remain #24 implementation work.
- [FDR-004](fdr/FDR-004-bundled-offline-text-fonts.md) selects pinned Noto Sans Regular and JetBrains Mono Regular assets with bounded Latin-focused coverage, SIL OFL 1.1 notice obligations, U+FFFD replacement, and explicit issue #26 binary/runtime verification; assets remain undelivered.
- [FDR-003](fdr/FDR-003-multi-widget-composition-workspace.md) defines a dark neutral workspace at 1280×800 initial and 1024×640 minimum logical size, including panel, spacing, typography, checkerboard, selection, hover, disabled, and error targets; issue23 code and headless interaction/geometry checks are delivered, layout has been visually reviewed at both sizes, and direct canvas-click inspector feedback remains under investigation.
- [ADR-006](adr/ADR-006-ordered-authoritative-widget-model.md) and [ADR-007](adr/ADR-007-version-2-overlay-document-persistence.md) define the delivered ordered model and format-2 persistence contracts. [FDR-005](fdr/FDR-005-singleton-native-settings-window.md) and [ADR-009](adr/ADR-009-secondary-native-settings-viewport-lifecycle.md) define the not-yet-delivered singleton Settings lifecycle.

These design items are complete as decisions; the unchecked requirements below remain the source of truth for implementation completion.

## Quality Requirements

- [x] Tests cover widget identity, ordered mutations, selection validity, persistence round trips, and non-destructive rejection of unsupported saved formats.
- [x] Tests cover browser projection and live changes for multiple widgets, including reorder and removal.
- [ ] Undo/redo tests cover grouped edits, deletion restoration, redo invalidation, and dirty state around saving; close-flow checks cover Save, Discard, Cancel, and save failure.
- [x] Headless egui pointer scenarios cover obscured-widget selection, overlap selection, row reveal, empty-canvas clearing, overlay switching, and deletion fallback.
- [ ] Verify keyboard shortcuts and text-field focus behavior when #24 implements them.
- [ ] Measure representative idle resource use and responsiveness with a documented multi-widget workload; investigate material regressions against the [0.0.1 measurement](measurements/0.0.1-idle-resource-usage.md).
- [x] Update setup, editing, settings, and troubleshooting guides for the issue22 and issue23 checkpoints.
- [x] Record current feature and architecture contracts using the owning documentation skills; update indexes, glossary, and current architecture as implementation makes them stale.
- [x] Verify documentation links and ensure milestone requirements distinguish delivered checkpoint behavior from accepted-but-incomplete scope.

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
| Workspace layout, stable-ID selection, ordering, drag bounds, and appearance | #23 | Headless pointer/list selection including overlap, empty canvas, reveal, and deletion fallback; panel/canvas geometry at 1024×640 and 1280×800; visual layout review at both sizes; direct canvas-click feedback follow-up |
| Workspace history, keyboard focus, dirty baseline, save failure, and editor close | #24 | Named history/save scenarios, 100-action capacity, grouping, redo invalidation, text-focus shortcut suppression, and Save/Discard/Cancel |
| Singleton native Settings window | #25 | Reuse/focus, discard-on-close, independent save boundaries, next-launch port behavior, and macOS/Linux native lifecycle checks |
| Exact bundled fonts, coverage, replacement, and licenses | #26 | Pinned-byte lengths and SHA-256, cmap/U+FFFD checks, complete SIL OFL 1.1 notices, decoded data-URL equality, and native/browser rendering |
| Combined OBS/platform and representative-resource verification | #27 | macOS/Linux OBS composition, stable URLs, transparent stacking/live updates, Settings failures, font behavior, and history memory measurement |

The Python documentation tests validate that accepted contracts, assignments, and issue22/issue23 checkpoint claims name current evidence; they do not exercise runtime features.

## Implementation Sequence

1. Record the changed feature/model contracts and settle the dependent design details. **Design complete on 2026-09-17; issue22 and issue23 implementation checkpoints are delivered, with direct canvas-click feedback follow-up pending.**
2. Establish ordered widget state, mutation behavior, saved-format handling, and complete browser array reconciliation (#22). **Checkpoint delivered in the current source branch.**
3. Build the workspace panels and shared selection behavior (#23). **Layout and headless checks delivered; corrected visual review covered the panel/canvas layout at 1280×800 and a separate 1024×640 QA variant. Direct canvas-click inspector feedback needs a fix and follow-up.**
4. Complete widget editing, layer actions, and agreed recovery/keyboard behavior (#24). **Pending history, shortcuts, and close recovery.**
5. Move Settings into a native window (#25). **Pending.**
6. Integrate bundled fonts (#26), then complete combined platform/OBS verification and documentation (#27). **Pending; no font files are bundled yet.**

## Completion Gate

`0.0.2` is complete when every in-scope checkbox is satisfied, the design details above are resolved and recorded, the macOS and Linux OBS workflows pass, relevant tests pass, and documentation matches the implementation. The issue22 and issue23 implementation checkpoints are not the milestone completion gate; issue23's direct canvas-click follow-up, remaining visual checks, the #24–#27 requirements, and platform validation still apply. Deferred requirements must be explicitly removed through the appropriate product decision rather than left implicitly incomplete. Release publication is tracked separately from implementation completion.
