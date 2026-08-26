import importlib.util
import shutil
import sys
import tempfile
import unittest
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "tests" / "fixtures" / "valid"
SPEC = importlib.util.spec_from_file_location("check_docs", ROOT / "scripts" / "check_docs.py")
CHECK_DOCS = importlib.util.module_from_spec(SPEC)
sys.modules["check_docs"] = CHECK_DOCS
SPEC.loader.exec_module(CHECK_DOCS)


FDR_TEMPLATE = """# FDR-{number}: {title}

**Status:** {status}
**Date:** {date}
**Supersedes:** {supersedes}

## Overview

Overview.

## User-visible Behavior

Behavior.

## Feature Decisions

### 1. Choice

**Decision:** Choice.

**Why:** Reason.

**Tradeoff:** Cost.

## Open Questions

None.

## Related

None.
"""

ADR_TEMPLATE = """# ADR-{number}: {title}

**Status:** {status}
**Date:** {date}
**Supersedes:** {supersedes}

## Context

Context.

## Decision

Decision.

## Rationale

Rationale.

## Alternatives Considered

Alternative.

## Consequences

### Positive

Positive.

### Negative

Negative.

## Related

Related.
"""


def write_record(root, kind="FDR", number="001", title="Fixture Feature", status="Accepted", date="2026-01-02", supersedes="None", filename=None, text=None):
    directory = root / "docs" / kind.lower()
    directory.mkdir(parents=True, exist_ok=True)
    filename = filename or f"{kind}-{number}-fixture-feature.md"
    template = FDR_TEMPLATE if kind == "FDR" else ADR_TEMPLATE
    record = text or template.format(number=number, title=title, status=status, date=date, supersedes=supersedes)
    path = directory / filename
    path.write_text(record, encoding="utf-8")
    return path


def write_index(root, kind="FDR", rows=None):
    directory = root / "docs" / kind.lower()
    directory.mkdir(parents=True, exist_ok=True)
    header = "Feature" if kind == "FDR" else "Decision"
    lines = [f"# {'Feature' if kind == 'FDR' else 'Architecture'} Decision Records", "", "## Records", "", f"| Record | {header} | Status | Date |", "|---|---|---|---|"]
    for row in rows or []:
        identifier, title, status, date, target = row
        lines.append(f"| [{identifier}]({target}) | {title} | {status} | {date} |")
    (directory / "INDEX.md").write_text("\n".join(lines) + "\n", encoding="utf-8")


class CheckerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        shutil.copytree(FIXTURE, self.root, dirs_exist_ok=True)

    def tearDown(self):
        self.temp.cleanup()

    def errors(self):
        return "\n".join(CHECK_DOCS.check_tree(self.root))

    def assertInvalid(self, needle):
        errors = self.errors()
        self.assertIn(needle, errors, errors)

    def test_valid_fixture(self):
        self.assertEqual(CHECK_DOCS.check_tree(self.root), [])

    def test_skill_frontmatter_exact_description_and_delimiters(self):
        skill = self.root / ".agents" / "skills" / "fixture" / "SKILL.md"
        skill.parent.mkdir(parents=True)
        skill.write_text("description: missing opening\n# Skill\n", encoding="utf-8")
        self.assertInvalid("frontmatter must start")
        skill.write_text("---\ndescription:\n---\n# Skill\n", encoding="utf-8")
        self.assertInvalid("non-empty")
        skill.write_text("---\ndescription:\ndescription: second\n---\n# Skill\n", encoding="utf-8")
        self.assertInvalid("exactly one description")
        skill.write_text("---\ndescription: |\n  multiline\n---\n# Skill\n", encoding="utf-8")
        self.assertInvalid("single-line")
        skill.write_text("---\ndescription: good\n---oops\n# Skill\n", encoding="utf-8")
        self.assertInvalid("missing a closing")

    def test_filename_and_heading_identifier(self):
        record = self.root / "docs/fdr/FDR-001-fixture-feature.md"
        original = record.read_text()
        record.rename(record.with_name("FDR-001-bad_slug!.md"))
        self.assertInvalid("filename must be")
        record = self.root / "docs/fdr/FDR-001-fixture-feature.md"
        record.write_text(original.replace("# FDR-001:", "# ADR-001:"), encoding="utf-8")
        self.assertInvalid("does not match filename")

    def test_metadata_required_status_date_supersedes(self):
        record = self.root / "docs/fdr/FDR-001-fixture-feature.md"
        text = record.read_text()
        record.write_text(text.replace("**Supersedes:** None\n", "").replace("**Status:** Accepted", "**Status:** Unknown").replace("**Date:** 2026-01-02", "**Date:** 2026-02-31"), encoding="utf-8")
        self.assertInvalid("missing metadata field Supersedes")
        self.assertInvalid("invalid record status")
        self.assertInvalid("calendar-valid")

    def test_required_sections_and_fdr_fields(self):
        record = self.root / "docs/fdr/FDR-001-fixture-feature.md"
        original = record.read_text()
        text = original.replace("## Open Questions\n\n- None.\n", "## Deferred Notes\n\n- None.\n")
        record.write_text(text, encoding="utf-8")
        self.assertInvalid("missing required section ## Open Questions")
        record.write_text(original.replace("**Why:** It is easy to verify.\n", ""), encoding="utf-8")
        self.assertInvalid("non-empty Why")
        record.write_text(re.sub(r"^\*\*Tradeoff:\*\*.*$", "**Tradeoff:**", original, count=1, flags=re.MULTILINE), encoding="utf-8")
        self.assertInvalid("non-empty Tradeoff")

    def test_adr_sections_and_consequence_polarity(self):
        record = self.root / "docs/adr/ADR-001-fixture-decision.md"
        text = record.read_text().replace("### Positive", "### Benefit").replace("### Negative", "### Cost").replace("## Rationale\n", "")
        record.write_text(text, encoding="utf-8")
        self.assertInvalid("missing required section ## Rationale")
        self.assertInvalid("Positive subsection")
        self.assertInvalid("Negative subsection")

    def test_index_duplicate_missing_and_target_title_status_date(self):
        index = self.root / "docs/fdr/INDEX.md"
        index.write_text(index.read_text() + "| [FDR-001](FDR-001-fixture-feature.md) | Wrong | Implemented | 2026-01-03 |\n| [FDR-002](missing.md) | Missing | Proposed | 2026-01-02 |\n", encoding="utf-8")
        self.assertInvalid("more than once")
        self.assertInvalid("does not exist")
        self.assertInvalid("does not agree with record H1 title")
        self.assertInvalid("does not agree with record metadata")

    def test_index_identifier_must_match_target_filename(self):
        index = self.root / "docs/fdr/INDEX.md"
        index.write_text(index.read_text().replace("FDR-001-fixture-feature.md", "../adr/ADR-001-fixture-decision.md"), encoding="utf-8")
        self.assertInvalid("does not agree with target filename")

    def test_missing_index_row_and_missing_indexed_file(self):
        index = self.root / "docs/fdr/INDEX.md"
        index.write_text(index.read_text().replace("| [FDR-001](FDR-001-fixture-feature.md) | Fixture Feature | Accepted | 2026-01-02 |\n", ""), encoding="utf-8")
        self.assertInvalid("indexed exactly once")
        write_index(self.root, "FDR", [("FDR-001", "Fixture Feature", "Accepted", "2026-01-02", "missing.md")])
        self.assertInvalid("does not exist")

    def test_status_matrix_and_supersession_pairs(self):
        old = self.root / "docs/fdr/FDR-001-fixture-feature.md"
        old.write_text(old.read_text().replace("**Status:** Accepted", "**Status:** Accepted"), encoding="utf-8")
        new = write_record(self.root, number="002", title="New Feature", status="Accepted", date="2026-01-03", supersedes="FDR-001", filename="FDR-002-new-feature.md")
        write_index(self.root, "FDR", [
            ("FDR-001", "Fixture Feature", "[Superseded by FDR-002](FDR-002-new-feature.md)", "2026-01-02", "FDR-001-fixture-feature.md"),
            ("FDR-002", "New Feature", "Accepted", "2026-01-03", "FDR-002-new-feature.md"),
        ])
        self.assertEqual(CHECK_DOCS.check_tree(self.root), [], CHECK_DOCS.check_tree(self.root))
        write_index(self.root, "FDR", [
            ("FDR-001", "Fixture Feature", "[Superseded by FDR-002](missing.md)", "2026-01-02", "FDR-001-fixture-feature.md"),
            ("FDR-002", "New Feature", "Accepted", "2026-01-03", "FDR-002-new-feature.md"),
        ])
        self.assertInvalid("broken successor link")

    def test_adr_supersession_pair_and_mismatched_supersedes(self):
        write_record(self.root, kind="ADR", number="002", title="Next Decision", status="Accepted", date="2026-01-03", supersedes="ADR-001", filename="ADR-002-next-decision.md")
        write_index(self.root, "ADR", [
            ("ADR-001", "Fixture Decision", "[Superseded by ADR-002](ADR-002-next-decision.md)", "2026-01-01", "ADR-001-fixture-decision.md"),
            ("ADR-002", "Next Decision", "Accepted", "2026-01-03", "ADR-002-next-decision.md"),
        ])
        self.assertEqual(CHECK_DOCS.check_tree(self.root), [], CHECK_DOCS.check_tree(self.root))
        successor = self.root / "docs/adr/ADR-002-next-decision.md"
        successor.write_text(successor.read_text().replace("**Supersedes:** ADR-001", "**Supersedes:** None"), encoding="utf-8")
        self.assertInvalid("must declare Supersedes: ADR-001")

    def test_accepted_file_implemented_retired_and_missing_successor(self):
        write_index(self.root, "FDR", [("FDR-001", "Fixture Feature", "Implemented", "2026-01-02", "FDR-001-fixture-feature.md")])
        self.assertEqual(CHECK_DOCS.check_tree(self.root), [])
        write_index(self.root, "FDR", [("FDR-001", "Fixture Feature", "Retired", "2026-01-02", "FDR-001-fixture-feature.md")])
        self.assertEqual(CHECK_DOCS.check_tree(self.root), [])
        write_index(self.root, "FDR", [("FDR-001", "Fixture Feature", "[Superseded by FDR-002](missing.md)", "2026-01-02", "FDR-001-fixture-feature.md")])
        self.assertInvalid("successor FDR-002 is missing")

    def test_reject_proposed_transition_unknown_and_cross_type_successor(self):
        record = self.root / "docs/fdr/FDR-001-fixture-feature.md"
        record.write_text(record.read_text().replace("**Supersedes:** None", "**Supersedes:** ADR-001"), encoding="utf-8")
        self.assertInvalid("same-type FDR")
        record.write_text(record.read_text().replace("**Supersedes:** ADR-001", "**Supersedes:** None"), encoding="utf-8")
        record.write_text(record.read_text().replace("**Status:** Accepted", "**Status:** Proposed"), encoding="utf-8")
        write_index(self.root, "FDR", [("FDR-001", "Fixture Feature", "Implemented", "2026-01-02", "FDR-001-fixture-feature.md")])
        self.assertInvalid("may only be indexed as Proposed")
        write_index(self.root, "FDR", [("FDR-001", "Fixture Feature", "Unknown", "2026-01-02", "FDR-001-fixture-feature.md")])
        self.assertInvalid("invalid FDR index status")
        record.write_text(record.read_text().replace("**Status:** Proposed", "**Status:** Accepted"), encoding="utf-8")
        write_index(self.root, "FDR", [("FDR-001", "Fixture Feature", "[Superseded by ADR-001](../adr/ADR-001-fixture-decision.md)", "2026-01-02", "FDR-001-fixture-feature.md")])
        self.assertInvalid("same-type FDR successor")

    def test_adr_rejects_implemented_retired_and_proposed_transition(self):
        index = self.root / "docs/adr/INDEX.md"
        for status in ("Implemented", "Retired"):
            write_index(self.root, "ADR", [("ADR-001", "Fixture Decision", status, "2026-01-01", "ADR-001-fixture-decision.md")])
            self.assertInvalid("invalid ADR index status")
        record = self.root / "docs/adr/ADR-001-fixture-decision.md"
        record.write_text(record.read_text().replace("**Status:** Accepted", "**Status:** Proposed"), encoding="utf-8")
        write_index(self.root, "ADR", [("ADR-001", "Fixture Decision", "[Superseded by ADR-002](ADR-002-next.md)", "2026-01-01", "ADR-001-fixture-decision.md")])
        self.assertInvalid("only an Accepted ADR")

    def test_operational_links_fenced_encoded_and_url_components(self):
        doc = self.root / "docs" / "link-test.md"
        doc.write_text("""# Links

[structure](Project%20Structure.md)
[local query](fdr/INDEX.md?x=1)
[fragment](fdr/INDEX.md#records)
[combined](fdr/INDEX.md?x=1#records)
[protocol-relative](//example.invalid/missing.md)
[external](https://example.invalid/missing.md)
[mailto](mailto:test@example.invalid)

```markdown
[fenced](missing-fenced.md)
```
""", encoding="utf-8")
        (self.root / "docs" / "Project Structure.md").write_text("# Project Structure\n", encoding="utf-8")
        self.assertEqual(CHECK_DOCS.check_tree(self.root), [], CHECK_DOCS.check_tree(self.root))
        doc.write_text(doc.read_text().replace("[combined](fdr/INDEX.md?x=1#records)", "[combined](missing.md?x=1#records)"), encoding="utf-8")
        self.assertInvalid("broken local link")

