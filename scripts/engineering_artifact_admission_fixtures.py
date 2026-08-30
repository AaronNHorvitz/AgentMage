#!/usr/bin/env python3
"""Build and verify the synthetic, identity-bound Story 2.4 admission corpus."""

from __future__ import annotations

import argparse
import copy
import hashlib
import io
import json
import os
import sys
import tempfile
import zipfile
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts import document_fixture_generator as documents  # noqa: E402


CORPUS_DIR: Final = ROOT / "fixtures" / "artifact-admission" / "v1"
ARCHIVE_PATH: Final = CORPUS_DIR / "artifact-admission-corpus-v1.zip"
MANIFEST_PATH: Final = CORPUS_DIR / "manifest.json"
DOCUMENT_PROFILE_PATH: Final = ROOT / "fixtures" / "document-fixture-profile.json"
FIXED_TIME: Final = "2026-08-29T00:00:00Z"
ZIP_TIME: Final = (2024, 1, 1, 0, 0, 0)
ZERO_SHA256: Final = "0" * 64
REQUIRED_CATEGORIES: Final = (
    "paste",
    "local_file",
    "virtual_uri",
    "remote_uri",
    "directory",
    "archive",
    "text",
    "log",
    "pdf",
    "docx",
    "spreadsheet",
    "image_ocr",
    "duplicate",
    "stale",
    "inaccessible",
    "unsupported",
)


@dataclass(frozen=True)
class FixtureSpec:
    fixture_id: str
    category: str
    reference_class: str
    origin_class: str
    artifact_kind: str
    media_type: str
    archive_entry: str | None
    content: bytes | None
    support_state: str = "supported"
    freshness_state: str = "fresh"
    capture_state: str = "captured"
    relation: dict[str, str] | None = None
    current_entry: str | None = None
    current_content: bytes | None = None


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sealed(record: dict[str, Any], field: str) -> dict[str, Any]:
    result = copy.deepcopy(record)
    result[field] = ZERO_SHA256
    result[field] = sha256_bytes(canonical_json(result))
    return result


def safe_archive_entry(value: str) -> bool:
    path = PurePosixPath(value)
    return bool(value) and not path.is_absolute() and ".." not in path.parts and not value.endswith("/")


def directory_manifest() -> bytes:
    return canonical_json(
        {
            "schema_version": 1,
            "directory_id": "synthetic-directory-1",
            "entries": [
                {"logical_name": "notes/readme.txt", "kind": "file"},
                {"logical_name": "records/items.json", "kind": "file"},
            ],
            "synthetic": True,
        }
    )


