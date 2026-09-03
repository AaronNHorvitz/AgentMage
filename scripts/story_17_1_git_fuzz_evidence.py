#!/usr/bin/env python3
"""Run and validate the bounded offline Story 17.1 Git parser fuzz campaign."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import tempfile
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-17/git-parser-fuzz-report.json"
FUZZ_ROOT: Final = ROOT / "fuzzing/git-inspection"
CORPUS: Final = FUZZ_ROOT / "corpus/git_inspection"
DICTIONARY: Final = ROOT / "fuzzing/dictionaries/git.dict"
SOURCE_PATHS: Final = (
    "fuzzing/git-inspection/Cargo.toml",
    "fuzzing/git-inspection/fuzz_targets/git_inspection.rs",
    "fuzzing/git-inspection/corpus/git_inspection/empty-request.json",
    "fuzzing/git-inspection/corpus/git_inspection/hostile-status.bin",
    "fuzzing/git-inspection/corpus/git_inspection/valid-status.json",
    "fuzzing/dictionaries/git.dict",
    "scripts/story_17_1_git_fuzz_evidence.py",
    "tests/test_story_17_1_git_fuzz_evidence.py",
)
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
DONE: Final = re.compile(rb"Done [0-9]+ runs in [0-9]+ second")


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        timeout=30,
    )
    if result.returncode:
        raise ValueError(f"committed fuzz source unavailable: {path}")
    return result.stdout


def source_records(revision: str) -> list[dict[str, Any]]:
    return [
        {
            "path": path,
            "bytes": len(data := git_bytes(revision, path)),
            "sha256": sha256_bytes(data),
        }
        for path in SOURCE_PATHS
    ]


def run_campaign() -> dict[str, Any]:
    with tempfile.TemporaryDirectory(prefix="agentmage-git-fuzz-") as name:
        corpus = Path(name) / "corpus"
        shutil.copytree(CORPUS, corpus)
        artifact_prefix = Path(name) / "artifacts"
        artifact_prefix.mkdir()
        argv = [
            "cargo",
            "+nightly-2026-08-01",
            "fuzz",
            "run",
            "git_inspection",
            corpus.as_posix(),
            "--",
            "-max_total_time=60",
            "-timeout=5",
            "-rss_limit_mb=1024",
            "-max_len=4096",
            "-jobs=1",
            "-workers=1",
            f"-dict={DICTIONARY.as_posix()}",
            f"-artifact_prefix={artifact_prefix.as_posix()}/",
        ]
        environment = dict(os.environ)
        environment["CARGO_NET_OFFLINE"] = "true"
        result = subprocess.run(
            argv,
            cwd=FUZZ_ROOT,
            env=environment,
            capture_output=True,
            timeout=120,
        )
        output = result.stdout + result.stderr
        artifacts = [path for path in artifact_prefix.iterdir() if path.is_file()]
        return {
            "argv": [
                *argv[:5],
                "<temporary-corpus>",
                *argv[6:13],
                "-dict=fuzzing/dictionaries/git.dict",
                "-artifact_prefix=<temporary-artifacts>/",
            ],
            "exit_code": result.returncode,
            "output_sha256": sha256_bytes(output),
            "completion_marker_present": DONE.search(output) is not None,
            "crash_artifact_count": len(artifacts),
        }


def build_report(revision: str, campaign: dict[str, Any]) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "story_17_1_git_parser_fuzz_campaign",
        "target_id": "FT-GIT-001",
        "source_revision": revision,
        "sources": source_records(revision),
        "campaign": campaign,
        "toolchain": {
            "cargo_fuzz": "0.13.2",
            "rust": "nightly-2026-08-01",
            "sanitizer": "address+leak",
        },
        "limits": {
            "duration_seconds": 60,
            "input_bytes": 4096,
            "per_input_timeout_seconds": 5,
            "resident_memory_mib": 1024,
            "parallel_jobs": 1,
        },
        "seed_count": len(list(CORPUS.iterdir())),
        "network_used": False,
        "private_user_data_used": False,
        "manual_parser_fuzzing_complete": campaign["exit_code"] == 0
        and campaign["completion_marker_present"]
        and campaign["crash_artifact_count"] == 0,
        "product_support_claim": "none",
        "release_claim": False,
    }


def validate_report(report: dict[str, Any], *, verify_current: bool = True) -> list[str]:
    failures: list[str] = []
    revision = str(report.get("source_revision", ""))
    if (
        report.get("schema_version") != 1
        or report.get("record_type") != "story_17_1_git_parser_fuzz_campaign"
        or report.get("target_id") != "FT-GIT-001"
        or REVISION.fullmatch(revision) is None
    ):
        failures.append("Git fuzz report identity changed")
    campaign = report.get("campaign", {})
    if (
        campaign.get("exit_code") != 0
        or campaign.get("completion_marker_present") is not True
        or campaign.get("crash_artifact_count") != 0
        or SHA256.fullmatch(str(campaign.get("output_sha256", ""))) is None
    ):
        failures.append("Git fuzz campaign did not complete cleanly")
    if report.get("manual_parser_fuzzing_complete") is not True:
        failures.append("Git parser fuzz completion is absent")
    if report.get("limits") != {
        "duration_seconds": 60,
        "input_bytes": 4096,
        "per_input_timeout_seconds": 5,
        "resident_memory_mib": 1024,
        "parallel_jobs": 1,
    }:
        failures.append("Git fuzz resource limits changed")
    if (
        report.get("seed_count") != 3
        or report.get("network_used") is not False
        or report.get("private_user_data_used") is not False
        or report.get("product_support_claim") != "none"
        or report.get("release_claim") is not False
    ):
        failures.append("Git fuzz scope or safety claim changed")
    sources = report.get("sources", [])
    if [item.get("path") for item in sources] != list(SOURCE_PATHS):
        failures.append("Git fuzz source closure changed")
    elif verify_current and REVISION.fullmatch(revision):
        if sources != source_records(revision):
            failures.append("Git fuzz source binding is stale")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    args = parser.parse_args()
    if args.write:
        revision = subprocess.run(
            ["git", "rev-parse", f"{args.source_revision}^{{commit}}"],
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
        report = build_report(revision, run_campaign())
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    report = json.loads(OUTPUT.read_text())
    failures = validate_report(report)
    if failures:
        for failure in failures:
            print(failure)
        return 1
    print("Story 17.1 Git parser fuzz campaign: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
