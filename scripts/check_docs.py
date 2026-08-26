#!/usr/bin/env python3
"""Validate the repository's decision records, indexes, skills, and links."""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from datetime import date as date_type
from pathlib import Path
from urllib.parse import unquote, urlsplit


IDENTIFIER_RE = re.compile(r"^(ADR|FDR)-\d{3}$")
FILENAME_RE = re.compile(r"^(ADR|FDR)-(\d{3})-([a-z0-9]+(?:-[a-z0-9]+)*)\.md$")
DATE_RE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
METADATA_RE = re.compile(r"^\*\*(Status|Date|Supersedes):\*\*\s*(.*?)\s*$")
H1_RE = re.compile(r"^#\s+((?:ADR|FDR)-\d{3}):\s*(.+?)\s*$")
LINK_RE = re.compile(r"(?<!!)(?:\[([^\]]*)\])\(([^)]+)\)")


@dataclass
class Record:
    kind: str
    identifier: str
    path: Path
    title: str
    status: str
    date: str
    supersedes: str
    text: str
    lines: list[str]


@dataclass
class IndexRow:
    kind: str
    identifier: str
    target: Path
    title: str
    status: str
    date: str
    status_cell: str
    line_number: int


def _display(path: Path, root: Path) -> str:
    try:
        return str(path.relative_to(root))
    except ValueError:
        return str(path)


def _error(errors: list[str], path: Path, root: Path, message: str) -> None:
    errors.append(f"{_display(path, root)}: {message}")


def _resolve_link(root: Path, document: Path, destination: str) -> Path | None:
    """Resolve a relative local destination, or return None for non-file links."""
    destination = destination.strip()
    if destination.startswith("<"):
        end = destination.find(">")
        if end < 0:
            return None
        destination = destination[1:end]
    else:
        # Markdown permits an optional title after the destination.
        destination = destination.split(None, 1)[0] if destination else ""
    if not destination or destination.startswith("//"):
        return None
    parsed = urlsplit(destination)
    if parsed.scheme or parsed.netloc or parsed.path.startswith("/"):
        return None
    # Decode each segment independently: encoded spaces work, while an encoded
    # traversal still resolves normally and is rejected by the root checks.
    segments = [unquote(segment) for segment in parsed.path.split("/")]
    if any("\x00" in segment for segment in segments):
        return None
    path_part = "/".join(segments)
    if not path_part:
        return None
    return (document.parent / path_part).resolve()


def _within(root: Path, path: Path) -> bool:
    try:
        path.resolve().relative_to(root.resolve())
    except ValueError:
        return False
    return True


def _links_outside_fences(text: str):
    in_fence = False
    fence_marker = ""
    for line_number, line in enumerate(text.splitlines(), 1):
        fence = re.match(r"^\s{0,3}(`{3,}|~{3,})", line)
        if fence:
            marker = fence.group(1)[0]
            if not in_fence:
                in_fence = True
                fence_marker = marker
            elif marker == fence_marker:
                in_fence = False
            continue
        if not in_fence:
            for match in LINK_RE.finditer(line):
                yield line_number, match.group(1), match.group(2)


def _check_links(root: Path, errors: list[str]) -> None:
    for document in sorted(root.rglob("*.md")):
        text = document.read_text(encoding="utf-8")
        for line_number, label, destination in _links_outside_fences(text):
            target = _resolve_link(root, document, destination)
            if target is None:
                continue
            if not _within(root, target) or not target.is_file():
                _error(
                    errors,
                    document,
                    root,
                    f"line {line_number}: broken local link [{label}]({destination})",
                )


def _check_skills(root: Path, errors: list[str]) -> None:
    for skill in sorted(root.glob(".agents/skills/*/SKILL.md")):
        lines = skill.read_text(encoding="utf-8").splitlines()
        if not lines or lines[0] != "---":
            _error(errors, skill, root, "frontmatter must start with ---")
            continue
        closing = next((i for i in range(1, len(lines)) if lines[i] == "---"), None)
        if closing is None:
            _error(errors, skill, root, "frontmatter is missing a closing ---")
            continue
        if closing == 1:
            _error(errors, skill, root, "frontmatter is empty")
        descriptions = [line for line in lines[1:closing] if re.match(r"^description:", line)]
        if len(descriptions) != 1:
            _error(errors, skill, root, "frontmatter must contain exactly one description")
        else:
            match = re.fullmatch(r"description:\s*(.*)", descriptions[0])
            value = match.group(1).strip() if match else ""
            if not value:
                _error(errors, skill, root, "frontmatter description must be non-empty")
            if value in {"|", ">", "|-", ">-", "|+", ">+"}:
                _error(errors, skill, root, "frontmatter description must be single-line")
            description_line = lines.index(descriptions[0])
            if any(line[:1].isspace() for line in lines[description_line + 1 : closing]):
                _error(errors, skill, root, "frontmatter description must be single-line")
        if any(line.startswith("---") and line != "---" for line in lines[: closing + 1]):
            _error(errors, skill, root, "frontmatter delimiters must be standalone --- lines")


