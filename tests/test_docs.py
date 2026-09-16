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
        self.assertIn("Native application runtime", inventory)
        self.assertNotIn("FDR-001", inventory)


def re_decision(line):
    return bool(__import__("re").match(r"^###\s+\d+\.\s+.+", line))


FOUNDATION_MANIFEST = {
    "ADR-001": [
        ("Decision", "eframe/egui owns the native GUI event loop on the main thread.", "remove_adr001_decision_01"),
        ("Decision", "A dedicated server thread owns the Tokio runtime and the axum HTTP server.", "remove_adr001_decision_02"),
        ("Decision", "The server reports its successfully bound address to the GUI before the GUI presents a usable browser-source URL.", "remove_adr001_decision_03"),
        ("Decision", "Normal GUI shutdown signals graceful server shutdown and joins the server thread before process exit.", "remove_adr001_decision_04"),
        ("Decision", "The dedicated server thread uses a current-thread Tokio runtime because 0.0.1 has low local concurrency.", "remove_adr001_decision_05"),
        ("Decision", "A future change from the current-thread runtime requires a superseding ADR justified by measured needs.", "remove_adr001_decision_06"),
        ("Decision", "Browser HTML, CSS, and JavaScript assets are compiled into the executable with standard `include_str!` and `include_bytes!` macros.", "remove_adr001_decision_07"),
        ("Decision", "The embedded asset set stays small and does not depend on the runtime working directory.", "remove_adr001_decision_08"),
        ("Alternatives Considered", "Separate GUI and server processes are rejected because they add IPC, lifecycle coordination, and duplicated failure surfaces for this local slice.", "remove_adr001_alternative_01"),
        ("Alternatives Considered", "An embedded Chromium editor is rejected because it adds a heavyweight browser runtime when the native editor and browser output already have separate responsibilities.", "remove_adr001_alternative_02"),
        ("Alternatives Considered", "Running the current-thread Tokio runtime under the GUI event loop is rejected because server work could block native event processing.", "remove_adr001_alternative_03"),
        ("Alternatives Considered", "A heavier asset framework is rejected because the small 0.0.1 asset set needs no runtime asset packaging or discovery layer.", "remove_adr001_alternative_04"),
    ],
    "ADR-002": [
        ("Decision", "There is exactly one authoritative, framework-independent domain model for an overlay document.", "remove_adr002_decision_01"),
        ("Decision", "The egui editor and browser output are adapters or projections of that model, not independent state stores.", "remove_adr002_decision_02"),
        ("Decision", "The domain model has no egui, axum, filesystem, or browser dependencies.", "remove_adr002_decision_03"),
        ("Decision", "Each overlay receives one generated opaque UUID v4 identity when it is created.", "remove_adr002_decision_04"),
        ("Decision", "The optional text widget receives one generated opaque UUID v4 identity when it is created.", "remove_adr002_decision_05"),
        ("Decision", "Generated overlay and widget identities are created once and persisted unchanged.", "remove_adr002_decision_06"),
        ("Decision", "Names, positions, timestamps, collection indexes, and hashes are never identity sources.", "remove_adr002_decision_07"),
        ("Decision", "0.0.1 permits zero or one text widget and does not introduce a generic plugin or widget hierarchy.", "remove_adr002_decision_08"),
        ("Decision", "Revisions order browser snapshots but are not identities.", "remove_adr002_decision_09"),
        ("Decision", "UI mutations go through domain or store operations rather than directly changing adapter state.", "remove_adr002_decision_10"),
        ("Decision", "The HTTP adapter is read-only in 0.0.1.", "remove_adr002_decision_11"),
        ("Alternatives Considered", "Name-derived or collection-index-derived identity is rejected because renames, reordering, and edits would break durable references.", "remove_adr002_alternative_01"),
        ("Alternatives Considered", "Separate UI and server models are rejected because duplicated state can diverge and makes live updates harder to reason about.", "remove_adr002_alternative_02"),
        ("Alternatives Considered", "A speculative generic widget or plugin framework is rejected because 0.0.1 has one supported text widget and no plugin requirement.", "remove_adr002_alternative_03"),
    ],
    "ADR-003": [
        ("Decision", "Persistence uses a strongly typed Serde JSON envelope.", "remove_adr003_decision_01"),
        ("Decision", "The JSON envelope has an explicit top-level format version and an overlays collection.", "remove_adr003_decision_02"),
        ("Decision", "Path resolution uses `directories::ProjectDirs` with the unqualified `Chikachika` identity and `data_local_dir`.", "remove_adr003_decision_03"),
        ("Decision", "Exact platform-resolved paths are surfaced in implementation and documentation and are covered on macOS and Linux.", "remove_adr003_decision_04"),
        ("Decision", "Path resolution is fallible and its failure is reported to the user.", "remove_adr003_decision_05"),
        ("Decision", "Application-local directories are created explicitly before persistence operations.", "remove_adr003_decision_06"),
        ("Decision", "Persistence never silently falls back to the current working directory.", "remove_adr003_decision_07"),
        ("Decision", "Unsupported format versions and malformed data are rejected visibly and non-destructively.", "remove_adr003_decision_08"),
        ("Decision", "A source file is never overwritten when loading it fails.", "remove_adr003_decision_09"),
        ("Decision", "Saving clones a complete document snapshot and performs file I/O outside the model lock.", "remove_adr003_decision_10"),
        ("Decision", "Saving writes a temporary file in the same directory as the source.", "remove_adr003_decision_11"),
        ("Decision", "Saving replaces the source with an atomic or platform-safe replacement operation.", "remove_adr003_decision_12"),
        ("Decision", "A failed save leaves the in-memory document dirty and exposes the save error.", "remove_adr003_decision_13"),
        ("Decision", "When implementation versions are selected, dependency APIs and replacement guarantees are verified against those versions.", "remove_adr003_decision_14"),
        ("Alternatives Considered", "Unversioned JSON is rejected because incompatible documents cannot be identified deliberately.", "remove_adr003_alternative_01"),
        ("Alternatives Considered", "Opaque binary persistence is rejected because it is harder to inspect, diagnose, and evolve for this small local document.", "remove_adr003_alternative_02"),
        ("Alternatives Considered", "Operating-system config directories for user-created overlay data are rejected because configuration and user documents have different ownership and lifecycle expectations.", "remove_adr003_alternative_03"),
        ("Alternatives Considered", "Manually constructed home-directory paths are rejected because they bypass platform-specific app-local conventions and edge cases.", "remove_adr003_alternative_04"),
        ("Alternatives Considered", "iroh is rejected because peer-to-peer transport is outside the local persistence requirement.", "remove_adr003_alternative_05"),
        ("Alternatives Considered", "Steam is rejected because distribution or account services are outside the local persistence requirement.", "remove_adr003_alternative_06"),
        ("Alternatives Considered", "Cloud synchronization is rejected because accounts, remote storage, and synchronization are outside the 0.0.1 local-first scope.", "remove_adr003_alternative_07"),
    ],
    "ADR-004": [
        ("Decision", "Production binds to `127.0.0.1` only and does not expose LAN or internet interfaces in 0.0.1.", "remove_adr004_decision_01"),
        ("Decision", "The deterministic default browser-source URL is exactly `http://127.0.0.1:51737/overlay/{overlay-id}`.", "remove_adr004_decision_02"),
        ("Decision", "Port 51737 is in the IANA dynamic/private range and can still be occupied by another process.", "remove_adr004_decision_03"),
        ("Decision", "An explicitly configured port may intentionally change the default port.", "remove_adr004_decision_04"),
        ("Decision", "The selected port is persisted and documented so copied URLs remain stable.", "remove_adr004_decision_05"),
        ("Decision", "`127.0.0.1:0` is used only by tests.", "remove_adr004_decision_06"),
        ("Decision", "An occupied configured or default port fails visibly instead of silently changing a copied URL.", "remove_adr004_decision_07"),
        ("Decision", "The exact same-origin routes are `GET /overlay/{id}` and `GET /overlay/{id}/events`.", "remove_adr004_decision_08"),
        ("Decision", "`GET /overlay/{id}` serves the exact browser output.", "remove_adr004_decision_09"),
        ("Decision", "Browser delivery uses the browser-native SSE `EventSource` API.", "remove_adr004_decision_10"),
        ("Decision", "SSE updates use named JSON events.", "remove_adr004_decision_11"),
        ("Decision", "A client subscribes to the events route before it receives the initial snapshot.", "remove_adr004_decision_12"),
        ("Decision", "The server sends one complete current snapshot with a monotonically increasing revision and then sends complete replacements after mutations.", "remove_adr004_decision_13"),
        ("Decision", "Update delivery is bounded so a slow client cannot require unbounded queued history.", "remove_adr004_decision_14"),
        ("Decision", "A reconnect receives the current state rather than depending on historical replay.", "remove_adr004_decision_15"),
        ("Decision", "A lagging client recovers from the latest complete snapshot.", "remove_adr004_decision_16"),
        ("Decision", "The events stream sends periodic keepalive comments.", "remove_adr004_decision_17"),
        ("Decision", "The server makes no promise of historical event replay.", "remove_adr004_decision_18"),
        ("Decision", "There are no browser-to-application mutation routes.", "remove_adr004_decision_19"),
        ("Decision", "No CORS configuration is required for same-origin assets and events.", "remove_adr004_decision_20"),
        ("Alternatives Considered", "Production ephemeral ports are rejected because copied browser-source URLs would not remain stable.", "remove_adr004_alternative_01"),
        ("Alternatives Considered", "Wildcard binding is rejected because 0.0.1 must not expose the local server to LAN or internet interfaces.", "remove_adr004_alternative_02"),
        ("Alternatives Considered", "Polling is rejected because it adds repeated requests and latency where the requirement is server-pushed updates.", "remove_adr004_alternative_03"),
        ("Alternatives Considered", "WebSockets are rejected for this one-way requirement and can be reconsidered only through a superseding ADR if bidirectional needs arise.", "remove_adr004_alternative_04"),
    ],
}


