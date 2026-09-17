# Glossary

This is the canonical source for project-specific terminology. Define recurring product, UI, architecture, and documentation concepts here; link to the ADR, FDR, or architecture page that owns the longer explanation.

Use [the glossary skill](../.agents/skills/glossary/SKILL.md) to look up, add, rename, or audit terms.

## Product

**Overlay** — A named, locally saved composition of zero or more text widgets that a streamer edits and exposes as a stable browser output; see [FDR-003](fdr/FDR-003-multi-widget-composition-workspace.md).

**Browser Source** — The transparent browser output of an overlay, used as a source in OBS and kept current for a connected viewer through complete `widgets`-array snapshots; see [FDR-001](fdr/FDR-001-overlay-editing-and-local-browser-source.md).

## UI

**Canvas** — The fixed, explicitly configured visual area in which an overlay is previewed and its ordered text widgets are positioned; see [FDR-003](fdr/FDR-003-multi-widget-composition-workspace.md).

**Widget** — A supported editable element on a canvas; the current implementation supports zero or more independently identified text widgets in an ordered collection.

**Frontmost** — The paint-order position represented by model index 0; the native preview paints the collection back-to-front and the initial browser HTML traverses the model in reverse so index 0 is painted last.

**Widget selection** — The application coordinator’s optional stable widget ID for the selected overlay; switching overlays clears it, and it is not persisted in the overlay document.

## Architecture

**Format 2** — The current versioned `overlays.json` envelope for complete ordered overlay documents, including durable widget identities and properties but excluding selection, delivery revisions, and history; see [ADR-007](adr/ADR-007-version-2-overlay-document-persistence.md).

**Delivery revision** — A running-session number assigned by `OverlayHub` to complete browser snapshots for stale-update ordering; it is not a durable model field or identity and is reset by a fresh process, with deleted-ID high-water marks retained within one hub session.

**Saved-content baseline** — The coordinator’s last-successful whole-collection document snapshot used for dirty-state comparison; selection and delivery revisions do not participate.

**OverlayHub** — The synchronized runtime hosting component that owns current overlay snapshots, delivery revisions, retained per-ID high-water marks, and bounded latest-value browser channels; see [architecture inventory](architecture/INDEX.md#runtime-and-process).

**Settings** — Application configuration kept separate from overlay documents; the current version-1 `settings.json` envelope stores the loopback server port in the platform config-local location and applies changes on the next launch; see [ADR-005](adr/ADR-005-separate-server-settings-from-overlay-documents.md).

**ProjectDirs** — The `directories` crate’s platform path resolver, used with the unqualified `Chikachika` identity for `data_local_dir` overlay documents and `config_local_dir` settings; see [ADR-007](adr/ADR-007-version-2-overlay-document-persistence.md).

## Documentation

**Living documentation** — Current README, architecture, glossary, user-guide, troubleshooting, and milestone-status material updated to match verified implementation; accepted ADR and FDR records remain append-only.

**Documentation checkpoint** — A named review of issue22’s delivered contracts, documentation evidence, automated Python assertions, and explicitly incomplete follow-on work.
