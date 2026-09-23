# Create and edit an overlay

## Create an overlay

1. Launch Chikachika and choose **Create overlay** from the top strip or
   **Create Overlay** from the File menu.
2. Enter a name and positive whole-number values for **Fixed canvas width**
   and **Fixed canvas height**. These dimensions are the output size in pixels.
3. Choose **Create**. The new overlay is selected and marked as having
   unsaved changes.

The top strip switches overlays and keeps Create, Rename, and confirmed Delete
controls together. The initial workspace size is 1280×800 logical pixels, with
a 1024×640 minimum. The widget list and inspector are resizable; the center
canvas fits the available space while preserving its aspect ratio.

## Add and edit text widgets

Use **Add text** in the left **Widgets** list. The frontmost widget appears at
the top. Rows have a **TXT** type indicator, an editable name, and a **Rename**
button that focuses that widget's name field in the inspector. Select a row to
edit that widget in the right inspector. The headless UI tests exercise
frontmost canvas selection, row reveal, and empty-canvas clearing. Manual QA
found that a canvas text click did not visibly switch the inspector to the
clicked caption; that direct-click behavior is under follow-up.

The inspector provides:

- **Name** — an editable widget name; duplicate names are allowed.
- **Content** — the text, including multiple lines.
- **Font family** — the persisted IDs `noto-sans` and `jetbrains-mono`.
- **Font size** — the size in pixels.
- **Color** — the text color, including its alpha (opacity) channel.
- **Alignment** — **Left**, **Center**, or **Right**.
- **Position** — the X and Y coordinates on the fixed canvas.

The inspector also provides **Duplicate**, **Delete widget**, **Forward**, and
**Backward**. Duplication preserves the source's properties and position but
creates a new stable ID, inserts the copy at the front, and selects it. Forward
and Backward swap one adjacent layer without wrapping. Add, duplicate, and
delete repair selection through the coordinator; switching overlays clears
widget selection rather than targeting a stale widget.

The File menu provides **Create Overlay** and **Save**. The Edit menu provides
**Add Text**, **Duplicate**, **Delete**, **Forward**, and **Backward**. The View
menu provides **Fit Canvas**, which recomputes the fit instead of adding a zoom
mode. The Help menu opens **User Documentation**. These are the currently
supported menu actions; focus-aware shortcuts and undo/redo remain #24 work.

Drag the selected widget on the center canvas. Its position stays within the
model's canvas bounds, and dragging preserves the grab offset. The preview is a
layout aid; the browser output used by OBS is the final rendering authority.
Native and browser text metrics or line breaks can differ. Font IDs are
persisted and selectable, but no font files are bundled yet.

## Save and restore

After creating or editing widgets, click **Save** in the status area or choose
**File > Save**. The status changes from **Unsaved changes** to **Saved** only
after the complete overlay collection is written successfully. A format-2 save
stores overlay and widget IDs, names, order, canvas data, multiline content,
font IDs, positions, font sizes, RGBA colors, and alignment. It does not store
selection, browser delivery revisions, pending edits, gestures, hover, or
history.

On the next launch, Chikachika restores the saved format-2 overlay collection.
An overlay keeps its stable identity when you rename it, so its Browser Source
URL remains the same across renames, edits, saves, and restarts. Deleting an
overlay requires explicit confirmation and removes its browser output.

## Find the browser output

Select the overlay you want to use and wait for the server-ready state. The
compact status area shows the exact URL for that overlay when it is ready. Use
**Copy URL** to place it on the clipboard, or **Open output** to inspect it in
a browser. These actions are unavailable until the server is ready and the
selected overlay is registered.

The browser receives complete snapshots through the existing read-only SSE
route. The client validates every widget and the whole array before changing the
DOM or advancing its revision, maps nodes by stable widget ID, removes absent
nodes, and appends reverse model order so index 0 paints frontmost. It accepts
`Number.MAX_SAFE_INTEGER` as a safe revision and rejects unsafe larger values.

Continue with [OBS Browser Source setup](obs-browser-source.md).
