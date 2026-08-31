#!/usr/bin/env python3
"""Emit, verify, and source-bind preliminary hosted macOS compatibility evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Callable, Mapping


ROOT = Path(__file__).resolve().parents[1]
REPORT_PATH = ROOT / "artifacts/sprints/sprint-8/story-8.1/macos-hosted-compatibility-source-contract.json"
WORKFLOW_PATH = ".github/workflows/macos.yml"
CHECKOUT_SHA = "11d5960a326750d5838078e36cf38b85af677262"
UPLOAD_SHA = "043fb46d1a93c77aae656e7c1c64a875d1fc6a0a"
COMMANDS = (
    ("swift", "build", "--package-path", "platforms/macos"),
    ("swift", "test", "--package-path", "platforms/macos"),
)
SOURCE_PATHS = (
    WORKFLOW_PATH,
    "architecture/product-ci-policy.json",
    "docs/product-ci-and-clean-build.md",
    "scripts/macos_hosted_compatibility.py",
    "scripts/product_ci.py",
    "tests/test_macos_hosted_compatibility.py",
    "tests/test_product_ci.py",
)
LIMITATIONS = (
    "no-physical-macbook-pro-m5-qualification",
    "no-signing-notarization-stapling-or-gatekeeper-evidence",
    "no-app-sandbox-xpc-keychain-metal-install-or-lifecycle-evidence",
    "no-macos-support-or-release-authority",
)
OUTCOMES = {"success", "failure", "skipped", "cancelled"}
HEX40 = re.compile(r"^[0-9a-f]{40}$")
HEX64 = re.compile(r"^[0-9a-f]{64}$")


class CompatibilityEvidenceError(ValueError):
    """Raised when hosted compatibility evidence is incomplete or unsafe."""


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def sha256(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def exact_keys(value: Any, expected: set[str], label: str) -> list[str]:
    if not isinstance(value, dict):
        return [f"{label} must be an object"]
    if set(value) != expected:
        return [f"{label} field closure changed"]
    return []


def git(*args: str, root: Path = ROOT) -> str:
    result = subprocess.run(
        ["git", *args], cwd=root, stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, check=False, timeout=20,
    )
    if result.returncode != 0:
        raise CompatibilityEvidenceError(f"git operation failed: {args[0]}")
    return result.stdout.decode("utf-8", "strict").strip()


def git_bytes(revision: str, path: str, root: Path = ROOT) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=root, stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, check=False, timeout=20,
    )
    if result.returncode != 0:
        raise CompatibilityEvidenceError(f"source is absent at revision: {path}")
    return result.stdout


def command_output(argv: list[str], root: Path = ROOT) -> str:
    result = subprocess.run(
        argv, cwd=root, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT, check=False, text=True, timeout=20,
    )
    value = result.stdout.strip()
    if result.returncode != 0 or not value or len(value) > 2048:
        raise CompatibilityEvidenceError(f"environment identity command failed: {argv[0]}")
    return value


def required_env(environ: Mapping[str, str], name: str) -> str:
    value = environ.get(name, "").strip()
    if not value or len(value) > 512:
        raise CompatibilityEvidenceError(f"required workflow identity is absent: {name}")
    return value


def build_result(
    environ: Mapping[str, str] = os.environ,
    root: Path = ROOT,
    runner: Callable[[list[str], Path], str] = command_output,
) -> dict[str, Any]:
    commit = required_env(environ, "GITHUB_SHA")
    if not HEX40.fullmatch(commit) or git("rev-parse", "HEAD", root=root) != commit:
        raise CompatibilityEvidenceError("checked-out source does not match GITHUB_SHA")
    architecture = runner(["uname", "-m"], root)
    if architecture != "arm64":
        raise CompatibilityEvidenceError("hosted compatibility run is not arm64")
    build = required_env(environ, "AGENTMAGE_BUILD_OUTCOME")
    test = required_env(environ, "AGENTMAGE_TEST_OUTCOME")
    if build not in OUTCOMES or test not in OUTCOMES:
        raise CompatibilityEvidenceError("workflow outcome is not closed")
    passed = build == test == "success"
    record = {
        "schema_version": 1,
        "task_id": "8.1.4.2",
        "artifact_id": "hosted-apple-silicon-source-compatibility",
        "status": "pass-preliminary-compatibility" if passed else "fail-preliminary-compatibility",
        "disposition": "preliminary-apple-silicon-source-compatibility-only",
        "source": {
            "repository": required_env(environ, "GITHUB_REPOSITORY"),
            "commit": commit,
            "tree": git("rev-parse", f"{commit}^{{tree}}", root=root),
            "workflow_path": WORKFLOW_PATH,
            "workflow_sha256": sha256((root / WORKFLOW_PATH).read_bytes()),
        },
        "workflow": {
            "workflow_ref": required_env(environ, "GITHUB_WORKFLOW_REF"),
            "run_id": required_env(environ, "GITHUB_RUN_ID"),
            "run_attempt": required_env(environ, "GITHUB_RUN_ATTEMPT"),
            "trigger": "workflow_dispatch",
            "budget_confirmed": True,
            "checkout_action_sha": CHECKOUT_SHA,
            "upload_action_sha": UPLOAD_SHA,
        },
        "runner": {
            "requested_label": "macos-15",
            "image_os": required_env(environ, "ImageOS"),
            "image_version": required_env(environ, "ImageVersion"),
            "architecture": architecture,
            "macos_product_version": runner(["sw_vers", "-productVersion"], root),
            "macos_build_version": runner(["sw_vers", "-buildVersion"], root),
            "xcode_version": runner(["xcodebuild", "-version"], root),
            "swift_version": runner(["swift", "--version"], root),
        },
        "commands": [
            {"argv": list(COMMANDS[0]), "outcome": build},
            {"argv": list(COMMANDS[1]), "outcome": test},
        ],
        "preliminary_compatibility": passed,
        "native_operations": {
            "signing": False,
            "notarization": False,
            "installation": False,
            "support_or_release_promotion": False,
        },
        "limitations": list(LIMITATIONS),
    }
    failures = validate_result(record, root=root, verify_git=True)
    if failures:
        raise CompatibilityEvidenceError("; ".join(failures))
    return record


def validate_result(value: Any, root: Path = ROOT, verify_git: bool = True) -> list[str]:
    failures = exact_keys(value, {
        "schema_version", "task_id", "artifact_id", "status", "disposition",
        "source", "workflow", "runner", "commands", "preliminary_compatibility",
        "native_operations", "limitations",
    }, "compatibility result")
    if failures:
        return failures
    if value["schema_version"] != 1 or value["task_id"] != "8.1.4.2":
        failures.append("compatibility result identity changed")
    if value["artifact_id"] != "hosted-apple-silicon-source-compatibility":
        failures.append("compatibility artifact identity changed")
    if value["disposition"] != "preliminary-apple-silicon-source-compatibility-only":
        failures.append("compatibility disposition overclaims authority")
    failures += exact_keys(value["source"], {
        "repository", "commit", "tree", "workflow_path", "workflow_sha256",
    }, "source")
    failures += exact_keys(value["workflow"], {
        "workflow_ref", "run_id", "run_attempt", "trigger", "budget_confirmed",
        "checkout_action_sha", "upload_action_sha",
    }, "workflow")
    failures += exact_keys(value["runner"], {
        "requested_label", "image_os", "image_version", "architecture",
        "macos_product_version", "macos_build_version", "xcode_version", "swift_version",
    }, "runner")
    failures += exact_keys(value["native_operations"], {
        "signing", "notarization", "installation", "support_or_release_promotion",
    }, "native operations")
    if failures:
        return failures
    source = value["source"]
    if not isinstance(source["repository"], str) or not source["repository"]:
        failures.append("source repository is absent")
    if not HEX40.fullmatch(source["commit"] or "") or not HEX40.fullmatch(source["tree"] or ""):
        failures.append("source commit or tree is invalid")
    if source["workflow_path"] != WORKFLOW_PATH or not HEX64.fullmatch(source["workflow_sha256"] or ""):
        failures.append("workflow source identity is invalid")
    workflow = value["workflow"]
    if workflow["trigger"] != "workflow_dispatch" or workflow["budget_confirmed"] is not True:
        failures.append("manual budget-confirmed trigger is not proven")
    if workflow["checkout_action_sha"] != CHECKOUT_SHA or workflow["upload_action_sha"] != UPLOAD_SHA:
        failures.append("workflow action identity changed")
    if not all(isinstance(workflow[key], str) and workflow[key] for key in ("workflow_ref", "run_id", "run_attempt")):
        failures.append("workflow run identity is incomplete")
    runner = value["runner"]
    if runner["requested_label"] != "macos-15" or runner["architecture"] != "arm64":
        failures.append("runner label or architecture changed")
    if not all(isinstance(runner[key], str) and 0 < len(runner[key]) <= 2048 for key in (
        "image_os", "image_version", "macos_product_version", "macos_build_version",
        "xcode_version", "swift_version",
    )):
        failures.append("runner image or tool identity is incomplete")
    commands = value["commands"]
    if not isinstance(commands, list) or len(commands) != 2:
        failures.append("command result closure changed")
        outcomes: list[str] = []
    else:
        outcomes = []
        for index, item in enumerate(commands):
            failures += exact_keys(item, {"argv", "outcome"}, f"command {index}")
            if isinstance(item, dict):
                if item.get("argv") != list(COMMANDS[index]):
                    failures.append(f"command {index} changed")
                if item.get("outcome") not in OUTCOMES:
                    failures.append(f"command {index} outcome is invalid")
                outcomes.append(item.get("outcome"))
    passed = outcomes == ["success", "success"]
    expected_status = "pass-preliminary-compatibility" if passed else "fail-preliminary-compatibility"
    if value["status"] != expected_status or value["preliminary_compatibility"] is not passed:
        failures.append("compatibility status contradicts command outcomes")
    if value["native_operations"] != {
        "signing": False, "notarization": False, "installation": False,
        "support_or_release_promotion": False,
    }:
        failures.append("native-operation authority was overclaimed")
    if value["limitations"] != list(LIMITATIONS):
        failures.append("compatibility limitations changed")
    if verify_git and not failures:
        try:
            if git("rev-parse", f"{source['commit']}^{{tree}}", root=root) != source["tree"]:
                failures.append("recorded source tree does not match commit")
            committed_workflow = git_bytes(source["commit"], WORKFLOW_PATH, root)
            if sha256(committed_workflow) != source["workflow_sha256"]:
                failures.append("recorded workflow hash does not match commit")
        except CompatibilityEvidenceError as error:
            failures.append(str(error))
    return failures


def build_source_contract(revision: str, root: Path = ROOT) -> dict[str, Any]:
    if not HEX40.fullmatch(revision):
        raise CompatibilityEvidenceError("source revision must be a full commit identity")
    sources = []
    for path in SOURCE_PATHS:
        committed = git_bytes(revision, path, root)
        if committed != (root / path).read_bytes():
            raise CompatibilityEvidenceError(f"source differs from reviewed revision: {path}")
        sources.append({"path": path, "sha256": sha256(committed), "bytes": len(committed)})
    workflow = (root / WORKFLOW_PATH).read_text(encoding="utf-8")
    return {
        "schema_version": 1,
        "task_id": "8.1.4.2",
        "artifact_id": "hosted-apple-silicon-compatibility-source-contract",
        "status": "ready-blocked-external",
        "source_revision": revision,
        "source_tree": git("rev-parse", f"{revision}^{{tree}}", root=root),
        "sources": sources,
        "workflow_contract": {
            "trigger": "workflow_dispatch",
            "budget_confirmation_required": True,
            "runner": "macos-15",
            "architecture": "arm64",
            "commands": [list(command) for command in COMMANDS],
            "checkout_action_sha": CHECKOUT_SHA,
            "upload_action_sha": UPLOAD_SHA,
            "artifact_retention_days": 30,
            "result_schema_version": 1,
            "automatic_trigger_count": 0,
            "secret_reference_count": workflow.count("secrets."),
        },
        "authority": {
            "workflow_dispatched": False,
            "actions_budget_confirmed": False,
            "hosted_result_retained": False,
            "preliminary_compatibility_proven": False,
            "macbook_pro_m5_qualified": False,
            "macos_support_or_release": False,
        },
        "remaining_blockers": [
            "maintainer-actions-budget-confirmation-absent",
            "manual-workflow-dispatch-not-performed",
            "hosted-result-artifact-not-retained-and-verified",
            "physical-macbook-pro-m5-qualification-remains-separate",
        ],
    }


def check_source_contract(root: Path = ROOT) -> list[str]:
    try:
        value = json.loads((root / REPORT_PATH.relative_to(ROOT)).read_text(encoding="utf-8"))
        revision = value.get("source_revision") if isinstance(value, dict) else None
        if not isinstance(revision, str):
            return ["hosted compatibility source revision is absent"]
        expected = build_source_contract(revision, root)
    except (OSError, json.JSONDecodeError, CompatibilityEvidenceError) as error:
        return [str(error)]
    return [] if value == expected else ["hosted compatibility source contract is stale"]


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-macos-hosted-", dir=path.parent)
    temporary = Path(name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        temporary.chmod(0o600)
        os.replace(temporary, path)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def main() -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command")
    emit = subparsers.add_parser("emit-result")
    emit.add_argument("path", type=Path)
    verify = subparsers.add_parser("verify-result")
    verify.add_argument("path", type=Path)
    source = subparsers.add_parser("write-source-contract")
    source.add_argument("revision")
    args = parser.parse_args()
    try:
        if args.command == "emit-result":
            write_atomic(args.path, canonical_json(build_result()))
        elif args.command == "verify-result":
            failures = validate_result(json.loads(args.path.read_text(encoding="utf-8")))
            if failures:
                raise CompatibilityEvidenceError("; ".join(failures))
        elif args.command == "write-source-contract":
            write_atomic(REPORT_PATH, canonical_json(build_source_contract(args.revision)))
            REPORT_PATH.chmod(0o644)
        else:
            failures = check_source_contract()
            if failures:
                raise CompatibilityEvidenceError("; ".join(failures))
    except (OSError, json.JSONDecodeError, CompatibilityEvidenceError) as error:
        print(str(error), file=sys.stderr)
        return 1
    print("hosted Apple Silicon compatibility evidence contract passed without dispatch or promotion")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