ADR_FOUNDATION_PATHS = {
    identifier: ROOT / "docs" / "adr" / {
        "ADR-001": "ADR-001-one-native-process-gui-and-server.md",
        "ADR-002": "ADR-002-shared-overlay-model-and-stable-ids.md",
        "ADR-003": "ADR-003-versioned-app-local-json-persistence.md",
        "ADR-004": "ADR-004-loopback-sse-browser-delivery.md",
    }[identifier]
    for identifier in FOUNDATION_MANIFEST
}


def _foundation_section_clauses(text, heading):
    lines = text.splitlines()
    try:
        start = lines.index(heading) + 1
    except ValueError:
        return [], [f"missing section {heading}"]
    end = next((index for index in range(start, len(lines)) if lines[index].startswith("## ")), len(lines))
    clauses = []
    errors = []
    for line in lines[start:end]:
        if not line.strip():
            continue
        if not line.startswith("- "):
            errors.append(f"unmanifested text in {heading}: {line}")
        else:
            clauses.append(line[2:].strip())
    return clauses, errors


def _check_adr_foundation(documents):
    errors = []
    manifest_ids = set(FOUNDATION_MANIFEST)
    if manifest_ids != set(ADR_FOUNDATION_PATHS):
        errors.append("foundation manifest and ADR path sets differ")
    for identifier, entries in FOUNDATION_MANIFEST.items():
        text = documents.get(identifier, "")
        expected = [(section, clause) for section, clause, _fixture in entries]
        actual = []
        for section in ("Decision", "Alternatives Considered"):
            clauses, section_errors = _foundation_section_clauses(text, f"## {section}")
            errors.extend(f"{identifier}: {error}" for error in section_errors)
            actual.extend((section, clause) for clause in clauses)
        if actual != expected:
            errors.append(f"{identifier}: Decision/Alternatives clauses do not match the explicit manifest")
        if len(expected) != len(set(expected)):
            errors.append(f"{identifier}: manifest contains duplicate clauses")
        fixture_ids = [fixture for _section, _clause, fixture in entries]
        if len(fixture_ids) != len(set(fixture_ids)):
            errors.append(f"{identifier}: a fixture is mapped to more than one clause")
        for section, clause, fixture in entries:
            if actual.count((section, clause)) != 1:
                errors.append(f"{identifier}: missing governed clause in {section}: {clause}")
            if fixture not in FOUNDATION_MUTATION_FIXTURES:
                errors.append(f"{identifier}: no mutation fixture for {clause}")
    all_fixture_ids = [fixture for entries in FOUNDATION_MANIFEST.values() for _section, _clause, fixture in entries]
    if set(FOUNDATION_MUTATION_FIXTURES) != set(all_fixture_ids):
        errors.append("mutation fixture set does not exactly match the clause manifest")
    return errors


