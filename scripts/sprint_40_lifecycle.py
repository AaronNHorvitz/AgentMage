#!/usr/bin/env python3
"""Record the local v0.2-to-v0.3 package and state lifecycle campaign."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
from pathlib import Path
from typing import Any, Final

try:
    from scripts.package_candidate import build_all
    from scripts.package_lifecycle import (
        validate_container_lifecycle,
        verify_container_lifecycle,
        verify_extracted_candidates,
    )
except ModuleNotFoundError:
    from package_candidate import build_all  # type: ignore[no-redef]
    from package_lifecycle import (  # type: ignore[no-redef]
        validate_container_lifecycle,
        verify_container_lifecycle,
        verify_extracted_candidates,
    )


ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-40/lifecycle-report.json"
CANDIDATE_ROOT: Final = ROOT / "target/sprint-40-lifecycle"
CLEAN_BUILD_REPORT: Final = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-1"
    / "story-1.1"
    / "clean-build-report.json"
)
BASELINE_VERSION: Final = "0.2.0"
UPGRADE_VERSION: Final = "0.3.0"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
TEST_RESULT: Final = re.compile(
    rb"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;"
)
SOURCE_PATHS: Final = (
    "architecture/clean-build-policy.json",
    "artifacts/sprints/sprint-1/story-1.1/clean-build-report.json",
    "scripts/package_candidate.py",
    "scripts/package_lifecycle.py",
    "scripts/sprint_40_lifecycle.py",
    "tests/test_package_lifecycle.py",
    "tests/test_sprint_40_lifecycle.py",
)
BUILD_COMMANDS: Final = (
    (
        "vscode-release-build",
        ("npm", "run", "build", "--workspace", "@agentmage/vscode-shell"),
    ),
    (
        "native-release-build",
        (
            "cargo",
            "build",
            "--release",
            "-p",
            "agentmage-host",
            "-p",
            "agentmage-capability-read-only",
            "-p",
            "agentmage-platform-linux-inference",
            "--bins",
            "--locked",
        ),
    ),
)
STATE_COMMANDS: Final = (
    (
        "configuration-backup-rollback",
        (
            "cargo",
            "test",
            "-p",
            "agentmage-platform-linux",
            "apply_retains_immutable_backup_and_rollback_restores_it",
            "--locked",
        ),
    ),
    (
        "configuration-migration-interruption",
        (
            "cargo",
            "test",
            "-p",
            "agentmage-platform-linux",
            "migration_interruptions_select_valid_state_and_rollback_is_repeatable",
            "--locked",
        ),
    ),
    (
        "operational-store-backup-restore",
        (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "encrypted_backup_restores_only_to_a_verified_fresh_candidate",
            "--locked",
        ),
    ),
    (
        "authority-interrupted-operation-resume",
        (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "encrypted_restart_recovery_never_replays_and_publishes_one_receipt",
            "--locked",
        ),
    ),
    (
        "write-interrupted-operation-resume",
        (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "restart_matrix_never_replays_a_consumed_or_completed_write",
            "--locked",
        ),
    ),
)
LIMITATIONS: Final = (
    "The campaign uses unsigned local candidates and does not approve or publish a release.",
    "Fedora and Ubuntu package-manager execution occurs in exact locally retained clean-build images with dependencies pre-provisioned, rootless Podman, and networking disabled; it is not installed-host or graphical-client evidence.",
    "The state campaign composes focused native test fixtures; it does not claim macOS, Windows, independent-review, or trusted-package-launcher evidence.",
    "Manual fuzzing remains deferred by the recorded project decision.",
)


class Sprint40LifecycleError(ValueError):
    """Raised when the retained lifecycle campaign is invalid or incomplete."""


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_output(*arguments: str) -> bytes:
    result = subprocess.run(
        ("git", *arguments), cwd=ROOT, check=False, capture_output=True, timeout=30
    )
    if result.returncode != 0:
        raise Sprint40LifecycleError("sprint40.lifecycle.git_identity")
    return result.stdout.strip()


def git_file(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ("git", "show", f"{revision}:{path}"),
        cwd=ROOT,
        check=False,
        capture_output=True,
        timeout=30,
    )
    if result.returncode != 0:
        raise Sprint40LifecycleError(f"sprint40.lifecycle.source_absent:{path}")
    return result.stdout


def run_record(identifier: str, argv: tuple[str, ...], *, test: bool) -> dict[str, Any]:
    executable = shutil.which(argv[0])
    if executable is None:
        raise Sprint40LifecycleError(f"sprint40.lifecycle.tool_absent:{argv[0]}")
    environment = os.environ.copy()
    environment["CARGO_NET_OFFLINE"] = "true"
    result = subprocess.run(
        (executable, *argv[1:]),
        cwd=ROOT,
        env=environment,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
        timeout=1800,
    )
    matches = TEST_RESULT.findall(result.stdout) if test else []
    passed = sum(int(match[0]) for match in matches)
    ignored = sum(int(match[1]) for match in matches)
    record = {
        "id": identifier,
        "argv": list(argv),
        "exit_code": result.returncode,
        "output_bytes": len(result.stdout),
        "output_sha256": digest(result.stdout),
        "observed_test_passes": passed if test else None,
        "observed_test_ignores": ignored if test else None,
    }
    if result.returncode != 0 or (test and (passed < 1 or ignored != 0)):
        raise Sprint40LifecycleError(f"sprint40.lifecycle.command_failed:{identifier}")
    return record


def lifecycle_images() -> tuple[str, str]:
    report = json.loads(CLEAN_BUILD_REPORT.read_text(encoding="utf-8"))
    if (
        report.get("status") != "pass-linux"
        or report.get("summary", {}).get("linux_scope_complete") is not True
        or report.get("summary", {}).get("cross_platform_task_complete") is not False
    ):
        raise Sprint40LifecycleError("sprint40.lifecycle.clean_build_report")
    runs = report.get("platform_runs")
    if not isinstance(runs, dict):
        raise Sprint40LifecycleError("sprint40.lifecycle.clean_build_report")
    references = []
    for platform_id in ("fedora-x86_64", "ubuntu-x86_64"):
        run = runs.get(platform_id)
        tag = f"localhost/agentmage-clean-build:{platform_id}"
        if not isinstance(run, dict) or run.get("status") != "pass":
            raise Sprint40LifecycleError("sprint40.lifecycle.clean_build_platform")
        inspected = subprocess.run(
            ("podman", "image", "inspect", tag),
            cwd=ROOT,
            check=False,
            capture_output=True,
            timeout=30,
        )
        try:
            image = json.loads(inspected.stdout)[0]
            image_id = image["Id"]
            repo_digests = sorted(image["RepoDigests"])
        except (IndexError, KeyError, TypeError, json.JSONDecodeError) as error:
            raise Sprint40LifecycleError(
                "sprint40.lifecycle.clean_build_image_absent"
            ) from error
        if (
            inspected.returncode != 0
            or f"sha256:{image_id}" != run.get("container_image_id")
            or len(repo_digests) != 1
            or not repo_digests[0].startswith("localhost/agentmage-clean-build@sha256:")
        ):
            raise Sprint40LifecycleError("sprint40.lifecycle.clean_build_image_drift")
        references.append(repo_digests[0])
    return references[0], references[1]


def artifact_records(artifacts: dict[str, dict[str, Path]]) -> list[dict[str, Any]]:
    records = []
    for version in (BASELINE_VERSION, UPGRADE_VERSION):
        for kind in ("deb", "rpm", "vsix"):
            path = artifacts[version][kind]
            records.append(
                {
                    "version": version,
                    "kind": kind,
                    "name": path.name,
                    "size": path.stat().st_size,
                    "sha256": digest(path.read_bytes()),
                    "signed": False,
                    "published": False,
                    "retained_in_repository": False,
                }
            )
    return records


def build_report(revision: str) -> dict[str, Any]:
    if not REVISION.fullmatch(revision):
        raise Sprint40LifecycleError("sprint40.lifecycle.revision")
    if git_output("rev-parse", "HEAD").decode() != revision:
        raise Sprint40LifecycleError("sprint40.lifecycle.revision_not_head")
    build_records = [
        run_record(identifier, argv, test=False)
        for identifier, argv in BUILD_COMMANDS
    ]
    artifacts = {
        version: build_all(CANDIDATE_ROOT / version, version)
        for version in (BASELINE_VERSION, UPGRADE_VERSION)
    }
    extraction = verify_extracted_candidates(
        artifacts[BASELINE_VERSION], artifacts[UPGRADE_VERSION]
    )
    fedora, ubuntu = lifecycle_images()
    package_results = verify_container_lifecycle(
        artifacts,
        fedora,
        ubuntu,
        BASELINE_VERSION,
        UPGRADE_VERSION,
    )
    state_records = [
        run_record(identifier, argv, test=True)
        for identifier, argv in STATE_COMMANDS
    ]
    report = {
        "schema_version": 1,
        "record_type": "sprint_40_v0_3_lifecycle",
        "source_revision": revision,
        "source_tree": git_output("rev-parse", f"{revision}^{{tree}}").decode(),
        "source_sha256": {
            path: digest(git_file(revision, path)) for path in SOURCE_PATHS
        },
        "environment": {
            "system": platform.system(),
            "release": platform.release(),
            "machine": platform.machine(),
            "rootless_container_runtime": "podman",
            "network_used_by_package_lifecycle": False,
        },
        "versions": {
            "baseline": BASELINE_VERSION,
            "upgrade": UPGRADE_VERSION,
        },
        "build_commands": build_records,
        "candidate_artifacts": artifact_records(artifacts),
        "extracted_candidate_checks": extraction,
        "package_lifecycle": package_results["container_lifecycle"],
        "state_lifecycle_commands": state_records,
        "acceptance": {
            "clean_install": True,
            "corrupt_upgrade_refusal": True,
            "upgrade": True,
            "downgrade": True,
            "uninstall_and_residue": True,
            "backup_and_restore": True,
            "migration_interruption_recovery": True,
            "interrupted_authority_resume_without_replay": True,
            "interrupted_write_resume_without_replay": True,
            "complete_local_lifecycle_campaign": True,
        },
        "limitations": list(LIMITATIONS),
        "release_claim": False,
    }
    failures = validate_report(report, verify_current=False)
    if failures:
        raise Sprint40LifecycleError("; ".join(failures))
    return report


def validate_report(value: Any, *, verify_current: bool = True) -> list[str]:
    if not isinstance(value, dict):
        return ["report must be an object"]
    failures: list[str] = []
    revision = value.get("source_revision")
    if (
        value.get("schema_version") != 1
        or value.get("record_type") != "sprint_40_v0_3_lifecycle"
        or not isinstance(revision, str)
        or REVISION.fullmatch(revision) is None
        or not isinstance(value.get("source_tree"), str)
        or REVISION.fullmatch(value["source_tree"]) is None
    ):
        failures.append("report identity changed")
    if value.get("versions") != {
        "baseline": BASELINE_VERSION,
        "upgrade": UPGRADE_VERSION,
    }:
        failures.append("version transition changed")
    sources = value.get("source_sha256")
    if (
        not isinstance(sources, dict)
        or set(sources) != set(SOURCE_PATHS)
        or any(not isinstance(item, str) or SHA256.fullmatch(item) is None for item in sources.values())
    ):
        failures.append("source inventory changed")
    for field, expected in (
        ("build_commands", BUILD_COMMANDS),
        ("state_lifecycle_commands", STATE_COMMANDS),
    ):
        records = value.get(field)
        if (
            not isinstance(records, list)
            or [item.get("id") for item in records if isinstance(item, dict)]
            != [item[0] for item in expected]
            or any(
                not isinstance(item, dict)
                or item.get("exit_code") != 0
                or not isinstance(item.get("output_bytes"), int)
                or item.get("output_bytes", -1) < 0
                or not isinstance(item.get("output_sha256"), str)
                or SHA256.fullmatch(item["output_sha256"]) is None
                for item in records or []
            )
        ):
            failures.append(f"{field} changed")
    state_records = value.get("state_lifecycle_commands")
    if isinstance(state_records, list) and any(
        item.get("observed_test_passes", 0) < 1
        or item.get("observed_test_ignores") != 0
        for item in state_records
        if isinstance(item, dict)
    ):
        failures.append("state lifecycle test result changed")
    artifacts = value.get("candidate_artifacts")
    expected_pairs = [
        (version, kind)
        for version in (BASELINE_VERSION, UPGRADE_VERSION)
        for kind in ("deb", "rpm", "vsix")
    ]
    if (
        not isinstance(artifacts, list)
        or [(item.get("version"), item.get("kind")) for item in artifacts if isinstance(item, dict)]
        != expected_pairs
        or any(
            not isinstance(item, dict)
            or not isinstance(item.get("size"), int)
            or item.get("size", 0) <= 0
            or not isinstance(item.get("sha256"), str)
            or SHA256.fullmatch(item["sha256"]) is None
            or item.get("signed") is not False
            or item.get("published") is not False
            or item.get("retained_in_repository") is not False
            for item in artifacts or []
        )
    ):
        failures.append("candidate artifact inventory changed")
    if value.get("extracted_candidate_checks") != {
        "extracted_payloads": "pass",
        "mutation_refusal": "pass",
    }:
        failures.append("extracted candidate checks changed")
    failures.extend(
        f"package lifecycle: {failure}"
        for failure in validate_container_lifecycle(
            value.get("package_lifecycle"), BASELINE_VERSION, UPGRADE_VERSION
        )
    )
    expected_acceptance = {
        "clean_install": True,
        "corrupt_upgrade_refusal": True,
        "upgrade": True,
        "downgrade": True,
        "uninstall_and_residue": True,
        "backup_and_restore": True,
        "migration_interruption_recovery": True,
        "interrupted_authority_resume_without_replay": True,
        "interrupted_write_resume_without_replay": True,
        "complete_local_lifecycle_campaign": True,
    }
    if value.get("acceptance") != expected_acceptance:
        failures.append("acceptance result changed")
    if value.get("limitations") != list(LIMITATIONS) or value.get("release_claim") is not False:
        failures.append("limitation or release boundary changed")
    if verify_current and isinstance(revision, str) and REVISION.fullmatch(revision):
        for path in SOURCE_PATHS:
            try:
                if not isinstance(sources, dict) or sources.get(path) != digest(git_file(revision, path)):
                    failures.append(f"source hash drift: {path}")
            except Sprint40LifecycleError as error:
                failures.append(str(error))
    return failures


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--record", action="store_true")
    parser.add_argument("--revision")
    parser.add_argument("--verify", type=Path)
    arguments = parser.parse_args()
    if arguments.verify:
        failures = validate_report(
            json.loads(arguments.verify.read_text(encoding="utf-8"))
        )
        if failures:
            print("Sprint 40 lifecycle validation failed:")
            for failure in failures:
                print(f"- {failure}")
            return 1
        print("Sprint 40 lifecycle validated")
        return 0
    if not arguments.record or not arguments.revision:
        parser.error("--record requires --revision")
    report = build_report(arguments.revision)
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(OUTPUT.relative_to(ROOT))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