def materialize_specs() -> list[FixtureSpec]:
    profile = json.loads(DOCUMENT_PROFILE_PATH.read_text(encoding="utf-8"))
    failures = documents.validate_profile(profile)
    if failures:
        raise ValueError("; ".join(failures))
    generated = documents.materialize_specs(profile)

    def document(path: str) -> bytes:
        return generated[path].content

    local = b"Synthetic local file fixture.\nIdentity remains byte-bound.\n"
    stale_original = b"Synthetic source revision one.\n"
    stale_current = b"Synthetic source revision two.\n"
    return [
        FixtureSpec(
            "admission-paste-1", "paste", "paste", "paste", "text", "text/plain",
            "payloads/paste/request.txt", b"Synthetic pasted request context.\n",
        ),
        FixtureSpec(
            "admission-local-file-1", "local_file", "file_path", "file", "text", "text/plain",
            "payloads/files/local-note.txt", local,
        ),
        FixtureSpec(
            "admission-virtual-uri-1", "virtual_uri", "virtual_uri", "uri", "text", "text/markdown",
            "payloads/uris/virtual-note.md", b"# Synthetic virtual document\n\nNo ambient workspace authority.\n",
        ),
        FixtureSpec(
            "admission-remote-uri-1", "remote_uri", "remote_uri", "uri", "structured_text", "application/json",
            "payloads/uris/prestaged-remote.json",
            canonical_json({"source": "pre-staged synthetic fixture", "network_used": False}),
        ),
        FixtureSpec(
            "admission-directory-1", "directory", "directory", "directory", "directory_manifest", "application/json",
            "payloads/directories/tree-manifest.json", directory_manifest(),
        ),
        FixtureSpec(
            "admission-archive-1", "archive", "archive", "archive", "archive", "application/zip",
            "payloads/documents/reference-bundle.zip",
            document("documents/archives/reference-bundle.zip"),
        ),
        FixtureSpec(
            "admission-text-1", "text", "file_path", "file", "text", "text/plain",
            "payloads/documents/meeting-note.txt", document("documents/text/meeting-note.txt"),
        ),
        FixtureSpec(
            "admission-log-1", "log", "file_path", "file", "log", "text/plain",
            "payloads/documents/application.log", document("documents/logs/application.log"),
        ),
        FixtureSpec(
            "admission-pdf-1", "pdf", "file_path", "file", "pdf", "application/pdf",
            "payloads/documents/status-report.pdf", document("documents/pdf/status-report.pdf"),
        ),
        FixtureSpec(
            "admission-docx-1", "docx", "file_path", "file", "docx",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "payloads/documents/status-report.docx", document("documents/word/status-report.docx"),
        ),
        FixtureSpec(
            "admission-spreadsheet-1", "spreadsheet", "file_path", "file", "spreadsheet",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            "payloads/documents/project-ledger.xlsx", document("documents/spreadsheets/project-ledger.xlsx"),
        ),
        FixtureSpec(
            "admission-image-ocr-1", "image_ocr", "file_path", "file", "image_ocr", "image/png",
            "payloads/documents/status-grid.png", document("documents/images/status-grid.png"),
        ),
        FixtureSpec(
            "admission-duplicate-1", "duplicate", "file_path", "file", "text", "text/plain",
            "payloads/files/local-note.txt", local,
            relation={"kind": "same_content", "target_fixture_id": "admission-local-file-1"},
        ),
        FixtureSpec(
            "admission-stale-1", "stale", "file_path", "file", "text", "text/plain",
            "payloads/stale/observed.txt", stale_original,
            freshness_state="stale", capture_state="failed",
            relation={"kind": "replaced_before_capture", "target_fixture_id": "admission-stale-1-current"},
            current_entry="payloads/stale/current.txt", current_content=stale_current,
        ),
        FixtureSpec(
            "admission-inaccessible-1", "inaccessible", "file_path", "file", "unknown", "application/octet-stream",
            None, None, freshness_state="unavailable", capture_state="unavailable",
        ),
        FixtureSpec(
            "admission-unsupported-1", "unsupported", "unsupported", "unsupported", "unsupported",
            "application/x-agentmage-unsupported", None, None, support_state="unsupported",
            freshness_state="unsupported", capture_state="unsupported",
        ),
    ]


def archive_entries(specs: list[FixtureSpec]) -> dict[str, bytes]:
    entries: dict[str, bytes] = {}
    for spec in specs:
        if spec.archive_entry is not None and spec.content is not None:
            if not safe_archive_entry(spec.archive_entry):
                raise ValueError(f"unsafe fixture archive entry: {spec.archive_entry}")
            previous = entries.setdefault(spec.archive_entry, spec.content)
            if previous != spec.content:
                raise ValueError(f"conflicting fixture bytes: {spec.archive_entry}")
        if spec.current_entry is not None and spec.current_content is not None:
            if not safe_archive_entry(spec.current_entry) or spec.current_entry in entries:
                raise ValueError(f"invalid current fixture entry: {spec.current_entry}")
            entries[spec.current_entry] = spec.current_content
    return dict(sorted(entries.items()))


def build_archive(entries: dict[str, bytes]) -> bytes:
    output = io.BytesIO()
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_STORED) as archive:
        for path, content in entries.items():
            info = zipfile.ZipInfo(path, ZIP_TIME)
            info.compress_type = zipfile.ZIP_STORED
            info.create_system = 3
            info.external_attr = 0o100644 << 16
            archive.writestr(info, content)
    return output.getvalue()


def byte_identity(content: bytes | None) -> dict[str, Any] | None:
    if content is None:
        return None
    return {"byte_length": len(content), "sha256": sha256_bytes(content)}


