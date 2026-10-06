# Issue #24 runtime evidence checkpoint

**Status:** Automated implementation checks passed at source checkpoint `3d29d1f`; native macOS/Linux verification remains pending.
**Decision:** [FDR-006: Guarded Editor Quit](../fdr/FDR-006-guarded-editor-quit.md)
**Evidence baseline:** Rust source checkpoint `3d29d1f` (`cargo test --locked --all-targets`: 131 tests passed, as reported for this checkpoint). Documentation changes are not included in that Rust result; rerun the suite after this documentation update.

## Scope and evidence boundary

This page records source-verified automated coverage for issue #24's workspace history, edit grouping and gestures, restoration, save behavior, focus-safe commands, and guarded close flow. The names below were checked against the Rust test functions in the linked source files; they are not names copied only from the original test plan. The 131-pass result is the supplied checkpoint result for `3d29d1f`, not a claim about a post-documentation Rust run.

Automated headless tests do not establish native macOS menu/event-loop behavior, native Linux window-manager behavior, or prompt usability in a real desktop at either target size. Those checks remain pending in [issue24 native close validation](issue24-native-close-validation.md). Issue #25's singleton Settings window, #26's bundled fonts, and #27's OBS/platform/resource certification remain incomplete.

## AC.2–AC.7 automated implementation evidence

| Acceptance criterion | Source-verified Rust test names | Result |
|---|---|---|
| AC.2 — workspace history, selection restoration, shared capacity, pending edits and redo invalidation | `workspace_history_command_matrix`; `history_selection_restoration_matrix`; `history_capacity_and_redo_invalidation`; `pending_undo_enforces_shared_history_capacity`; `pending_edit_command_boundaries`; `history_restore_failure_preserves_workspace` (`src/app.rs`) | **AUTOMATED PASS** |
| AC.3 — grouped text, numeric/color edits, gestures, cancellation, and pending-edit command boundaries | `grouped_inspector_text_edits`; `color_slider_drag_updates_live_and_undoes_as_one_edit`; `color_channel_numeric_edit_is_one_undoable_change`; `numeric_drag_commits_on_release_and_cancels_with_escape`; `color_popup_open_and_close_preserve_document_history`; `drag_commit_cancel_and_live_updates`; `focused_fields_suppress_document_shortcuts_and_global_save_remains_available`; `global_save_commits_and_persists_pending_inspector_edit_while_text_focused` (`src/gui.rs`); `pending_edit_command_boundaries` (`src/app.rs`) | **AUTOMATED PASS** |
| AC.4 — atomic restore, fresh runtime revisions, routes/streams and paired HTML state | `history_restore_failure_preserves_workspace` (`src/app.rs`); `collection_restore_preflight_is_atomic`; `html_snapshot_revision_pair`; `undo_redo_overlay_delete_routes_and_streams` (`src/server.rs`); `restart_revision_initialization` (`src/app.rs`) | **AUTOMATED PASS** |
| AC.5 — saved-content dirty baseline, save failures and session-only history | `save_edit_undo_clean`; `history_eviction_does_not_change_dirty_baseline`; `failed_save_preserves_history_and_pending_work`; `history_is_session_only` (`src/app.rs`) | **AUTOMATED PASS** |
| AC.6 — menus, focus-safe shortcuts, text-native undo/newline and previous-selection isolation | `focus_safe_shortcut_matrix_and_text_native_undo_and_newline`; `focused_fields_suppress_document_shortcuts_and_global_save_remains_available`; `issue24_menu_actions_and_enabled_states`; `document_undo_cannot_restore_native_text_undo_from_previous_selection` (`src/gui.rs`) | **AUTOMATED PASS** |
| AC.7 — guarded close choices, save/discard/cancel, save failures, pending edits, quit routes and shutdown ownership | `close_prompt_save_discard_cancel_matrix`; `close_save_failure_keeps_work_and_server`; `pending_close_failure_cancels_native_close_and_preserves_live_work`; `close_save_writes_the_new_baseline_before_closing`; `close_discard_does_not_write_overlay_or_settings_files`; `clean_close_does_not_open_unsaved_prompt`; `quit_while_text_focused_resolves_pending_edit`; `file_quit_while_text_focused_resolves_pending_edit`; `command_q_while_text_focused_resolves_pending_edit`; `macos_event_loop_disables_default_menu` (`src/gui.rs`); `normal_gui_return_shuts_down_once` (`src/main.rs`) | **AUTOMATED PASS** |

`normal_gui_return_shuts_down_once` verifies the main-process shutdown ownership seam after the GUI returns; it is not an end-to-end native accepted-close test. Similarly, the macOS event-loop menu test is an automated source-level/headless check, not native macOS QA.

## AC.8 documentation contracts

The Python suite owns documentation assertions, not product-runtime evidence. Focused contracts include:

- `issue24_documentation_checkpoint` — checkpoint result and the separation between automated evidence and native QA.
- `issue24_documentation_checkpoint_names_runtime_evidence` — evidence names, their source files, and source-level test-function presence.
- `published_v001_prerelease_documentation` — published v0.0.1 source-only prerelease status.
- `issue23_obsolete_warning_removed` — living user guidance does not retain the obsolete separate-caption warning; historical issue23 evidence remains.
- `guarded_quit_fdr_contract` — FDR-006 remains Accepted, supersedes FDR-003, and records the decision separately from implementation verification.

These assertions verify documentation after they run. They do not establish Rust test passes, native platform behavior, OBS certification, or completion of #25–#27.
