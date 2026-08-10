#!/usr/bin/env python3
"""Scan Sprint 2 fixtures and evidence for prohibited active or private content."""

from __future__ import annotations

import argparse
import ast
import io
import json
import os
import re
import stat
import sys
import tempfile
import zipfile
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Any
from urllib.parse import urlsplit


ROOT = Path(__file__).resolve().parents[1]
REPORT_PATH = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-2"
    / "story-2.1"
    / "fixture-security-scan-report.json"
)
FINAL_EVIDENCE_ENVELOPE = (
    "artifacts/sprints/sprint-2/story-2.1/security-evidence-map.json",
    "artifacts/sprints/sprint-2/story-2.1/summary-comparison.json",
    "artifacts/sprints/sprint-2/story-2.1/summary-comparison-public.pem",
    "artifacts/sprints/sprint-2/story-2.1/summary-comparison.sig",
    "artifacts/sprints/sprint-2/story-2.1/summary-comparison-signature.json",
)
FINAL_EVIDENCE_VERIFIER = "scripts/story_2_1_security_evidence.py"
CORPUS_PATH = ROOT / "fixtures/corpus/v1/agentmage-synthetic-corpus-v1.zip"
EXPECTED_CATEGORIES = (
    "active-formula",
    "hidden-executable",
    "network-behavior",
    "private-path",
    "real-credential",
    "remote-reference",
)
EXECUTABLE_EXTENSIONS = {
    ".app",
    ".bat",
    ".cmd",
    ".com",
    ".dll",
    ".dylib",
    ".exe",
    ".msi",
    ".ps1",
    ".so",
}
CODE_EXTENSIONS = {".go", ".js", ".py", ".rs", ".sh", ".ts"}
NETWORK_IMPORTS = {
    "aiohttp",
    "ftplib",
    "http.client",
    "httpx",
    "requests",
    "smtplib",
    "socket",
    "subprocess",
    "urllib.request",
}
BINARY_MAGICS = (
    bytes((0x7F, 0x45, 0x4C, 0x46)),
    bytes((0x4D, 0x5A)),
    bytes((0xCF, 0xFA, 0xED, 0xFE)),
    bytes((0xFE, 0xED, 0xFA, 0xCF)),
)
SECRET_PATTERNS = (
    re.compile(rb"-----BEGIN [A-Z ]*PRIVATE KEY-----"),
    re.compile(rb"\bAKIA[0-9A-Z]{16}\b"),
    re.compile(rb"\bgh[pousr]_[A-Za-z0-9]{20,}\b"),
    re.compile(rb"\bxox[baprs]-[A-Za-z0-9-]{10,}\b"),
    re.compile(rb"\bsk-[A-Za-z0-9_-]{20,}\b"),
    re.compile(rb"\bBearer\s+[A-Za-z0-9._-]{16,}\b", re.IGNORECASE),
)
PRIVATE_PATH = re.compile(
    rb"(?:^|[\s\"'=:(])/(?:home|Users|var/home)/[A-Za-z0-9._-]+/"
)
URL = re.compile(r"https?://[^\s\"'<>]+", re.IGNORECASE)
SYNTHETIC_CANARY = re.compile(rb"AM_SYNTHETIC_(?:CANARY|SECRET)_[A-Z0-9_]+")


@dataclass(frozen=True)
class ScanFinding:
    category: str
    path: str
    reason: str

    def as_record(self) -> dict[str, str]:
        return {
            "category": self.category,
            "path": self.path,
            "reason": self.reason,
        }


