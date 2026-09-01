# Troubleshooting

## The server is not ready or the URL is unavailable

The Browser Source URL is intentionally hidden until the local server has
started successfully and the selected overlay is registered. Check the status
and error text in Chikachika’s workspace and **Local server settings** panel.

If the configured port is occupied, Chikachika keeps the workspace and saved
overlay data intact but does not serve a URL. It does not silently choose an
alternate port. To recover:

1. Close the other local application using the configured port, or enter an
   available port from `1` through `65535` in **Local server settings**.
2. Click **Save port for next launch** if you changed the value.
3. Restart Chikachika. A changed port does not take effect in the current
   server session.
4. Select the overlay, copy its newly displayed exact URL, and update the OBS
   Browser Source URL.

If you deliberately want to reproduce a conflict for testing, stop Chikachika
and have another local service listen on the configured loopback port before
launching it. The expected result is a visible bind error and no fallback
port. Stop that service and restart Chikachika to recover.

## Settings are invalid

An invalid port, malformed `settings.json`, or unsupported settings version is
reported visibly. Chikachika leaves the source unchanged, does not fall back
to the default port, and does not start the server while the settings are
invalid. Back up the file, repair or remove the settings source, and restart
Chikachika. A missing settings file uses `127.0.0.1:51737`.

The settings path is shown in the **Local server settings** panel. The usual
platform locations are listed in [Where Chikachika saves data](getting-started.md#where-chikachika-saves-data).

## Saved overlays do not appear

Confirm that you clicked **Save** after the last edit. Chikachika saves the
complete overlay collection in `overlays.json`; a successful save is shown by
the **Saved** status. Restart the application to test restoration.

If the overlay file is malformed or uses an unsupported format version,
Chikachika blocks workspace startup rather than replacing the file with an
empty collection. Back up and repair the source, then restart. The blocked
startup view shows the source path and the error that needs attention.

## OBS shows the wrong size or a background

Set OBS Browser Source **Width** and **Height** to the exact **Canvas** width
and height shown for the overlay. Then confirm that OBS contains the exact URL
copied from Chikachika and that no other source is covering the transparent
area. Chikachika’s served output is transparent and is the authority for the
final appearance.

## OBS does not update after an edit

Confirm that:

- Chikachika’s server is ready.
- The intended overlay is selected.
- OBS uses the exact URL shown under **Browser-source URL**.
- The Browser Source is still connected and visible in the scene.

A connected source should update automatically without recreating the source
or manually refreshing the page. If the port was changed, restart Chikachika
and replace the old URL with the newly copied one.

## The native preview looks slightly different

Compare the output in the browser opened by **Open in browser** or in OBS.
That browser/OBS output is authoritative. The native preview can differ in
font metrics or line breaks, and font-family selection is not supported in
`0.0.1`.

## Keep a recovery copy

Before manually editing either persisted file, copy it to a safe location. The
overlay document and settings file are separate; a problem in one does not
require overwriting the other. Chikachika’s load and save failures are
non-destructive, so the original source remains available for backup or
repair.
