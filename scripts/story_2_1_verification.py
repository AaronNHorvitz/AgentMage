#!/usr/bin/env python3
"""Build explicit verification evidence for Sprint 2 Story 2.1."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import os
import shutil
import sys
import tempfile
from collections import Counter
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts import versioned_corpus  # noqa: E402
from scripts import test_result_bundle as result_bundles  # noqa: E402
from fixtures.fake_adapters import (  # noqa: E402
    CrashInjector,
    FakeClock,
    FakeConnector,
    FakeInferenceRuntime,
    FakeMode,
    FakeModel,
    FakeSecretStore,
    FakeStatus,
    FakeTool,
    ScenarioPlan,
    SyntheticSecret,
    trace_sha256,
)


CORPUS_REPORT_PATH = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-2"
    / "story-2.1"
    / "corpus-reproducibility-report.json"
)
ADAPTER_REPORT_PATH = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-2"
    / "story-2.1"
    / "adapter-mode-verification-report.json"
)
SUMMARY_REPORT_PATH = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-2"
    / "story-2.1"
    / "summary-reconciliation-report.json"
)
EXPECTED_SOURCE_SEEDS = {
    "base": "agentmage-sprint-2-synthetic-v1",
    "paths": "agentmage-path-fixtures-synthetic-v1",
    "adversarial": "agentmage-adversarial-synthetic-v1",
    "documents": "agentmage-document-synthetic-v1",
    "golden": "agentmage-expected-output-synthetic-v1",
}
EXPECTED_FAKE_STATUSES = {
    FakeMode.SUCCESS: FakeStatus.SUCCEEDED,
    FakeMode.DENIAL: FakeStatus.DENIED,
    FakeMode.MALFORMED: FakeStatus.MALFORMED,
    FakeMode.CANCELLATION: FakeStatus.CANCELLED,
    FakeMode.TIMEOUT: FakeStatus.TIMED_OUT,
    FakeMode.CRASH_BEFORE: FakeStatus.CRASHED,
    FakeMode.CRASH_AFTER: FakeStatus.CRASHED,
    FakeMode.UNCERTAIN: FakeStatus.UNCERTAIN,
}


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def artifact(root: Path, relative_path: str) -> dict[str, str]:
    path = root / relative_path
    if not path.is_file():
        raise ValueError(f"Story 2.1 evidence artifact is missing: {relative_path}")
    return {"path": relative_path, "sha256": sha256_bytes(path.read_bytes())}


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-story-2-1-", dir=path.parent
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


def source_seed_records(
    profile: dict[str, Any], root: Path = ROOT
) -> list[dict[str, str]]:
    records = []
    for source in profile["sources"]:
        source_profile = read_json(root / source["path"])
        expected_seed = EXPECTED_SOURCE_SEEDS[source["family"]]
        if source_profile.get("seed") != expected_seed:
            raise ValueError(f"source seed drifted: {source['family']}")
        records.append(
            {
                "family": source["family"],
                "profile_id": source["profile_id"],
                "profile_path": source["path"],
                "profile_sha256": sha256_bytes((root / source["path"]).read_bytes()),
                "seed": expected_seed,
            }
        )
    return records


def regenerated_outputs(
    profile: dict[str, Any], root: Path = ROOT
) -> tuple[list[dict[str, Any]], bool]:
    with tempfile.TemporaryDirectory(prefix="agentmage-corpus-reproduction-") as temporary:
        temporary_root = Path(temporary)
        destinations = [temporary_root / "run-1", temporary_root / "run-2"]
        results = [
            versioned_corpus.generate(profile, destination, root)
            for destination in destinations
        ]
        archive_name = Path(profile["archive"]["path"]).name
        manifest_name = Path(profile["manifest_path"]).name
        output_records = []
        for index, destination in enumerate(destinations, start=1):
            archive_content = (destination / archive_name).read_bytes()
            manifest_content = (destination / manifest_name).read_bytes()
            output_records.append(
                {
                    "run": index,
                    "archive_sha256": sha256_bytes(archive_content),
                    "manifest_file_sha256": sha256_bytes(manifest_content),
                    "manifest_self_sha256": results[index - 1]["manifest_sha256"],
                    "entry_count": results[index - 1]["entry_count"],
                }
            )
        identical = all(
            (destinations[0] / name).read_bytes()
            == (destinations[1] / name).read_bytes()
            for name in (archive_name, manifest_name)
        )
        return output_records, identical


def build_corpus_report(root: Path = ROOT) -> dict[str, Any]:
    profile_path = root / versioned_corpus.PROFILE_PATH.relative_to(ROOT)
    profile = read_json(profile_path)
    profile_failures = versioned_corpus.validate_profile(profile)
    if profile_failures:
        raise ValueError("; ".join(profile_failures))
    if profile.get("seed") != "agentmage-versioned-corpus-synthetic-v1":
        raise ValueError("corpus reproduction seed drifted")
    checked_failures = versioned_corpus.check_checked_corpus(root)
    if checked_failures:
        raise ValueError("; ".join(checked_failures))

    archive, manifest = versioned_corpus.build_corpus(profile, root)
    entries = versioned_corpus.materialize_entries(profile, root)
    runs, identical = regenerated_outputs(profile, root)
    checked_archive = (root / profile["archive"]["path"]).read_bytes()
    checked_manifest = (root / profile["manifest_path"]).read_bytes()
    checked_matches = (
        checked_archive == archive
        and checked_manifest == versioned_corpus.canonical_json(manifest)
    )

    corrupted_archive = bytearray(archive)
    corrupted_archive[len(corrupted_archive) // 3] ^= 0xFF
    archive_corruption_failures = versioned_corpus.validate_archive(
        bytes(corrupted_archive), entries
    )
    corrupted_manifest = copy.deepcopy(manifest)
    corrupted_manifest["archive"]["entry_count"] -= 1
    manifest_corruption_failures = versioned_corpus.validate_manifest(
        corrupted_manifest, profile, entries, archive, root
    )
    if not archive_corruption_failures or not manifest_corruption_failures:
        raise ValueError("corpus corruption seed was not detected")

    source_seeds = source_seed_records(profile, root)
    golden_states = sorted(
        item["evidence_state"] for item in manifest["golden_manifests"]
    )
    report = {
        "schema_version": 1,
        "task_id": "2.1.3.1",
        "test_id": "S-002-UT01",
        "status": "pass",
        "corpus": {
            "id": manifest["corpus_id"],
            "version": manifest["corpus_version"],
            "seed": profile["seed"],
            "profile": artifact(
                root, versioned_corpus.PROFILE_PATH.relative_to(ROOT).as_posix()
            ),
            "archive": {
                "path": profile["archive"]["path"],
                "sha256": manifest["archive"]["sha256"],
                "entry_count": manifest["archive"]["entry_count"],
            },
            "manifest": {
                "path": profile["manifest_path"],
                "file_sha256": sha256_bytes(checked_manifest),
                "self_sha256": manifest["manifest_sha256"],
            },
        },
        "source_seeds": source_seeds,
        "reproduction": {
            "run_count": len(runs),
            "runs": runs,
            "runs_byte_identical": identical,
            "checked_artifacts_match": checked_matches,
            "current_host_data_used": False,
            "private_user_data_used": False,
        },
        "metadata": {
            "family_counts": manifest["family_counts"],
            "golden_manifest_count": len(manifest["golden_manifests"]),
            "golden_evidence_states": golden_states,
            "source_family_count": len(source_seeds),
            "provenance_record_count": len(manifest["source_provenance"]),
        },
        "corruption": {
            "archive_mutation_detected_before_use": True,
            "archive_failure_count": len(archive_corruption_failures),
            "manifest_mutation_detected_before_use": True,
            "manifest_failure_count": len(manifest_corruption_failures),
            "corrupt_artifact_persisted": False,
        },
        "product_support_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }
    if not identical or not checked_matches:
        raise ValueError("corpus reproduction did not match checked artifacts")
    return report


def validate_corpus_report(report: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(report, dict):
        return ["Story 2.1 corpus report must be an object"]
    failures: list[str] = []
    if (
        report.get("schema_version") != 1
        or report.get("task_id") != "2.1.3.1"
        or report.get("test_id") != "S-002-UT01"
    ):
        failures.append("Story 2.1 corpus report identity is invalid")
    if report.get("status") != "pass":
        failures.append("Story 2.1 corpus report did not pass")
    reproduction = report.get("reproduction", {})
    if (
        reproduction.get("run_count") != 2
        or reproduction.get("runs_byte_identical") is not True
        or reproduction.get("checked_artifacts_match") is not True
        or reproduction.get("current_host_data_used") is not False
        or reproduction.get("private_user_data_used") is not False
    ):
        failures.append("Story 2.1 corpus reproduction contract was weakened")
    corruption = report.get("corruption", {})
    if (
        corruption.get("archive_mutation_detected_before_use") is not True
        or corruption.get("manifest_mutation_detected_before_use") is not True
        or corruption.get("corrupt_artifact_persisted") is not False
    ):
        failures.append("Story 2.1 corruption detection contract was weakened")
    if report.get("product_support_claim") != "none":
        failures.append("Story 2.1 corpus report made a product support claim")
    if report.get("macos_execution_status") != "blocked-macos" or report.get(
        "macos_support_claim"
    ) != "none":
        failures.append("Story 2.1 corpus report made an invalid macOS claim")
    try:
        expected = build_corpus_report(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        failures.append(f"cannot rebuild Story 2.1 corpus report: {error}")
    else:
        if report != expected:
            failures.append("Story 2.1 corpus report is stale or non-deterministic")
    return failures


def write_corpus_report(root: Path = ROOT) -> None:
    write_atomic(
        root / CORPUS_REPORT_PATH.relative_to(ROOT),
        canonical_json(build_corpus_report(root)),
    )


def check_corpus_report(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(root / CORPUS_REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read Story 2.1 corpus report: {error}"]
    return validate_corpus_report(report, root)


def adapter_case(
    adapter_id: str, mode: FakeMode
) -> tuple[Any, Any, str]:
    operation_key = {
        "fake-model": "fake-model.generate",
        "fake-tool": "fake-tool.invoke",
        "fake-inference-runtime": "fake-inference-runtime.start",
        "fake-connector": "fake-connector.list",
        "fake-clock": "fake-clock.probe",
        "fake-secret-store": "fake-secret-store.store",
        "crash-injector": "crash-injector.checkpoint",
    }[adapter_id]
    plan = ScenarioPlan({operation_key: [mode]})
    if adapter_id == "fake-model":
        adapter = FakeModel(plan)
        operation = lambda: adapter.generate("synthetic verification prompt")
    elif adapter_id == "fake-tool":
        adapter = FakeTool(plan)
        operation = lambda: adapter.invoke("synthetic-read", {"path": "fixture.txt"})
    elif adapter_id == "fake-inference-runtime":
        adapter = FakeInferenceRuntime(plan)
        operation = lambda: adapter.start("synthetic-profile")
    elif adapter_id == "fake-connector":
        adapter = FakeConnector(plan)
        operation = adapter.list
    elif adapter_id == "fake-clock":
        adapter = FakeClock(plan=plan)
        operation = adapter.probe
    elif adapter_id == "fake-secret-store":
        adapter = FakeSecretStore(plan)
        operation = lambda: adapter.store(
            "synthetic-handle",
            SyntheticSecret("AM_SYNTHETIC_SECRET_STORY_2_1"),
        )
    else:
        adapter = CrashInjector(plan)
        operation = lambda: adapter.checkpoint("synthetic-checkpoint")
    return adapter, operation, operation_key


def build_adapter_report(root: Path = ROOT) -> dict[str, Any]:
    adapter_ids = (
        "fake-model",
        "fake-tool",
        "fake-inference-runtime",
        "fake-connector",
        "fake-clock",
        "fake-secret-store",
        "crash-injector",
    )
    all_adapters = []
    adapter_summaries = []
    total_post_close_rejections = 0
    for adapter_id in adapter_ids:
        statuses: Counter[str] = Counter()
        instances = []
        for mode, expected_status in EXPECTED_FAKE_STATUSES.items():
            adapter, operation, _operation_key = adapter_case(adapter_id, mode)
            outcome = operation()
            if outcome.status is not expected_status:
                raise ValueError(
                    f"fake adapter status mismatch: {adapter_id}/{mode.value}"
                )
            if adapter.plan.pending() != 0:
                raise ValueError(
                    f"fake adapter plan was not consumed: {adapter_id}/{mode.value}"
                )
            statuses[outcome.status.value] += 1
            adapter.close()
            adapter.close()
            try:
                operation()
            except RuntimeError:
                total_post_close_rejections += 1
            else:
                raise ValueError(
                    f"fake adapter accepted operation after close: {adapter_id}"
                )
            if isinstance(adapter, FakeInferenceRuntime) and adapter.running:
                raise ValueError("fake runtime remained active after close")
            if isinstance(adapter, FakeSecretStore) and adapter.stored_count != 0:
                raise ValueError("fake secret store retained state after close")
            if isinstance(adapter, FakeClock) and adapter.current != 1704067200:
                raise ValueError("fake clock did not reset after close")
            instances.append(adapter)
            all_adapters.append(adapter)
        adapter_summaries.append(
            {
                "adapter_id": adapter_id,
                "mode_count": len(instances),
                "event_count": sum(len(adapter.events) for adapter in instances),
                "status_counts": dict(sorted(statuses.items())),
                "trace_sha256": trace_sha256(instances),
                "cleanup_passed": all(adapter.closed for adapter in instances),
            }
        )

    events = [event for adapter in all_adapters for event in adapter.events]
    serialized_events = json.dumps(
        [event.as_record() for event in events], sort_keys=True
    )
    if "AM_SYNTHETIC_SECRET_" in serialized_events:
        raise ValueError("adapter trace retained a synthetic secret value")
    return {
        "schema_version": 1,
        "task_id": "2.1.3.2",
        "test_id": "S-002-UT02",
        "status": "pass",
        "contract": artifact(root, "fixtures/fake-adapter-contract.json"),
        "adapters": adapter_summaries,
        "summary": {
            "adapter_count": len(adapter_ids),
            "mode_count": len(EXPECTED_FAKE_STATUSES),
            "matrix_case_count": len(events),
            "expected_matrix_case_count": len(adapter_ids)
            * len(EXPECTED_FAKE_STATUSES),
            "post_close_rejection_count": total_post_close_rejections,
            "all_typed_outcomes_matched": True,
            "all_cleanup_passed": True,
            "raw_secret_retained": False,
            "trace_sha256": trace_sha256(all_adapters),
        },
        "side_effect_contract": {
            "external_commands": False,
            "network": False,
            "real_workspace": False,
            "persistent_state": False,
            "real_secrets": False,
        },
        "product_support_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }


def validate_adapter_report(report: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(report, dict):
        return ["Story 2.1 adapter report must be an object"]
    failures: list[str] = []
    if (
        report.get("schema_version") != 1
        or report.get("task_id") != "2.1.3.2"
        or report.get("test_id") != "S-002-UT02"
    ):
        failures.append("Story 2.1 adapter report identity is invalid")
    if report.get("status") != "pass":
        failures.append("Story 2.1 adapter report did not pass")
    summary = report.get("summary", {})
    if (
        summary.get("adapter_count") != 7
        or summary.get("mode_count") != 8
        or summary.get("matrix_case_count") != 56
        or summary.get("expected_matrix_case_count") != 56
        or summary.get("post_close_rejection_count") != 56
        or summary.get("all_typed_outcomes_matched") is not True
        or summary.get("all_cleanup_passed") is not True
        or summary.get("raw_secret_retained") is not False
    ):
        failures.append("Story 2.1 adapter matrix or cleanup contract was weakened")
    if any(
        value is not False for value in report.get("side_effect_contract", {}).values()
    ):
        failures.append("Story 2.1 adapter report contains a real side effect")
    if report.get("product_support_claim") != "none":
        failures.append("Story 2.1 adapter report made a product support claim")
    if report.get("macos_execution_status") != "blocked-macos" or report.get(
        "macos_support_claim"
    ) != "none":
        failures.append("Story 2.1 adapter report made an invalid macOS claim")
    try:
        expected = build_adapter_report(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        failures.append(f"cannot rebuild Story 2.1 adapter report: {error}")
    else:
        if report != expected:
            failures.append("Story 2.1 adapter report is stale or non-deterministic")
    return failures


def write_adapter_report(root: Path = ROOT) -> None:
    write_atomic(
        root / ADAPTER_REPORT_PATH.relative_to(ROOT),
        canonical_json(build_adapter_report(root)),
    )


def check_adapter_report(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(root / ADAPTER_REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read Story 2.1 adapter report: {error}"]
    return validate_adapter_report(report, root)


def independent_summary(results: list[dict[str, Any]]) -> dict[str, Any]:
    statuses = ("cancelled", "error", "fail", "pass", "skipped", "timeout")
    classifications = (
        "cancelled",
        "flaky",
        "pass",
        "persistent-failure",
        "quarantined-failure",
        "retry-exhausted",
        "skipped",
    )
    status_counts = Counter(result["final_status"] for result in results)
    classification_counts = Counter(result["classification"] for result in results)
    non_pass_count = sum(result["final_status"] != "pass" for result in results)
    return {
        "classification_counts": {
            item: classification_counts[item] for item in classifications
        },
        "final_status_counts": {item: status_counts[item] for item in statuses},
        "flaky_test_count": sum(bool(result["flaky"]) for result in results),
        "non_pass_test_count": non_pass_count,
        "overall_status": "pass" if non_pass_count == 0 else "fail",
        "quarantined_test_count": sum(
            bool(result["quarantined"]) for result in results
        ),
        "redacted_attempt_count": sum(
            attempt["output"]["redaction_count"] > 0
            for result in results
            for attempt in result["attempts"]
        ),
        "retried_test_count": sum(result["retry_count"] > 0 for result in results),
        "retry_attempt_count": sum(result["retry_count"] for result in results),
        "skipped_test_count": sum(
            result["final_status"] == "skipped" for result in results
        ),
        "test_count": len(results),
        "truncated_attempt_count": sum(
            bool(attempt["output"]["truncated"])
            for result in results
            for attempt in result["attempts"]
        ),
    }


def independent_reconciliation(bundle: dict[str, Any]) -> dict[str, Any]:
    shards = bundle.get("shards", [])
    if not isinstance(shards, list):
        raise ValueError("result bundle shards are invalid")
    results = []
    shard_indices = set()
    for shard in shards:
        index = shard.get("index")
        if index in shard_indices:
            raise ValueError("result bundle contains duplicate shard indices")
        shard_indices.add(index)
        shard_results = shard.get("results", [])
        if shard.get("result_count") != len(shard_results):
            raise ValueError("result bundle shard count does not reconcile")
        for result in shard_results:
            if result.get("shard") != index:
                raise ValueError("result bundle result is assigned to the wrong shard")
        results.extend(shard_results)
    test_ids = [result.get("test_id") for result in results]
    if len(test_ids) != len(set(test_ids)):
        raise ValueError("result bundle contains duplicate test identities")
    expected_ids = set(result_bundles.EXPECTED_CASE_IDS)
    if set(test_ids) != expected_ids:
        raise ValueError("result bundle omitted or added a test identity")
    if any(
        result["final_status"] != "pass" and result["classification"] == "pass"
        for result in results
    ):
        raise ValueError("result bundle converted a non-pass result to pass")
    summary = independent_summary(results)
    return {
        "summary": summary,
        "result_count": len(results),
        "result_set_sha256": sha256_bytes(
            canonical_json(sorted(results, key=lambda item: item["test_id"]))
        ),
        "test_id_set_sha256": sha256_bytes(canonical_json(sorted(test_ids))),
    }


def build_summary_report(root: Path = ROOT) -> dict[str, Any]:
    profile_path = root / result_bundles.PROFILE_PATH.relative_to(ROOT)
    profile = read_json(profile_path)
    bundle = result_bundles.build_bundle(profile, root)
    bundle_failures = result_bundles.validate_bundle(bundle, profile, root)
    if bundle_failures:
        raise ValueError("; ".join(bundle_failures))
    independent = independent_reconciliation(bundle)
    producer_summary = bundle["summary"]
    independently_recomputed = independent["summary"]
    if producer_summary != independently_recomputed:
        raise ValueError("independent summary does not match producer summary")
    required_classifications = {
        "persistent-failure",
        "skipped",
        "flaky",
        "quarantined-failure",
        "retry-exhausted",
        "cancelled",
    }
    observed_classifications = {
        key
        for key, count in independently_recomputed["classification_counts"].items()
        if count > 0
    }
    if not required_classifications <= observed_classifications:
        raise ValueError("summary fixture classification closure is incomplete")
    comparison = {
        "bundle_sha256": bundle["bundle_sha256"],
        "producer_summary_sha256": sha256_bytes(canonical_json(producer_summary)),
        "independent_summary_sha256": sha256_bytes(
            canonical_json(independently_recomputed)
        ),
        "result_set_sha256": independent["result_set_sha256"],
        "test_id_set_sha256": independent["test_id_set_sha256"],
    }
    comparison["comparison_sha256"] = sha256_bytes(canonical_json(comparison))
    return {
        "schema_version": 1,
        "task_id": "2.1.3.4",
        "test_id": "S-002-IT01",
        "status": "pass",
        "profile": artifact(
            root, result_bundles.PROFILE_PATH.relative_to(ROOT).as_posix()
        ),
        "comparison": comparison,
        "reconciliation": {
            "exact_match": True,
            "test_count": independently_recomputed["test_count"],
            "non_pass_test_count": independently_recomputed[
                "non_pass_test_count"
            ],
            "skipped_test_count": independently_recomputed["skipped_test_count"],
            "retried_test_count": independently_recomputed["retried_test_count"],
            "flaky_test_count": independently_recomputed["flaky_test_count"],
            "quarantined_test_count": independently_recomputed[
                "quarantined_test_count"
            ],
            "retry_exhausted_test_count": independently_recomputed[
                "classification_counts"
            ]["retry-exhausted"],
            "cancelled_test_count": independently_recomputed[
                "final_status_counts"
            ]["cancelled"],
            "timeout_test_count": independently_recomputed["final_status_counts"][
                "timeout"
            ],
            "overall_status": independently_recomputed["overall_status"],
            "omitted_or_duplicate_result_count": 0,
            "non_pass_converted_to_pass_count": 0,
        },
        "raw_results_persisted_by_verifier": False,
        "raw_output_retained": False,
        "product_acceptance_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }


def validate_summary_report(report: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(report, dict):
        return ["Story 2.1 summary report must be an object"]
    failures: list[str] = []
    if (
        report.get("schema_version") != 1
        or report.get("task_id") != "2.1.3.4"
        or report.get("test_id") != "S-002-IT01"
    ):
        failures.append("Story 2.1 summary report identity is invalid")
    if report.get("status") != "pass":
        failures.append("Story 2.1 summary report did not pass")
    reconciliation = report.get("reconciliation", {})
    if (
        reconciliation.get("exact_match") is not True
        or reconciliation.get("test_count") != 9
        or reconciliation.get("non_pass_test_count") != 5
        or reconciliation.get("omitted_or_duplicate_result_count") != 0
        or reconciliation.get("non_pass_converted_to_pass_count") != 0
        or reconciliation.get("overall_status") != "fail"
    ):
        failures.append("Story 2.1 summary reconciliation contract was weakened")
    if report.get("raw_results_persisted_by_verifier") is not False or report.get(
        "raw_output_retained"
    ) is not False:
        failures.append("Story 2.1 summary verifier retained raw results or output")
    if report.get("product_acceptance_claim") != "none":
        failures.append("Story 2.1 summary report made a product acceptance claim")
    if report.get("macos_execution_status") != "blocked-macos" or report.get(
        "macos_support_claim"
    ) != "none":
        failures.append("Story 2.1 summary report made an invalid macOS claim")
    try:
        expected = build_summary_report(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        failures.append(f"cannot rebuild Story 2.1 summary report: {error}")
    else:
        if report != expected:
            failures.append("Story 2.1 summary report is stale or non-deterministic")
    return failures


def write_summary_report(root: Path = ROOT) -> None:
    write_atomic(
        root / SUMMARY_REPORT_PATH.relative_to(ROOT),
        canonical_json(build_summary_report(root)),
    )


def check_summary_report(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(root / SUMMARY_REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read Story 2.1 summary report: {error}"]
    return validate_summary_report(report, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write-corpus-report", action="store_true")
    parser.add_argument("--write-adapter-report", action="store_true")
    parser.add_argument("--write-summary-report", action="store_true")
    args = parser.parse_args()
    try:
        if args.write_corpus_report:
            write_corpus_report()
        if args.write_adapter_report:
            write_adapter_report()
        if args.write_summary_report:
            write_summary_report()
        failures = (
            check_corpus_report()
            + check_adapter_report()
            + check_summary_report()
        )
    except (OSError, ValueError, KeyError, TypeError, shutil.Error) as error:
        print(f"Story 2.1 verification failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 2.1 verification failed: {failure}", file=sys.stderr)
        return 1
    print("Story 2.1 corpus, adapter, and summary evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
