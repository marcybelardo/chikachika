# `0.0.1` Milestone

**Status:** Planning

`0.0.1` is the first functional vertical slice of the application. It remains an unreleased pre-release target: implementation and architecture may change substantially while this checklist is in progress.

This document tracks milestone scope and completion. It does not replace Feature Decision Records (FDRs) for feature behavior or Architecture Decision Records (ADRs) for architectural rationale.

## Outcome

A streamer can create a basic text overlay in the desktop application, save it locally, serve it at a stable local URL, and use that URL as a transparent browser source in OBS. Editor changes reach the browser source without requiring a manual refresh.

## Product Requirements

### Application and overlay management

- [ ] The desktop application opens successfully on a supported development platform.
- [ ] A user can create an overlay.
- [ ] A user can rename an overlay.
- [ ] A user can delete an overlay with protection against accidental deletion.
- [ ] Each overlay has a stable identity and browser-source URL that survive application restarts.

### Canvas and text editing

- [ ] An overlay uses an explicitly configured fixed-size canvas.
- [ ] A user can add a text widget to an overlay.
- [ ] A user can select and move the text widget on the canvas.
- [ ] A user can edit the widget's text content.
- [ ] A user can configure the text's font size, color, and alignment.
- [ ] The editor provides a useful visual preview of the overlay.

### Local persistence

- [ ] Overlays and their supported settings are saved locally.
- [ ] Saved overlays are restored after restarting the application.
- [ ] Persisted data has an explicit format version so incompatible pre-release changes can be detected and handled deliberately.

### Browser-source hosting

- [ ] The application serves each overlay over HTTP at its stable local URL.
- [ ] The browser output has a transparent background and respects the overlay's configured canvas dimensions.
- [ ] Changes made in the editor appear in a connected browser source without a manual page refresh.
- [ ] The application provides a straightforward way to copy the browser-source URL.
- [ ] The application provides a straightforward way to open the exact browser output for preview or troubleshooting.
- [ ] The local server binds only to the loopback interface by default.

### OBS verification

- [ ] A served overlay can be added to OBS as a browser source using the displayed URL.
- [ ] Text content and supported styling render correctly in OBS.
- [ ] Transparency works in OBS.
- [ ] Live editor changes appear in OBS without recreating or manually refreshing the browser source.

## Quality Requirements

- [ ] Automated tests cover the persisted overlay model and its round-trip behavior.
- [ ] Automated tests cover the browser representation or update protocol where practical.
- [ ] The application handles malformed or unsupported persisted data without silently losing user data.
- [ ] Idle CPU and memory behavior are measured in a representative development build and any material concerns are recorded.
- [ ] Setup, run, test, and OBS connection instructions are documented.
- [ ] The glossary, architecture inventory, ADRs, and FDRs are current for the implemented vertical slice.

## Explicitly Out of Scope

- Twitch or other streaming-service integrations
- Chatbots, moderation, analytics, or general stream management
- Cloud accounts, required cloud hosting, collaboration, or synchronization
- Plugin systems or marketplaces
- Multiple widget types beyond text
- Rich animation and trigger systems
- General OBS scene control
- LAN or internet exposure of the local overlay server

## Completion Gate

`0.0.1` is complete when every in-scope checkbox is satisfied, the OBS workflow has been exercised end to end, relevant tests pass, and current documentation matches the implementation. Any deferred requirement must be explicitly removed from this milestone through an accepted product decision rather than left implicitly incomplete.