@dataclass
class ScanMetrics:
    files_scanned: int = 0
    archive_entries_scanned: int = 0
    nested_archives_scanned: int = 0
    text_payloads_scanned: int = 0
    python_modules_parsed: int = 0
    malformed_python_fixtures_skipped: int = 0
    synthetic_canary_occurrences: int = 0
    reserved_invalid_urls: int = 0

    def as_record(self) -> dict[str, int]:
        return {
            "files_scanned": self.files_scanned,
            "archive_entries_scanned": self.archive_entries_scanned,
            "nested_archives_scanned": self.nested_archives_scanned,
            "text_payloads_scanned": self.text_payloads_scanned,
            "python_modules_parsed": self.python_modules_parsed,
            "malformed_python_fixtures_skipped": self.malformed_python_fixtures_skipped,
            "synthetic_canary_occurrences": self.synthetic_canary_occurrences,
            "reserved_invalid_urls": self.reserved_invalid_urls,
        }


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-fixture-scan-", dir=path.parent
    )
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        temporary.chmod(0o644)
        os.replace(temporary, path)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def controlled_files(root: Path = ROOT) -> list[Path]:
    fixture_files = [
        path
        for path in (root / "fixtures").rglob("*")
        if path.is_file()
        and "__pycache__" not in path.parts
        and path != root / "fixtures/corpus/v1/agentmage-synthetic-corpus-v1.zip"
    ]
    excluded_artifacts = {
        root / REPORT_PATH.relative_to(ROOT),
        *(root / path for path in FINAL_EVIDENCE_ENVELOPE),
    }
    artifact_files = [
        path
        for path in (root / "artifacts/sprints/sprint-2").rglob("*")
        if path.is_file() and path not in excluded_artifacts
    ]
    return sorted(fixture_files + artifact_files)


def finding(category: str, path: str, reason: str) -> ScanFinding:
    return ScanFinding(category=category, path=path, reason=reason)


def python_imports(text: str) -> set[str] | None:
    try:
        tree = ast.parse(text)
    except SyntaxError:
        return None
    imports = set()
    for item in ast.walk(tree):
        if isinstance(item, ast.Import):
            imports.update(alias.name for alias in item.names)
        elif isinstance(item, ast.ImportFrom) and item.module:
            imports.add(item.module)
    return imports


def valid_reserved_url(value: str) -> bool:
    parsed = urlsplit(value.rstrip(".,);]"))
    return (
        parsed.scheme in {"http", "https"}
        and parsed.hostname is not None
        and parsed.hostname.endswith(".invalid")
        and parsed.username is None
        and parsed.password is None
    )


def scan_text(
    path: str,
    text: str,
    metrics: ScanMetrics,
    *,
    office_xml: bool = False,
) -> list[ScanFinding]:
    findings: list[ScanFinding] = []
    metrics.text_payloads_scanned += 1
    if not office_xml:
        for value in URL.findall(text):
            if valid_reserved_url(value):
                metrics.reserved_invalid_urls += 1
            else:
                findings.append(finding("remote-reference", path, "non-reserved URL"))
    lower = text.lower()
    if office_xml and (
        "<f>" in lower
        or "<f " in lower
        or 'targetmode="external"' in lower
        or "targetmode='external'" in lower
    ):
        findings.append(finding("active-formula", path, "active Office relationship or formula"))
    suffix = Path(path.split("!", 1)[-1]).suffix.lower()
    if suffix == ".py":
        imports = python_imports(text)
        if imports is None:
            metrics.malformed_python_fixtures_skipped += 1
        else:
            metrics.python_modules_parsed += 1
            prohibited = sorted(imports & NETWORK_IMPORTS)
            if prohibited:
                findings.append(
                    finding("network-behavior", path, f"prohibited imports: {','.join(prohibited)}")
                )
    if suffix in CODE_EXTENSIONS:
        network_tokens = (
            "curl ",
            "wget ",
            "tcpstream::connect",
            "net.dial(",
            "fetch(",
            "xmlhttprequest",
        )
        if any(token in lower for token in network_tokens):
            findings.append(finding("network-behavior", path, "network operation token"))
    if suffix == ".ipynb":
        try:
            notebook = json.loads(text)
        except json.JSONDecodeError:
            pass
        else:
            if any(cell.get("cell_type") == "code" for cell in notebook.get("cells", [])):
                findings.append(finding("hidden-executable", path, "notebook code cell"))
    return findings


