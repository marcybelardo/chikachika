# `0.0.2` Milestone

**Status:** In progress — issue22 checkpoint delivered; remaining implementation pending

`0.0.2` develops the completed vertical slice into a usable overlay composition workspace. The central canvas, left widget list, and right properties inspector are the agreed layout direction. Milestone scope was confirmed on 2026-09-08, and the dependent design contracts were accepted on 2026-09-17. Issue22’s ordered model, format-2 persistence, coordinator selection/baseline behavior, native multi-widget controls, complete browser array reconciliation, and documentation checkpoint are delivered in the current source branch. The remaining issues below are not delivered here.

This document tracks milestone scope and completion. It does not replace Feature Decision Records (FDRs) for user-visible behavior or Architecture Decision Records (ADRs) for architectural rationale. [FDR-003](fdr/FDR-003-multi-widget-composition-workspace.md), [FDR-004](fdr/FDR-004-bundled-offline-text-fonts.md), and [FDR-005](fdr/FDR-005-singleton-native-settings-window.md) govern the accepted 0.0.2 behavior. [ADR-006](adr/ADR-006-ordered-authoritative-widget-model.md), [ADR-007](adr/ADR-007-version-2-overlay-document-persistence.md), [ADR-008](adr/ADR-008-coordinator-owned-workspace-history.md), and [ADR-009](adr/ADR-009-secondary-native-settings-viewport-lifecycle.md) govern its architecture. These records resolve design; their Accepted status does not claim runtime implementation.

## Outcome

The delivered issue22 checkpoint provides a native ordered collection of zero or more text widgets per overlay. Users can add, name, edit, duplicate, delete, and move widgets one adjacent layer step at a time; stable IDs and durable properties survive format-2 save/load. The coordinator owns selection and the saved-content dirty baseline. The browser client consumes complete `widgets`-array snapshots, validates them atomically, reconciles DOM nodes by stable widget ID, and preserves frontmost paint order; the complete multi-widget browser-output outcome is delivered for this checkpoint.

## Issue22 documentation checkpoint

- **Delivered:** Ordered concrete text-widget collections with stable UUID v4 identities; model index 0 is frontmost. Add/duplicate insert at index 0, forward/backward operations swap adjacent items without wrapping, and native preview paints back-to-front.
- **Delivered:** Coordinator-owned selected widget ID, selection repair for accepted add/duplicate/delete/update operations, overlay-switch selection clearing, and content-based dirty state using the last successful whole-collection save baseline. Selection and runtime revisions are not persisted.
- **Delivered:** Format-2 `overlays.json` persistence containing ordered overlays and durable widget fields. Format 1 is not migrated or converted; malformed, unsupported, duplicate, unknown-font, or invalid data is rejected non-destructively and an incompatible existing store blocks startup without an empty editable replacement.
- **Delivered:** Separate format-1 `settings.json` persistence for the loopback port. It is resolved under the platform config-local directory, saved for the next launch, and does not live-rebind the running server. The GUI surfaces the exact settings path when available.
- **Delivered:** Native widget selector, inspector, add/duplicate/delete/forward/backward controls, and current collection preview. This is not the agreed resizable three-panel workspace, overlap hit-testing, history, Settings window, bundled-font delivery, or OBS/platform certification.
- **Delivered browser output:** Complete `widgets`-array snapshots are validated atomically before DOM mutation or revision advancement. The client reconciles an ID-to-node map, removes absent nodes, preserves existing node identity, and appends reverse model order so index 0 paints frontmost. `Number.MAX_SAFE_INTEGER` is accepted while unsafe larger revisions are rejected. Connected output receives complete current snapshots through the existing SSE boundary.
- **Recovery:** If an existing overlay store is format 1 or otherwise incompatible, copy it to a separately named backup and move the original source aside yourself before restarting. Chikachika does not migrate, overwrite, delete, or automatically move user data. Start a fresh workspace only after preserving that recovery copy. Apply the same user-managed backup discipline to malformed settings data.
- **Save safety:** Replacement uses a same-directory temporary file and ordinary filesystem replacement semantics. This preserves the previous file when write/sync/replace operations fail, but it is not a power-loss durability guarantee or protection from physical I/O failure.

### Named issue22 evidence

The focused Python contract is `issue22_documentation_checkpoint` in `tests/test_docs.py`; it checks the living documentation claims, exact surfaced paths, explicit recovery guidance, integrated browser guarantees, and remaining issue status. The implementation evidence named by this checkpoint includes `ordered_widget_mutations`, `widget_selection_lifecycle`, `format_two_round_trip_and_transient_omission`, `format_one_rejected_non_destructively`, `blocked_bootstrap_preserves_incompatible_store`, `hub_session_revision_lifecycle`, `browser_multi_widget_projection_and_html`, `multi_widget_editor_scenario`, and `multi_widget_preview_paint_order`. The integrated browser contract is additionally covered by Node `multi_widget_snapshot_reconciliation`, `invalid_snapshot_is_atomic`, and `stale_snapshot_preserves_dom`.

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

**Progress note:** The existing fixed layout exposes a widget selector, inspector, collection preview, status, and readiness-gated URL actions, but the #23 resizable three-panel layout and overlap behavior remain incomplete.

### Widget identity and selection

