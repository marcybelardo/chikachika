---
description: Turn a specified GitHub issue into bounded, dependency-aware subagent work and reconcile the verified implementation without unauthorized repository mutations.
---
# GitHub Issue Orchestrator

Use this skill when an issue number or URL is explicitly supplied for implementation or research orchestration. The orchestrator owns delegation, aggregation, integration, and the final verification summary.

## Read and cross-check the issue

1. Require a GitHub issue number or URL as input. Do not infer an issue from a TODO checkbox or silently select one.
2. Read the issue body, metadata, labels, state, and comments before delegating. Use `gh issue view <number-or-url> --comments` and request the metadata needed to understand current state, author, labels, milestone, and URL.
3. Cross-check the stated scope and acceptance criteria against the relevant repository evidence: `AGENTS.md`, the applicable `docs/TODO-*.md` milestone, current code and tests, ADRs, FDRs, `docs/GLOSSARY.md`, and `docs/architecture/`.
4. Preserve the issue’s acceptance criteria. Surface ambiguity, contradictions, missing decisions, and stale scope for resolution; do not invent product behavior to make an assignment appear complete.

## Make bounded assignments

Convert the issue into assignments with all of the following:

- An explicit goal and the issue acceptance criteria it covers.
- Explicit ownership of repository paths and an explicit list of paths the subagent must not change.
- Constraints from `AGENTS.md`, applicable ADRs/FDRs, glossary terminology, architecture inventory, and the milestone TODO.
- Expected result format, including commit hashes for file-modifying work and factual findings for read-only work.
- Exact tests or verification commands, plus required documentation and decision-record obligations.

Choose sequential delegation when one task establishes a shared interface, model, decision, or prerequisite. Use parallel delegation only for independent bounded work after shared foundations are established. Read-only research and review assignments must not receive unnecessary worktrees.

For file-modifying assignments, apply the repository’s worktree, branch, commit, and integration rules: begin from a committed known-good baseline, use one dedicated branch and worktree under `/.worktrees/` per subagent, keep ownership disjoint, and leave integration to the orchestrator. Tell each subagent not to merge, rebase, cherry-pick, or otherwise integrate its branch.

Every delegated prompt must state that the subagent must not create or update GitHub issues or pull requests, rename branches, or broaden scope unless that exact mutation is explicitly part of the delegated task. The orchestrator owns repository orchestration and integration, not the subagents.

## Aggregate and verify

Collect every subagent result. Review commits and diffs, reconcile conflicts, check for uncovered acceptance criteria, and integrate in dependency order on the coordinating branch. Remove completed worktrees and temporary branches only after integration or intentional rejection. Run the relevant combined checks after integration rather than assuming independent success composes.

Return an execution summary containing:

- Completed, blocked, and intentionally deferred acceptance criteria.
- Integrated commits and exact paths changed.
- Exact verification commands and results, including documentation and test updates.
- Conflicts or uncovered criteria and how they were resolved.
- Remaining risks, unknowns, and required follow-up.

Comments or issue status changes are optional and require the issue-management authorization in `AGENTS.md`; do not perform them merely to report progress. Never create a draft PR implicitly. PR submission belongs to the GitHub PR checklist workflow and must be a full, ready-for-review PR unless the user explicitly requests a draft.
