---
description: Maintain and audit the current-state architecture inventory, including components, boundaries, ownership, lifecycle, and authoritative source links.
---
# Architecture Inventory

Use this skill when architecture components, boundaries, interfaces, persistence, runtime processes, or ownership are added, changed, removed, audited, or documented.

## Documentation boundary

The inventory describes **what exists now**:

- Components and responsibilities
- Architectural boundaries and ownership
- Runtime lifecycle and communication
- Data ownership and persistence
- Interfaces and external integrations
- Operationally relevant current contracts

It is not history, rationale, a roadmap, or a feature specification.

- Rationale, alternatives, and consequences belong in ADRs.
- User-visible behavior and feature-specific design belong in FDRs.
- Canonical definitions belong in `docs/GLOSSARY.md`.
- Procedures, tutorials, and API references belong in their dedicated documentation.

Link to owning records instead of repeating them. Remove stale facts in place; the inventory is a living current-state document.

## Canonical location

```text
docs/architecture/
├── INDEX.md
└── topic-name.md
```

`docs/architecture/INDEX.md` is the navigation and component-ownership map. Read it first and update it when pages or categories change.

Create topic pages only when the project contains enough verified architecture to describe. Do not pre-populate speculative components.

## Required content

Each topic page must be independently understandable and include:

1. **Scope** — what the page inventories and excludes.
2. **Authoritative sources** — 2–5 relative links to current manifests, wiring, schemas, modules, configuration, or tests near the top.
3. **Inventory** — concise prose or tables identifying what exists, where it lives, who owns it, and its responsibilities.
4. **Current contracts** — relevant lifecycle, persistence, interface, failure, security, or operational constraints.
5. **Related decisions** — links to applicable ADRs and FDRs without repeating rationale.

Use `None yet` when a small bootstrap index intentionally has no pages. Never present planned architecture as current fact.

## Ownership method

Derive ownership from authoritative repository evidence, such as:

- Construction and application wiring
- Module and package boundaries
- State mutation and read paths
- Interface definitions and route registration
- Storage creation and migration code
- Background task startup and shutdown
- Focused tests that enforce boundaries or invariants

For every inventoried component, answer as applicable:

- What is it?
- Where does it live?
- Which architectural area owns it?
- What does it own or mutate?
- How and when does it start, stop, or run?
- What does it communicate with?
- What persists, and where?
- What failure or recovery contract currently exists?

Do not infer ownership from filenames alone. If evidence is unavailable or conflicting, mark it unknown and report the gap.

## Update workflow

Maintenance requests authorize in-scope inventory edits unless the user asks for report-only output.

1. Read `docs/architecture/INDEX.md`.
2. Identify only the categories affected by the task or code change.
3. Read affected pages and authoritative sources.
4. Read related ADRs/FDRs only as needed to preserve boundaries and links.
5. Correct every resolvable in-scope omission, stale fact, broken link, duplication, and ownership error.
6. Update the index for added, removed, or renamed pages.
7. Validate the changed inventory.
8. Report exact categories, evidence, checks, and remaining unknowns.

If evidence exposes a likely implementation defect or a missing decision, inventory current reality accurately and report the separate issue. Do not change product behavior as part of documentation maintenance.

## Report-only mode

For an audit, review, proposal, or explicit no-edit request:

- Make no changes.
- Report missing components, stale facts, ownership ambiguity, broken sources, boundary violations, and misplaced rationale.
- Distinguish verified drift from unanswered questions.
- Recommend the smallest appropriate inventory, ADR, FDR, glossary, or implementation follow-up.

## Validation

Validation must be proportional to the pages changed:

- Every authoritative source and relative documentation link resolves.
- Inventoried components match current construction/wiring and manifests.
- Ownership matches current mutation and lifecycle responsibility.
- Interfaces, persistence, and background processes match registration/creation code.
- No planned or retired component is presented as current.
- Rationale is linked to ADRs rather than duplicated.
- Feature behavior is linked to FDRs rather than duplicated.
- Terminology matches `docs/GLOSSARY.md`.
- The index lists every topic page exactly once.

Finish with the exact scope checked and any known documentation drift left unresolved.
