# Feature Decision Records

Feature Decision Records (FDRs) preserve decisions about user-visible behavior and feature-specific design. Accepted records are append-only: when a feature decision changes, create a new FDR that supersedes the earlier record rather than editing history.

Use [the FDR skill](../../.agents/skills/fdr/SKILL.md) to create, audit, or supersede records.

## Records

| Record | Feature | Status | Date |
|---|---|---|---|
| [FDR-001](FDR-001-overlay-editing-and-local-browser-source.md) | Overlay Editing and Local Browser Source | [Superseded by FDR-003](FDR-003-multi-widget-composition-workspace.md) | 2026-08-26 |
| [FDR-002](FDR-002-browser-source-url-actions-and-port-settings.md) | Browser-Source URL Actions and Port Settings | Accepted | 2026-08-29 |
| [FDR-003](FDR-003-multi-widget-composition-workspace.md) | Multi-Widget Composition Workspace | Accepted | 2026-09-17 |
| [FDR-004](FDR-004-bundled-offline-text-fonts.md) | Bundled Offline Text Fonts | Accepted | 2026-09-17 |
| [FDR-005](FDR-005-singleton-native-settings-window.md) | Singleton Native Settings Window | Accepted | 2026-09-17 |
