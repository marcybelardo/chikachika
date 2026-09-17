# Chikachika user guide

This guide describes the current source-based development build for Chikachika
on macOS and Linux. There is no packaged installer or release bundle documented
for this version yet.

Chikachika is a local-first overlay editor. The native workspace creates and
saves an ordered collection of text widgets, while a transparent browser output
makes the same overlay available to OBS as a Browser Source. Start with
[Getting started](getting-started.md), then follow [Overlay workflow](overlay-workflow.md)
and [OBS Browser Source](obs-browser-source.md). If something does not look
right, see [Troubleshooting](troubleshooting.md).

## The issue22 checkpoint workflow

1. Install the development prerequisites for [macOS or Linux](getting-started.md).
2. Launch Chikachika from the repository with `cargo run`.
3. Create an overlay with a fixed canvas, then use the widget selector to add,
   name, edit, duplicate, delete, and layer text widgets. The list is
   frontmost-first: index 0 is frontmost, and adding or duplicating inserts at
   that position.
4. Edit the selected widget’s multiline content, font family ID, size, RGBA
   color, alignment, and position. The coordinator owns selection; switching
   overlays clears widget selection and accepted deletion repairs it.
5. Save the overlay collection. The format-2 document stores durable widget
   content, names, properties, IDs, and order, but not selection, runtime
   delivery revisions, or history.
6. Select the saved overlay after the local server is ready, copy the exact
   Browser Source URL, and add it to OBS.
7. Keep the Browser Source connected while editing. Complete current snapshots
   reach the browser without recreating or manually refreshing the source.

The browser and OBS output is authoritative. The native preview is a useful
layout aid, but its font metrics can differ from the browser renderer. The
accepted font IDs are `noto-sans` and `jetbrains-mono`; the IDs persist in the
document, but no font files are bundled yet. The final 0.0.2 workspace layout,
overlap hit-testing, history, native Settings window, font binary fidelity,
and macOS/Linux OBS certification remain incomplete work tracked in
[`TODO-0-0-2`](../TODO-0-0-2.md).

## Related project documentation

- [0.0.2 milestone and issue22 checkpoint](../TODO-0-0-2.md)
- [Where Chikachika saves data](getting-started.md#where-chikachika-saves-data)
- [Persistence recovery](troubleshooting.md#saved-overlays-do-not-appear)
- [FDR-003: Multi-Widget Composition Workspace](../fdr/FDR-003-multi-widget-composition-workspace.md)
- [ADR-007: Version-2 Overlay Document Persistence](../adr/ADR-007-version-2-overlay-document-persistence.md)
- [Repository setup and developer checks](../../README.md)
