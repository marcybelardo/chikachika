# Issue #24 native close validation

**Status:** Pending manual macOS and Linux QA; no native scenario has been executed for issue #24.
**Decision under test:** [FDR-006: Guarded Editor Quit](../fdr/FDR-006-guarded-editor-quit.md)

## Environment and result record

| Platform | OS/build | Application build | Environment | Result |
|---|---|---|---|---|
| macOS | macOS 27.0, build 26A428 (host identified; QA not run) | Pending | Native application menu and window/event-loop behavior not exercised | **PENDING** |
| Linux | Pending identification | Pending | Native window-manager behavior not exercised | **PENDING** |

Host identification is not a test result. No macOS or Linux issue24 native QA has been performed. For each actual run, record the OS version/build, application commit/build command, desktop/window manager where relevant, configured server port, exact scenario result, and any failure details here before changing its status.

## macOS checklist

Run on the native macOS application and record **PASS**, **FAIL**, or **BLOCKED** for each item with observed details. Leave unchecked/pending unless executed.

| Status | Scenario | Required observation |
|---|---|---|
| PENDING | Default application menu | Default About, Hide, and Quit menu items are absent; the editor remains available. |
| PENDING | File > Quit, clean document | Closes without a dirty-document prompt; normal application shutdown completes. |
| PENDING | Cmd+Q with dirty document | Uses the same Save/Discard/Cancel prompt as window close. |
| PENDING | Cmd+Q while editing text/name/numeric content | Pending edit resolves before dirty-state evaluation; quit does not discard the last edit silently. |
| PENDING | File > Quit while editing | Uses the same guarded path and preserves the pending edit before prompting. |
| PENDING | Cancel | Prompt closes, editor remains usable, dirty work is intact, and the local server `/ping` remains available. |
| PENDING | Save failure | Editor remains open with work/history intact and an actionable visible error. |
| PENDING | Successful Save | Editor closes only after successful save and normal server shutdown. |
| PENDING | Discard | Editor closes without writing the discarded document changes and normal server shutdown follows. |
| PENDING | Window close | Native window close uses the same guarded dirty-document flow. |
| PENDING | Prompt layout at 1024×640 | Prompt and all choices are usable at the documented minimum window size. |
| PENDING | Prompt layout at 1280×800 | Prompt and all choices are usable at the documented initial window size. |

## Linux checklist

Run on a native Linux desktop and record **PASS**, **FAIL**, or **BLOCKED** for each item with observed details. Leave unchecked/pending unless executed.

| Status | Scenario | Required observation |
|---|---|---|
| PENDING | Primary+S, Ctrl+Z/Shift+Z, Ctrl+D, and Delete/Backspace during native text entry | No unintended save/history/widget action; text-native undo is preserved where applicable. |
| PENDING | Dialog/Settings/port-field text input | Typing does not alter selected widgets or trigger document undo. |
| PENDING | Window-manager close, clean document | Closes without a dirty-document prompt and completes normal shutdown. |
| PENDING | Window-manager close, dirty document | Uses the guarded Save/Discard/Cancel flow. |
| PENDING | File > Quit | Uses the same guarded close path as native window-manager close. |
| PENDING | Pending text/numeric edit | Resolves before dirty-state evaluation; work is not silently lost. |
| PENDING | Cancel | Editor remains usable, dirty work is intact, and the local server `/ping` remains available. |
| PENDING | Save failure | Editor remains open with work/history intact and an actionable visible error. |
| PENDING | Successful Save and Discard | Save closes only on success; Discard does not write the discarded document change. |
| PENDING | Prompt layout at 1024×640 | Prompt and all choices are usable at the documented minimum window size. |
| PENDING | Prompt layout at 1280×800 | Prompt and all choices are usable at the documented initial window size. |

## Limits

This is focused issue #24 native close/quit validation, not issue #27's OBS, platform, font, or resource certification. OS shutdown, macOS Force Quit, process termination, crashes, and power loss are outside the guarded-close contract. Headless tests and source-level checks cannot prove AppKit menu behavior or native Linux window-manager behavior.
