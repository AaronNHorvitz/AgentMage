#!/usr/bin/env python3
"""Assemble and validate the checked-in synthetic corpus and golden manifests."""

from __future__ import annotations

import argparse
import copy
import hashlib
import io
import json
import os
import shutil
import sys
import tempfile
import unicodedata
import zipfile
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Any, Callable


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts import adversarial_fixture_generator as adversarial  # noqa: E402
from scripts import document_fixture_generator as documents  # noqa: E402
from scripts import expected_output_manifests as golden  # noqa: E402
from scripts import fixture_generator as base  # noqa: E402
from scripts import path_fixture_generator as paths  # noqa: E402


PROFILE_PATH = ROOT / "fixtures" / "corpus-profile.json"
REPORT_PATH = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-2"
    / "story-2.1"
    / "versioned-corpus-report.json"
)
EXPECTED_SOURCES = (
    (
        "base",
        "base",
        "fixtures/generator-profile.json",
        "scripts/fixture_generator.py",
        "agentmage-synthetic-corpus-v1",
    ),
    (
        "paths",
        "paths",
        "fixtures/path-fixture-profile.json",
        "scripts/path_fixture_generator.py",
        "agentmage-path-fixtures-v1",
    ),
    (
        "adversarial",
        "adversarial",
        "fixtures/adversarial-fixture-profile.json",
        "scripts/adversarial_fixture_generator.py",
        "agentmage-adversarial-fixtures-v1",
    ),
    (
        "documents",
        "documents",
        "fixtures/document-fixture-profile.json",
        "scripts/document_fixture_generator.py",
        "agentmage-document-fixtures-v1",
    ),
    (
        "golden",
        "golden",
        "fixtures/expected-output-profile.json",
        "scripts/expected_output_manifests.py",
        "agentmage-expected-output-manifests-v1",
    ),
)
EXPECTED_ARCHIVE_SAFETY = {
    "compressed_entries": False,
    "duplicate_entries": False,
    "executable_entries": False,
    "path_traversal_entries": False,
    "symlink_entries": False,
    "absolute_entries": False,
    "encrypted_entries": False,
}
EXPECTED_CONTENT_CONTRACT = {
    "private_user_data": False,
    "real_credentials": False,
    "remote_execution": False,
    "product_parser_support_claim": "none",
}
ZIP_TIMESTAMP = (2024, 1, 1, 0, 0, 0)


@dataclass(frozen=True)
class CorpusEntry:
    family: str
    kind: str
    source_relative_path: str
    content: bytes


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def safe_relative(value: str) -> bool:
    path = PurePosixPath(value)
    return bool(value) and not path.is_absolute() and ".." not in path.parts


def source_tuple(source: Any) -> tuple[Any, Any, Any, Any, Any]:
    if not isinstance(source, dict):
        return (None, None, None, None, None)
    return (
        source.get("family"),
        source.get("namespace"),
        source.get("path"),
        source.get("generator"),
        source.get("profile_id"),
    )


def validate_profile(profile: Any) -> list[str]:
    if not isinstance(profile, dict):
        return ["versioned corpus profile must be an object"]
    failures: list[str] = []
    if (
        profile.get("schema_version") != 1
        or profile.get("profile_id") != "agentmage-versioned-synthetic-corpus-v1"
        or profile.get("status") != "versioned-synthetic-corpus-contract"
        or profile.get("corpus_version") != "1.0.0"
    ):
        failures.append("versioned corpus profile identity is invalid")
    if profile.get("fixed_zip_timestamp") != "2024-01-01T00:00:00Z":
        failures.append("versioned corpus ZIP timestamp is not pinned")
    if profile.get("archive") != {
        "format": "zip-stored",
        "path": "fixtures/corpus/v1/agentmage-synthetic-corpus-v1.zip",
    }:
        failures.append("versioned corpus archive contract drifted")
    if profile.get("manifest_path") != "fixtures/corpus/v1/manifest.json":
        failures.append("versioned corpus manifest path drifted")
    sources = profile.get("sources")
    if not isinstance(sources, list) or tuple(source_tuple(source) for source in sources) != EXPECTED_SOURCES:
        failures.append("versioned corpus source closure drifted")
    if profile.get("archive_safety_contract") != EXPECTED_ARCHIVE_SAFETY:
        failures.append("versioned corpus archive-safety contract was weakened")
    if profile.get("content_contract") != EXPECTED_CONTENT_CONTRACT:
        failures.append("versioned corpus content contract was weakened")
    if profile.get("path_fixture_materialization") != "inert-declarations-only":
        failures.append("versioned corpus path fixtures are not inert declarations")
    if profile.get("macos_execution_status") != "blocked-macos":
        failures.append("versioned corpus lost blocked macOS status")
    for source in sources if isinstance(sources, list) else []:
        for field in ("path", "generator"):
            if not safe_relative(source.get(field, "")):
                failures.append(f"versioned corpus source path is unsafe: {field}")
    return failures