def _metadata(path: Path, root: Path, errors: list[str], lines: list[str]) -> dict[str, str]:
    values: dict[str, str] = {}
    for line in lines:
        match = METADATA_RE.match(line)
        if not match:
            continue
        key, value = match.groups()
        if key in values:
            _error(errors, path, root, f"duplicate metadata field {key}")
        values[key] = value
    for key in ("Status", "Date", "Supersedes"):
        if key not in values:
            _error(errors, path, root, f"missing metadata field {key}")
    if values.get("Status") not in {"Proposed", "Accepted"}:
        _error(errors, path, root, f"invalid record status {values.get('Status', '<missing>')!r}")
    date = values.get("Date", "")
    if not DATE_RE.fullmatch(date):
        _error(errors, path, root, f"date must be ISO YYYY-MM-DD, got {date!r}")
    else:
        try:
            date_type.fromisoformat(date)
        except ValueError:
            _error(errors, path, root, f"date is not calendar-valid: {date!r}")
    supersedes = values.get("Supersedes", "")
    if supersedes != "None" and not IDENTIFIER_RE.fullmatch(supersedes):
        _error(errors, path, root, f"Supersedes must be None or a record identifier, got {supersedes!r}")
    elif supersedes != "None" and not supersedes.startswith(path.stem[:3] + "-"):
        _error(errors, path, root, f"Supersedes must name a same-type {path.stem[:3]} record, got {supersedes!r}")
    return values


def _section_body(lines: list[str], heading: str) -> list[str]:
    start = next((i for i, line in enumerate(lines) if line.strip() == heading), None)
    if start is None:
        return []
    heading_match = re.match(r"^(#+)\s+", heading)
    level = len(heading_match.group(1)) if heading_match else 2
    end = len(lines)
    for i in range(start + 1, len(lines)):
        next_heading = re.match(r"^(#+)\s+", lines[i])
        if next_heading and len(next_heading.group(1)) <= level:
            end = i
            break
    return lines[start + 1 : end]


