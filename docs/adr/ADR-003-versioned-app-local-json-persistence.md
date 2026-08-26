# ADR-003: Persist a Versioned JSON Envelope in the Platform App-Local Data Directory

**Status:** Accepted
**Date:** 2026-08-26
**Supersedes:** None

## Context

The local-first workflow needs durable overlays without treating a working directory or an operating-system settings directory as the user document store. Persisted data may outlive the current implementation, so its format and failure behavior must be explicit. Saving must not expose a partially written document or silently replace usable in-memory work when the filesystem rejects an operation.

## Decision

- Persistence uses a strongly typed Serde JSON envelope.
- The JSON envelope has an explicit top-level format version and an overlays collection.
- Path resolution uses `directories::ProjectDirs` with the unqualified `Chikachika` identity and `data_local_dir`.
- Exact platform-resolved paths are surfaced in implementation and documentation and are covered on macOS and Linux.
- Path resolution is fallible and its failure is reported to the user.
- Application-local directories are created explicitly before persistence operations.
- Persistence never silently falls back to the current working directory.
- Unsupported format versions and malformed data are rejected visibly and non-destructively.
- A source file is never overwritten when loading it fails.
- Saving clones a complete document snapshot and performs file I/O outside the model lock.
- Saving writes a temporary file in the same directory as the source.
- Saving replaces the source with an atomic or platform-safe replacement operation.
- A failed save leaves the in-memory document dirty and exposes the save error.
- When implementation versions are selected, dependency APIs and replacement guarantees are verified against those versions.

## Rationale

A typed, versioned envelope makes compatibility checks deliberate and keeps persisted documents inspectable. ProjectDirs expresses platform conventions without hand-built home paths, while visible fallible resolution avoids pretending data is safe when no location is available. Snapshotting outside the lock limits contention, and same-directory replacement protects readers from incomplete writes. Keeping failed loads and saves non-destructive follows the product’s explicit error-handling contract.

## Alternatives Considered

- Unversioned JSON is rejected because incompatible documents cannot be identified deliberately.
- Opaque binary persistence is rejected because it is harder to inspect, diagnose, and evolve for this small local document.
- Operating-system config directories for user-created overlay data are rejected because configuration and user documents have different ownership and lifecycle expectations.
- Manually constructed home-directory paths are rejected because they bypass platform-specific app-local conventions and edge cases.
- iroh is rejected because peer-to-peer transport is outside the local persistence requirement.
- Steam is rejected because distribution or account services are outside the local persistence requirement.
- Cloud synchronization is rejected because accounts, remote storage, and synchronization are outside the 0.0.1 local-first scope.

## Consequences

### Positive

- Users get inspectable, version-checkable documents in the platform-appropriate local data location.
- Load and save failures preserve both the source file and the in-memory work for recovery.
- Atomic replacement and lock-free file I/O reduce partial-write and model-contention risks.

### Negative

- Platform path behavior and dependency guarantees require macOS/Linux tests and documentation rather than one universal hard-coded path.
- Temporary files and replacement semantics add implementation and cleanup cases.
- A failed save leaves a dirty document that requires user-visible retry or recovery instead of silently claiming success.

## Related

- **ADRs:** None
- **FDRs:** [FDR-001: Overlay Editing and Local Browser Source](../fdr/FDR-001-overlay-editing-and-local-browser-source.md)
