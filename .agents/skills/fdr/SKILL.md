---
description: Create and supersede append-only Feature Decision Records covering user-visible behavior and feature-specific choices.
---
# Feature Decision Records

Use this skill when proposing, accepting, locating, or superseding a decision about a feature’s user-visible behavior or feature-specific design.

## Documentation boundary

- FDRs record what a feature does, who it serves, non-obvious product/design decisions, rationale, tradeoffs, and open questions at decision time.
- Cross-cutting technical choices belong in ADRs.
- Current technical composition and ownership belong in `docs/architecture/`.
- Canonical terms belong in `docs/GLOSSARY.md`.
- An FDR is not a code walkthrough, file inventory, implementation guide, API dump, or changelog.

## Canonical location

```text
docs/fdr/
├── INDEX.md
└── FDR-NNN-kebab-case-slug.md
```

Read `docs/fdr/INDEX.md` before reading or creating individual records.

## Append-only policy

An accepted FDR is an immutable historical decision record. Do not rewrite, rename, delete, or renumber it when a feature evolves. Create a new FDR that supersedes it so earlier behavior and reasoning remain available. Record supersession in the living `INDEX.md`; do not modify the old FDR.

A draft may be revised before acceptance. The index is a living navigation document and may be updated.

## Numbering and filenames

Use `FDR-{NNN}-{kebab-case-slug}.md`.

- `NNN` is the next unused zero-padded three-digit number.
- Never reuse a number.
- Keep the slug short and feature-oriented.

## Workflow

### Create a record

1. Read `docs/fdr/INDEX.md`, relevant FDRs, and any applicable ADRs.
2. Research current product language, behavior, and constraints in docs and code.
3. Confirm the scope is feature-specific.
4. Determine the next unused number and draft using the template.
5. Confirm number, title, scope, and acceptance with the user unless already explicit.
6. Add or update the index row.
7. Cite applicable ADRs and sibling FDRs without restating them.
8. Add or update glossary entries only through the glossary workflow.
9. Verify filenames, links, dates, statuses, and cross-references.

### Supersede a record

1. Create a new FDR describing the newly decided behavior and choices.
2. Set `Supersedes` and explain the changed feature context.
3. Mark the earlier index row `Superseded by FDR-NNN`.
4. Do not edit the earlier FDR.
5. Check related ADRs, FDRs, glossary terms, and current architecture documentation.

## Record template

```markdown
# FDR-NNN: Feature Name

**Status:** Proposed
**Date:** YYYY-MM-DD
**Supersedes:** None

## Overview

Explain what the feature is, who uses it, and why it exists.

## User-visible Behavior

- Describe behavior without implementation detail.

## Feature Decisions

### 1. Short decision title

**Decision:** What was chosen.

**Why:** Why this choice supports users and product goals.

**Tradeoff:** What this choice costs or excludes.

## Open Questions

- Deliberately unresolved questions at decision time, or `None`.

## Related

- **ADRs:** None
- **FDRs:** None
```

Feature decisions must be numbered and contain `Decision`, `Why`, and `Tradeoff`. Mention permissions or access constraints only when user-visible. Use `None` for required metadata with no value.

## Status values

- `Proposed` — drafted but not accepted.
- `Accepted` — approved decision governing planned or current work.
- `Implemented` — accepted behavior is present and supported.
- `Superseded by FDR-NNN` — index-only status for an earlier record.
- `Retired` — retained historical record for a deliberately removed feature.

## Index format

```markdown
| Record | Feature | Status | Date |
|---|---|---|---|
| [FDR-001](FDR-001-example.md) | Example feature | Accepted | YYYY-MM-DD |
```

Every FDR file must appear exactly once. The index, not historical file edits, reflects later status transitions and supersession.

## Audit mode

When asked to audit, default to report-only:

- Verify behavior against current code and user documentation.
- Check decision and rationale clarity.
- Validate citations, vocabulary, index entries, and supersession chains.
- Separate implementation drift from an intentional but undocumented product change.
- Propose a new superseding FDR rather than changes to an accepted file.

Apply changes only when explicitly requested.
