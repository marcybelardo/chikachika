# Use an overlay in OBS

Chikachika serves the selected overlay as a local transparent Browser Source.
The following setup applies to OBS on both supported platforms.

## Add the Browser Source

1. Launch Chikachika and [create or restore an overlay](overlay-workflow.md).
2. Select the overlay and wait until its **Browser-source URL** is shown.
3. Click **Copy URL** in Chikachika.
4. In OBS, add a **Browser Source** to the scene.
5. Paste the copied value into the Browser Source **URL** field.
6. Set the Browser Source **Width** and **Height** to the overlay’s displayed
   **Canvas** dimensions, in pixels.
7. Confirm the source and position it in the scene like any other OBS source.

Use the exact URL displayed for the selected overlay. Do not reconstruct it by
hand or substitute a different port. The URL normally begins with
`http://127.0.0.1:51737/overlay/`; an intentionally configured port changes
the port portion, while the stable overlay ID keeps the path stable across
rename, edit, save, and restart.

## Transparency, stacking, and canvas size

The served browser output has a transparent background. The Browser Source
canvas uses the fixed dimensions chosen when the overlay was created, so
matching OBS **Width** and **Height** to those dimensions preserves the layout.
The overlay’s text is composited over the OBS scene; no solid Chikachika
background is expected.

The widget collection is frontmost-first in the model: index 0 is frontmost.
Initial HTML and browser updates use reverse DOM order, and the browser appends
existing nodes rather than replacing them, so reordering preserves widget node
identity while changing paint order. Browser rendering is the authority if it
differs from the native preview, including font-metric or line-break
differences. Font IDs are persisted, but no bundled font files are delivered
yet.

## Live updates

Keep the Browser Source connected while editing the selected overlay in
Chikachika. Supported changes to text widgets—including content, names, font
ID, size, color, alignment, position, add, duplicate, delete, and adjacent
layer moves—are sent as complete current snapshots automatically. You do not
need to recreate the Browser Source or manually refresh the page after each
change.

The browser client validates the complete snapshot, every widget, and widget-ID
uniqueness before touching the DOM or advancing its revision. It reconciles an
ID-to-node map, removes absent nodes, uses safe text/style properties, and
rejects stale, duplicate, unsafe, or malformed snapshots without partial
mutation. `Number.MAX_SAFE_INTEGER` is accepted as a safe revision; larger
values are rejected.

If you intentionally change the server port, save it for the next launch and
restart Chikachika. The current server and any currently configured Browser
Source remain on the old port until that restart; after restarting, copy the
new exact URL into OBS.

For a source that does not update, first confirm that Chikachika still shows a
ready server, that the selected overlay is the one used by OBS, and that OBS
contains the exact copied URL. Then see [Troubleshooting](troubleshooting.md).

## Scope note

This checkpoint does not claim the final 0.0.2 resizable workspace, overlap
pointer-selection polish, session history, separate native Settings window,
bundled font assets/fidelity, or macOS/Linux OBS certification. Those remain
tracked in [TODO-0-0-2](../TODO-0-0-2.md).
