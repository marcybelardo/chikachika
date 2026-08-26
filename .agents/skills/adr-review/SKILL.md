---
description: Audit all Architecture Decision Records for drift, contradictions, weak rationale, broken links, and missing supersession without rewriting history.
---
# ADR Review

Use this skill to audit the complete ADR set. Reviews are report-only unless the user explicitly asks to apply fixes.

## Ground rules

- Read `docs/adr/INDEX.md` first and use it to establish scope.
- Audit every indexed ADR and detect unindexed ADR files.
- Read related decision chains together.
- Treat accepted ADR files as immutable historical records.
- Never “fix” an accepted record in place. Recommend a superseding ADR when a decision changed or a corrective ADR when the record is materially wrong.
- Index, glossary, and inventory corrections may be proposed separately because they are living documents.
- Use code and current documentation as evidence; do not infer drift from preference alone.

## Evidence sources

Inspect sources relevant to each decision:

- `docs/adr/INDEX.md` and all ADRs
- `docs/fdr/INDEX.md` and related FDRs
- `docs/architecture/INDEX.md` and relevant inventory pages
- `docs/GLOSSARY.md`
- Root and path-specific `AGENTS.md` files
- Current implementation, manifests, schemas, interfaces, tests, configuration, deployment, and operational documentation

## Audit procedure

1. Validate every index row and find files missing from the index.
2. Group records by topic and supersession chain.
3. For every ADR, verify decision clarity, rationale, alternatives, consequences, links, vocabulary, and relation metadata.
4. Compare each accepted decision with the current implementation and current-state inventory.
5. Confirm supersession is represented in the replacement ADR and index without requiring edits to the old ADR.
6. Check related FDRs for alignment and appropriate citations.
7. Produce one consolidated report ordered by severity.

Use parallel subagents for large audits when available, dividing by complete topic groups rather than arbitrary file slices.

## Finding categories

- **Contradiction:** Active decisions conflict with one another or with current implementation.
- **Implementation Drift:** The implementation no longer follows an accepted, non-superseded decision.
- **Missing Supersession:** A decision changed without a replacement record or index status.
- **Weak Decision:** The record does not make a concrete architectural choice.
- **Weak Rationale:** The reason or rejected alternatives are insufficient to understand the choice.
- **Weak Consequences:** Material costs, risks, compatibility effects, or operational burden are absent.
- **Missing Cross-reference:** A related ADR, FDR, inventory page, or glossary concept should be linked.
- **Vocabulary Drift:** Terminology conflicts with the glossary.
- **Index Issue:** A row is absent, duplicated, stale, or broken.
- **No Issue:** The record was checked and needs no material follow-up.

## Report format

```markdown
## Findings

- **Severity — Category:** [ADR-NNN](docs/adr/ADR-NNN-slug.md) ...
  - **Evidence:** `path:line` or related record.
  - **Impact:** ...
  - **Proposed action:** ...

## Clean Records

- ADR-NNN — checked against ...; no material issue found.

## Open Questions

- ...

## Proposed Changes

- Create ADR-NNN to supersede ADR-NNN.
- Correct the ADR index.
- Refresh the relevant inventory page.
```

Use `Critical`, `High`, `Medium`, or `Low` severity. Keep evidence factual and distinguish observed facts from inference.

## Applying approved fixes

When explicitly asked to apply fixes:

1. Invoke the `adr` skill workflow.
2. Preserve all accepted ADR files exactly.
3. Create new records for changed decisions.
4. Update `docs/adr/INDEX.md` and approved living documents.
5. Do not alter accepted FDRs; use the `fdr` supersession workflow when needed.
6. Verify links, indexes, numbering, and relevant implementation/documentation claims.
7. Report the exact scope checked and any remaining risk.
