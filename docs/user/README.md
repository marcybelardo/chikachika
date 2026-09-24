# Chikachika user guide

This guide describes the current source-based development build for Chikachika
on macOS and Linux. There is no packaged installer or release bundle
documented for this version yet.

Chikachika is a local-first overlay editor. The native workspace creates and
saves an ordered collection of text widgets, while a transparent browser output
makes the same overlay available to OBS as a Browser Source. Start with
[Getting started](getting-started.md), then follow [Overlay workflow](overlay-workflow.md)
and [OBS Browser Source](obs-browser-source.md). If something does not look
right, see [Troubleshooting](troubleshooting.md).

## Current workspace workflow

1. Install the development prerequisites for [macOS or Linux](getting-started.md)
   and launch Chikachika from the repository with `cargo run`.
2. Create or switch overlays from the compact top strip. Rename is available
   there, and Delete asks for confirmation.
3. Add text from the left **Widgets** list. Rows show a text type indicator,
   names, and a **Rename** action; the list is frontmost-first.
4. Select a row to edit that widget in the inspector. The headless UI tests
   exercise frontmost canvas selection, row reveal, and empty-canvas clearing.
   Manual QA found that a canvas text click did not visibly switch the
   inspector; that direct-click behavior is under follow-up.
5. Edit the selected widget in the right inspector, or drag it on the center
   canvas. The canvas fits the available space and preserves its aspect ratio.
6. Save the collection. Format-2 documents store widget content, names,
   properties, IDs, and order, but not selection or runtime delivery revisions.
7. Wait for the local server to be ready, then use **Copy URL** or **Open
   output** in the compact status area. Keep the connected Browser Source open
   while editing; complete current snapshots reach it without manual refresh.

The initial window size is 1280×800 logical pixels and the minimum is 1024×640.
The left list and right inspector can be resized. File, Edit, View, and Help
menus expose their current supported actions; keyboard shortcuts, document
history, and unsaved-close recovery remain #24 work. Local server settings
are collapsed in the status area by default. The separate native Settings
window remains #25 work.

The browser and OBS output is authoritative. The native preview can differ in
font metrics or line breaks. The accepted font IDs are `noto-sans` and
`jetbrains-mono`, but no font files are bundled yet. Automated UI checks cover
issue23 pointer and layout behavior. Corrected-build visual review at
1280×800 confirmed contrast for near-black and white overlapping text and
visibility of all panels. An overlapping canvas hit retained the frontmost
selection as expected. A separate QA variant initialized at 1024×640 confirmed panels, controls,
the fitted canvas, overlapping light/dark text, and the status/error area.
Server readiness was not verified in that variant because its test port was
occupied. Bundled font
files/fidelity remain #26 work, and macOS/Linux OBS certification remains #27
work tracked in [the milestone checklist](../TODO-0-0-2.md).

## Related project documentation

- [0.0.2 milestone and issue22/issue23 checkpoints](../TODO-0-0-2.md)
- [Where Chikachika saves data](getting-started.md#where-chikachika-saves-data)
- [Persistence recovery](troubleshooting.md#saved-overlays-do-not-appear)
- [FDR-003: Multi-Widget Composition Workspace](../fdr/FDR-003-multi-widget-composition-workspace.md)
- [ADR-007: Version-2 Overlay Document Persistence](../adr/ADR-007-version-2-overlay-document-persistence.md)
- [Repository setup and developer checks](../../README.md)