def scan_blob(
    path: str,
    content: bytes,
    metrics: ScanMetrics,
    *,
    executable: bool = False,
    office_xml: bool = False,
) -> list[ScanFinding]:
    findings: list[ScanFinding] = []
    metrics.synthetic_canary_occurrences += len(SYNTHETIC_CANARY.findall(content))
    if any(pattern.search(content) for pattern in SECRET_PATTERNS):
        findings.append(finding("real-credential", path, "credential-shaped material"))
    if PRIVATE_PATH.search(content):
        findings.append(finding("private-path", path, "private absolute path"))
    suffix = Path(path.split("!", 1)[-1]).suffix.lower()
    if executable or suffix in EXECUTABLE_EXTENSIONS or any(
        content.startswith(magic) for magic in BINARY_MAGICS
    ):
        findings.append(finding("hidden-executable", path, "executable mode, name, or magic"))
    if suffix == ".pdf" and any(
        token in content
        for token in (
            b"/JavaScript",
            b"/JS",
            b"/Launch",
            b"/EmbeddedFile",
            b"/OpenAction",
            b"/AA",
        )
    ):
        findings.append(finding("hidden-executable", path, "active PDF action"))
    try:
        text = content.decode("utf-8")
    except UnicodeDecodeError:
        return findings
    findings.extend(scan_text(path, text, metrics, office_xml=office_xml))
    return findings


def scan_archive(
    label: str,
    content: bytes,
    metrics: ScanMetrics,
    *,
    nested: bool = False,
) -> list[ScanFinding]:
    findings: list[ScanFinding] = []
    if nested:
        metrics.nested_archives_scanned += 1
    try:
        archive = zipfile.ZipFile(io.BytesIO(content))
    except zipfile.BadZipFile:
        return [finding("hidden-executable", label, "invalid archive container")]
    with archive:
        seen: set[str] = set()
        for info in archive.infolist():
            metrics.archive_entries_scanned += 1
            entry_label = f"{label}!{info.filename}"
            if info.filename in seen:
                findings.append(finding("hidden-executable", entry_label, "duplicate archive entry"))
            seen.add(info.filename)
            mode = info.external_attr >> 16
            executable = bool(mode & 0o111) or stat.S_IFMT(mode) == stat.S_IFLNK
            payload = archive.read(info)
            is_office_xml = label.lower().endswith((".docx", ".xlsx", ".pptx"))
            findings.extend(
                scan_blob(
                    entry_label,
                    payload,
                    metrics,
                    executable=executable,
                    office_xml=is_office_xml,
                )
            )
            entry_suffix = Path(info.filename).suffix.lower()
            if entry_suffix in {".zip", ".docx", ".xlsx", ".pptx"}:
                findings.extend(scan_archive(entry_label, payload, metrics, nested=True))
    return findings


def scan_surfaces(root: Path = ROOT) -> tuple[list[ScanFinding], ScanMetrics]:
    metrics = ScanMetrics()
    findings: list[ScanFinding] = []
    for path in controlled_files(root):
        metrics.files_scanned += 1
        relative = path.relative_to(root).as_posix()
        findings.extend(
            scan_blob(
                relative,
                path.read_bytes(),
                metrics,
                executable=bool(path.stat().st_mode & 0o111),
            )
        )
    metrics.files_scanned += 1
    corpus_path = root / CORPUS_PATH.relative_to(ROOT)
    findings.extend(
        scan_archive(CORPUS_PATH.relative_to(ROOT).as_posix(), corpus_path.read_bytes(), metrics)
    )
    unique = {
        (item.category, item.path, item.reason): item for item in findings
    }
    return [unique[key] for key in sorted(unique)], metrics


def seeded_cases() -> list[dict[str, Any]]:
    cases = []
    seeds = {
        "active-formula": ("fixture.xlsx!xl/worksheets/sheet1.xml", b"<f>1+1</f>", True, False),
        "hidden-executable": ("fixture.bin", bytes((0x7F, 0x45, 0x4C, 0x46)), False, False),
        "network-behavior": ("fixture.py", b"import socket\n", False, False),
        "private-path": ("fixture.txt", b"path=/" + b"home/person/private.txt", False, False),
        "real-credential": ("fixture.txt", b"ghp_" + (b"A" * 24), False, False),
        "remote-reference": ("fixture.txt", b"https://example.com/value", False, False),
    }
    for category in EXPECTED_CATEGORIES:
        path, content, office_xml, executable = seeds[category]
        metrics = ScanMetrics()
        observed = {
            item.category
            for item in scan_blob(
                path,
                content,
                metrics,
                office_xml=office_xml,
                executable=executable,
            )
        }
        cases.append(
            {
                "category": category,
                "detected": category in observed,
                "observed_categories": sorted(observed),
            }
        )
    return cases