def add_entry(
    entries: dict[str, CorpusEntry],
    archive_path: str,
    family: str,
    source_relative_path: str,
    content: bytes,
    kind: str = "file",
) -> None:
    if not safe_relative(archive_path) or archive_path in entries:
        raise ValueError(f"invalid or duplicate corpus archive path: {archive_path}")
    entries[archive_path] = CorpusEntry(family, kind, source_relative_path, content)


def materialize_entries(profile: dict[str, Any], root: Path = ROOT) -> dict[str, CorpusEntry]:
    source_profiles = {
        source["family"]: read_json(root / source["path"])
        for source in profile["sources"]
    }
    validators: dict[str, Callable[[Any], list[str]]] = {
        "base": base.validate_profile,
        "paths": paths.validate_profile,
        "adversarial": adversarial.validate_profile,
        "documents": documents.validate_profile,
        "golden": golden.validate_profile,
    }
    for source in profile["sources"]:
        source_profile = source_profiles[source["family"]]
        failures = validators[source["family"]](source_profile)
        if failures:
            raise ValueError("; ".join(failures))
        if source_profile.get("profile_id") != source["profile_id"]:
            raise ValueError(f"versioned corpus source profile identity drifted: {source['family']}")

    entries: dict[str, CorpusEntry] = {}
    for relative, fixture in base.materialize_specs(source_profiles["base"]).items():
        add_entry(entries, f"base/{relative}", "base", relative, fixture.content)

    symlinks = []
    for relative, fixture in paths.materialize_specs(source_profiles["paths"]).items():
        if fixture.kind == "file":
            add_entry(entries, f"paths/{relative}", "paths", relative, fixture.content)
        else:
            symlinks.append(
                {
                    "materialize_only_in_temporary_sandbox": True,
                    "path": relative,
                    "target": fixture.target,
                }
            )
    add_entry(
        entries,
        "paths/symlink-declarations.json",
        "paths",
        "<generated-from-path-profile>",
        canonical_json({"schema_version": 1, "symlinks": symlinks}),
        kind="symlink-declarations",
    )

    for relative, fixture in adversarial.materialize_specs(
        source_profiles["adversarial"]
    ).items():
        stripped = relative.removeprefix("adversarial/")
        add_entry(
            entries,
            f"adversarial/{stripped}",
            "adversarial",
            relative,
            fixture.content,
        )

    for relative, fixture in documents.materialize_specs(
        source_profiles["documents"]
    ).items():
        stripped = relative.removeprefix("documents/")
        add_entry(
            entries,
            f"documents/{stripped}",
            "documents",
            relative,
            fixture.content,
        )

    for relative, fixture in golden.materialize_specs(
        source_profiles["golden"], root
    ).items():
        stripped = relative.removeprefix("expected-outputs/")
        add_entry(entries, f"golden/{stripped}", "golden", relative, fixture.content)
    return dict(sorted(entries.items()))


def archive_bytes(entries: dict[str, CorpusEntry]) -> bytes:
    output = io.BytesIO()
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_STORED) as archive:
        for path, entry in sorted(entries.items()):
            info = zipfile.ZipInfo(path, ZIP_TIMESTAMP)
            info.compress_type = zipfile.ZIP_STORED
            info.create_system = 3
            info.external_attr = 0o100644 << 16
            archive.writestr(info, entry.content)
    return output.getvalue()


