# FDR-001: Overlay Editing and Local Browser Source

**Status:** Accepted
**Date:** 2026-08-26
**Supersedes:** None

## Overview

This feature gives streamers a small local-first desktop workflow for creating and editing a basic overlay, then using its exact browser output as a transparent OBS browser source. The decision governs the user-visible 0.0.1 behavior without prescribing implementation technology.

## User-visible Behavior

A user can create, name, rename, and explicitly confirm deletion of an overlay. The overlay can be edited in a fixed canvas preview, saved locally, and opened at a stable browser-source URL. A connected browser output reflects supported editor changes without a manual refresh. Errors remain visible and non-destructive.

## Feature Decisions

### 1. Local-first vertical slice and exclusions

**Decision:** The 0.0.1 workflow is local-first: it supports basic overlay authoring, local saving, a local browser-source output, and OBS use. It does not include accounts, cloud storage, collaboration, synchronization, streaming-service integrations, general OBS control, or LAN/internet exposure.

**Why:** A focused local workflow gives a streamer a complete useful path while keeping the first release understandable and testable.

**Tradeoff:** Users cannot share or synchronize overlays and must wait for later product decisions for integrations and broader stream-management features.

### 2. Overlay lifecycle and durable identity

**Decision:** Users can create and rename overlays and can delete one only after an explicit confirmation. Each overlay keeps its identity and browser-source URL across rename and application restart.

**Why:** Names are user-facing labels, while durable identity prevents a rename or restart from breaking a configured browser source.

**Tradeoff:** Deletion requires an extra step, and stable links require the product to retain identity even when presentation details change.

### 3. Fixed canvas and preview authority

**Decision:** Every overlay uses a fixed, explicitly configured canvas, and the transparent browser output is authoritative when it differs from the editor preview.

**Why:** A declared canvas makes layout predictable in OBS, while browser authority reflects the output streamers actually use.

**Tradeoff:** Users cannot resize the canvas freely in this release, and the editor preview is useful but not the final rendering authority.

### 4. One optional text widget

**Decision:** An overlay may contain exactly zero or one optional text widget. When present, it has editable content, position, font size, color, and alignment. Multiple widgets, layering, canvas resizing, rich text, and animation are deferred.

**Why:** One text widget covers the smallest meaningful editing workflow and makes the supported behavior explicit.

**Tradeoff:** Users cannot compose richer layouts or animated scenes until a later decision expands the model.

### 5. Exact preview access and live browser updates

**Decision:** The application lets users copy an overlay’s browser-source URL and open the exact browser output for preview or troubleshooting. A connected browser source receives supported editor changes without a manual refresh.

**Why:** Copying and opening the exact output removes setup ambiguity, while live updates make iteration practical during a stream.

**Tradeoff:** The workflow depends on a connected local browser session; disconnected viewers do not receive historical updates until they reconnect.

### 6. Visible, non-destructive failure handling

**Decision:** Malformed or unsupported saved data, persistence errors, and server startup or port conflicts are shown clearly without silently discarding user data or pretending the output is current. The user can recover, retry, or continue with an unaffected overlay where possible.

**Why:** Honest visible failure preserves trust and gives users a chance to protect or repair their work.

**Tradeoff:** Some workflows stop or require intervention instead of automatically choosing a potentially destructive fallback.

### 7. 0.0.1 platform gate

**Decision:** macOS and Linux are required 0.0.1 targets. Support for other platforms may be considered later and is not a release gate for this milestone.

**Why:** Requiring both target platforms keeps the release promise explicit while matching the project’s immediate validation capacity.

**Tradeoff:** Portability beyond macOS and Linux receives no 0.0.1 completion guarantee.

## Open Questions

- None for the 0.0.1 behavior covered here.

## Related

- **ADRs:** None
- **FDRs:** None
