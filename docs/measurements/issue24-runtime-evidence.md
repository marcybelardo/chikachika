# Issue #24 runtime evidence checkpoint

**Status:** Implementation verification in progress; no AC.2–AC.7 result is recorded as passed here.
**Decision:** [FDR-006: Guarded Editor Quit](../fdr/FDR-006-guarded-editor-quit.md)
**Evidence plan:** The approved durable issue #24 implementation plan names the expected tests; this checkpoint records their mapping, not execution results.

## Scope and current evidence boundary

This page tracks the runtime evidence expected for issue #24's workspace history, editing boundaries, restoration, saving, focus-safe commands, and close flow. The documentation branch has not verified the Rust worker's test run or native behavior. Test names below are expected names from the approved implementation plan and remain **Pending verification** until the implementation worker reports source matches and test results. A name in a plan is not evidence of implementation or a passing test.

The Python documentation contracts check that this evidence map and the accepted decision are documented; they do not exercise runtime features. Issue #25's singleton Settings window, #26's bundled fonts, and #27's OBS/platform/resource certification remain incomplete and are not certified by this checkpoint.

## AC.2–AC.7 expected implementation evidence

| Acceptance criterion | Expected Rust test names | Status |
|---|---|---|
| AC.2 — workspace history, selection restoration, capacity and redo | `workspace_history_command_matrix`; `history_selection_restoration_matrix`; `history_capacity_and_redo_invalidation` | Pending verification |
| AC.3 — grouped real edits, gestures, cancellation, pending boundaries | `grouped_inspector_text_edits`; `grouped_numeric_and_color_gestures`; `drag_commit_cancel_and_live_updates`; `pending_edit_command_boundaries` | Pending verification |
| AC.4 — atomic restore, fresh revisions, routes/streams and paired HTML state | `history_restoration_live_sse`; `undo_redo_overlay_delete_routes_and_streams`; `collection_restore_preflight_is_atomic`; `history_restore_failure_preserves_workspace`; `html_snapshot_revision_pair`; `restart_revision_initialization` | Pending verification |
| AC.5 — saved-content dirty baseline, save failures and session-only history | `save_edit_undo_clean`; `history_eviction_does_not_change_dirty_baseline`; `failed_save_preserves_history_and_pending_work`; `history_is_session_only` | Pending verification |
| AC.6 — real menus, shortcut focus, text-native undo/newline | `focus_safe_shortcut_matrix`; `focused_text_native_undo_and_newline`; `issue24_menu_actions_and_enabled_states` | Pending verification |
| AC.7 — guarded close, quit surfaces and shutdown lifecycle | `close_prompt_save_discard_cancel_matrix`; `close_save_failure_keeps_work_and_server`; `quit_while_text_focused_resolves_pending_edit`; `accepted_close_shuts_down_once` | Pending verification |

These names map to the approved plan; this documentation work has not confirmed their presence in Rust sources or their test outcomes. The native manual scenarios are listed separately in [issue24 native close validation](issue24-native-close-validation.md) and remain pending.

## AC.8 documentation contracts

The Python suite owns documentation assertions, not product-runtime evidence. Its focused contracts are:

- `issue24_documentation_checkpoint` — acceptance status and separation of automated evidence from native QA.
- `issue24_documentation_checkpoint_names_runtime_evidence` — the expected AC.2–AC.7 names above are present and explicitly pending.
- `published_v001_prerelease_documentation` — published v0.0.1 source-only prerelease status and evidence are consistent.
- `issue23_obsolete_warning_removed` — living user instructions no longer warn that a separate caption click fails; historical issue23 evidence remains.
- `guarded_quit_fdr_contract` — FDR-006 supersession, guarded quit surfaces, macOS native menu removal, retained predecessor contracts, and exclusions are documented.

These assertions can verify the documentation after they run; they cannot establish Rust test passes, macOS/Linux native behavior, OBS certification, or completion of #25–#27.