def _remove_foundation_clause(text, clause):
    marker = f"- {clause}\n"
    if text.count(marker) != 1:
        raise AssertionError(f"targeted fixture could not find exactly one clause: {clause}")
    return text.replace(marker, "", 1)


def _build_foundation_mutation_fixtures():
    fixtures = {}
    for entries in FOUNDATION_MANIFEST.values():
        for _section, clause, fixture in entries:
            fixtures[fixture] = lambda text, clause=clause: _remove_foundation_clause(text, clause)
    return fixtures


FOUNDATION_MUTATION_FIXTURES = _build_foundation_mutation_fixtures()


class ArchitectureFoundationContractTests(unittest.TestCase):
    def _documents(self):
        return {identifier: path.read_text(encoding="utf-8") for identifier, path in ADR_FOUNDATION_PATHS.items()}

    def test_adr_foundation_set(self):
        documents = self._documents()
        self.assertEqual(_check_adr_foundation(documents), [])
        for identifier, text in documents.items():
            self.assertIn("**Status:** Accepted", text)
            self.assertIn("**Date:** 2026-08-26", text)
            self.assertIn("**Supersedes:** None", text)
            self.assertEqual(text.count("FDR-001-overlay-editing-and-local-browser-source.md"), 1)

    def test_adr_foundation_clause_mutations_are_targeted(self):
        documents = self._documents()
        manifest_entries = [
            (identifier, section, clause, fixture)
            for identifier, entries in FOUNDATION_MANIFEST.items()
            for section, clause, fixture in entries
        ]
        self.assertEqual(len(manifest_entries), len(FOUNDATION_MUTATION_FIXTURES))
        self.assertEqual(len({fixture for _id, _section, _clause, fixture in manifest_entries}), len(manifest_entries))
        for identifier, section, clause, fixture in manifest_entries:
            with self.subTest(identifier=identifier, section=section, clause=clause):
                original = documents[identifier]
                mutated = FOUNDATION_MUTATION_FIXTURES[fixture](original)
                self.assertEqual(original.count(f"- {clause}\n"), mutated.count(f"- {clause}\n") + 1)
                mutated_documents = dict(documents)
                mutated_documents[identifier] = mutated
                errors = _check_adr_foundation(mutated_documents)
                self.assertTrue(errors, f"removing {fixture} did not fail semantic validation")
                self.assertTrue(any(clause in error for error in errors), errors)


DECISION_002_PATHS = {
    "ADR-006": ROOT / "docs/adr/ADR-006-ordered-authoritative-widget-model.md",
    "ADR-007": ROOT / "docs/adr/ADR-007-version-2-overlay-document-persistence.md",
    "ADR-008": ROOT / "docs/adr/ADR-008-coordinator-owned-workspace-history.md",
    "ADR-009": ROOT / "docs/adr/ADR-009-secondary-native-settings-viewport-lifecycle.md",
    "FDR-003": ROOT / "docs/fdr/FDR-003-multi-widget-composition-workspace.md",
    "FDR-004": ROOT / "docs/fdr/FDR-004-bundled-offline-text-fonts.md",
    "FDR-005": ROOT / "docs/fdr/FDR-005-singleton-native-settings-window.md",
}

