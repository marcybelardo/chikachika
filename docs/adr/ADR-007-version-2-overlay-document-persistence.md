# ADR-007: Persist Version-2 Overlay Documents

**Status:** Accepted
**Date:** 2026-09-17
**Supersedes:** ADR-003

## Context

The 0.0.2 overlay document must persist an ordered multi-widget collection, including stable identities, editable names, and bundled-font choices. The temporary format-1 schema cannot represent that contract. Persistence must retain the prior non-destructive path, load, and save guarantees while making delivery revisions and editor session state explicitly non-persistent.

## Decision

- Overlay persistence uses a strongly typed Serde JSON envelope with top-level format version `2` and an ordered overlays collection.
- Each persisted overlay includes its stable identity, name, canvas data, and ordered text-widget collection. Each widget includes its stable identity, editable name, multiline content, finite position, size, RGBA color, alignment, and stable font-family ID.
- Format 2 omits delivery revisions, revision high-water marks, selection, hover, pending edits, gestures, and undo/redo history.
- There is no format-1 conversion. Unknown versions and malformed data fail visibly and non-destructively.
- Invalid or duplicate overlay/widget identities, non-finite required coordinates, and unknown font IDs are rejected visibly and non-destructively before replacing the in-memory collection.
- A failed load never overwrites its source or replaces the currently usable in-memory workspace.
- To start fresh after an incompatible format-1 or temporary file, the user moves the old overlay data aside to a separately named backup; the application does not overwrite or silently convert it.
- Path resolution uses `directories::ProjectDirs` with the unqualified `Chikachika` identity and `data_local_dir`.
- Exact platform-resolved paths are surfaced in implementation and documentation and covered on macOS and Linux.
- Path resolution is fallible, failures are visible, required app-local directories are created explicitly, and persistence never falls back to the working directory.
- Saving clones a complete whole-collection snapshot and performs file I/O outside model locks.
- Saving writes a temporary file in the same directory and replaces the source using an atomic or platform-safe replacement operation.
- A failed save preserves the previous source, current work, history, dirty state, and a visible error.
- The dirty baseline is persistent document-content equality with the last successful whole-collection save; revisions and transient session state do not participate.
- When implementation dependency versions are selected, their APIs and same-directory replacement guarantees must be verified against those actual versions before the atomic-save claim is considered implemented.
- Server settings remain a separate format-1 `settings.json` envelope in the platform config location under ADR-005; this decision changes only overlay documents.

## Rationale

A new explicit version prevents the old cardinality and schema from being mistaken for the multi-widget document. Rejecting rather than converting temporary format-1 data keeps migration complexity out of the milestone and preserves user recovery options. Persisting only durable content lets revisions restart safely and makes dirty state a content question rather than a history-position question. Retaining same-directory replacement and visible failures protects both disk and memory state.

## Predecessor Clause Audit

### Retained

- Strongly typed Serde JSON, a top-level version, and an overlays collection remain required.
- `ProjectDirs`, unqualified `Chikachika`, `data_local_dir`, surfaced platform paths, macOS/Linux coverage, fallible visible resolution, explicit directory creation, and no working-directory fallback remain unchanged.
- Unsupported or malformed input is non-destructive; failed loads never overwrite the source.
- Saving clones a complete snapshot, performs I/O outside model locks, writes a same-directory temporary file, and uses atomic or platform-safe replacement.
- Failed saves retain dirty in-memory work and expose errors.
- Actual dependency APIs and replacement guarantees must be verified for selected versions.
- Rejection of unversioned JSON, opaque binary data, config directories for documents, manually built home paths, peer-to-peer transport, distribution services, and cloud synchronization remains unchanged.

### Changed

- The envelope version changes from format 1 to format 2 and now persists ordered widget collections and font IDs.
- The complete snapshot is explicitly the whole overlay collection.
- Validation expands to duplicate/invalid identities, non-finite required coordinates, and unknown font IDs.
- Persistent equality explicitly excludes revisions and editor session state.

### Retired

- Reading or converting format-1 overlay documents is deliberately not supported.
- No path, error, snapshot, lock, temporary-file, replacement, or dependency-verification guarantee is retired.

## Successor Clause Manifest

The following material clauses are intentionally test-governed by `test_002_decision_record_set`:

| Clause ID | Required outcome |
|---|---|
| `adr007_format_2` | Typed JSON uses overlay format version 2 with ordered widgets and stable IDs/font IDs. |
| `adr007_path_contract` | Fallible ProjectDirs app-local resolution is visible, covered, explicit, and never falls back to the working directory. |
| `adr007_non_destructive_load` | Unsupported, malformed, invalid, duplicate, or unknown-font input preserves source and workspace. |
| `adr007_snapshot_io` | A whole-collection snapshot is cloned and I/O occurs outside model locks. |
| `adr007_replacement` | Same-directory temporary replacement remains required and must be verified against actual dependency versions. |
| `adr007_transient_omission` | Revisions, selection, gestures, and history are omitted from format 2. |
| `adr007_settings_unchanged` | Settings remain separate config-local format 1 data. |

Removal or material mutation of any manifested outcome must fail the focused documentation contract test.

## Alternatives Considered

- **Convert format 1 automatically:** Rejected because 0.0.1 data is explicitly temporary and migration would add unneeded ambiguity and risk.
- **Persist revisions or history:** Rejected because they are running-session delivery and interaction state, not durable document content.
- **Store settings in format 2:** Rejected because ADR-005 assigns configuration an independent lifecycle and location.
- **Overwrite incompatible data with defaults:** Rejected because it destroys recovery evidence and violates non-destructive failure behavior.
- **In-place writes:** Rejected because interruption could expose a partial document.

## Consequences

### Positive

- Saved order, identities, names, font choices, and widget properties have a deliberate schema.
- Corrupt or incompatible data cannot silently damage the source or current workspace.
- Dirty-state comparison remains correct across undo/redo and fresh delivery revisions.
- Overlay documents and server settings keep independent versions and locations.

### Negative

- Existing format-1 overlay data must be moved aside and recreated rather than converted.
- Validation and whole-collection snapshots add implementation and test work.
- Atomic or platform-safe replacement remains dependent on verified library and operating-system behavior.

## Related

- **ADRs:** [ADR-005: Separate Versioned Server Settings from Overlay Documents](ADR-005-separate-server-settings-from-overlay-documents.md), [ADR-006: Use an Ordered Authoritative Widget Model](ADR-006-ordered-authoritative-widget-model.md), [ADR-008: Keep Workspace History in the Coordinator](ADR-008-coordinator-owned-workspace-history.md)
- **FDRs:** [FDR-003: Multi-Widget Composition Workspace](../fdr/FDR-003-multi-widget-composition-workspace.md), [FDR-004: Bundled Offline Text Fonts](../fdr/FDR-004-bundled-offline-text-fonts.md)