class ProductContractTests(unittest.TestCase):
    def test_fdr_001_contract(self):
        path = ROOT / "docs/fdr/FDR-001-overlay-editing-and-local-browser-source.md"
        text = path.read_text(encoding="utf-8")
        self.assertIn("**Status:** Accepted", text)
        self.assertEqual(len([line for line in text.splitlines() if line.startswith("### ") and re_decision(line)]), 7)
        self.assertNotRegex(text, r"\b(crate|API|HTTP|WebSocket|iroh|egui|transport)\b")

    def test_product_docs_alignment(self):
        todo = (ROOT / "docs/TODO-0-0-1.md").read_text(encoding="utf-8")
        project = (ROOT / "docs/Project Structure.md").read_text(encoding="utf-8")
        self.assertIn("FDR-001", todo)
        self.assertIn("macOS", todo)
        self.assertIn("Linux", todo)
        self.assertIn("zero or one", todo)
        self.assertNotIn("supported development platform", todo)
        self.assertNotRegex(project, r"\b(iroh|Steam|cloud|synchroni[sz])\b")

    def test_inventory_current_state_guard(self):
        inventory = (ROOT / "docs/architecture/INDEX.md").read_text(encoding="utf-8")
        self.assertIn("currently exists", inventory)
        self.assertIn("No architecture areas", inventory)
        self.assertNotIn("FDR-001", inventory)


def re_decision(line):
    return bool(__import__("re").match(r"^###\s+\d+\.\s+.+", line))


if __name__ == "__main__":
    unittest.main()
