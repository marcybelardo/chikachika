# ADR-008: Keep Workspace History in the Coordinator

**Status:** Accepted
**Date:** 2026-09-17
**Supersedes:** None

## Context

Undo and redo in 0.0.2 span widget edits and overlay lifecycle operations across the complete workspace. The model must remain persistence- and framework-independent, browser delivery needs fresh monotonic revisions, and selection restoration needs metadata that is neither document content nor renderer state.

## Decision

- The application coordinator owns one session-only workspace history timeline across all overlays.
- History is absent from the domain model, persisted files, publication hub, server, and browser output.
- Each completed action stores bounded before/after whole-collection snapshots plus selected overlay/widget IDs and their recorded indices.
- The timeline holds at most 100 completed actions total across undo and redo. Capacity overflow evicts the oldest undo entries.
- No-op interactions create no entry. A new document-changing action after undo invalidates redo. Saving does not create an action and does not clear history.
- Widget mutations and overlay create, rename, and confirmed delete are actions. Switching overlays and changing selection alone are not actions.
- Pending same-field text edits form one transaction until focus loss, selection change, Save, or another command. A completed drag or continuous numeric/color gesture forms one action.
- Escape cancels an active drag, restores its before-state, and creates no action. Other canceled gestures likewise restore their pre-gesture state.
- Undo, redo, close, and menu document-history commands first resolve pending edits into a completed transaction. Save commits pending text before saving.
- Restoration validates a complete target snapshot and applies document plus selection restoration atomically; partial restoration is not observable.
- A recorded null selection stays null. Valid recorded overlay and widget IDs are restored exactly.
- If a recorded overlay ID is absent, restoration selects the overlay at the recorded index, otherwise the last overlay, otherwise none, and clears widget selection.
- If the recorded overlay exists but the recorded widget ID is absent, restoration selects the widget at the recorded index, otherwise the last widget, otherwise none.
- History restoration allocates fresh delivery revisions and never restores old counters. Per-overlay revision high-water marks remain for the server session, including deleted IDs.
- Restoring a deleted overlay re-registers its same stable route and URL. Current connected outputs receive complete snapshots; deletion closes its streams. Automatic recovery of a page whose stream was deleted is not promised beyond ADR-004's existing reconnect contract without dependent implementation validation.
- Snapshot history is bounded by action count, not memory bytes; issue #27 must measure a representative multi-widget workload rather than infer a memory guarantee.

## Rationale

Coordinator ownership can combine document snapshots, cross-overlay lifecycle, selection, publication, and save boundaries without coupling those concerns into the model. Whole-collection before/after snapshots favor clear, atomic semantics over inverse-operation complexity. A fixed action count prevents unbounded entry growth, while fresh revisions keep history navigation compatible with latest-value browser delivery.

## History Restoration Scenarios

| Scenario ID | Starting condition and action | Required outcome |
|---|---|---|
| `cross_overlay_undo` | An action edits overlay B while overlay A was selected before it; Undo is invoked later. | The before collection is restored atomically, the valid recorded overlay A and its recorded widget selection are restored exactly, and affected outputs receive fresh complete snapshots. |
| `absent_overlay_selection` | The recorded overlay ID is absent in the restored collection. | Select the overlay now at its recorded index, otherwise the last overlay, otherwise none; clear widget selection. A recorded null overlay remains null. |
| `absent_widget_selection` | The recorded overlay exists but the recorded widget ID is absent. | Select the widget now at its recorded index, otherwise the last widget, otherwise none. A recorded null widget remains null. |
| `undo_overlay_delete` | Undo follows a confirmed deletion. | Restore the original overlay with the same ID and URL, then restore the recorded before-selection exactly when valid, using the documented fallback only when absent. |
| `redo_overlay_delete` | Redo reapplies the confirmed deletion. | Delete the overlay again, close its streams, and restore the recorded after-selection using same-index, last, or none; widget selection is clear when overlay fallback is required. |
| `redo_invalidation` | Undo is followed by a new document-changing action. | All redo entries are discarded while prior undo history remains subject to the 100-action bound. |

## Save and Revision Scenarios

| Scenario ID | Starting condition and action | Required outcome |
|---|---|---|
| `save_edit_undo_clean` | Save succeeds, an edit occurs, then Undo returns persistent content to the saved snapshot. | The workspace is clean by content equality even if a corresponding history entry was evicted; revisions and selection do not affect cleanliness. |
| `failed_save_preserves_state` | Saving dirty work fails. | Previous source, current work, history, dirty state, and a visible error all remain; no save action is added. |
| `pending_text_then_undo` | A field has pending grouped edits and document Undo is invoked. | Commit the pending transaction first, then apply document Undo; text-native undo remains separate while the field has focus. |
| `pending_text_then_save` | A field has pending grouped edits and Save is invoked. | Commit the pending transaction first, save the resulting whole collection, and retain history. |
| `restart_revision_initialization` | Save format 2, restart and load, publish, edit, then Undo. | Loaded delivery revision/high-water starts at 0; initial publication may use 0; the accepted edit publishes revision 1; Undo restores content with fresh revision 2. No revision is restored from disk. |

## Alternatives Considered

- **History inside the model:** Rejected because interaction transactions, selection, and publication coordination would couple the framework-independent domain to session concerns.
- **Persist history:** Rejected because history is session-only, potentially large, and not required to recreate document content.
- **Per-overlay timelines:** Rejected because overlay creation, deletion, rename, and cross-overlay selection need one coherent order.
- **Inverse commands only:** Rejected because complex lifecycle and selection fallback are harder to make atomic and auditable.
- **Restore old revisions:** Rejected because clients use revisions to reject stale delivery within a running session.

## Consequences

### Positive

- Cross-overlay undo and redo have one deterministic order and atomic restoration contract.
- Model, persistence, and browser payloads remain free of editor history.
- Fresh revisions preserve latest-value publication semantics during restoration.

### Negative

- Whole-collection snapshots can consume substantial memory; 100 actions bounds count, not bytes.
- The coordinator must carefully resolve pending interactions and selection fallbacks.
- Deleted/restored route and connected-client behavior needs integration testing in dependent work.

## Related

- **ADRs:** [ADR-004: Serve Stable Loopback URLs and Push Complete Snapshots with SSE](ADR-004-loopback-sse-browser-delivery.md), [ADR-006: Use an Ordered Authoritative Widget Model](ADR-006-ordered-authoritative-widget-model.md), [ADR-007: Persist Version-2 Overlay Documents](ADR-007-version-2-overlay-document-persistence.md)
- **FDRs:** [FDR-003: Multi-Widget Composition Workspace](../fdr/FDR-003-multi-widget-composition-workspace.md)