DECISION_002_MANIFEST = {
    "ADR-006": [
        ("## Decision", "There is exactly one authoritative, framework-independent domain model for the overlay collection and its overlay documents."),
        ("## Decision", "Native editor, persistence, server, and browser output are adapters or projections of that model, not independent authoritative state stores."),
        ("## Decision", "The domain model has no egui, axum, filesystem, or browser dependencies."),
        ("## Decision", "UI mutations go through domain or store operations rather than directly changing adapter state."),
        ("## Decision", "The HTTP adapter remains read-only; there are no browser-to-application mutation routes."),
        ("## Decision", "Each overlay and each text widget receives one generated opaque UUID v4 identity when created. Duplication creates a new widget UUID."),
        ("## Decision", "Generated identities are persisted unchanged. Names, positions, timestamps, collection indexes, and hashes are never identity sources."),
        ("## Decision", "Revisions order browser snapshots and are delivery metadata, not identities or persisted content."),
        ("## Decision", "This is a concrete text-widget collection, not a generic plugin or widget hierarchy. Images, rich text, and arbitrary font imports are outside this decision."),
        ("### Retained", "One authoritative framework-independent model remains the source for all projections."),
        ("### Retained", "Adapter state is not authoritative, the domain has no adapter dependencies, mutations pass through domain/store ownership, and HTTP remains read-only."),
        ("### Retained", "Overlay and widget identities remain generated opaque UUID v4 values, persisted unchanged and never derived from names, positions, timestamps, indexes, or hashes."),
        ("### Retained", "Revisions remain ordering metadata rather than identity."),
        ("### Retained", "Rejection of name/index identity and separate UI/server models remains unchanged."),
        ("### Retained", "A speculative generic widget/plugin framework remains rejected."),
    ],
    "ADR-007": [
        ("## Decision", "Overlay persistence uses a strongly typed Serde JSON envelope with top-level format version `2` and an ordered overlays collection."),
        ("## Decision", "Each persisted overlay includes its stable identity, name, canvas data, and ordered text-widget collection. Each widget includes its stable identity, editable name, multiline content, finite position, size, RGBA color, alignment, and stable font-family ID."),
        ("## Decision", "Format 2 omits delivery revisions, revision high-water marks, selection, hover, pending edits, gestures, and undo/redo history."),
        ("## Decision", "There is no format-1 conversion. Unknown versions and malformed data fail visibly and non-destructively."),
        ("## Decision", "Invalid or duplicate overlay/widget identities, non-finite required coordinates, and unknown font IDs are rejected visibly and non-destructively before replacing the in-memory collection."),
        ("## Decision", "A failed load never overwrites its source or replaces the currently usable in-memory workspace."),
        ("## Decision", "To start fresh after an incompatible format-1 or temporary file, the user moves the old overlay data aside to a separately named backup; the application does not overwrite or silently convert it."),
        ("## Decision", "Path resolution uses `directories::ProjectDirs` with the unqualified `Chikachika` identity and `data_local_dir`."),
        ("## Decision", "Exact platform-resolved paths are surfaced in implementation and documentation and covered on macOS and Linux."),
        ("## Decision", "Path resolution is fallible, failures are visible, required app-local directories are created explicitly, and persistence never falls back to the working directory."),
        ("## Decision", "Saving clones a complete whole-collection snapshot and performs file I/O outside model locks."),
        ("## Decision", "Saving writes a temporary file in the same directory and replaces the source using an atomic or platform-safe replacement operation."),
        ("## Decision", "A failed save preserves the previous source, current work, history, dirty state, and a visible error."),
        ("## Decision", "The dirty baseline is persistent document-content equality with the last successful whole-collection save; revisions and transient session state do not participate."),
        ("## Decision", "When implementation dependency versions are selected, their APIs and same-directory replacement guarantees must be verified against those actual versions before the atomic-save claim is considered implemented."),
        ("## Decision", "Server settings remain a separate format-1 `settings.json` envelope in the platform config location under ADR-005; this decision changes only overlay documents."),
        ("### Retained", "Strongly typed Serde JSON, a top-level version, and an overlays collection remain required."),
        ("### Retained", "`ProjectDirs`, unqualified `Chikachika`, `data_local_dir`, surfaced platform paths, macOS/Linux coverage, fallible visible resolution, explicit directory creation, and no working-directory fallback remain unchanged."),
        ("### Retained", "Unsupported or malformed input is non-destructive; failed loads never overwrite the source."),
        ("### Retained", "Saving clones a complete snapshot, performs I/O outside model locks, writes a same-directory temporary file, and uses atomic or platform-safe replacement."),
        ("### Retained", "Failed saves retain dirty in-memory work and expose errors."),
        ("### Retained", "Actual dependency APIs and replacement guarantees must be verified for selected versions."),
        ("### Retained", "Rejection of unversioned JSON, opaque binary data, config directories for documents, manually built home paths, peer-to-peer transport, distribution services, and cloud synchronization remains unchanged."),
    ],
    "FDR-003": [
        ("## User-visible Behavior", "Users create, rename, switch, and explicitly confirm deletion of overlays; stable overlay identity preserves the browser-source URL across rename, edits, saves, and restarts."),
        ("## User-visible Behavior", "Each overlay contains zero or more independently identified text widgets with editable names, multiline content, position, size, RGBA color, alignment, and bundled font family."),
        ("## User-visible Behavior", "A left widget list, aspect-preserving fit-to-window center canvas, and right inspector share one widget selection. With no widget selected, the inspector shows overlay information and canvas dimensions."),
        ("## User-visible Behavior", "Compact overlay lifecycle controls, status/errors, and readiness-gated Copy URL/Open output remain available. The transparent browser output remains final rendering authority and receives supported edits without manual refresh."),
        ("## User-visible Behavior", "Failures remain visible and non-destructive. Users can recover, retry, or continue with an unaffected overlay where possible. macOS and Linux remain the milestone platform gates."),
        ("### Retained", "Local-first authoring, local saving, loopback browser output, and OBS use remain the product boundary."),
        ("### Retained", "Overlay create/rename/confirmed-delete behavior and durable identity/URL across rename and restart remain."),
        ("### Retained", "Fixed explicit canvases and authoritative transparent browser output remain."),
        ("### Retained", "Text content, position, font size, color, and alignment remain editable."),
        ("### Retained", "Exact browser output can still be copied/opened when ready, and supported edits update connected output without manual refresh."),
        ("### Retained", "Visible, non-destructive persistence and server failures remain; users can recover, retry, or continue with an unaffected overlay where possible."),
        ("### Retained", "macOS and Linux remain required targets; accounts, cloud, collaboration, synchronization, integrations, general OBS control, and LAN/internet exposure remain excluded."),
    ],
}

DECISION_002_TRACE_ANCHORS = {
    "ADR-006": ("**Supersedes:** ADR-002", "`adr006_mutation_ownership`"),
    "ADR-007": ("**Supersedes:** ADR-003", "`adr007_replacement`"),
    "FDR-003": ("**Supersedes:** FDR-001",),
}


def _section_bullets(text, heading):
    lines = text.splitlines()
    if lines.count(heading) != 1:
        return [], [f"expected one owning section {heading}"]
    start = lines.index(heading) + 1
    level = len(heading) - len(heading.lstrip("#"))
    end = next((index for index in range(start, len(lines)) if re.match(rf"^#{{1,{level}}} ", lines[index])), len(lines))
    return [line[2:] for line in lines[start:end] if line.startswith("- ")], []


def _check_002_manifest(documents):
    errors = []
    if set(documents) != set(DECISION_002_MANIFEST):
        errors.append("decision document and substantive manifest sets differ")
    for identifier, entries in DECISION_002_MANIFEST.items():
        text = documents.get(identifier, "")
        parsed = {}
        for heading in {heading for heading, _clause in entries}:
            parsed[heading], section_errors = _section_bullets(text, heading)
            errors.extend(f"{identifier}: {error}" for error in section_errors)
        for heading, clause in entries:
            if parsed.get(heading, []).count(clause) != 1:
                errors.append(f"{identifier}: missing substantive clause in {heading}: {clause}")
        for anchor in DECISION_002_TRACE_ANCHORS.get(identifier, ()):
            if text.count(anchor) != 1:
                errors.append(f"{identifier}: expected one traceability anchor: {anchor}")
    return errors