def case_record(spec: FixtureSpec) -> dict[str, Any]:
    request_id = "request-artifact-admission-v1"
    authority_id = "authority-request-artifact-admission-v1"
    reference_id = f"reference-{spec.fixture_id}"
    origin_id = f"origin-{spec.fixture_id}"
    artifact_id = f"source-{spec.fixture_id}"
    protected_descriptor = canonical_json(
        {
            "fixture_id": spec.fixture_id,
            "locator_token": f"protected-synthetic-locator:{spec.fixture_id}",
            "reference_class": spec.reference_class,
        }
    )
    reference_sha = sha256_bytes(protected_descriptor)
    origin = sealed(
        {
            "schema_version": 1,
            "origin_id": origin_id,
            "request_id": request_id,
            "authority_id": authority_id,
            "origin_class": spec.origin_class,
            "captured_at": FIXED_TIME,
            "origin_sha256": ZERO_SHA256,
        },
        "origin_sha256",
    )
    reference = {
        "schema_version": 1,
        "reference_id": reference_id,
        "request_id": request_id,
        "authority_id": authority_id,
        "reference_class": spec.reference_class,
        "reference_display": f"synthetic {spec.category} fixture",
        "support_state": spec.support_state,
        "reference_sha256": reference_sha,
        "collected_at": FIXED_TIME,
    }
    provenance = sealed(
        {
            "schema_version": 1,
            "provenance_id": f"provenance-{spec.fixture_id}",
            "source_artifact_id": artifact_id,
            "reference_id": reference_id,
            "origin_id": origin_id,
            "classification": "public",
            "freshness_state": spec.freshness_state,
            "observed_at": FIXED_TIME,
            "collected_at": FIXED_TIME,
            "provenance_sha256": ZERO_SHA256,
        },
        "provenance_sha256",
    )
    captured_identity = byte_identity(spec.content) if spec.capture_state == "captured" else None
    source_artifact = sealed(
        {
            "schema_version": 1,
            "source_artifact_id": artifact_id,
            "request_id": request_id,
            "authority_id": authority_id,
            "reference_id": reference_id,
            "origin_id": origin_id,
            "provenance_sha256": provenance["provenance_sha256"],
            "declared_media_type": spec.media_type,
            "classification": "public",
            "freshness_state": spec.freshness_state,
            "capture_state": spec.capture_state,
            "byte_length": None if captured_identity is None else captured_identity["byte_length"],
            "sha256": None if captured_identity is None else captured_identity["sha256"],
            "collected_at": FIXED_TIME,
            "source_artifact_sha256": ZERO_SHA256,
        },
        "source_artifact_sha256",
    )
    return {
        "fixture_id": spec.fixture_id,
        "category": spec.category,
        "artifact_kind": spec.artifact_kind,
        "media_type": spec.media_type,
        "archive_entry": spec.archive_entry,
        "offered_byte_identity": byte_identity(spec.content),
        "current_archive_entry": spec.current_entry,
        "current_byte_identity": byte_identity(spec.current_content),
        "protected_descriptor_sha256": reference_sha,
        "capture_expectation": spec.capture_state,
        "identity_relation": spec.relation,
        "records": {
            "origin": origin,
            "source_reference": reference,
            "source_provenance": provenance,
            "source_artifact": source_artifact,
        },
    }


def build_manifest(specs: list[FixtureSpec], archive: bytes, entries: dict[str, bytes]) -> dict[str, Any]:
    value = {
        "schema_version": 1,
        "corpus_id": "engineering-artifact-admission-v1",
        "task_id": "2.4.1.1",
        "generated_on": "2026-08-29",
        "status": "synthetic-identity-bound-fixture-corpus",
        "synthetic_only": True,
        "network_access": False,
        "active_content_execution": False,
        "product_support_claim": "none",
        "archive": {
            "path": str(ARCHIVE_PATH.relative_to(ROOT)),
            "format": "zip-stored",
            "entry_count": len(entries),
            "byte_length": len(archive),
            "sha256": sha256_bytes(archive),
        },
        "generator": {
            "path": "scripts/engineering_artifact_admission_fixtures.py",
            "sha256": sha256_bytes(Path(__file__).read_bytes()),
        },
        "document_fixture_source": {
            "profile_path": str(DOCUMENT_PROFILE_PATH.relative_to(ROOT)),
            "profile_sha256": sha256_bytes(DOCUMENT_PROFILE_PATH.read_bytes()),
            "generator_path": "scripts/document_fixture_generator.py",
            "generator_sha256": sha256_bytes((ROOT / "scripts/document_fixture_generator.py").read_bytes()),
        },
        "required_categories": list(REQUIRED_CATEGORIES),
        "cases": [case_record(spec) for spec in specs],
    }
    value["manifest_sha256"] = sha256_bytes(canonical_json(value))
    return value