def _check_record(path: Path, root: Path, errors: list[str]) -> Record | None:
    filename = path.name
    match = FILENAME_RE.fullmatch(filename)
    if not match:
        _error(errors, path, root, "filename must be ADR-NNN-kebab-case.md or FDR-NNN-kebab-case.md")
        return None
    kind, number, _slug = match.groups()
    identifier = f"{kind}-{number}"
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines()
    h1s = [H1_RE.match(line) for line in lines if H1_RE.match(line)]
    if not h1s:
        _error(errors, path, root, f"missing H1 identifier {identifier}")
        title = ""
    else:
        if len(h1s) != 1:
            _error(errors, path, root, "record must contain exactly one matching H1")
        heading_identifier, title = h1s[0].groups()
        if heading_identifier != identifier:
            _error(errors, path, root, f"H1 identifier {heading_identifier} does not match filename {identifier}")
        if not title.strip():
            _error(errors, path, root, "H1 title must be non-empty")
    values = _metadata(path, root, errors, lines)
    status = values.get("Status", "")
    date = values.get("Date", "")
    supersedes = values.get("Supersedes", "")
    required = ["## Overview", "## User-visible Behavior", "## Feature Decisions", "## Open Questions", "## Related"] if kind == "FDR" else ["## Context", "## Decision", "## Rationale", "## Alternatives Considered", "## Consequences", "## Related"]
    for heading in required:
        if heading not in lines:
            _error(errors, path, root, f"missing required section {heading}")
        elif not any(line.strip() for line in _section_body(lines, heading)):
            _error(errors, path, root, f"required section {heading} must be non-empty")
    if kind == "FDR":
        decisions = []
        feature_start = next((i for i, line in enumerate(lines) if line.strip() == "## Feature Decisions"), -1)
        feature_end = next((i for i in range(feature_start + 1, len(lines)) if re.match(r"^##\s+", lines[i])), len(lines))
        for i, line in enumerate(lines[feature_start + 1 : feature_end], feature_start + 1):
            decision = re.match(r"^###\s+(\d+)\.\s+(.+?)\s*$", line)
            if decision:
                number_text, decision_title = decision.groups()
                if not decision_title.strip():
                    _error(errors, path, root, f"decision {number_text} title must be non-empty")
                decisions.append((int(number_text), i))
        if not decisions:
            _error(errors, path, root, "FDR must contain numbered feature decisions")
        else:
            expected = list(range(1, len(decisions) + 1))
            actual = [number for number, _ in decisions]
            if actual != expected:
                _error(errors, path, root, "FDR decision headings must be numbered consecutively from 1")
            for position, (number, start) in enumerate(decisions):
                end = decisions[position + 1][1] if position + 1 < len(decisions) else len(lines)
                body = lines[start + 1 : end]
                for field in ("Decision", "Why", "Tradeoff"):
                    matches = [line for line in body if re.match(rf"^\*\*{field}:\*\*\s*(.*?)\s*$", line)]
                    if len(matches) != 1 or not re.match(rf"^\*\*{field}:\*\*\s*\S", matches[0] if matches else ""):
                        _error(errors, path, root, f"decision {number} must have one non-empty {field} field")
    else:
        consequences = _section_body(lines, "## Consequences")
        positive = _section_body(consequences, "### Positive")
        negative = _section_body(consequences, "### Negative")
        if not positive:
            _error(errors, path, root, "ADR Consequences must contain a Positive subsection")
        elif not any(line.strip() for line in positive):
            _error(errors, path, root, "ADR Positive consequences must be non-empty")
        if not negative:
            _error(errors, path, root, "ADR Consequences must contain a Negative subsection")
        elif not any(line.strip() for line in negative):
            _error(errors, path, root, "ADR Negative consequences must be non-empty")
    return Record(kind, identifier, path, title, status, date, supersedes, text, lines)


def _table_rows(index: Path):
    lines = index.read_text(encoding="utf-8").splitlines()
    header = None
    for i, line in enumerate(lines):
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if cells and cells[0] == "Record":
            header = (i, cells)
            break
    if header is None:
        return [], None
    header_line, columns = header
    rows = []
    for i in range(header_line + 1, len(lines)):
        line = lines[i]
        if not line.strip().startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if len(cells) != len(columns) or all(set(cell) <= {"-", ":", " "} for cell in cells):
            continue
        rows.append((i + 1, cells))
    return rows, columns


def _first_link(cell: str):
    match = LINK_RE.search(cell)
    return (match.group(1), match.group(2)) if match else (None, None)