def normalized_names(names: list[str]) -> list[str]:
    return [unicodedata.normalize("NFC", name).casefold() for name in names]


def validate_archive(content: bytes, entries: dict[str, CorpusEntry]) -> list[str]:
    failures: list[str] = []
    try:
        archive = zipfile.ZipFile(io.BytesIO(content))
    except (OSError, zipfile.BadZipFile) as error:
        return [f"versioned corpus archive is invalid: {error}"]
    with archive:
        infos = archive.infolist()
        names = [info.filename for info in infos]
        if names != sorted(entries):
            failures.append("versioned corpus archive entry closure or order drifted")
        if len(names) != len(set(names)) or len(normalized_names(names)) != len(
            set(normalized_names(names))
        ):
            failures.append("versioned corpus archive has duplicate normalized names")
        try:
            corrupt_name = archive.testzip()
        except (OSError, RuntimeError, zipfile.BadZipFile) as error:
            failures.append(f"versioned corpus archive corruption check failed: {error}")
            corrupt_name = None
        if corrupt_name is not None:
            failures.append(f"versioned corpus archive has a corrupt entry: {corrupt_name}")
        for info in infos:
            path = PurePosixPath(info.filename)
            if path.is_absolute() or ".." in path.parts or not safe_relative(info.filename):
                failures.append(f"versioned corpus archive has an unsafe path: {info.filename}")
            mode = info.external_attr >> 16
            if mode & 0o170000 == 0o120000:
                failures.append(f"versioned corpus archive contains a symlink: {info.filename}")
            if mode & 0o111:
                failures.append(f"versioned corpus archive contains executable mode: {info.filename}")
            if info.compress_type != zipfile.ZIP_STORED:
                failures.append(f"versioned corpus archive contains compressed data: {info.filename}")
            if info.flag_bits & 0x1:
                failures.append(f"versioned corpus archive contains encrypted data: {info.filename}")
            expected = entries.get(info.filename)
            try:
                observed = archive.read(info.filename)
            except (OSError, RuntimeError, zipfile.BadZipFile) as error:
                failures.append(
                    f"versioned corpus archive entry cannot be read: {info.filename}: {error}"
                )
                continue
            if expected is None or observed != expected.content:
                failures.append(f"versioned corpus archive entry content drifted: {info.filename}")
    return failures


def profile_provenance(profile: dict[str, Any], root: Path) -> list[dict[str, Any]]:
    return [
        {
            "family": source["family"],
            "generator": source["generator"],
            "generator_sha256": sha256_bytes((root / source["generator"]).read_bytes()),
            "namespace": source["namespace"],
            "profile_id": source["profile_id"],
            "profile_path": source["path"],
            "profile_sha256": sha256_bytes((root / source["path"]).read_bytes()),
        }
        for source in profile["sources"]
    ]


