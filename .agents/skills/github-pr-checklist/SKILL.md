---
description: Review a complete branch change and prepare or submit a reviewer-oriented, ready-for-review GitHub pull request with verified documentation and tests.
---
# GitHub Pull Request Checklist

Use this skill to review a branch and, when explicitly authorized, prepare and submit a pull request. In review or checklist mode, output only actionable findings; omit areas that need no action.

## Review the complete change

1. Inspect the complete branch diff against its intended base, including committed and relevant working-tree changes. Understand the observable behavior, implementation decisions, tests, documentation, configuration, and operational effects before drafting a PR.
2. Compare the change with the issue acceptance criteria and repository contracts. Check `AGENTS.md`, the applicable `docs/TODO-*.md` milestone, current code/tests, and user or setup documentation.
3. Check current decision and vocabulary documentation: applicable ADRs, FDRs, `docs/GLOSSARY.md`, and `docs/architecture/`. Update stale living documentation when the task authorizes it; otherwise report the exact follow-up. Do not rewrite accepted ADRs or FDRs in place.
4. Identify actionable test gaps. Add or fix tests when authorized, and report behavior that remains untestable plus any required manual validation explicitly. State exact commands and results, not just that tests were run.
5. Assess compatibility, security, operational, rollout, and failure implications that matter to reviewers. Keep claims grounded in the complete diff and current repository documentation.

## Prepare the reviewer-oriented body

Use a Markdown body with these headings or their clear equivalents:

- **Why / problem** — the user or maintainer need and relevant issue context.
- **What changed** — the implementation, tests, documentation, and decisions changed.
- **Test plan and exact results** — automated checks, manual validation, and any remaining gaps.
- **Compatibility, security, operational, and rollout implications** — relevant risks or explicitly none.

Link relevant FDRs, ADRs, glossary terms, architecture inventory pages, milestone TODO sections, and issues. If the PR is intended to close an issue, include a GitHub closing keyword such as `Closes #123.` (or an equivalent keyword) in the body. Keep the body accurate to the complete diff and verification status.

## Submit only a ready-for-review PR

Create a full, ready-for-review PR with `gh pr create`. Use `--draft` only after the user explicitly requests a draft. For multiline Markdown, write the body to a temporary file or stdin and pass it with `--body-file`; never encode newlines as `\\n` in `--body`.

For example:

```bash
body_file=$(mktemp)
trap 'rm -f "$body_file"' EXIT
# Write the reviewed Markdown body to "$body_file".
gh pr create --title "Reviewer-oriented title" --body-file "$body_file"
```

Do not rename the current branch. Preserve the existing branch unless the user explicitly authorizes a rename.

After creating or editing a PR, read its stored title, body, and state back with `gh pr view <number-or-url>` and confirm that it represents the complete diff and the actual verification status. Report any discrepancy rather than silently accepting it.

## Improve the repository instructions carefully

Assess whether a concise repository rule or instruction update would have prevented an observed mistake or materially improved future execution or review. Update `AGENTS.md` only when the task authorizes it and the change is relevant; otherwise report the actionable follow-up without broadening the PR.