- [ ] Each widget row has a type indicator and editable name, with a useful default derived from its content where appropriate.
- [ ] Clicking a widget row selects its canvas object; clicking a canvas object selects and reveals its row. Both update the same inspector selection.
- [ ] Hover and selection have visually distinct canvas boundaries, and the selected row is clearly distinguishable without relying solely on color.
- [ ] Canvas selection chooses the frontmost widget at the pointer; obscured widgets remain selectable through the list.
- [x] The top list row represents the frontmost widget, matching native preview and browser output stacking order.
- [x] Switching overlays or deleting a selected widget cannot leave an inspector targeting a stale widget; selection is repaired or cleared by the coordinator.
- [ ] Selection outlines, hover guides, and other editing controls never appear in browser/OBS output.
- [x] The initial workflow supports one selected widget at a time.

**Progress note:** Stable-ID list selection, selection lifecycle, and stale-selection clearing are delivered; canvas pointer/overlap selection and the final visual treatment remain #23 work.

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

**Progress note:** Separate format-1 settings persistence, validation, surfaced path, and restart-bound port behavior are delivered in the existing workspace panel. The separate singleton native Settings window remains #25 work.

### Visual clarity

- [ ] Establish a coherent treatment of spacing, typography, panel headings, control grouping, and selected/hover/disabled/error states.
- [ ] Visually separate the application panels, canvas surroundings, and transparent output area so widget boundaries are understandable.
- [ ] Verify readability and selection visibility with light, dark, small, and overlapping widget content.
- [ ] Review the layout with representative overlays at the minimum window size and a larger desktop size.
- [ ] Deliver one improved, readable appearance; selectable light/dark/system themes are outside this milestone.

**Progress note:** No #23 layout/appearance certification is claimed.

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

The following choices were confirmed on 2026-09-08 and completed as accepted design contracts on 2026-09-17. Runtime implementation and verification remain pending except where the issue22 checkpoint above explicitly marks behavior delivered.

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
- [FDR-003](fdr/FDR-003-multi-widget-composition-workspace.md) defines a dark neutral workspace at 1280×800 initial and 1024×640 minimum logical size, including panel, spacing, typography, checkerboard, selection, hover, disabled, and error targets; #23 remains incomplete.
- [ADR-006](adr/ADR-006-ordered-authoritative-widget-model.md) and [ADR-007](adr/ADR-007-version-2-overlay-document-persistence.md) define the delivered ordered model and format-2 persistence contracts. [FDR-005](fdr/FDR-005-singleton-native-settings-window.md) and [ADR-009](adr/ADR-009-secondary-native-settings-viewport-lifecycle.md) define the not-yet-delivered singleton Settings lifecycle.

These design items are complete as decisions; the unchecked requirements below remain the source of truth for implementation completion.

## Quality Requirements

- [x] Tests cover widget identity, ordered mutations, selection validity, persistence round trips, and non-destructive rejection of unsupported saved formats.
- [ ] Tests cover browser projection and live changes for multiple widgets, including reorder and removal.
- [ ] Undo/redo tests cover grouped edits, deletion restoration, redo invalidation, and dirty state around saving; close-flow checks cover Save, Discard, Cancel, and save failure.
- [ ] Exercise real pointer and keyboard selection workflows, including obscured widgets and text-field focus.
- [ ] Measure representative idle resource use and responsiveness with a documented multi-widget workload; investigate material regressions against the [0.0.1 measurement](measurements/0.0.1-idle-resource-usage.md).
- [x] Update setup, editing, settings, and troubleshooting guides for the issue22 checkpoint.
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
| Workspace layout, stable-ID selection, ordering, drag bounds, and appearance | #23 | Pointer/list selection including overlap and deletion fallback, panel sizing at 1024×640 and 1280×800, and visual-state review |
| Workspace history, keyboard focus, dirty baseline, save failure, and editor close | #24 | Named history/save scenarios, 100-action capacity, grouping, redo invalidation, text-focus shortcut suppression, and Save/Discard/Cancel |
| Singleton native Settings window | #25 | Reuse/focus, discard-on-close, independent save boundaries, next-launch port behavior, and macOS/Linux native lifecycle checks |
| Exact bundled fonts, coverage, replacement, and licenses | #26 | Pinned-byte lengths and SHA-256, cmap/U+FFFD checks, complete SIL OFL 1.1 notices, decoded data-URL equality, and native/browser rendering |
| Combined OBS/platform and representative-resource verification | #27 | macOS/Linux OBS composition, stable URLs, transparent stacking/live updates, Settings failures, font behavior, and history memory measurement |

The Python documentation tests validate that accepted contracts, assignments, and this checkpoint’s status assertions are present; they do not exercise runtime features.

## Implementation Sequence

1. Record the changed feature/model contracts and settle the dependent design details. **Complete on 2026-09-17; implementation remains pending except for the issue22 checkpoint.**
2. Establish ordered widget state, mutation behavior, saved-format handling, and complete browser array reconciliation (#22). **Checkpoint delivered in the current source branch.**
3. Build the workspace panels and shared selection behavior (#23). **Pending layout/overlap/visual certification.**
4. Complete widget editing, layer actions, and agreed recovery/keyboard behavior (#24). **Pending history, shortcuts, and close recovery.**
5. Move Settings into a native window (#25). **Pending.**
6. Integrate bundled fonts (#26), then complete combined platform/OBS verification and documentation (#27). **Pending; no font files are bundled yet.**

## Completion Gate

`0.0.2` is complete when every in-scope checkbox is satisfied, the design details above are resolved and recorded, the macOS and Linux OBS workflows pass, relevant tests pass, and documentation matches the implementation. The issue22 checkpoint is not the milestone completion gate; the remaining #23–#27 requirements and platform validation still apply. Deferred requirements must be explicitly removed through the appropriate product decision rather than left implicitly incomplete. Release publication is tracked separately from implementation completion.
