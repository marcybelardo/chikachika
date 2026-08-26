# ADR-002: Use One Framework-Independent Overlay Model with Stable Opaque IDs

**Status:** Accepted
**Date:** 2026-08-26
**Supersedes:** None

## Context

The editor and browser output must describe the same overlay while remaining independently testable and free of accidental framework coupling. Overlay names and presentation data change during normal editing, but browser-source URLs and persisted documents need durable identities across those changes and across restarts. The 0.0.1 feature deliberately supports one optional text widget rather than a general extension system.

## Decision

- There is exactly one authoritative, framework-independent domain model for an overlay document.
- The egui editor and browser output are adapters or projections of that model, not independent state stores.
- The domain model has no egui, axum, filesystem, or browser dependencies.
- Each overlay receives one generated opaque UUID v4 identity when it is created.
- The optional text widget receives one generated opaque UUID v4 identity when it is created.
- Generated overlay and widget identities are created once and persisted unchanged.
- Names, positions, timestamps, collection indexes, and hashes are never identity sources.
- 0.0.1 permits zero or one text widget and does not introduce a generic plugin or widget hierarchy.
- Revisions order browser snapshots but are not identities.
- UI mutations go through domain or store operations rather than directly changing adapter state.
- The HTTP adapter is read-only in 0.0.1.

## Rationale

A single small domain model gives both projections one source of truth and keeps product behavior independent of UI or transport libraries. Opaque UUIDs remain stable when user-facing or derived values change. Explicitly limiting cardinality and operations keeps the first implementation clear while preserving a safe basis for later decisions.

## Alternatives Considered

- Name-derived or collection-index-derived identity is rejected because renames, reordering, and edits would break durable references.
- Separate UI and server models are rejected because duplicated state can diverge and makes live updates harder to reason about.
- A speculative generic widget or plugin framework is rejected because 0.0.1 has one supported text widget and no plugin requirement.

## Consequences

### Positive

- Editor, server, persistence, and browser projections can be tested against one authoritative representation.
- Stable opaque identities preserve browser-source references across renames, restarts, reordering, and presentation changes.
- The deliberately narrow 0.0.1 model avoids premature extension APIs and makes mutation ownership explicit.

### Negative

- Adapters need conversion code and cannot maintain convenient private authoritative copies of document state.
- UUIDs are less readable than names and require explicit persistence and migration handling.
- Adding widget types, richer identity semantics, or browser mutations later requires a new decision and likely broader tests.

## Related

- **ADRs:** None
- **FDRs:** [FDR-001: Overlay Editing and Local Browser Source](../fdr/FDR-001-overlay-editing-and-local-browser-source.md)
