# Repository Guidance

This file is the root context and navigation entry point for contributors and coding agents. More specific `AGENTS.md` files may be added later; when present, they govern their directory subtree and must be listed here.

> **Status:** Pre-release implementation of the `0.0.1` vertical slice; the core workspace, editor, persistence, browser renderer, and loopback update path are implemented, while the URL workflow and release-platform validation remain pending.

## Context Map

### Repository guidance

| Scope | Context |
|---|---|
| Entire repository | This file |

### Canonical documentation

| Subject | Source | Purpose |
|---|---|---|
| Initial product direction | [`docs/Project Structure.md`](docs/Project%20Structure.md) | Early high-level product and vertical-slice notes |
| `0.0.1` milestone | [`docs/TODO-0-0-1.md`](docs/TODO-0-0-1.md) | Requirements and completion checklist for the first release |
| Canonical terminology | [`docs/GLOSSARY.md`](docs/GLOSSARY.md) | Current project-specific terms and meanings |
| Architecture decisions | [`docs/adr/INDEX.md`](docs/adr/INDEX.md) | Append-only cross-cutting technical decisions |
| Feature decisions | [`docs/fdr/INDEX.md`](docs/fdr/INDEX.md) | Append-only feature behavior and design decisions |
| Current architecture | [`docs/architecture/INDEX.md`](docs/architecture/INDEX.md) | Current components, boundaries, ownership, and operational contracts |

### Documentation skills

| Skill | Location | Use |
|---|---|---|
| ADR | [`.agents/skills/adr/SKILL.md`](.agents/skills/adr/SKILL.md) | Create or supersede architecture decisions |
| ADR review | [`.agents/skills/adr-review/SKILL.md`](.agents/skills/adr-review/SKILL.md) | Audit architecture decisions against current reality |
| FDR | [`.agents/skills/fdr/SKILL.md`](.agents/skills/fdr/SKILL.md) | Create or supersede feature decisions |
| Glossary | [`.agents/skills/glossary/SKILL.md`](.agents/skills/glossary/SKILL.md) | Look up, add, rename, or audit terms |
| Architecture inventory | [`.agents/skills/architecture-inventory/SKILL.md`](.agents/skills/architecture-inventory/SKILL.md) | Maintain current-state architecture documentation |

### GitHub collaboration skills

| Skill | Location | Use |
|---|---|---|
| GitHub triage | [`.agents/skills/github-triage/SKILL.md`](.agents/skills/github-triage/SKILL.md) | Reconcile milestone work with GitHub issues |
| GitHub issue orchestrator | [`.agents/skills/github-issue-orchestrator/SKILL.md`](.agents/skills/github-issue-orchestrator/SKILL.md) | Turn an issue into bounded delegated work |
| GitHub PR checklist | [`.agents/skills/github-pr-checklist/SKILL.md`](.agents/skills/github-pr-checklist/SKILL.md) | Review and prepare a ready-for-review pull request |

## Project Status

The project is in pre-release implementation. The repository contains the native overlay workspace and one-widget editor, versioned app-local persistence, a transparent browser renderer, and stable loopback HTTP/SSE hosting. The next target is `0.0.1`; its remaining URL workflow, platform validation, documentation, and measurement requirements are tracked in [`docs/TODO-0-0-1.md`](docs/TODO-0-0-1.md).

Until `0.0.1` is released, the architecture and implementation may change substantially. There are no compatibility or migration guarantees for unreleased code or data. Agents should change, replace, or remove existing code and structure when doing so produces a simpler, clearer, more functional product. Preserve an earlier approach only when it remains the best current choice, not merely because it already exists.

Keep this section current and factual. Milestone checklists belong in the applicable `docs/TODO-{version}.md` file; architectural rationale belongs in ADRs; feature behavior and design decisions belong in FDRs.

## Prime Directives

1. Prefer simple, clear, maintainable code over clever constructions. Optimize for a reader's ability to understand behavior and verify correctness.
2. Keep the product focused, lightweight, and performant. Treat simplicity and performance as complementary design constraints; do not compromise either without a measured reason and an explicitly documented tradeoff.
3. Make the smallest coherent change that delivers the required behavior. Avoid speculative abstractions, premature extensibility, and unrelated scope.
4. Tests and documentation are part of the change, not follow-up work. Add or update them in the same change whenever behavior, architecture, terminology, or contributor workflow changes.
5. Verify changes proportionally to their risk. Exercise affected behavior and report exactly what was checked and what remains unverified.
6. During the pre-release phase, favor correcting weak foundations over preserving unreleased compatibility. Keep the repository functional at coherent checkpoints and record consequential architecture or feature decisions in the appropriate ADR or FDR.

