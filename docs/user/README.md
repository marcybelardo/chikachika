# Chikachika user guide

This is the user-facing guide for Chikachika `0.0.1`. It describes the
source-based development build that is currently available for macOS and
Linux. There is no packaged installer or release bundle documented for this
version yet.

Chikachika is a local-first overlay editor. The native workspace creates and
saves an overlay, while a transparent browser output makes the same overlay
available to OBS as a Browser Source. Start with [Getting started](getting-started.md),
then follow [Overlay workflow](overlay-workflow.md) and [OBS Browser Source](obs-browser-source.md).
If something does not look right, see [Troubleshooting](troubleshooting.md).

## The 0.0.1 workflow

1. Install the development prerequisites for [macOS or Linux](getting-started.md).
2. Launch Chikachika from the repository with `cargo run`.
3. Create an overlay with a fixed canvas size, add its optional text widget,
   and edit the text, size, color, alignment, and position.
4. Save the overlay. Chikachika restores saved overlays the next time it
   starts.
5. Select the saved overlay after the local server is ready, copy the exact
   Browser Source URL, and add it to OBS.
6. Keep the Browser Source connected while editing. Supported changes appear
   in the browser output without recreating or manually refreshing the source.

The browser and OBS output is authoritative. The native preview is a useful
layout aid, but its font metrics can differ from the browser renderer. Font
family selection is not supported in `0.0.1`.

## Related project documentation

- [0.0.1 milestone checklist](../TODO-0-0-1.md)
- [FDR-001: Overlay Editing and Local Browser Source](../fdr/FDR-001-overlay-editing-and-local-browser-source.md)
- [FDR-002: Browser-Source URL Actions and Port Settings](../fdr/FDR-002-browser-source-url-actions-and-port-settings.md)
- [Repository setup and developer checks](../../README.md)
