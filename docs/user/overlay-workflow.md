# Create and edit an overlay

## Create an overlay

1. Launch Chikachika and choose **Create overlay**.
2. Enter a name and positive whole-number values for **Fixed canvas width**
   and **Fixed canvas height**. These dimensions are the output size in pixels.
3. Choose **Create**. The new overlay is selected and marked as having
   unsaved changes.

An overlay has a fixed canvas and can contain zero or more independently
identified text widgets. The widget selector is ordered **Frontmost first**:
model index 0 is frontmost, while the native preview paints widgets back-to-front
so the frontmost widget is painted last. The browser client uses the same model
order and reverse DOM order.

## Add and edit text widgets

In **Overlay details**, use **Add text widget** to add a widget. Select a row to
target that stable widget ID in the inspector. The editor provides:

- **Name** — an editable widget name; duplicate names are allowed.
- **Content** — the text, including multiple lines.
- **Font family** — the persisted IDs `noto-sans` and `jetbrains-mono`.
- **Font size** — the size in pixels.
- **Color** — the text color, including its alpha (opacity) channel.
- **Alignment** — **Left**, **Center**, or **Right**.
- **Position** — the X and Y coordinates on the fixed canvas.

The inspector also provides **Duplicate**, **Delete widget**, **Forward**, and
**Backward**. Duplication preserves the source’s properties and position but
creates a new stable ID, inserts the copy at the front, and selects it. Forward
and backward swap one adjacent layer without wrapping. Add, duplicate, and
delete repair selection through the coordinator; switching overlays clears
widget selection rather than targeting a stale widget.

You can drag the selected widget in **Canvas preview — drag the selected widget
to move it**. The position stays within the model’s canvas bounds and dragging
preserves its grab offset. The preview is a layout aid; the browser output used
by OBS is the final rendering authority. Native and browser text metrics or line
breaks can differ. Font IDs are persisted and selectable, but no bundled font
files are delivered yet.

The current editor retains the existing layout. The agreed resizable three-panel
workspace, canvas-object overlap selection, polished hover/selection treatment,
keyboard shortcuts, and history are not part of this checkpoint.

## Save and restore

After creating or editing widgets, click **Save**. The status changes from
**Unsaved changes** to **Saved** only after the complete overlay collection is
written successfully. A successful format-2 save stores overlay and widget IDs,
names, order, canvas data, multiline content, font IDs, positions, font sizes,
RGBA colors, and alignment. It does not store selection, browser delivery
revisions, pending edits, gestures, hover, or history.

On the next launch, Chikachika restores the saved format-2 overlay collection.
An overlay keeps its stable identity when you rename it, so its Browser Source
URL remains the same across renames, edits, saves, and restarts. Deleting an
overlay requires explicit confirmation and removes its browser output.

## Find the browser output

Select the overlay you want to use and wait for the server-ready state. Under
**Browser-source URL**, Chikachika displays the exact URL for that overlay. Use
**Copy URL** to place that exact value on the clipboard, or **Open in browser**
to inspect the browser output directly. These actions are unavailable until the
server is ready and the selected overlay is registered.

The browser receives complete snapshots through the existing read-only SSE
route. The client validates every widget and the whole array before changing the
DOM or advancing its revision, maps nodes by stable widget ID, removes absent
nodes, and appends reverse model order so index 0 paints frontmost. It accepts
`Number.MAX_SAFE_INTEGER` as a safe revision and rejects unsafe larger values.

Continue with [OBS Browser Source setup](obs-browser-source.md).