def build_report(root: Path = ROOT) -> dict[str, Any]:
    findings, metrics = scan_surfaces(root)
    seeds = seeded_cases()
    if findings:
        raise ValueError(f"fixture security scan found {len(findings)} prohibited item(s)")
    if not all(item["detected"] for item in seeds):
        raise ValueError("fixture security scanner did not detect every seeded category")
    return {
        "schema_version": 1,
        "task_id": "2.1.3.3",
        "test_id": "S-002-ST01",
        "status": "pass",
        "scope": {
            "fixture_root": "fixtures",
            "artifact_root": "artifacts/sprints/sprint-2",
            "bytecode_caches_included": False,
            "corpus_archive_recursively_scanned": True,
            "excluded_final_evidence_envelope": list(FINAL_EVIDENCE_ENVELOPE),
            "final_evidence_envelope_verifier": FINAL_EVIDENCE_VERIFIER,
        },
        "metrics": metrics.as_record(),
        "findings": [item.as_record() for item in findings],
        "summary": {
            "blocking_finding_count": len(findings),
            "seeded_category_count": len(seeds),
            "seeded_categories_detected": sum(item["detected"] for item in seeds),
            "synthetic_canaries_permitted_by_label": True,
            "only_reserved_invalid_urls_permitted": True,
        },
        "seeded_cases": seeds,
        "prohibited_categories": list(EXPECTED_CATEGORIES),
        "network_calls_performed": 0,
        "raw_sensitive_values_retained": False,
        "product_support_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }


def validate_report(report: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(report, dict):
        return ["fixture security scan report must be an object"]
    failures: list[str] = []
    if (
        report.get("schema_version") != 1
        or report.get("task_id") != "2.1.3.3"
        or report.get("test_id") != "S-002-ST01"
    ):
        failures.append("fixture security scan report identity is invalid")
    if report.get("status") != "pass" or report.get("findings") != []:
        failures.append("fixture security scan did not pass with zero findings")
    scope = report.get("scope", {})
    if scope.get("excluded_final_evidence_envelope") != list(
        FINAL_EVIDENCE_ENVELOPE
    ) or scope.get("final_evidence_envelope_verifier") != FINAL_EVIDENCE_VERIFIER:
        failures.append("fixture security scan final-envelope boundary is invalid")
    summary = report.get("summary", {})
    if (
        summary.get("blocking_finding_count") != 0
        or summary.get("seeded_category_count") != 6
        or summary.get("seeded_categories_detected") != 6
        or summary.get("synthetic_canaries_permitted_by_label") is not True
        or summary.get("only_reserved_invalid_urls_permitted") is not True
    ):
        failures.append("fixture security scan summary was weakened")
    if report.get("network_calls_performed") != 0:
        failures.append("fixture security scanner performed a network call")
    if report.get("raw_sensitive_values_retained") is not False:
        failures.append("fixture security scanner retained sensitive values")
    if report.get("product_support_claim") != "none":
        failures.append("fixture security scan made a product support claim")
    if report.get("macos_execution_status") != "blocked-macos" or report.get(
        "macos_support_claim"
    ) != "none":
        failures.append("fixture security scan made an invalid macOS claim")
    try:
        expected = build_report(root)
    except (OSError, ValueError, KeyError, TypeError, zipfile.BadZipFile) as error:
        failures.append(f"cannot rebuild fixture security scan report: {error}")
    else:
        if report != expected:
            failures.append("fixture security scan report is stale or non-deterministic")
    return failures


def write_report(root: Path = ROOT) -> None:
    write_atomic(root / REPORT_PATH.relative_to(ROOT), canonical_json(build_report(root)))


def check_report(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read fixture security scan report: {error}"]
    return validate_report(report, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_report()
        failures = check_report()
    except (OSError, ValueError, KeyError, TypeError, zipfile.BadZipFile) as error:
        print(f"fixture security scan failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"fixture security scan failed: {failure}", file=sys.stderr)
        return 1
    print("Sprint 2 fixtures and generated artifacts passed the security scan")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
