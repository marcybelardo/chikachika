# ADR-006: Use an Ordered Authoritative Widget Model

**Status:** Accepted
**Date:** 2026-09-17
**Supersedes:** ADR-002

## Context

The 0.0.2 composition workspace expands an overlay from one optional text widget to an ordered collection of text widgets. The editor, persistence layer, native preview, and browser output still need one testable source of truth, while selection and history add session state that must not become persisted content. Bundled fonts also require stable document identifiers and self-contained browser delivery without widening the local HTTP API.

## Decision

- There is exactly one authoritative, framework-independent domain model for the overlay collection and its overlay documents.
- Native editor, persistence, server, and browser output are adapters or projections of that model, not independent authoritative state stores.
- The domain model has no egui, axum, filesystem, or browser dependencies.
- UI mutations go through domain or store operations rather than directly changing adapter state.
- The HTTP adapter remains read-only; there are no browser-to-application mutation routes.
- Each overlay and each text widget receives one generated opaque UUID v4 identity when created. Duplication creates a new widget UUID.
- Generated identities are persisted unchanged. Names, positions, timestamps, collection indexes, and hashes are never identity sources.
- Revisions order browser snapshots and are delivery metadata, not identities or persisted content.
- An overlay contains an ordered collection of zero or more text widgets. Each widget has an editable name, multiline content, finite anchor position, size, RGBA color, alignment, and a stable font-family ID.
- Collection index 0 is frontmost. Add and duplicate insert at index 0; forward and backward operations swap one adjacent item without wrapping. Browser and native rendering traverse the collection in reverse drawing order, while hit-testing examines frontmost items first.
- Selection, hover, editing gestures, pending form state, and history are session state outside persistent document content and browser projection.
- The only supported font-family IDs for this baseline are `noto-sans` and `jetbrains-mono`; `noto-sans` is the default. Both identify the pinned Regular static TTF assets decided by FDR-004.
- The exact same static TTF bytes are used by native preview and browser output. Native preview embeds the TTF assets, and generated overlay HTML embeds them as base64 data URLs in CSS `@font-face` declarations.
- Font delivery adds no HTTP routes. ADR-004's exact route set remains `GET /overlay/{id}` and `GET /overlay/{id}/events`.
- Original input text remains unchanged in the model. Rendering unsupported characters uses U+FFFD from the selected bundled face rather than a platform fallback; exact asset coverage and native/browser behavior must be verified during issue #26 before implementation is accepted.
- This is a concrete text-widget collection, not a generic plugin or widget hierarchy. Images, rich text, and arbitrary font imports are outside this decision.

## Rationale

One ordered domain representation prevents renderer, persistence, and editor state from diverging while making stacking behavior deterministic. Stable UUIDs preserve durable references independently of mutable names and order. Keeping ephemeral interaction state outside the document prevents selection and history from leaking into saved data or OBS output. Stable font IDs avoid persisting platform font names, and embedding identical font bytes in both projections gives offline, self-contained output without creating an asset-serving API.

## Predecessor Clause Audit

### Retained

- One authoritative framework-independent model remains the source for all projections.
- Adapter state is not authoritative, the domain has no adapter dependencies, mutations pass through domain/store ownership, and HTTP remains read-only.
- Overlay and widget identities remain generated opaque UUID v4 values, persisted unchanged and never derived from names, positions, timestamps, indexes, or hashes.
- Revisions remain ordering metadata rather than identity.
- Rejection of name/index identity and separate UI/server models remains unchanged.
- A speculative generic widget/plugin framework remains rejected.

### Changed

- The one optional text widget limit becomes an ordered collection of zero or more text widgets.
- Widget identity generation now applies to every created or duplicated widget.
- The model boundary now explicitly separates persistent content from selection, gestures, history, and delivery revisions.

### Retired

- The 0.0.1 cardinality restriction of exactly zero or one text widget is deliberately retired.
- No framework-independence, ownership, read-only HTTP, UUID, or revision-not-identity guarantee is retired.

## Successor Clause Manifest

The following material clauses are intentionally test-governed by `test_002_decision_record_set`:

| Clause ID | Required outcome |
|---|---|
| `adr006_framework_independence` | One authoritative model remains free of egui, axum, filesystem, and browser dependencies. |
| `adr006_mutation_ownership` | UI mutations use domain/store operations and HTTP remains read-only. |
| `adr006_uuid_semantics` | Overlay and widget UUID v4 identities persist unchanged and are not derived from mutable values. |
| `adr006_revision_not_identity` | Revisions are delivery metadata, not identity or persisted content. |
| `adr006_ordering` | Index 0 is frontmost, mutations preserve explicit order, reverse drawing and front-first hit-testing. |
| `adr006_font_delivery` | Stable font IDs use identical embedded TTF bytes and base64 data URLs without new routes. |

Removal or material mutation of any manifested outcome must fail the focused documentation contract test.

## Alternatives Considered

- **Separate editor and browser models:** Rejected because duplicated authoritative state can diverge and complicates live updates and history restoration.
- **Name- or index-derived identity:** Rejected because renaming and reordering would break persistence, selection, and stable browser URLs.
- **A generic plugin hierarchy:** Rejected because 0.0.2 needs only text widgets and speculative extensibility would obscure the model.
- **System fonts or network font fetches:** Rejected because output must remain offline and consistent across native preview and browser environments.
- **New font asset HTTP routes:** Rejected because base64 data URLs preserve ADR-004's small, stable same-origin route contract.

## Consequences

### Positive

- All projections share deterministic identity, ordering, and content semantics.
- Session-only state cannot accidentally become saved or visible in browser output.
- Bundled font selection is durable, offline, and independent of installed fonts.
- Existing browser routes and one-way mutation authority remain unchanged.

### Negative

- Ordered collection operations, ID validation, and adapter projections require broader tests.
- Base64 font embedding enlarges generated HTML and retransmits font bytes on document load.
- Identical assets do not guarantee identical line breaks or metrics between native and browser renderers.
- Binary glyph and replacement-character coverage remains a required issue #26 verification, not a completed result of this record.

## Related

- **ADRs:** [ADR-004: Serve Stable Loopback URLs and Push Complete Snapshots with SSE](ADR-004-loopback-sse-browser-delivery.md), [ADR-007: Persist Version-2 Overlay Documents](ADR-007-version-2-overlay-document-persistence.md), [ADR-008: Keep Workspace History in the Coordinator](ADR-008-coordinator-owned-workspace-history.md)
- **FDRs:** [FDR-003: Multi-Widget Composition Workspace](../fdr/FDR-003-multi-widget-composition-workspace.md), [FDR-004: Bundled Offline Text Fonts](../fdr/FDR-004-bundled-offline-text-fonts.md)