def expected() -> tuple[bytes, dict[str, Any], dict[str, bytes]]:
    specs = materialize_specs()
    entries = archive_entries(specs)
    archive = build_archive(entries)
    return archive, build_manifest(specs, archive, entries), entries


def validate_archive(content: bytes, entries: dict[str, bytes]) -> list[str]:
    failures: list[str] = []
    try:
        archive = zipfile.ZipFile(io.BytesIO(content))
    except (OSError, zipfile.BadZipFile) as error:
        return [f"artifact admission archive is invalid: {error}"]
    with archive:
        infos = archive.infolist()
        if [info.filename for info in infos] != list(entries):
            failures.append("artifact admission archive closure or order drifted")
        try:
            corrupt_name = archive.testzip()
        except (OSError, RuntimeError, zipfile.BadZipFile) as error:
            failures.append(f"artifact admission archive corruption check failed: {error}")
            corrupt_name = None
        if corrupt_name is not None:
            failures.append(f"artifact admission archive contains corrupt bytes: {corrupt_name}")
        for info in infos:
            mode = info.external_attr >> 16
            if (
                not safe_archive_entry(info.filename)
                or info.compress_type != zipfile.ZIP_STORED
                or info.flag_bits & 0x1
                or mode & 0o111
                or mode & 0o170000 == 0o120000
            ):
                failures.append(f"artifact admission archive entry is unsafe: {info.filename}")
            try:
                observed = archive.read(info.filename)
            except (OSError, RuntimeError, zipfile.BadZipFile) as error:
                failures.append(f"artifact admission archive entry cannot be read: {info.filename}: {error}")
                continue
            if entries.get(info.filename) != observed:
                failures.append(f"artifact admission archive bytes drifted: {info.filename}")
    return failures


def validate_manifest(value: Any, archive: bytes, entries: dict[str, bytes]) -> list[str]:
    if not isinstance(value, dict):
        return ["artifact admission manifest must be an object"]
    failures: list[str] = []
    unhashed = copy.deepcopy(value)
    recorded = unhashed.pop("manifest_sha256", None)
    if recorded != sha256_bytes(canonical_json(unhashed)):
        failures.append("artifact admission manifest self-hash is invalid")
    expected_manifest = build_manifest(materialize_specs(), archive, entries)
    if value != expected_manifest:
        failures.append("artifact admission manifest is stale, incomplete, reordered, or widened")
    if value.get("required_categories") != list(REQUIRED_CATEGORIES):
        failures.append("artifact admission required category closure drifted")
    cases = value.get("cases", [])
    if not isinstance(cases, list) or [item.get("category") for item in cases] != list(REQUIRED_CATEGORIES):
        failures.append("artifact admission cases do not cover each required category exactly once")
    if value.get("synthetic_only") is not True or value.get("network_access") is not False:
        failures.append("artifact admission corpus may use private or network data")
    if value.get("product_support_claim") != "none" or value.get("active_content_execution") is not False:
        failures.append("artifact admission corpus makes an unsupported capability claim")
    if value.get("archive", {}).get("sha256") != sha256_bytes(archive):
        failures.append("artifact admission archive identity is invalid")
    return failures


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(prefix=".artifact-admission-", dir=path.parent)
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


def check() -> list[str]:
    try:
        expected_archive, expected_manifest, entries = expected()
        actual_archive = ARCHIVE_PATH.read_bytes()
        actual_manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    except (OSError, ValueError, json.JSONDecodeError) as error:
        return [f"cannot validate artifact admission corpus: {error}"]
    failures: list[str] = []
    if actual_archive != expected_archive:
        failures.append("checked artifact admission archive is stale or corrupt")
    failures.extend(validate_archive(actual_archive, entries))
    failures.extend(validate_manifest(actual_manifest, actual_archive, entries))
    if actual_manifest != expected_manifest:
        failures.append("checked artifact admission manifest is stale")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write:
        archive, manifest, _entries = expected()
        write_atomic(ARCHIVE_PATH, archive)
        write_atomic(MANIFEST_PATH, canonical_json(manifest))
    failures = check()
    if failures:
        for failure in failures:
            print(f"Artifact admission fixture validation failed: {failure}", file=sys.stderr)
        return 1
    print(f"Validated {len(REQUIRED_CATEGORIES)} identity-bound artifact admission fixtures")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
