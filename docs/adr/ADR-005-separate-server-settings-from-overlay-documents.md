# ADR-005: Separate Versioned Server Settings from Overlay Documents

**Status:** Accepted
**Date:** 2026-08-29
**Supersedes:** None

## Context

The overlay collection is user-created document data, while the loopback server port is application configuration with a different lifecycle and failure surface. The existing `overlays.json` document store lives in the platform app-local data directory. Issue #8 adds a configurable loopback port, so the application needs an explicit settings boundary that does not mix server configuration into overlay documents or silently choose a different endpoint when configuration is unusable.

The issue #8 implementation follows this contract: settings are loaded before server startup, exposed to the native GUI, and saved independently from overlay documents.

## Decision

- Overlay documents remain in the versioned `overlays.json` envelope under the platform app-local data directory selected by `directories::ProjectDirs`.
- Server settings are a separate versioned `settings.json` envelope under the platform config location selected by the same `ProjectDirs` identity. The settings envelope stores the loopback server port for 0.0.1.
- A missing settings file means the loopback endpoint is `127.0.0.1:51737`.
- A configured port is valid only in the inclusive range `1..=65535`; production binding remains loopback-only.
- Malformed or unsupported settings are reported visibly and non-destructively, leave the source file unchanged, and prevent server startup. The application does not silently fall back to the default or choose an alternate port in this case.
- A port change is saved for the next application launch. The running server is not live-rebound, and the current endpoint remains unchanged until restart.
- The native application may copy or open only the exact selected overlay URL after server readiness has been reported. It must not synthesize an action URL from an unready or fallback endpoint.

## Rationale

Separating settings from overlay documents keeps user content and application configuration independently owned, versioned, and recoverable. A stable default preserves the existing browser-source setup, while strict validation and visible startup failure avoid making a copied URL point at an unexpected server. Deferring a changed port to the next launch keeps the server lifecycle simple and makes the endpoint used by the current browser source unambiguous. Readiness-gated URL actions preserve the same guarantee at the UI boundary.

## Alternatives Considered

- Storing the server port inside `overlays.json` is rejected because server configuration would become coupled to user-created document data and overlay save operations.
- Storing `settings.json` beside the overlay documents in the app-local data directory is rejected because application configuration and user documents have different ownership and lifecycle semantics.
- Falling back to `127.0.0.1:51737` for malformed or unsupported settings is rejected because it hides an unusable configuration and can make a copied URL refer to the wrong server.
- Live-rebinding the current server when the port changes is rejected because it complicates lifecycle coordination and can invalidate an active browser-source URL without an application restart.
- Automatically selecting an alternate port after a bind conflict is rejected because it silently changes the endpoint users configure in OBS.

## Consequences

### Positive

- Overlay documents and server settings have clear, independent persistence ownership and failure handling.
- Missing settings preserve a deterministic loopback URL, while valid explicit ports can be retained across launches.
- Strict validation, visible errors, and readiness-gated URL actions prevent silent endpoint drift.

### Negative

- Users and support tooling must understand two versioned files and two platform-resolved locations.
- A changed port requires an application restart before the running server uses it.
- A malformed or unsupported settings file blocks server startup and requires repair rather than automatic recovery.
- A configured port can still be occupied by another process; no alternate port is selected automatically.

## Related

- **ADRs:** [ADR-003: Persist a Versioned JSON Envelope in the Platform App-Local Data Directory](ADR-003-versioned-app-local-json-persistence.md), [ADR-004: Serve Stable Loopback URLs and Push Complete Snapshots with SSE](ADR-004-loopback-sse-browser-delivery.md)
- **FDRs:** [FDR-001: Overlay Editing and Local Browser Source](../fdr/FDR-001-overlay-editing-and-local-browser-source.md), [FDR-002: Browser-Source URL Actions and Port Settings](../fdr/FDR-002-browser-source-url-actions-and-port-settings.md)
