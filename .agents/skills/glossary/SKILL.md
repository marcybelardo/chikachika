---
description: Look up, add, and audit canonical project terminology in the glossary and link terms to their owning decisions.
---
# Glossary Maintenance

Use this skill to look up, add, rename, or audit project-specific terms in `docs/GLOSSARY.md`.

## Purpose and boundary

The glossary is the canonical naming surface for:

- Product nouns and visible UI concepts
- Contributor-specific architectural terms
- Project-specific meanings of otherwise common words
- Acronyms and recurring shorthand

It is not a tutorial, API reference, code symbol index, technology dictionary, implementation walkthrough, or changelog. Do not define generic terms unless their project-specific meaning is important.

Long explanations belong in FDRs, ADRs, or architecture inventory pages. Link to those documents rather than duplicating them.

## Canonical sources

- Glossary: `docs/GLOSSARY.md`
- Feature records: `docs/fdr/INDEX.md`
- Architecture records: `docs/adr/INDEX.md`
- Current architecture: `docs/architecture/INDEX.md`
- Repository guidance: root and path-specific `AGENTS.md` files

Read the entire glossary before editing it.

## Sections

Use these initial sections:

1. **Product** — concepts streamers encounter or use.
2. **UI** — visible application surfaces and editor controls.
3. **Architecture** — contributor-facing system concepts and boundaries.
4. **Documentation** — project governance and record terminology.

Add or rename sections only when the vocabulary demands it. Put each term in exactly one section based on its primary audience and cross-reference related terms rather than duplicating entries.

Within sections, order terms conceptually: foundational concepts before dependent concepts. Do not alphabetize merely for convenience.

## Entry format

```markdown
**Term** — Concise project-specific definition. Link the owning [FDR](fdr/FDR-NNN-slug.md), [ADR](adr/ADR-NNN-slug.md), or [inventory page](architecture/page.md) when one exists.
```

- Bold the canonical term.
- Expand acronyms on first mention.
- Keep entries to one sentence when practical and at most one short paragraph.
- Mention an alias in the canonical entry; do not create a duplicate entry.
- Definitions use present tense and describe current vocabulary, not naming history.

## Operating modes

### Look up

1. Search case-insensitively for each requested term and aliases.
2. Return matching entries verbatim with their section.
3. For absent terms, identify close matches and offer a proposed addition.

### Add or rename

1. Confirm there is no exact or near-duplicate.
2. Research usage in code, FDRs, ADRs, inventory, and repository guidance.
3. Draft the canonical term, section, concise definition, aliases, and links.
4. Ask for approval unless the terminology decision is already explicit.
5. Insert or rewrite the entry in conceptual order.
6. If code or docs conflict with approved terminology, report the drift; do not silently redefine the glossary to match accidental usage.

The glossary is a living current-state document, so approved entries may be rewritten or removed when terminology changes. Historical rationale remains in append-only ADRs/FDRs.

### Audit

Audit mode is report-only unless edits are explicitly requested.

1. Read the full glossary.
2. Validate every link and cited claim.
3. Scan ADR/FDR indexes, architecture inventory, repository guidance, and current code for recurring unexplained project language.
4. Report stale definitions, dead links, aliases/duplicates, misplaced entries, naming drift, and up to ten strong missing-term candidates.
5. Give each candidate a section, one-line draft, and likely owning document.

Strong candidates recur across sources, carry a project-specific meaning, represent an important distinction, or have caused naming ambiguity. Exclude ordinary library names, protocols, languages, function/type names, file paths, and concepts adequately defined by general references.

## Verification

After edits:

- Confirm each term appears once.
- Confirm conceptual ordering and section placement.
- Verify all relative links resolve.
- Check cited record identifiers and titles.
- Search touched terminology for obvious conflicts in current docs and code.
- Report what was checked and any follow-up naming drift.
