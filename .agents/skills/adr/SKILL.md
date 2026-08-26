---
description: Create and supersede append-only Architecture Decision Records and maintain the ADR index.
---
# Architecture Decision Records

Use this skill when proposing, accepting, locating, or superseding a cross-cutting architectural decision.

## Documentation boundary

- ADRs record cross-cutting technical choices, their context, rationale, alternatives, and consequences.
- Feature behavior and feature-specific choices belong in FDRs.
- Current architecture facts belong in `docs/architecture/`.
- Canonical project terminology belongs in `docs/GLOSSARY.md`.

Link to those sources rather than duplicating them.

## Canonical location

```text
docs/adr/
├── INDEX.md
└── ADR-NNN-kebab-case-slug.md
```

Read `docs/adr/INDEX.md` before reading or creating individual records.

## Append-only policy

An accepted ADR is an immutable historical record. Do not rewrite, rename, delete, or renumber it. Corrections and changed decisions require a new ADR that references and supersedes the earlier record. Record supersession in the living `INDEX.md`; do not edit the old ADR to add a supersession note.

A draft may be revised before acceptance. The index is a living navigation document and may be updated.

## Numbering and filenames

Use `ADR-{NNN}-{kebab-case-slug}.md`.

- `NNN` is the next unused zero-padded three-digit number.
- Never reuse a number, including one belonging to a superseded record.
- Keep the slug short and descriptive.

## Workflow

### Create a record

1. Read `docs/adr/INDEX.md` and relevant ADRs.
2. Inspect the current code, inventory, FDRs, and glossary as needed.
3. Confirm that the choice is architectural rather than feature-specific.
4. Determine the next unused number.
5. Draft the record from the template below. Use `Proposed` until the decision is accepted.
6. Ask for confirmation before changing the status to `Accepted`, unless acceptance is already explicit.
7. Add or update its index row.
8. Sweep `docs/fdr/INDEX.md` and relevant FDRs for relationships. Because accepted FDRs are also immutable, add an FDR citation only while it is a draft; otherwise create or propose a superseding FDR when the relationship materially changes its decision record.
9. Verify filenames, index links, dates, statuses, and cross-references.

### Supersede a record

1. Create a new ADR; never modify the earlier ADR.
2. Set `Supersedes` in the new ADR and explain what changed.
3. Mark the old row `Superseded by ADR-NNN` in `INDEX.md`.
4. Mark the new row with its current status.
5. Check related FDRs, inventory pages, and glossary terms for necessary follow-up.

## Record template

```markdown
# ADR-NNN: Title

**Status:** Proposed
**Date:** YYYY-MM-DD
**Supersedes:** None

## Context

Describe the architectural problem, constraints, and forces that require a decision.

## Decision

State the concrete architectural choice.

## Rationale

Explain why this option was selected.

## Alternatives Considered

- **Alternative:** Why it was not selected.

## Consequences

### Positive

- What becomes easier or possible.

### Negative

- Costs, constraints, risks, and maintenance burden.

## Related

- **ADRs:** None
- **FDRs:** None
```

Use `None` rather than leaving required metadata ambiguous. Omit no meaningful tradeoffs. Do not turn the record into a code walkthrough or implementation plan.

## Index format

```markdown
| Record | Decision | Status | Date |
|---|---|---|---|
| [ADR-001](ADR-001-example.md) | Example decision | Accepted | YYYY-MM-DD |
```

For an old record, status may read `Superseded by ADR-NNN` with a link to the successor in the index. Every ADR file must appear exactly once.
