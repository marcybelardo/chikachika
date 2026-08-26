---
description: Reconcile the next milestone with repository reality and existing GitHub issues, then create authorized, deduplicated work items.
---
# GitHub Milestone Triage

Use this skill only for an explicitly invoked GitHub milestone-triage workflow. Ordinary implementation, review, and documentation work must not create or update GitHub issues.

## Authorization and scope

- Treat issue creation as authorized only when the user explicitly invokes this triage workflow for issue or roadmap management. A dry run, audit, or request for recommendations stops before any mutating `gh` command.
- Do not rename the current branch, push commits, create pull requests, or mark milestone checkboxes complete.
- Keep the work specific to this repository and its milestone terminology; do not copy procedures or product assumptions from another project.

## Inspect the milestone completely

1. Read `AGENTS.md` and discover the next-version document under `docs/TODO-*.md`. For the current repository, the target is `docs/TODO-0-0-1.md`.
2. Read the complete TODO, including its outcome, product requirements, quality requirements, explicitly out-of-scope items, and completion gate. Preserve the distinction between in-scope requirements and explicit exclusions.
3. For every unchecked item, inspect current code, tests, and documentation. Reconcile the observed implementation rather than treating an unchecked box as automatically missing. Record partial, verified, blocked, and unresolved work separately.
4. Inspect existing open and relevant closed GitHub issues before proposing work to avoid duplicates. Use read-only commands such as `gh issue list --state all --limit 1000 --json number,state,title,body,url` and `gh issue view` as needed. Existing issues can contain decisions or scope that supersede an apparent TODO interpretation.

## Shape a deduplicated issue plan

Group the remaining work into manageable, independently understandable issues. Avoid one issue per checkbox and avoid an oversized milestone issue. Separate user-visible feature work from enabling architecture or infrastructure work when ownership, sequencing, or review becomes clearer that way. Map each TODO item to an existing issue, a proposed new issue, or an explicit unresolved/deferred outcome.

Each proposed issue needs:

- A concise, outcome-oriented title.
- A Markdown body with **Why / outcome**, **Scope**, **Non-goals**, **Acceptance criteria**, **References**, and **Dependencies / sequencing** headings.
- Concrete acceptance criteria that can be reviewed and verified without restating the whole milestone.
- Relevant file and documentation references, including the milestone TODO and applicable FDRs, ADRs, glossary terms, or architecture inventory pages.
- Dependencies and sequencing notes where an architecture or infrastructure issue enables later feature work.

## Create only authorized issues

Create nothing until the triage workflow has been explicitly invoked for issue creation. Before creating, show or retain the proposed mapping and issue bodies so the authorization boundary is clear. Use only existing repository labels when a label is present and clearly applicable; never invent unrelated labels.

For each authorized issue, write the Markdown body to a temporary file or stdin and use `--body-file`; never put escaped newlines in `--body`. For example:

```bash
body_file=$(mktemp)
trap 'rm -f "$body_file"' EXIT
# Write the reviewed Markdown body to "$body_file".
gh issue create --title "Concise issue title" --body-file "$body_file"
```

The `gh issue create` command is permitted here only after the explicit invocation and authorization above. Do not use a multiline `--body` string.

## Report without hiding gaps

Return a reconciliation table or equivalent report that names:

- TODO items mapped to existing issue numbers and URLs.
- New issue numbers and URLs created during this authorized invocation.
- Items verified in the repository despite remaining unchecked.
- Unresolved, blocked, or deliberately deferred items, with reasons and dependencies.

Do not silently mark the TODO complete. Report the exact read-only commands and repository files inspected, and distinguish observed facts from proposed scope.