def _remove_clause(text, clause):
    marker = f"- {clause}\n"
    if text.count(marker) != 1:
        raise AssertionError(f"expected one clause to remove: {clause}")
    return text.replace(marker, "", 1)


def _mutate_clause(text, clause):
    marker = f"- {clause}\n"
    if text.count(marker) != 1:
        raise AssertionError(f"expected one clause to mutate: {clause}")
    mutated_clause = clause + " This guarantee is optional."
    return text.replace(marker, f"- {mutated_clause}\n", 1)


SCENARIO_MANIFEST = {
    ("ADR-008", "## History Restoration Scenarios"): {
        "cross_overlay_undo": "The before collection is restored atomically, the valid recorded overlay A and its recorded widget selection are restored exactly, and affected outputs receive fresh complete snapshots.",
        "absent_overlay_selection": "Select the overlay now at its recorded index, otherwise the last overlay, otherwise none; clear widget selection. A recorded null overlay remains null.",
        "absent_widget_selection": "Select the widget now at its recorded index, otherwise the last widget, otherwise none. A recorded null widget remains null.",
        "undo_overlay_delete": "Restore the original overlay with the same ID and URL, then restore the recorded before-selection exactly when valid, using the documented fallback only when absent.",
        "redo_overlay_delete": "Delete the overlay again, close its streams, and restore the recorded after-selection using same-index, last, or none; widget selection is clear when overlay fallback is required.",
        "redo_invalidation": "All redo entries are discarded while prior undo history remains subject to the 100-action bound.",
    },
    ("ADR-008", "## Save and Revision Scenarios"): {
        "save_edit_undo_clean": "The workspace is clean by content equality even if a corresponding history entry was evicted; revisions and selection do not affect cleanliness.",
        "failed_save_preserves_state": "Previous source, current work, history, dirty state, and a visible error all remain; no save action is added.",
        "pending_text_then_undo": "Commit the pending transaction first, then apply document Undo; text-native undo remains separate while the field has focus.",
        "pending_text_then_save": "Commit the pending transaction first, save the resulting whole collection, and retain history.",
        "restart_revision_initialization": "Loaded delivery revision/high-water starts at 0; initial publication may use 0; the accepted edit publishes revision 1; Undo restores content with fresh revision 2. No revision is restored from disk.",
    },
    ("FDR-003", "## Selection and Ordering Scenarios"): {
        "overlap_selection": "Select the frontmost hit by stable ID; an obscured widget remains selectable from the list.",
        "selected_widget_deletion": "Select the row now at its index, otherwise the preceding last row, otherwise none.",
        "overlay_switch_selection": "Clear widget selection; show overlay information/dimensions in the inspector.",
        "duplicate_front": "Retain properties/position, assign a new ID and copy-derived editable name, insert at index 0, and select it.",
        "layer_step": "Swap one adjacent item with no wrapping; list and renderers update consistently.",
        "drag_cancel": "Preserve grab offset during movement, restore pre-drag state, and add no history action.",
    },
    ("FDR-005", "## Settings Close and Lifecycle Scenarios"): {
        "settings_open": "Open one secondary native window showing running/saved state, path, errors, and restart requirement.",
        "settings_reopen": "Focus/reuse the existing window; preserve its current form edits and create no duplicate.",
        "settings_close_unsaved": "Discard form edits without another prompt; editor and server continue; errors remain available in the workspace.",
        "settings_save_next_launch": "Persist format-1 settings for next launch; running port and exact current URLs remain unchanged.",
        "settings_invalid": "Show visible validation/source errors; do not overwrite the source, start a fallback, choose an alternate, or live-rebind.",
        "editor_close_cancel": "Leave editor, Settings, server, and work running.",
        "editor_close_complete": "Close Settings, stop and join server, and exit; Discard writes no document changes.",
    },
}


def _scenario_rows(text, heading):
    lines = text.splitlines()
    if lines.count(heading) != 1:
        return {}, [f"expected one scenario section {heading}"]
    start = lines.index(heading) + 1
    end = next((index for index in range(start, len(lines)) if lines[index].startswith("## ")), len(lines))
    table_lines = [line for line in lines[start:end] if line.startswith("|")]
    errors = []
    expected_headers = (["Scenario ID", "Starting condition and action", "Required outcome"], ["Scenario ID", "Action", "Required outcome"])
    headers = [cell.strip() for cell in table_lines[0].strip("|").split("|")] if table_lines else []
    if len(table_lines) < 2 or headers not in expected_headers:
        errors.append(f"unexpected scenario table headers in {heading}: {headers}")
    rows = {}
    for line in table_lines[2:]:
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        if len(cells) != 3:
            errors.append(f"malformed scenario row in {heading}: {line}")
            continue
        match = re.fullmatch(r"`([a-z0-9_]+)`", cells[0])
        if not match:
            errors.append(f"invalid scenario ID cell in {heading}: {cells[0]}")
            continue
        scenario = match.group(1)
        if scenario in rows:
            errors.append(f"duplicate scenario ID in {heading}: {scenario}")
        rows[scenario] = cells[2]
    return rows, errors


def _check_scenario_manifest(documents):
    errors = []
    for (identifier, heading), expected_rows in SCENARIO_MANIFEST.items():
        rows, row_errors = _scenario_rows(documents.get(identifier, ""), heading)
        errors.extend(f"{identifier}: {error}" for error in row_errors)
        if set(rows) != set(expected_rows):
            errors.append(f"{identifier} {heading}: scenario IDs differ: {sorted(rows)}")
        for scenario, outcome in expected_rows.items():
            if rows.get(scenario) != outcome:
                errors.append(f"{identifier} {heading}: wrong outcome for {scenario}: {rows.get(scenario)}")
    return errors


def _remove_scenario_row(text, scenario):
    lines = text.splitlines(keepends=True)
    matches = [line for line in lines if line.startswith(f"| `{scenario}` |")]
    if len(matches) != 1:
        raise AssertionError(f"expected one scenario row to remove: {scenario}")
    return "".join(line for line in lines if line != matches[0])


