# FDR-004: Bundled Offline Text Fonts

**Status:** Accepted
**Date:** 2026-09-17
**Supersedes:** None

## Overview

The 0.0.2 text inspector offers two bundled Regular font families that work offline in native preview and browser output. This record fixes their identities, exact upstream assets, licensing obligations, bounded character requirement, and deferred binary/runtime verification.

## User-visible Behavior

- Users select `Noto Sans` or `JetBrains Mono` for each text widget; `Noto Sans` is the default.
- Font selection persists and publishes live to the browser output.
- Native preview and browser output use the same bundled static TTF bytes without network fetches, installed-system fonts, or arbitrary imports.
- Original text remains stored unchanged. Unsupported characters render as U+FFFD from the selected bundled face, never a platform fallback.
- Exact line breaks and metrics may differ between native and browser renderers; browser output remains authoritative.

## Feature Decisions

### 1. Bundle exactly two Regular static families

**Decision:** The baseline font IDs are `noto-sans` and `jetbrains-mono`, displayed as Noto Sans and JetBrains Mono. `noto-sans` is the default. Only Regular static TTF faces are included; bold, italic, variable-axis configuration, arbitrary imports, and additional families are excluded.

**Why:** One proportional and one monospaced family cover common overlay text while keeping assets, controls, and verification bounded. egui 0.29 `FontData` supports TTF/OTF, so static TTF avoids variable-axis configuration.

**Tradeoff:** Users do not receive style variants or broad font customization in this milestone.

### 2. Pin exact upstream assets and provenance

**Decision:** Planned assets are pinned exactly as follows. Issue #26 must acquire these bytes, record SHA-256 values, and verify byte counts before integration.

| Font ID | Upstream revision and path | Expected bytes | Asset URL | License URL |
|---|---|---:|---|---|
| `noto-sans` | commit `ffebf8c1ee449e544955a7e813c54f9b73848eac`, `hinted/ttf/NotoSans/NotoSans-Regular.ttf` | 569208 | https://raw.githubusercontent.com/notofonts/noto-fonts/ffebf8c1ee449e544955a7e813c54f9b73848eac/hinted/ttf/NotoSans/NotoSans-Regular.ttf | https://raw.githubusercontent.com/notofonts/noto-fonts/ffebf8c1ee449e544955a7e813c54f9b73848eac/LICENSE |
| `jetbrains-mono` | v2.304 commit `cd5227bd1f61dff3bbd6c814ceaf7ffd95e947d9`, `fonts/ttf/JetBrainsMono-Regular.ttf` | 273900 | https://raw.githubusercontent.com/JetBrains/JetBrainsMono/cd5227bd1f61dff3bbd6c814ceaf7ffd95e947d9/fonts/ttf/JetBrainsMono-Regular.ttf | https://raw.githubusercontent.com/JetBrains/JetBrainsMono/cd5227bd1f61dff3bbd6c814ceaf7ffd95e947d9/OFL.txt |

**Why:** Immutable revisions, paths, URLs, and expected lengths make acquisition reproducible and reviewable.

**Tradeoff:** Any upstream correction requires a new decision rather than silently changing bytes.

### 3. Meet SIL OFL 1.1 obligations without modifying fonts

**Decision:** Both assets are governed by SIL Open Font License 1.1. Distribution must include complete applicable copyright and license notices and recorded provenance. This baseline does not modify or subset the fonts. Issue #26 adds assets and notices; issue #21 does not.

**Why:** Explicit obligations prevent bundled binaries from losing their legal and provenance context.

**Tradeoff:** Packaging must carry notices and acquisition evidence alongside the assets.

### 4. Require bounded Latin-focused coverage and replacement behavior

**Decision:** Each exact pinned face must support U+0020–U+007E and U+00A0–U+00FF. NBSP and soft hyphen retain their text-layout semantics; LF provides multiline structure rather than requiring a visible glyph. Each face must also contain a visibly rendered U+FFFD. There is no CJK, emoji, or universal-script guarantee. Unsupported characters preserve their original document text but render as U+FFFD from the selected bundled face.

**Why:** A bounded requirement states what common Latin text must work and gives unsupported input deterministic behavior without falsely promising universal coverage.

**Tradeoff:** Some scripts and emoji render as replacement characters, and layout semantics mean not every covered code point appears as a visible mark.

### 5. Defer binary and renderer validation to issue #26

**Decision:** Coverage is a requirement, not a completed font audit. Issue #26 must validate the exact decoded bytes, SHA-256, byte counts, cmap coverage including visible U+FFFD, complete notices, and native/browser rendering. Google family subset metadata is not proof of per-file coverage. If either pinned face fails, issue #26 must obtain a revised font decision rather than use a system fallback or unapproved asset. Issue #27 performs combined OBS/platform verification.

**Why:** Documentation can select a reproducible source but cannot truthfully claim binary or renderer properties before assets are acquired and exercised.

**Tradeoff:** Font integration remains blocked on explicit verification, and acceptance of this record is not acceptance of runtime behavior.

### 6. Deliver fonts self-contained in both renderers

**Decision:** Native preview embeds each exact TTF; generated HTML/CSS embeds the identical bytes as base64 data URLs in `@font-face`. No font HTTP routes, network fetches, or system-font dependencies are introduced. The browser output is HTML/JavaScript, not an eframe WASM application.

**Why:** Self-contained delivery keeps overlays offline and preserves the existing two-route server contract.

**Tradeoff:** Base64 increases HTML size and retransmits fonts on document load.

## Font Verification Scenarios

| Scenario ID | Verification action | Required outcome |
|---|---|---|
| `font_asset_identity` | Acquire each pinned URL at its exact commit. | Byte length matches 569208/273900 and recorded SHA-256 identifies the bundled/native/browser bytes. |
| `font_latin_coverage` | Inspect cmap and render required ranges. | U+0020–007E and U+00A0–00FF meet the requirement with NBSP/soft-hyphen semantics documented. |
| `font_replacement_glyph` | Inspect and render U+FFFD in each face. | Each selected bundled face visibly renders its own replacement glyph; no platform fallback occurs. |
| `font_native_browser_bytes` | Decode browser data URLs and compare to native assets. | Bytes are exact matches and browser delivery uses CSS `@font-face` data URLs. |
| `font_license_notice` | Inspect packaged notices and provenance. | Complete SIL OFL 1.1 copyright/license notices and source revisions are included. |
| `font_unsupported_text` | Enter unsupported text. | Original input is preserved while rendering uses U+FFFD; no CJK/emoji guarantee is implied. |

## Open Questions

None for the selected baseline. Binary coverage, decoded-byte equality, notices, and renderer behavior are mandatory deferred verification under issue #26, with combined platform/OBS verification under issue #27.

## Related

- **ADRs:** [ADR-004: Serve Stable Loopback URLs and Push Complete Snapshots with SSE](../adr/ADR-004-loopback-sse-browser-delivery.md), [ADR-006: Use an Ordered Authoritative Widget Model](../adr/ADR-006-ordered-authoritative-widget-model.md), [ADR-007: Persist Version-2 Overlay Documents](../adr/ADR-007-version-2-overlay-document-persistence.md)
- **FDRs:** [FDR-003: Multi-Widget Composition Workspace](FDR-003-multi-widget-composition-workspace.md)
