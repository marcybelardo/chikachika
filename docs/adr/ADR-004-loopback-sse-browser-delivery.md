# ADR-004: Serve Stable Loopback URLs and Push Complete Snapshots with SSE

**Status:** Accepted
**Date:** 2026-08-26
**Supersedes:** None

## Context

OBS needs a stable local browser-source URL, while a connected browser must receive supported editor changes without polling or manual refresh. The 0.0.1 server is intentionally local-only and one-way: the native editor is the mutation authority, and browser clients consume the exact output and its updates. Delivery must recover from reconnects and slow consumers without promising an unbounded event history.

## Decision

- Production binds to `127.0.0.1` only and does not expose LAN or internet interfaces in 0.0.1.
- The deterministic default browser-source URL is exactly `http://127.0.0.1:51737/overlay/{overlay-id}`.
- Port 51737 is in the IANA dynamic/private range and can still be occupied by another process.
- An explicitly configured port may intentionally change the default port.
- The selected port is persisted and documented so copied URLs remain stable.
- `127.0.0.1:0` is used only by tests.
- An occupied configured or default port fails visibly instead of silently changing a copied URL.
- The exact same-origin routes are `GET /overlay/{id}` and `GET /overlay/{id}/events`.
- `GET /overlay/{id}` serves the exact browser output.
- Browser delivery uses the browser-native SSE `EventSource` API.
- SSE updates use named JSON events.
- A client subscribes to the events route before it receives the initial snapshot.
- The server sends one complete current snapshot with a monotonically increasing revision and then sends complete replacements after mutations.
- Update delivery is bounded so a slow client cannot require unbounded queued history.
- A reconnect receives the current state rather than depending on historical replay.
- A lagging client recovers from the latest complete snapshot.
- The events stream sends periodic keepalive comments.
- The server makes no promise of historical event replay.
- There are no browser-to-application mutation routes.
- No CORS configuration is required for same-origin assets and events.

## Rationale

Loopback-only binding honors the local-first and no-LAN product boundary. A deterministic default and persisted explicit choice make copied OBS URLs useful, while visible conflicts avoid silently breaking them. SSE and complete snapshots fit one-way browser delivery, allow native reconnect behavior, and make bounded lag recovery straightforward without maintaining a replay log.

## Alternatives Considered

- Production ephemeral ports are rejected because copied browser-source URLs would not remain stable.
- Wildcard binding is rejected because 0.0.1 must not expose the local server to LAN or internet interfaces.
- Polling is rejected because it adds repeated requests and latency where the requirement is server-pushed updates.
- WebSockets are rejected for this one-way requirement and can be reconsidered only through a superseding ADR if bidirectional needs arise.

## Consequences

### Positive

- OBS and browser preview can use deterministic same-origin URLs without CORS configuration or a manual refresh loop.
- Complete revisioned snapshots simplify initial state, reconnect, and bounded lag recovery.
- Loopback binding and visible conflicts make the local security and URL contract explicit.

### Negative

- SSE is one-way and cannot support browser-originated edits without a later decision and mutation protocol.
- Bounded delivery intentionally drops intermediate history for slow or disconnected clients.
- A fixed default port can conflict with another process, and explicit port persistence adds configuration state.

## Related

- **ADRs:** None
- **FDRs:** [FDR-001: Overlay Editing and Local Browser Source](../fdr/FDR-001-overlay-editing-and-local-browser-source.md)