def _check_index(kind: str, index: Path, root: Path, records: dict[str, Record], errors: list[str]) -> None:
    rows, columns = _table_rows(index)
    expected_columns = ["Record", "Decision", "Status", "Date"] if kind == "ADR" else ["Record", "Feature", "Status", "Date"]
    if columns != expected_columns:
        _error(errors, index, root, f"index columns must be {' | '.join(expected_columns)}")
        return
    seen_paths: list[Path] = []
    seen_ids: set[str] = set()
    row_by_id: dict[str, IndexRow] = {}
    for line_number, cells in rows:
        label, destination = _first_link(cells[0])
        if not label or not destination:
            _error(errors, index, root, "index row Record cell must contain a link")
            continue
        id_match = re.fullmatch(rf"{kind}-\d{{3}}", label)
        if not id_match:
            _error(errors, index, root, f"index identifier {label!r} does not match {kind} identifier format")
            continue
        identifier = label
        if identifier in seen_ids:
            _error(errors, index, root, f"record {identifier} is indexed more than once")
        seen_ids.add(identifier)
        target = _resolve_link(root, index, destination)
        if target is None or not _within(root, target) or not target.is_file():
            _error(errors, index, root, f"index target for {identifier} does not exist: {destination}")
            continue
        filename_match = FILENAME_RE.fullmatch(target.name)
        if not filename_match or f"{filename_match.group(1)}-{filename_match.group(2)}" != identifier:
            _error(errors, index, root, f"index identifier {identifier} does not agree with target filename")
        if target in seen_paths:
            _error(errors, index, root, f"index target {_display(target, root)} is listed more than once")
        seen_paths.append(target)
        title = cells[1]
        status_cell = cells[2]
        status_label, status_destination = _first_link(status_cell)
        status = status_label or status_cell
        date = cells[3]
        record = records.get(identifier)
        if record is None:
            _error(errors, index, root, f"indexed record {identifier} is not a valid record")
            continue
        if title != record.title:
            _error(errors, index, root, f"index title for {identifier} does not agree with record H1 title")
        if date != record.date:
            _error(errors, index, root, f"index date for {identifier} does not agree with record metadata")
        row = IndexRow(kind, identifier, target, title, status, date, status_cell, line_number)
        row_by_id[identifier] = row
        if status not in ({"Proposed", "Accepted"} if kind == "ADR" else {"Proposed", "Accepted", "Implemented", "Retired"}) and not status.startswith("Superseded by "):
            _error(errors, index, root, f"invalid {kind} index status {status!r}")
        if status.startswith("Superseded by "):
            successor_match = re.fullmatch(rf"Superseded by ({kind}-\d{{3}})", status)
            if not successor_match:
                _error(errors, index, root, f"supersession status must name a same-type {kind} successor")
            elif not status_label or not status_destination:
                _error(errors, index, root, f"supersession for {identifier} is missing a successor link")
            else:
                successor_id = successor_match.group(1)
                if successor_id == identifier:
                    _error(errors, index, root, "record cannot supersede itself")
                successor_target = _resolve_link(root, index, status_destination)
                if status != f"Superseded by {successor_id}":
                    _error(errors, index, root, f"supersession status must name {successor_id}")
                if successor_target is None or not successor_target.is_file():
                    _error(errors, index, root, f"supersession for {identifier} has a broken successor link")
    for identifier, record in records.items():
        if record.kind != kind:
            continue
        matching = [path for path in seen_paths if path == record.path.resolve()]
        if len(matching) != 1:
            _error(errors, record.path, root, "record must be indexed exactly once")
        row = row_by_id.get(identifier)
        if not row:
            continue
        if kind == "ADR":
            if row.status in {"Proposed", "Accepted"} and row.status != record.status:
                _error(errors, index, root, f"ADR {identifier} index status must equal record status")
        else:
            if record.status == "Proposed" and row.status != "Proposed":
                _error(errors, index, root, f"Proposed FDR {identifier} may only be indexed as Proposed")
            elif record.status == "Accepted" and row.status == "Proposed":
                _error(errors, index, root, f"Accepted FDR {identifier} may not be indexed as Proposed")
        if row.status.startswith("Superseded by "):
            if record.status != "Accepted":
                _error(errors, index, root, f"only an Accepted {kind} may be superseded")
            successor_id = row.status.removeprefix("Superseded by ")
            successor = records.get(successor_id)
            if successor is None:
                _error(errors, index, root, f"supersession successor {successor_id} is missing")
            else:
                if successor.supersedes != identifier:
                    _error(errors, successor.path, root, f"successor {successor_id} must declare Supersedes: {identifier}")
                label, destination = _first_link(row.status_cell)
                target = _resolve_link(root, index, destination or "")
                if target is None or target.resolve() != successor.path.resolve():
                    _error(errors, index, root, f"supersession for {identifier} must link to successor {successor_id}")


def check_tree(root: str | Path) -> list[str]:
    """Return validation errors for *root*; an empty list means valid."""
    root = Path(root).resolve()
    errors: list[str] = []
    _check_skills(root, errors)
    _check_links(root, errors)
    records: dict[str, Record] = {}
    for kind, directory in (("ADR", root / "docs" / "adr"), ("FDR", root / "docs" / "fdr")):
        if not directory.exists():
            continue
        for path in sorted(directory.glob("*.md")):
            if path.name == "INDEX.md":
                continue
            record = _check_record(path, root, errors)
            if record:
                if record.identifier in records:
                    _error(errors, path, root, f"duplicate record identifier {record.identifier}")
                records[record.identifier] = record
        index = directory / "INDEX.md"
        if index.exists():
            _check_index(kind, index, root, records, errors)
        elif any(record.kind == kind for record in records.values()):
            _error(errors, directory, root, f"missing {kind} index")
    return errors


def main(argv: list[str] | None = None) -> int:
    root = Path(__file__).resolve().parents[1]
    errors = check_tree(root)
    if errors:
        print("Documentation validation failed:")
        print("\n".join(f"- {error}" for error in errors))
        return 1
    print("Documentation validation passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
