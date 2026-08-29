# Glossary

This is the canonical source for project-specific terminology. Define recurring product, UI, architecture, and documentation concepts here; link to the ADR, FDR, or architecture page that owns the longer explanation.

Use [the glossary skill](../.agents/skills/glossary/SKILL.md) to look up, add, rename, or audit terms.

## Product

**Overlay** — A named, locally saved composition that a streamer edits and exposes as a stable browser output; see [FDR-001](fdr/FDR-001-overlay-editing-and-local-browser-source.md).

**Browser Source** — The transparent browser output of an overlay, used as a source in OBS and kept current for a connected viewer; see [FDR-001](fdr/FDR-001-overlay-editing-and-local-browser-source.md).

## UI

**Canvas** — The fixed, explicitly configured visual area in which an overlay is previewed and its optional text widget is positioned; see [FDR-001](fdr/FDR-001-overlay-editing-and-local-browser-source.md).

**Widget** — A supported editable element on a canvas; in 0.0.1 this means the overlay’s optional text element, with zero-or-one cardinality; see [FDR-001](fdr/FDR-001-overlay-editing-and-local-browser-source.md).

## Architecture

**Settings** — Application configuration kept separate from overlay documents; in the issue #8 contract, versioned `settings.json` stores the loopback server port in the platform config location; see [ADR-005](adr/ADR-005-separate-server-settings-from-overlay-documents.md).

## Documentation

No additional documentation terms recorded yet.
