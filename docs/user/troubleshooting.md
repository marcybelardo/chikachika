# Troubleshooting

## The server is not ready or the URL is unavailable

The Browser Source URL is intentionally hidden until the local server has
started successfully and the selected overlay is registered. Check the status
and error text in Chikachika’s workspace and **Local server settings** panel.

If the configured port is occupied, Chikachika keeps the workspace and saved
data intact but does not serve a URL. It does not silently choose an alternate
port. To recover:

1. Close the other local application using the configured port, or enter an
   available port from `1` through `65535` in **Local server settings**.
2. Click **Save port for next launch** if you changed the value.
3. Restart Chikachika. A changed port does not take effect in the current
   server session.
4. Select the overlay, copy its newly displayed exact URL, and update the OBS
   Browser Source URL.

If you deliberately want to reproduce a conflict for testing, stop Chikachika
and have another local service listen on the configured loopback port before
launching it. The expected result is a visible bind error and no fallback port.
Stop that service and restart Chikachika to recover.

## Settings are invalid

An invalid port, malformed `settings.json`, or unsupported settings version is
reported visibly. Chikachika leaves the source unchanged, does not fall back to
the default port, and does not start the server while the settings are invalid.
The settings file is a separate format-1 envelope in the platform config-local
directory. Back up the file, repair or move it aside yourself, and restart
Chikachika. A missing settings file uses `127.0.0.1:51737`.

The settings path is shown in the **Local server settings** panel. The usual
platform locations are listed in [Where Chikachika saves data](getting-started.md#where-chikachika-saves-data).

## Saved overlays do not appear

Confirm that you clicked **Save** after the last edit. Chikachika saves the
complete ordered overlay collection in version-2 `overlays.json`; a successful
save is shown by the **Saved** status. Restart the application to test
restoration.

If the overlay file is malformed, uses unsupported format 1, or fails validation
because of invalid/duplicate identities, unknown font IDs, invalid dimensions,
non-finite positions, or other invalid fields, Chikachika blocks workspace
startup rather than replacing the file with an empty collection. The blocked
startup view shows the source path and the error that needs attention.

### User-managed overlay recovery

Chikachika does not migrate format-1 overlay documents and does not
automatically delete, overwrite, rename, or move incompatible user data. To
start fresh while preserving recovery options:

1. Close Chikachika if it is running.
2. Copy the exact overlay source to a safe location as a separately named
   backup; do not edit your only copy.
3. From the surfaced source path, move the incompatible `overlays.json` aside
   yourself to a separately named file in the same or another safe location.
4. Restart Chikachika. With no overlay source at the expected path, it can
   create a new empty workspace.
5. Keep the backup until you have verified the new workspace and decided how to
   recover any content manually. Do not replace the new file with the old
   format-1 document; this checkpoint has no conversion path.

Use the same copy-before-repair approach for `settings.json`. The overlay and
settings paths are separate and a problem in one does not require overwriting
the other.

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
or manually refreshing the page. The server sends complete current snapshots;
the browser validates the whole widgets array before mutation, reconciles nodes
by stable widget ID, removes absent nodes, and preserves reverse DOM order for
frontmost painting. If the port was changed, restart Chikachika and replace the
old URL with the newly copied one.

## The native preview looks slightly different

Compare the output in the browser opened by **Open in browser** or in OBS. That
browser/OBS output is authoritative. The native preview can differ in font
metrics or line breaks. The persisted font IDs are `noto-sans` and
`jetbrains-mono`, but no bundled font files are delivered yet; exact font
assets, coverage, and fidelity remain #26 work.

## Save failed or the previous file matters

Save creates a complete snapshot in a temporary file in the destination
directory, writes and syncs it, then performs ordinary same-directory/platform
replacement. Write, sync, or replacement failure leaves the previous file and
current in-memory work available; the workspace remains dirty and exposes the
error. This is not a guarantee against power loss, physical I/O failure, or
hardware failure.

Before manually editing either persisted file, copy it to a safe location. Do
not rely on automatic cleanup or deletion for recovery: Chikachika’s load and
save failures are non-destructive, and the original source remains available
for user-managed backup or repair.