## Documentation Maintenance

Use the documentation skill that owns the material being changed.

- Accepted ADR and FDR files are immutable. Represent changed decisions with a newly numbered superseding record and update the corresponding index.
- `docs/adr/INDEX.md` and `docs/fdr/INDEX.md` are living navigation/status files.
- `docs/GLOSSARY.md` and `docs/architecture/` are living current-state documentation; update stale facts in place.
- Do not put rationale in the architecture inventory, implementation detail in an FDR, feature behavior in an ADR, or tutorials in the glossary.
- Update documentation in the same change as the code or product decision that makes it stale.
- Verify relative links, index entries, identifiers, dates, statuses, and terminology after documentation changes.
- Add every future path-specific `AGENTS.md` to the Context Map in this file.

## Parallel Subagent Orchestration

These instructions apply to an orchestration agent coordinating subagents that modify code or other repository files in parallel.

- Start parallel work only from a committed, known-good baseline. Uncommitted and untracked files in the primary checkout are not inherited by new worktrees; commit the coherent baseline before delegation.
- Give each file-modifying subagent a dedicated Git branch and worktree under the ignored `/.worktrees/` directory. Use short task-oriented names, and start the subagent with its worktree as its working directory.
- Do not create a worktree for a read-only research or review subagent that will not modify repository files.
- Divide work along independent, explicitly bounded responsibilities. Tell each subagent what it owns, what it must not change, which established interfaces or decisions it must preserve, and what result it must return.
- Establish shared models, interfaces, and consequential architecture decisions before parallelizing dependent implementations. Do not ask separate subagents to invent competing versions of the same foundation.
- Instruct each file-modifying subagent to keep its worktree clean, make coherent Conventional Commits, include directly related tests and documentation, run proportionate verification, and report its commit hashes and exact checks.
- Subagents must not merge, rebase, cherry-pick, or otherwise integrate their branches into the coordinating branch unless the orchestration agent explicitly delegates that operation.
- The orchestration agent owns integration. Review each branch, integrate commits in dependency order on a non-`main` coordinating branch, resolve textual and semantic conflicts centrally, and verify the combined result rather than assuming independently valid branches compose correctly.
- Do not cherry-pick implementation or documentation commits directly into `main`. Before any change reaches `main`, push the complete coordinating branch to GitHub, create a full ready-for-review pull request, and merge through that pull request after review and required checks. Local cherry-picks may assemble a review branch, but must not bypass the GitHub pull-request boundary.
- After integration, run the relevant combined test and documentation checks on the coordinating branch. Report what was integrated, what was verified, and any remaining risk.
- Remove completed worktrees and delete temporary branches only after their commits are integrated or intentionally rejected and no longer needed.

## Commits, Issues, and Pull Requests

### Commits

Use Conventional Commits:

```text
<type>[optional scope][optional !]: <description>
```

- Use a lowercase type and a concise, imperative description.
- Include a scope when it identifies the affected product or architecture area, for example `fix(web server): keep overlay URLs stable`.
- Common types include `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `build`, `ci`, and `chore`.
- Append `!` only when the change truly breaks a supported external contract and requires users or integrations to adapt, for example `feat(editor)!: replace the saved overlay format`.
- Do not mark ordinary internal refactors, unreleased implementation churn, or merely large changes as breaking.
- Use a `BREAKING CHANGE:` footer when migration details or a longer explanation are necessary.
- Keep each commit coherent and independently understandable. Include directly related tests and documentation in the same commit.

### Issues

- Use or update GitHub issues only when the user asks for issue or roadmap management, or when an explicitly invoked workflow requires it.

### Pull requests

- Always create pull requests as full, ready-for-review pull requests. Create a draft only when the user explicitly asks for a draft.
- PR bodies summarize the changes and link relevant FDRs, ADRs, glossary terms, and issues.
- If a PR closes an issue, include a GitHub closing keyword such as `Closes #123.` in the body.
- The operator personally reviews and merges pull requests. Agents may push branches and submit full ready-for-review PRs, but must not merge PRs, merge locally, or bypass the PR review boundary unless the operator explicitly authorizes that specific merge.
- For multiline issue or PR bodies passed through `gh`, write Markdown to a file or stdin and use `--body-file`; never encode newlines as `\\n` in `--body`.

### Branches

- Do not rename the current branch unless explicitly stated.