def _mutate_scenario_outcome(text, scenario, outcome):
    marker = f"| `{scenario}` |"
    lines = text.splitlines(keepends=True)
    matching_indexes = [index for index, line in enumerate(lines) if line.startswith(marker)]
    if len(matching_indexes) != 1:
        raise AssertionError(f"expected one scenario row to mutate: {scenario}")
    index = matching_indexes[0]
    if outcome not in lines[index]:
        raise AssertionError(f"expected outcome not found for {scenario}")
    lines[index] = lines[index].replace(outcome, "Material outcome removed.", 1)
    return "".join(lines)


class Milestone002DecisionContractTests(unittest.TestCase):
    def _documents(self):
        return {identifier: path.read_text(encoding="utf-8") for identifier, path in DECISION_002_PATHS.items()}

    def test_002_decision_record_set(self):
        documents = self._documents()
        governed_documents = {identifier: documents[identifier] for identifier in DECISION_002_MANIFEST}
        self.assertEqual(_check_002_manifest(governed_documents), [])
        for identifier, text in documents.items():
            self.assertIn("**Status:** Accepted", text, identifier)
            self.assertIn("**Date:** 2026-09-17", text, identifier)
        for identifier, entries in DECISION_002_MANIFEST.items():
            for heading, clause in entries:
                with self.subTest(identifier=identifier, heading=heading, clause=clause, mutation="removal"):
                    mutated = dict(governed_documents)
                    mutated[identifier] = _remove_clause(documents[identifier], clause)
                    self.assertTrue(_check_002_manifest(mutated), f"removing substantive clause passed: {clause}")
                with self.subTest(identifier=identifier, heading=heading, clause=clause, mutation="material"):
                    mutated = dict(governed_documents)
                    mutated[identifier] = _mutate_clause(documents[identifier], clause)
                    self.assertTrue(_check_002_manifest(mutated), f"materially mutating substantive clause passed: {clause}")
        adr_index = (ROOT / "docs/adr/INDEX.md").read_text(encoding="utf-8")
        fdr_index = (ROOT / "docs/fdr/INDEX.md").read_text(encoding="utf-8")
        self.assertIn("[Superseded by ADR-006](ADR-006-ordered-authoritative-widget-model.md)", adr_index)
        self.assertIn("[Superseded by ADR-007](ADR-007-version-2-overlay-document-persistence.md)", adr_index)
        self.assertIn("[Superseded by FDR-003](FDR-003-multi-widget-composition-workspace.md)", fdr_index)
        for unchanged in ("ADR-001", "ADR-004", "ADR-005"):
            self.assertRegex(adr_index, rf"(?m)^\| \[{unchanged}\].*\| Accepted \|")
        self.assertRegex(fdr_index, r"(?m)^\| \[FDR-002\].*\| Accepted \|")

    def _assert_scenario_mutations_fail(self, documents, owners):
        selected = {key: rows for key, rows in SCENARIO_MANIFEST.items() if key[0] in owners}
        self.assertEqual(_check_scenario_manifest(documents), [])
        for (identifier, heading), rows in selected.items():
            for scenario, outcome in rows.items():
                with self.subTest(identifier=identifier, heading=heading, scenario=scenario, mutation="row removal"):
                    mutated = dict(documents)
                    mutated[identifier] = _remove_scenario_row(documents[identifier], scenario)
                    self.assertTrue(_check_scenario_manifest(mutated), f"removing {scenario} row passed")
                with self.subTest(identifier=identifier, heading=heading, scenario=scenario, mutation="outcome"):
                    mutated = dict(documents)
                    mutated[identifier] = _mutate_scenario_outcome(documents[identifier], scenario, outcome)
                    self.assertTrue(_check_scenario_manifest(mutated), f"mutating {scenario} outcome passed")

    def test_002_history_persistence_contracts(self):
        documents = self._documents()
        self._assert_scenario_mutations_fail(documents, {"ADR-008"})
        history = documents["ADR-008"]
        persistence = documents["ADR-007"]
        for clause in (
            "The timeline holds at most 100 completed actions total across undo and redo. Capacity overflow evicts the oldest undo entries.",
            "No-op interactions create no entry. A new document-changing action after undo invalidates redo. Saving does not create an action and does not clear history.",
            "The dirty baseline is persistent document-content equality with the last successful whole-collection save; revisions and transient session state do not participate.",
            "There is no format-1 conversion. Unknown versions and malformed data fail visibly and non-destructively.",
            "Server settings remain a separate format-1 `settings.json` envelope in the platform config location under ADR-005; this decision changes only overlay documents.",
        ):
            self.assertIn(f"- {clause}\n", history + persistence, clause)

    def test_002_workspace_settings_contracts(self):
        documents = self._documents()
        self._assert_scenario_mutations_fail(documents, {"FDR-003", "FDR-005"})
        workspace = documents["FDR-003"]
        settings = documents["FDR-005"] + documents["ADR-009"]
        for anchor in (
            "Index 0 and the top list row are frontmost",
            "Dragging preserves the initial grab offset",
            "Primary+S",
            "Primary+Z",
            "Primary+Shift+Z",
            "Primary+D",
            "Delete/Backspace outside text entry only",
            "1280×800",
            "1024×640",
            "8-point base spacing",
            "12-point panel padding",
            "14-point body text",
            "cyan selection outline",
        ):
            self.assertIn(anchor, workspace, anchor)
        for anchor in (
            "discard",
            "next launch",
            "does not live-rebind",
            "format-1",
            "main workspace",
            "macOS and Linux",
        ):
            self.assertIn(anchor, settings, anchor)

    def test_002_font_contract(self):
        font = self._documents()["FDR-004"]
        model = self._documents()["ADR-006"]
        expected = (
            "ffebf8c1ee449e544955a7e813c54f9b73848eac",
            "hinted/ttf/NotoSans/NotoSans-Regular.ttf",
            "569208",
            "https://raw.githubusercontent.com/notofonts/noto-fonts/ffebf8c1ee449e544955a7e813c54f9b73848eac/hinted/ttf/NotoSans/NotoSans-Regular.ttf",
            "https://raw.githubusercontent.com/notofonts/noto-fonts/ffebf8c1ee449e544955a7e813c54f9b73848eac/LICENSE",
            "cd5227bd1f61dff3bbd6c814ceaf7ffd95e947d9",
            "fonts/ttf/JetBrainsMono-Regular.ttf",
            "273900",
            "https://raw.githubusercontent.com/JetBrains/JetBrainsMono/cd5227bd1f61dff3bbd6c814ceaf7ffd95e947d9/fonts/ttf/JetBrainsMono-Regular.ttf",
            "https://raw.githubusercontent.com/JetBrains/JetBrainsMono/cd5227bd1f61dff3bbd6c814ceaf7ffd95e947d9/OFL.txt",
            "SIL Open Font License 1.1",
            "U+0020–U+007E and U+00A0–U+00FF",
            "U+FFFD",
            "Coverage is a requirement, not a completed font audit.",
        )
        for anchor in expected:
            self.assertIn(anchor, font, anchor)
        self.assertIn("base64 data URLs", model)
        self.assertIn("`GET /overlay/{id}` and `GET /overlay/{id}/events`", model)
        self.assertIn("no HTTP routes", model)
        self.assertIn("#26", font)
        self.assertNotIn("SHA-256 values are", font)

    def test_002_milestone_contract_links(self):
        milestone = (ROOT / "docs/TODO-0-0-2.md").read_text(encoding="utf-8")
        self.assertIn("**Status:** Planned — implementation pending", milestone)
        for path in (
            "fdr/FDR-003-multi-widget-composition-workspace.md",
            "fdr/FDR-004-bundled-offline-text-fonts.md",
            "fdr/FDR-005-singleton-native-settings-window.md",
            "adr/ADR-006-ordered-authoritative-widget-model.md",
            "adr/ADR-007-version-2-overlay-document-persistence.md",
            "adr/ADR-008-coordinator-owned-workspace-history.md",
            "adr/ADR-009-secondary-native-settings-viewport-lifecycle.md",
        ):
            self.assertIn(path, milestone, path)
        for issue in range(22, 28):
            self.assertIn(f"| #{issue} |", milestone, issue)
        product_section = milestone[milestone.index("## Product Requirements"):milestone.index("## Confirmed Scope")]
        quality_section = milestone[milestone.index("## Quality Requirements"):milestone.index("## Explicitly Out of Scope")]
        self.assertNotRegex(product_section + quality_section, r"(?m)^- \[[xX]\]")
        self.assertIn("do not exercise future runtime features", milestone)


