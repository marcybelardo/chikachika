# Architecture Decision Records

Architecture Decision Records (ADRs) preserve cross-cutting technical decisions and their rationale. Accepted records are append-only: when a decision changes, create a new ADR that supersedes the earlier record rather than editing history.

Use [the ADR skill](../../.agents/skills/adr/SKILL.md) to create or supersede records and [the ADR review skill](../../.agents/skills/adr-review/SKILL.md) to audit them.

## Records

| Record | Decision | Status | Date |
|---|---|---|---|
| [ADR-001](ADR-001-one-native-process-gui-and-server.md) | Run the GUI and Local Web Server in One Native Process | Accepted | 2026-08-26 |
| [ADR-002](ADR-002-shared-overlay-model-and-stable-ids.md) | Use One Framework-Independent Overlay Model with Stable Opaque IDs | [Superseded by ADR-006](ADR-006-ordered-authoritative-widget-model.md) | 2026-08-26 |
| [ADR-003](ADR-003-versioned-app-local-json-persistence.md) | Persist a Versioned JSON Envelope in the Platform App-Local Data Directory | [Superseded by ADR-007](ADR-007-version-2-overlay-document-persistence.md) | 2026-08-26 |
| [ADR-004](ADR-004-loopback-sse-browser-delivery.md) | Serve Stable Loopback URLs and Push Complete Snapshots with SSE | Accepted | 2026-08-26 |
| [ADR-005](ADR-005-separate-server-settings-from-overlay-documents.md) | Separate Versioned Server Settings from Overlay Documents | Accepted | 2026-08-29 |
| [ADR-006](ADR-006-ordered-authoritative-widget-model.md) | Use an Ordered Authoritative Widget Model | Accepted | 2026-09-17 |
| [ADR-007](ADR-007-version-2-overlay-document-persistence.md) | Persist Version-2 Overlay Documents | Accepted | 2026-09-17 |
| [ADR-008](ADR-008-coordinator-owned-workspace-history.md) | Keep Workspace History in the Coordinator | Accepted | 2026-09-17 |
| [ADR-009](ADR-009-secondary-native-settings-viewport-lifecycle.md) | Use a Secondary Native Settings Viewport | Accepted | 2026-09-17 |