def build_manifest(
    profile: dict[str, Any],
    entries: dict[str, CorpusEntry],
    archive: bytes,
    root: Path = ROOT,
) -> dict[str, Any]:
    golden_states = []
    for path, entry in entries.items():
        if entry.family == "golden":
            record = json.loads(entry.content)
            golden_states.append(
                {
                    "evidence_state": record["expected_output"]["evidence"]["state"],
                    "manifest_id": record["manifest_id"],
                    "path": path,
                    "sha256": sha256_bytes(entry.content),
                }
            )
    manifest = {
        "schema_version": 1,
        "corpus_id": profile["profile_id"],
        "corpus_version": profile["corpus_version"],
        "status": "versioned-synthetic-corpus",
        "corpus_profile_sha256": sha256_bytes(
            (root / PROFILE_PATH.relative_to(ROOT)).read_bytes()
        ),
        "archive": {
            "bytes": len(archive),
            "entry_count": len(entries),
            "format": profile["archive"]["format"],
            "path": profile["archive"]["path"],
            "sha256": sha256_bytes(archive),
        },
        "assembler": {
            "path": "scripts/versioned_corpus.py",
            "sha256": sha256_bytes((root / "scripts/versioned_corpus.py").read_bytes()),
        },
        "source_provenance": profile_provenance(profile, root),
        "family_counts": {
            family: sum(entry.family == family for entry in entries.values())
            for family, _namespace, _path, _generator, _profile_id in EXPECTED_SOURCES
        },
        "entries": [
            {
                "archive_path": path,
                "bytes": len(entry.content),
                "family": entry.family,
                "kind": entry.kind,
                "sha256": sha256_bytes(entry.content),
                "source_relative_path": entry.source_relative_path,
            }
            for path, entry in sorted(entries.items())
        ],
        "golden_manifests": sorted(golden_states, key=lambda item: item["path"]),
        "archive_safety_contract": profile["archive_safety_contract"],
        "content_contract": profile["content_contract"],
        "path_fixture_materialization": profile["path_fixture_materialization"],
        "product_support_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }
    manifest["manifest_sha256"] = sha256_bytes(canonical_json(manifest))
    return manifest


def validate_manifest(
    manifest: Any,
    profile: dict[str, Any],
    entries: dict[str, CorpusEntry],
    archive: bytes,
    root: Path = ROOT,
) -> list[str]:
    if not isinstance(manifest, dict):
        return ["versioned corpus manifest must be an object"]
    failures: list[str] = []
    unhashed = copy.deepcopy(manifest)
    recorded_hash = unhashed.pop("manifest_sha256", None)
    if not isinstance(recorded_hash, str) or sha256_bytes(canonical_json(unhashed)) != recorded_hash:
        failures.append("versioned corpus manifest self-hash is invalid")
    if manifest.get("archive", {}).get("sha256") != sha256_bytes(archive):
        failures.append("versioned corpus archive hash is invalid")
    if manifest.get("archive", {}).get("entry_count") != len(entries):
        failures.append("versioned corpus manifest entry count is invalid")
    if manifest.get("path_fixture_materialization") != "inert-declarations-only":
        failures.append("versioned corpus manifest permits active path fixtures")
    if manifest.get("product_support_claim") != "none":
        failures.append("versioned corpus manifest made a product support claim")
    if manifest.get("macos_execution_status") != "blocked-macos" or manifest.get(
        "macos_support_claim"
    ) != "none":
        failures.append("versioned corpus manifest made an invalid macOS claim")
    if manifest != build_manifest(profile, entries, archive, root):
        failures.append("versioned corpus manifest is stale or non-deterministic")
    return failures


def build_corpus(profile: dict[str, Any], root: Path = ROOT) -> tuple[bytes, dict[str, Any]]:
    failures = validate_profile(profile)
    if failures:
        raise ValueError("; ".join(failures))
    entries = materialize_entries(profile, root)
    archive = archive_bytes(entries)
    failures = validate_archive(archive, entries)
    if failures:
        raise ValueError("; ".join(failures))
    manifest = build_manifest(profile, entries, archive, root)
    failures = validate_manifest(manifest, profile, entries, archive, root)
    if failures:
        raise ValueError("; ".join(failures))
    return archive, manifest


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(prefix=".agentmage-corpus-", dir=path.parent)
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


def generate(profile: dict[str, Any], destination: Path, root: Path = ROOT) -> dict[str, Any]:
    if destination.exists():
        raise FileExistsError("versioned corpus destination already exists")
    destination.parent.mkdir(parents=True, exist_ok=True)
    staging = Path(tempfile.mkdtemp(prefix=".agentmage-corpus-v1-", dir=destination.parent))
    archive, manifest = build_corpus(profile, root)
    try:
        archive_name = Path(profile["archive"]["path"]).name
        manifest_name = Path(profile["manifest_path"]).name
        (staging / archive_name).write_bytes(archive)
        (staging / manifest_name).write_bytes(canonical_json(manifest))
        for path in staging.iterdir():
            path.chmod(0o644)
        os.replace(staging, destination)
    except BaseException:
        shutil.rmtree(staging, ignore_errors=True)
        raise
    return {
        "archive_sha256": sha256_bytes(archive),
        "entry_count": manifest["archive"]["entry_count"],
        "manifest_sha256": manifest["manifest_sha256"],
    }


def write_checked_corpus(root: Path = ROOT) -> None:
    profile = read_json(root / PROFILE_PATH.relative_to(ROOT))
    archive, manifest = build_corpus(profile, root)
    write_atomic(root / profile["archive"]["path"], archive)
    write_atomic(root / profile["manifest_path"], canonical_json(manifest))


def check_checked_corpus(root: Path = ROOT) -> list[str]:
    try:
        profile = read_json(root / PROFILE_PATH.relative_to(ROOT))
        expected_archive, expected_manifest = build_corpus(profile, root)
        actual_archive = (root / profile["archive"]["path"]).read_bytes()
        actual_manifest = read_json(root / profile["manifest_path"])
    except (OSError, ValueError, json.JSONDecodeError) as error:
        return [f"cannot validate checked-in versioned corpus: {error}"]
    failures: list[str] = []
    if actual_archive != expected_archive:
        failures.append("checked-in versioned corpus archive is stale or corrupt")
    if actual_manifest != expected_manifest:
        failures.append("checked-in versioned corpus manifest is stale or corrupt")
    entries = materialize_entries(profile, root)
    failures.extend(validate_archive(actual_archive, entries))
    failures.extend(validate_manifest(actual_manifest, profile, entries, actual_archive, root))
    return failures


def build_report(root: Path = ROOT) -> dict[str, Any]:
    profile_path = root / PROFILE_PATH.relative_to(ROOT)
    profile = read_json(profile_path)
    failures = check_checked_corpus(root)
    if failures:
        raise ValueError("; ".join(failures))
    manifest = read_json(root / profile["manifest_path"])
    return {
        "schema_version": 1,
        "task_id": "2.1.2.1",
        "status": "pass",
        "corpus": {
            "archive_bytes": manifest["archive"]["bytes"],
            "archive_path": manifest["archive"]["path"],
            "archive_sha256": manifest["archive"]["sha256"],
            "entry_count": manifest["archive"]["entry_count"],
            "family_counts": manifest["family_counts"],
            "manifest_path": profile["manifest_path"],
            "manifest_sha256": manifest["manifest_sha256"],
            "version": manifest["corpus_version"],
        },
        "golden_manifest_count": len(manifest["golden_manifests"]),
        "golden_evidence_states": sorted(
            item["evidence_state"] for item in manifest["golden_manifests"]
        ),
        "source_provenance": manifest["source_provenance"],
        "assembler": manifest["assembler"],
        "archive_safety_contract": manifest["archive_safety_contract"],
        "content_contract": manifest["content_contract"],
        "path_fixture_materialization": "inert-declarations-only",
        "product_support_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }


def validate_report(report: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(report, dict):
        return ["versioned corpus report must be an object"]
    failures: list[str] = []
    if report.get("schema_version") != 1 or report.get("task_id") != "2.1.2.1":
        failures.append("versioned corpus report identity is invalid")
    if report.get("status") != "pass":
        failures.append("versioned corpus report did not pass")
    if report.get("product_support_claim") != "none":
        failures.append("versioned corpus report made a product support claim")
    if report.get("macos_execution_status") != "blocked-macos" or report.get(
        "macos_support_claim"
    ) != "none":
        failures.append("versioned corpus report made an invalid macOS claim")
    if report != build_report(root):
        failures.append("versioned corpus report is stale or non-deterministic")
    return failures


def write_report(root: Path = ROOT) -> None:
    write_atomic(root / REPORT_PATH.relative_to(ROOT), canonical_json(build_report(root)))


def check_report(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read versioned corpus report: {error}"]
    return validate_report(report, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path)
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--write-report", action="store_true")
    args = parser.parse_args()
    try:
        profile = read_json(PROFILE_PATH)
        if args.output is not None:
            print(json.dumps(generate(profile, args.output), indent=2, sort_keys=True))
        if args.write:
            write_checked_corpus()
        if args.write_report:
            write_report()
        failures = check_checked_corpus() + check_report()
    except (OSError, ValueError) as error:
        print(f"versioned corpus validation failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"versioned corpus validation failed: {failure}", file=sys.stderr)
        return 1
    if args.output is None:
        print("versioned synthetic corpus and golden manifests validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
