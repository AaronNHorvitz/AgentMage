#!/usr/bin/env python3
"""Scan Sprint 2 fixtures and evidence for prohibited active or private content."""

from __future__ import annotations

import argparse
import ast
import hashlib
import io
import json
import os
import re
import stat
import sys
import tempfile
import zipfile
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
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
    "retained-raw-canary",
)
ARCHIVE_EXTENSIONS = {".docx", ".pptx", ".xlsx", ".zip"}
ZIP_MAGICS = (b"PK\x03\x04", b"PK\x05\x06", b"PK\x07\x08")
MAX_ARCHIVE_DEPTH = 8
MAX_ARCHIVE_ENTRIES = 4096
MAX_ARCHIVE_ENTRY_BYTES = 64 * 1024 * 1024
APPROVED_CANARY_IDENTITIES = {
    "fixtures/corpus/v1/agentmage-synthetic-corpus-v1.zip": "1b8e7bc3ccb25d72a69b7957f184977694b5873d0e7d421a26827dd88ee17d50",
    "fixtures/corpus/v1/agentmage-synthetic-corpus-v1.zip!adversarial/secret-canaries/error.txt": "bcdd16d67a8faee96ed5997cf6e814d3c47b0eab12876fde849fd03d0bff6582",
    "fixtures/corpus/v1/agentmage-synthetic-corpus-v1.zip!adversarial/secret-canaries/plain.txt": "0f2d6f4b9bdae2692559f1b499f35fa0febb875736784dbaeccd1381a7c6e713",
    "fixtures/corpus/v1/agentmage-synthetic-corpus-v1.zip!adversarial/secret-canaries/structured.json": "504d7e71c5f482a351f269e90d6028a6a3e79d25f8d59ea48bb46931843815fe",
    "fixtures/corpus/v1/agentmage-synthetic-corpus-v1.zip!adversarial/secret-canaries/tool-output.json": "adf42a021d7c2bafef900f24d986672adad31922e99cd2ebf91905e1abf4f1e7",
    "fixtures/fake_adapters.py": "15dc64560f755eba2465b5bf0553664a053cd50072e69fe494cee8a05c90eb7c",
    "fixtures/test-result-bundle-profile.json": "b9de6a5526078f02ab8b9c5c880f6591ffb586b6aee117ef8fc732b2ac33bc60",
}
APPROVED_INERT_OFFICE_RELATIONSHIPS = {
    "fixtures/artifact-admission/v1/artifact-adversarial-corpus-v1.zip!hostile/relationships/external.docx!word/_rels/document.xml.rels": "34482c7a948c184a1348a91fce3ccdd9e25fc4d035d693ea062db1cd101d03c9",
    "fixtures/artifact-evaluation/v1/document-variant-corpus-v1.zip!documents/docx/relationship-hostile.docx!word/_rels/document.xml.rels": "6c8a16a5a04f75e3406c3493e2bccc36d63ae4af9ff530c2cd0ecbf47688c560",
    "fixtures/artifact-evaluation/v1/document-variant-corpus-v1.zip!documents/xlsx/relationship-hostile.xlsx!xl/externalLinks/_rels/externalLink1.xml.rels": "6c8a16a5a04f75e3406c3493e2bccc36d63ae4af9ff530c2cd0ecbf47688c560",
}
APPROVED_INERT_ARCHIVE_ENTRIES = {
    "fixtures/artifact-admission/v1/artifact-adversarial-corpus-v1.zip!hostile/archive/traversal.zip!../outside.txt": "b196108b44cda770aa9a2f79ef76b468d3d965b47bc05b8447e33836ea6f3bca",
}
APPROVED_MALFORMED_ARCHIVES = {
    "fixtures/artifact-admission/v1/artifact-adversarial-corpus-v1.zip!hostile/malformed/truncated.zip": "02fec2d5fdeb4e39e9ab09f9e761ad18177b306dbeba050761cd4e52e267fb12",
    "fixtures/artifact-evaluation/v1/document-variant-corpus-v1.zip!documents/docx/malformed.docx": "9c2efd6a5af124fd595bbc7391454379df079b73f281301c4e6c819bc94e3654",
    "fixtures/artifact-evaluation/v1/document-variant-corpus-v1.zip!documents/xlsx/malformed.xlsx": "d19263e02c7287173f43d9936beea4cf2eb28f4b9fd17425a9718b39ab53847d",
}
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
PRIVATE_PATHS = (
    re.compile(rb"(?:^|[\s\"'=:(])/(?:home|Users|var/home)/[A-Za-z0-9._-]+/"),
    re.compile(rb"(?:^|[\s\"'=:(])/(?:root|private/var/folders)/"),
    re.compile(
        rb"(?:^|[\s\"'=:(])[A-Za-z]:[\\/](?:Users|Documents and Settings)[\\/][A-Za-z0-9._-]+[\\/]",
        re.IGNORECASE,
    ),
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
    fixture_files_scanned: int = 0
    evidence_files_scanned: int = 0
    archive_containers_scanned: int = 0
    archive_entries_scanned: int = 0
    nested_archives_scanned: int = 0
    malformed_archives_inspected_as_blobs: int = 0
    text_payloads_scanned: int = 0
    python_modules_parsed: int = 0
    malformed_python_fixtures_skipped: int = 0
    synthetic_canary_occurrences: int = 0
    approved_synthetic_canary_occurrences: int = 0
    approved_inert_remote_references: int = 0
    approved_inert_archive_entries: int = 0
    approved_malformed_archives: int = 0
    reserved_invalid_urls: int = 0

    def as_record(self) -> dict[str, int]:
        return {
            "files_scanned": self.files_scanned,
            "fixture_files_scanned": self.fixture_files_scanned,
            "evidence_files_scanned": self.evidence_files_scanned,
            "archive_containers_scanned": self.archive_containers_scanned,
            "archive_entries_scanned": self.archive_entries_scanned,
            "nested_archives_scanned": self.nested_archives_scanned,
            "malformed_archives_inspected_as_blobs": self.malformed_archives_inspected_as_blobs,
            "text_payloads_scanned": self.text_payloads_scanned,
            "python_modules_parsed": self.python_modules_parsed,
            "malformed_python_fixtures_skipped": self.malformed_python_fixtures_skipped,
            "synthetic_canary_occurrences": self.synthetic_canary_occurrences,
            "approved_synthetic_canary_occurrences": self.approved_synthetic_canary_occurrences,
            "approved_inert_remote_references": self.approved_inert_remote_references,
            "approved_inert_archive_entries": self.approved_inert_archive_entries,
            "approved_malformed_archives": self.approved_malformed_archives,
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


def sha256_bytes(content: bytes) -> str:
    return hashlib.sha256(content).hexdigest()


def approved_identity(path: str, content: bytes, identities: dict[str, str]) -> bool:
    return identities.get(path) == sha256_bytes(content)


def approval_records(identities: dict[str, str]) -> list[dict[str, str]]:
    return [
        {"path": path, "sha256": identity}
        for path, identity in sorted(identities.items())
    ]


def safe_archive_entry(value: str) -> bool:
    path = PurePosixPath(value)
    return bool(value) and not path.is_absolute() and ".." not in path.parts


def zip_container(path: str, content: bytes) -> bool:
    return Path(path.split("!", 1)[-1]).suffix.lower() in ARCHIVE_EXTENSIONS or content.startswith(
        ZIP_MAGICS
    )


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
    active_office_content = office_xml and (
        "<f>" in lower
        or "<f " in lower
        or 'targetmode="external"' in lower
        or "targetmode='external'" in lower
    )
    if active_office_content:
        if approved_identity(
            path,
            text.encode("utf-8"),
            APPROVED_INERT_OFFICE_RELATIONSHIPS,
        ):
            metrics.approved_inert_remote_references += 1
        else:
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
    if suffix in {".htm", ".html", ".svg"} and (
        "<script" in lower or "javascript:" in lower
    ):
        findings.append(finding("hidden-executable", path, "active markup script"))
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
    canary_count = len(SYNTHETIC_CANARY.findall(content))
    metrics.synthetic_canary_occurrences += canary_count
    if canary_count:
        if approved_identity(path, content, APPROVED_CANARY_IDENTITIES):
            metrics.approved_synthetic_canary_occurrences += canary_count
        else:
            findings.append(
                finding(
                    "retained-raw-canary",
                    path,
                    "synthetic canary outside an approved source fixture",
                )
            )
    if any(pattern.search(content) for pattern in SECRET_PATTERNS):
        findings.append(finding("real-credential", path, "credential-shaped material"))
    if any(pattern.search(content) for pattern in PRIVATE_PATHS):
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
    depth: int = 0,
) -> list[ScanFinding]:
    findings: list[ScanFinding] = []
    metrics.archive_containers_scanned += 1
    if nested:
        metrics.nested_archives_scanned += 1
    if depth > MAX_ARCHIVE_DEPTH:
        return [finding("hidden-executable", label, "nested archive depth limit exceeded")]
    try:
        archive = zipfile.ZipFile(io.BytesIO(content))
    except zipfile.BadZipFile:
        metrics.malformed_archives_inspected_as_blobs += 1
        if approved_identity(label, content, APPROVED_MALFORMED_ARCHIVES):
            metrics.approved_malformed_archives += 1
            return findings
        return [finding("hidden-executable", label, "uninspectable malformed archive")]
    with archive:
        if len(archive.infolist()) > MAX_ARCHIVE_ENTRIES:
            return [finding("hidden-executable", label, "archive entry-count limit exceeded")]
        seen: set[str] = set()
        for info in archive.infolist():
            metrics.archive_entries_scanned += 1
            entry_label = f"{label}!{info.filename}"
            if not safe_archive_entry(info.filename):
                if approved_identity(
                    entry_label,
                    archive.read(info),
                    APPROVED_INERT_ARCHIVE_ENTRIES,
                ):
                    metrics.approved_inert_archive_entries += 1
                else:
                    findings.append(
                        finding("hidden-executable", entry_label, "unsafe archive entry path")
                    )
            if info.filename in seen:
                findings.append(finding("hidden-executable", entry_label, "duplicate archive entry"))
            seen.add(info.filename)
            mode = info.external_attr >> 16
            executable = bool(mode & 0o111) or stat.S_IFMT(mode) == stat.S_IFLNK
            office_package = label.lower().endswith((".docx", ".xlsx", ".pptx"))
            office_entry = info.filename.lower()
            if office_package and (
                office_entry.endswith("vbaproject.bin")
                or "/activex/" in f"/{office_entry}"
                or "/embeddings/" in f"/{office_entry}"
            ):
                findings.append(
                    finding("hidden-executable", entry_label, "active or embedded Office payload")
                )
            if info.flag_bits & 0x1:
                findings.append(
                    finding(
                        "hidden-executable",
                        entry_label,
                        "encrypted archive entry cannot be inspected",
                    )
                )
                continue
            if info.file_size > MAX_ARCHIVE_ENTRY_BYTES:
                findings.append(
                    finding(
                        "hidden-executable",
                        entry_label,
                        "archive entry inspection bound exceeded",
                    )
                )
                continue
            if info.is_dir():
                continue
            payload = archive.read(info)
            is_office_xml = office_package or info.filename.lower().endswith(".rels")
            findings.extend(
                scan_blob(
                    entry_label,
                    payload,
                    metrics,
                    executable=executable,
                    office_xml=is_office_xml,
                )
            )
            if zip_container(info.filename, payload):
                findings.extend(
                    scan_archive(
                        entry_label,
                        payload,
                        metrics,
                        nested=True,
                        depth=depth + 1,
                    )
                )
    return findings


def scan_surfaces(root: Path = ROOT) -> tuple[list[ScanFinding], ScanMetrics]:
    metrics = ScanMetrics()
    findings: list[ScanFinding] = []
    for path in controlled_files(root):
        metrics.files_scanned += 1
        relative = path.relative_to(root).as_posix()
        if relative.startswith("fixtures/"):
            metrics.fixture_files_scanned += 1
        else:
            metrics.evidence_files_scanned += 1
        content = path.read_bytes()
        findings.extend(
            scan_blob(
                relative,
                content,
                metrics,
                executable=bool(path.stat().st_mode & 0o111),
            )
        )
        if zip_container(relative, content):
            findings.extend(scan_archive(relative, content, metrics))
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
        "retained-raw-canary": (
            "evidence.json",
            b"AM_SYNTHETIC_CANARY_UNAPPROVED",
            False,
            False,
        ),
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
    paths = [path.relative_to(root).as_posix() for path in controlled_files(root)]
    return {
        "schema_version": 1,
        "task_id": "2.1.3.3",
        "coverage_task_ids": ["2.1.3.3", "2.3.4.2"],
        "test_id": "S-002-ST01",
        "status": "pass",
        "scope": {
            "fixture_root": "fixtures",
            "artifact_root": "artifacts/sprints/sprint-2",
            "bytecode_caches_included": False,
            "all_zip_and_office_packages_recursively_scanned": True,
            "archive_depth_limit": MAX_ARCHIVE_DEPTH,
            "archive_entry_count_limit": MAX_ARCHIVE_ENTRIES,
            "archive_entry_byte_limit": MAX_ARCHIVE_ENTRY_BYTES,
            "controlled_path_count": len(paths),
            "controlled_path_set_sha256": sha256_bytes(canonical_json(paths)),
            "approved_canary_identities": approval_records(APPROVED_CANARY_IDENTITIES),
            "approved_inert_office_relationships": approval_records(
                APPROVED_INERT_OFFICE_RELATIONSHIPS
            ),
            "approved_inert_archive_entries": approval_records(
                APPROVED_INERT_ARCHIVE_ENTRIES
            ),
            "approved_malformed_archives": approval_records(APPROVED_MALFORMED_ARCHIVES),
            "excluded_final_evidence_envelope": list(FINAL_EVIDENCE_ENVELOPE),
            "final_evidence_envelope_verifier": FINAL_EVIDENCE_VERIFIER,
        },
        "metrics": metrics.as_record(),
        "findings": [item.as_record() for item in findings],
        "summary": {
            "blocking_finding_count": len(findings),
            "seeded_category_count": len(seeds),
            "seeded_categories_detected": sum(item["detected"] for item in seeds),
            "synthetic_canaries_permitted_only_in_approved_source_fixtures": True,
            "only_reserved_invalid_urls_permitted": True,
            "unapproved_raw_canary_count": 0,
        },
        "seeded_cases": seeds,
        "prohibited_categories": list(EXPECTED_CATEGORIES),
        "network_calls_performed": 0,
        "active_content_executed": False,
        "product_runtime_executed": False,
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
    if report.get("coverage_task_ids") != ["2.1.3.3", "2.3.4.2"]:
        failures.append("fixture security scan task coverage is incomplete")
    scope = report.get("scope", {})
    if not isinstance(scope, dict):
        failures.append("fixture security scan scope is not an object")
        scope = {}
    controlled_paths = [
        path.relative_to(root).as_posix() for path in controlled_files(root)
    ]
    if scope.get("excluded_final_evidence_envelope") != list(
        FINAL_EVIDENCE_ENVELOPE
    ) or scope.get("final_evidence_envelope_verifier") != FINAL_EVIDENCE_VERIFIER:
        failures.append("fixture security scan final-envelope boundary is invalid")
    if (
        scope.get("all_zip_and_office_packages_recursively_scanned") is not True
        or scope.get("archive_depth_limit") != MAX_ARCHIVE_DEPTH
        or scope.get("archive_entry_count_limit") != MAX_ARCHIVE_ENTRIES
        or scope.get("archive_entry_byte_limit") != MAX_ARCHIVE_ENTRY_BYTES
        or scope.get("approved_canary_identities")
        != approval_records(APPROVED_CANARY_IDENTITIES)
        or scope.get("approved_inert_office_relationships")
        != approval_records(APPROVED_INERT_OFFICE_RELATIONSHIPS)
        or scope.get("approved_inert_archive_entries")
        != approval_records(APPROVED_INERT_ARCHIVE_ENTRIES)
        or scope.get("approved_malformed_archives")
        != approval_records(APPROVED_MALFORMED_ARCHIVES)
    ):
        failures.append("fixture security scan nested-package policy is incomplete")
    if (
        scope.get("controlled_path_count") != len(controlled_paths)
        or scope.get("controlled_path_set_sha256")
        != sha256_bytes(canonical_json(controlled_paths))
    ):
        failures.append("fixture security scan controlled path inventory is stale")
    metrics = report.get("metrics", {})
    if not isinstance(metrics, dict):
        failures.append("fixture security scan metrics are not an object")
        metrics = {}
    if (
        metrics.get("files_scanned")
        != metrics.get("fixture_files_scanned", 0)
        + metrics.get("evidence_files_scanned", 0)
        or metrics.get("synthetic_canary_occurrences")
        != metrics.get("approved_synthetic_canary_occurrences")
        or metrics.get("approved_inert_remote_references")
        != len(APPROVED_INERT_OFFICE_RELATIONSHIPS)
        or metrics.get("approved_inert_archive_entries")
        != len(APPROVED_INERT_ARCHIVE_ENTRIES)
        or metrics.get("approved_malformed_archives")
        != len(APPROVED_MALFORMED_ARCHIVES)
        or metrics.get("archive_containers_scanned", 0)
        < metrics.get("nested_archives_scanned", 0)
    ):
        failures.append("fixture security scan coverage or exception accounting is false")
    summary = report.get("summary", {})
    if not isinstance(summary, dict):
        failures.append("fixture security scan summary is not an object")
        summary = {}
    if (
        summary.get("blocking_finding_count") != 0
        or summary.get("seeded_category_count") != 7
        or summary.get("seeded_categories_detected") != 7
        or summary.get(
            "synthetic_canaries_permitted_only_in_approved_source_fixtures"
        )
        is not True
        or summary.get("only_reserved_invalid_urls_permitted") is not True
        or summary.get("unapproved_raw_canary_count") != 0
    ):
        failures.append("fixture security scan summary was weakened")
    if report.get("network_calls_performed") != 0:
        failures.append("fixture security scanner performed a network call")
    if report.get("active_content_executed") is not False or report.get(
        "product_runtime_executed"
    ) is not False:
        failures.append("fixture security scanner made an execution overclaim")
    if report.get("raw_sensitive_values_retained") is not False:
        failures.append("fixture security scanner retained sensitive values")
    if report.get("product_support_claim") != "none":
        failures.append("fixture security scan made a product support claim")
    if report.get("prohibited_categories") != list(EXPECTED_CATEGORIES) or report.get(
        "seeded_cases"
    ) != seeded_cases():
        failures.append("fixture security scan seeded detector closure is incomplete")
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