class CollaborationArtifactContractTests(unittest.TestCase):
    def _skill(self, name):
        return (ROOT / ".agents" / "skills" / name / "SKILL.md").read_text(encoding="utf-8")

    def _workflow_job(self, workflow, name):
        lines = workflow.splitlines()
        start = lines.index(f"  {name}:")
        end = next((index for index in range(start + 1, len(lines)) if re.fullmatch(r"  [A-Za-z0-9_-]+:", lines[index])), len(lines))
        return "\n".join(lines[start:end])

    def _assert_skill_frontmatter(self, text):
        lines = text.splitlines()
        self.assertGreaterEqual(len(lines), 4)
        self.assertEqual(lines[0], "---")
        self.assertRegex(lines[1], r"^description:\s+\S")
        self.assertEqual(lines[2], "---")

    def _workflow_run_scripts(self, workflow):
        lines = workflow.splitlines()
        scripts = []
        index = 0
        while index < len(lines):
            match = re.match(r"^(\s+)run:\s*(.*)$", lines[index])
            if not match:
                index += 1
                continue
            run_indent = len(match.group(1))
            body = [match.group(2)] if match.group(2) else []
            index += 1
            while index < len(lines):
                line = lines[index]
                if line.strip() and len(line) - len(line.lstrip()) <= run_indent:
                    break
                body.append(line.strip())
                index += 1
            scripts.append("\n".join(body))
        return scripts

    def test_agents_github_collaboration_policy(self):
        text = (ROOT / "AGENTS.md").read_text(encoding="utf-8")
        heading = "## Commits, Issues, and Pull Requests"
        self.assertEqual(text.count(heading), 1)
        collaboration = text[text.index(heading):]
        self.assertIn("Use or update GitHub issues only when the user asks for issue or roadmap management, or when an explicitly invoked workflow requires it.", collaboration)
        self.assertIn("full, ready-for-review", collaboration)
        self.assertIn("Create a draft only when the user explicitly asks for a draft.", collaboration)
        self.assertIn("link relevant FDRs, ADRs, glossary terms, and issues", collaboration)
        self.assertIn("GitHub closing keyword", collaboration)
        self.assertIn("Closes #123.", collaboration)
        self.assertIn("--body-file", collaboration)
        self.assertIn("never encode newlines", collaboration)
        self.assertIn("Do not rename the current branch unless explicitly stated.", collaboration)
        navigation = text[text.index("### GitHub collaboration skills"):text.index("## Project Status")]
        for skill in ("github-triage", "github-issue-orchestrator", "github-pr-checklist"):
            self.assertIn(f".agents/skills/{skill}/SKILL.md", navigation)

    def test_github_triage_skill_contract(self):
        text = self._skill("github-triage")
        self._assert_skill_frontmatter(text)
        for clause in (
            "explicitly invoked",
            "docs/TODO-0-0-1.md",
            "complete TODO",
            "product requirements",
            "quality requirements",
            "explicitly out-of-scope",
            "completion gate",
            "current code, tests, and documentation",
            "unchecked",
            "existing open and relevant closed GitHub issues",
            "gh issue list",
            "avoid duplicates",
            "user-visible feature work",
            "architecture or infrastructure work",
            "one issue per checkbox",
            "Acceptance criteria",
            "References",
            "Dependencies / sequencing",
            "gh issue create",
            "--body-file",
            "existing repository labels",
            "unresolved",
            "deferred",
            "Do not silently mark the TODO complete",
        ):
            self.assertIn(clause, text, clause)

    def test_github_issue_orchestrator_skill_contract(self):
        text = self._skill("github-issue-orchestrator")
        self._assert_skill_frontmatter(text)
        for clause in (
            "issue number or URL",
            "gh issue view",
            "--comments",
            "metadata",
            "AGENTS.md",
            "docs/TODO-*.md",
            "ADRs",
            "FDRs",
            "docs/GLOSSARY.md",
            "docs/architecture/",
            "bounded",
            "Explicit ownership of repository paths",
            "tests",
            "documentation",
            "sequential delegation",
            "parallel delegation",
            "dependency-aware",
            "worktree",
            "dedicated branch",
            "commit",
            "Read-only research and review",
            "unnecessary worktrees",
            "not create or update GitHub issues or pull requests",
            "rename branches",
            "broaden scope",
            "acceptance criteria",
            "Remaining risks",
            "Never create a draft PR implicitly",
        ):
            self.assertIn(clause, text, clause)
        self.assertNotIn("gh issue create", text)
        self.assertNotIn("gh pr create", text)

    def test_github_pr_checklist_skill_contract(self):
        text = self._skill("github-pr-checklist")
        self._assert_skill_frontmatter(text)
        for clause in (
            "complete branch diff",
            "actionable test gaps",
            "Add or fix tests when authorized",
            "manual validation",
            "AGENTS.md",
            "docs/TODO-*.md",
            "ADRs",
            "FDRs",
            "docs/GLOSSARY.md",
            "docs/architecture/",
            "Why / problem",
            "What changed",
            "Test plan and exact results",
            "Compatibility, security, operational, and rollout implications",
            "Link relevant FDRs, ADRs, glossary terms, architecture inventory pages",
            "Closes #123.",
            "gh pr create",
            "--body-file",
            "--draft",
            "user explicitly requests a draft",
            "gh pr view",
            "Do not rename the current branch",
            "repository rule or instruction update",
            "only actionable findings",
        ):
            self.assertIn(clause, text, clause)

    def test_ci_workflow_contract(self):
        text = (ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
        self.assertRegex(text, r"(?m)^on:\s*$")
        self.assertRegex(text, r"(?m)^  push:\s*$")
        self.assertRegex(text, r"(?m)^  pull_request:\s*$")
        self.assertRegex(text, r"(?m)^permissions:\n  contents: read\n")
        rust = self._workflow_job(text, "rust")
        documentation = self._workflow_job(text, "documentation")
        self.assertIn("runs-on: ${{ matrix.os }}", rust)
        self.assertIn("os: [ubuntu-latest, macos-latest]", rust)
        self.assertIn("uses: actions/checkout@", rust)
        self.assertIn("uses: dtolnay/rust-toolchain@stable", rust)
        self.assertIn("toolchain: stable", rust)
        self.assertIn("components: rustfmt", rust)
        self.assertIn("cargo fmt --all -- --check", rust)
        self.assertIn("cargo test --locked --all-targets", rust)
        self.assertIn("if: runner.os == 'Linux'", rust)
        for package in (
            "libxcb-render0-dev",
            "libxcb-shape0-dev",
            "libxcb-xfixes0-dev",
            "libxkbcommon-dev",
            "libssl-dev",
            "libgtk-3-dev",
        ):
            self.assertIn(package, rust)
        self.assertLess(rust.index("uses: actions/checkout@"), rust.index("cargo fmt"))
        self.assertLess(rust.index("uses: actions/checkout@"), rust.index("cargo test"))
        self.assertIn("runs-on: ubuntu-latest", documentation)
        self.assertIn("uses: actions/checkout@", documentation)
        self.assertIn("python3 scripts/check_docs.py", documentation)
        self.assertIn("python3 -m unittest discover -s tests -v", documentation)
        self.assertLess(documentation.index("uses: actions/checkout@"), documentation.index("python3 scripts/check_docs.py"))
        self.assertLess(documentation.index("uses: actions/checkout@"), documentation.index("python3 -m unittest"))

    def test_release_workflow_contract(self):
        workflow = (ROOT / ".github" / "workflows" / "release.yml").read_text(encoding="utf-8")
        release_config = (ROOT / ".github" / "release.yml").read_text(encoding="utf-8")
        guide = (ROOT / "docs" / "RELEASING.md").read_text(encoding="utf-8")

        self.assertRegex(workflow, r"(?m)^  workflow_dispatch:\s*$")
        self.assertIn("ref: ${{ inputs.tag }}", workflow)
        self.assertIn("fetch-depth: 0", workflow)
        self.assertIn("contents: read", workflow)
        publish = self._workflow_job(workflow, "publish")
        self.assertIn("needs: [rust, documentation]", publish)
        self.assertIn("contents: write", publish)
        self.assertIn("--verify-tag", publish)
        self.assertIn("--generate-notes", publish)
        self.assertIn("gh release create", publish)
        self.assertIn("gh release view", publish)
        self.assertIn("Cargo package version", workflow)
        self.assertIn("existing annotated Git tags", guide)
        self.assertIn("Never move, replace, or reuse", guide)
        for label in ("enhancement", "bug", "documentation", '"*"'):
            self.assertIn(f"- {label}", release_config)

    def test_setup_artifact_mutation_contract(self):
        workflow = (ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
        agents = (ROOT / "AGENTS.md").read_text(encoding="utf-8")
        prohibited_commands = r"\b(?:gh issue|gh pr|git push|git branch (?:-m|--move)|cargo publish|docker push|kubectl apply)\b"
        for script in self._workflow_run_scripts(workflow):
            self.assertNotRegex(script, prohibited_commands)
        self.assertNotRegex(agents, r"(?m)^\s+run:.*" + prohibited_commands)
        triage = self._skill("github-triage")
        pr = self._skill("github-pr-checklist")
        orchestrator = self._skill("github-issue-orchestrator")
        self.assertIn("gh issue create", triage)
        self.assertIn("explicitly invoked", triage)
        self.assertIn("--body-file", triage)
        self.assertIn("gh pr create", pr)
        self.assertIn("full, ready-for-review PR", pr)
        self.assertIn("--body-file", pr)
        self.assertNotRegex(orchestrator, r"\bgh (?:issue|pr) (?:create|edit|close|reopen)\b")


if __name__ == "__main__":
    unittest.main()
