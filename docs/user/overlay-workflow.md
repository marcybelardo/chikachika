# Create and edit an overlay

## Create an overlay

1. Launch Chikachika and choose **Create overlay**.
2. Enter a name and positive whole-number values for **Fixed canvas width**
   and **Fixed canvas height**. These dimensions are the output size in pixels.
3. Choose **Create**. The new overlay is selected and marked as having
   unsaved changes.

An overlay has a fixed canvas and can contain zero or one text widget in
`0.0.1`. Multiple widgets, layering, animation, rich text, and canvas resizing
are not part of this version.

## Add and edit the text widget

In **Overlay details**, use **Add text widget** when the overlay does not yet
have one. The editor provides:

- **Content** — the text, including multiple lines.
- **Font size** — the size in pixels.
- **Color** — the text color, including its alpha (opacity) channel.
- **Alignment** — **Left**, **Center**, or **Right**.
- **Position** — the X and Y coordinates on the fixed canvas.

You can also drag the text widget in **Canvas preview — drag to move the text
widget**. The position stays inside the canvas. The preview is a layout aid;
the browser output used by OBS is the final rendering authority. The preview
and browser can have slightly different glyph metrics because the native
preview and browser use different text-rendering environments. Font-family
selection is not supported in `0.0.1`.

## Save and restore

After creating or editing an overlay, click **Save**. The status changes from
**Unsaved changes** to **Saved** only after the complete overlay collection is
written successfully. Save after each set of changes you want to keep before
closing the application.

On the next launch, Chikachika restores the saved overlay collection. An
overlay keeps its stable identity when you rename it, so its Browser Source
URL remains the same across renames and restarts. Deleting an overlay requires
an explicit confirmation and removes its browser output.

## Find the browser output

Select the overlay you want to use and wait for the server-ready state. Under
**Browser-source URL**, Chikachika displays the exact URL for that overlay.
Use **Copy URL** to place that exact value on the clipboard, or **Open in
browser** to inspect the browser output directly. These actions are unavailable
until the server is ready and the selected overlay is registered.

Continue with [OBS Browser Source setup](obs-browser-source.md).
