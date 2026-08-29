# FDR-002: Browser-Source URL Actions and Port Settings

**Status:** Accepted
**Date:** 2026-08-29
**Supersedes:** None

## Overview

Issue #8 completes the local browser-source workflow by giving a user direct copy and open actions for the selected overlay URL and a way to configure the loopback server port. It extends the behavior in [FDR-001](FDR-001-overlay-editing-and-local-browser-source.md) with explicit endpoint readiness, validation, and restart semantics. The issue #8 implementation is pending in the baseline where this record is introduced; this record is the accepted user-visible contract for that implementation.

## User-visible Behavior

- The selected overlay’s browser-source URL is shown and can be copied or opened only after the local server is ready and that overlay is selected and registered.
- Copy and open actions use the exact URL shown for the selected overlay. They are unavailable when readiness or selection is missing.
- The user can configure the loopback server port. A port is valid only from `1` through `65535`.
- When no settings have been saved, the server uses `127.0.0.1:51737`.
- A changed port is saved for the next launch; the running server keeps its current endpoint until the application restarts.
- Malformed or unsupported settings are shown as an error without replacing the settings source, and the server does not start until the source is repaired. The application does not select an alternate port automatically.

## Feature Decisions

### 1. Exact readiness-gated URL actions

**Decision:** Copy and open actions operate on the exact selected browser-source URL only after the server reports readiness and the selected overlay is registered.

**Why:** The URL is useful only when it identifies a live, served overlay, and using the displayed value keeps copy, open, preview, and OBS setup consistent.

**Tradeoff:** The actions are unavailable during startup, after a server failure, or when no overlay is selected.

### 2. Loopback port configuration

**Decision:** The user can choose the loopback server port, with valid values limited to `1..=65535`. A missing setting uses `127.0.0.1:51737`.

**Why:** The default preserves a predictable first-run URL while allowing users to resolve a deliberate local port choice.

**Tradeoff:** Users must update external browser-source configuration when they intentionally choose a different port.

### 3. Restart-required port changes

**Decision:** A changed port is persisted for the next launch and does not live-rebind the current server.

**Why:** Keeping one endpoint for the lifetime of a launch makes readiness, displayed URLs, and active browser sources unambiguous and keeps server lifecycle coordination small.

**Tradeoff:** The user must restart the application before a saved port change takes effect.

### 4. Visible settings failure and no alternate port

**Decision:** Malformed or unsupported settings remain visible as a non-destructive error and prevent server startup; a bind conflict also remains visible, and no automatic alternate port is selected.

**Why:** Silent fallback or endpoint substitution can break OBS configuration and conceal a damaged or incompatible settings source.

**Tradeoff:** The user must repair the source or resolve the conflict before the local browser-source server becomes available.

## Open Questions

None for the issue #8 behavior covered here.

## Related

- **ADRs:** [ADR-005: Separate Versioned Server Settings from Overlay Documents](../adr/ADR-005-separate-server-settings-from-overlay-documents.md), [ADR-004: Serve Stable Loopback URLs and Push Complete Snapshots with SSE](../adr/ADR-004-loopback-sse-browser-delivery.md)
- **FDRs:** [FDR-001: Overlay Editing and Local Browser Source](FDR-001-overlay-editing-and-local-browser-source.md)
