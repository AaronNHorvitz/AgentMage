#!/usr/bin/env python3
"""Build and validate the synthetic fixture provenance ledger."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path, PurePosixPath
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.platform_result_recorder import (  # noqa: E402
    validate_record as validate_platform_record,
)
from scripts.versioned_corpus import check_checked_corpus  # noqa: E402


LEDGER_PATH = ROOT / "fixtures" / "corpus" / "v1" / "provenance-ledger.json"
CORPUS_MANIFEST_PATH = ROOT / "fixtures" / "corpus" / "v1" / "manifest.json"
PLATFORM_REPORT_PATH = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-2"
    / "story-2.1"
    / "platform-result-recorder-report.json"
)
REPORT_PATH = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-2"
    / "story-2.1"
    / "fixture-provenance-report.json"
)
PLATFORM_SCHEMA_PATH = ROOT / "schemas" / "testing" / "platform-result.schema.json"
LEDGER_SCHEMA_PATH = (
    ROOT / "schemas" / "testing" / "fixture-provenance-ledger.schema.json"
)

SOURCE_REPORTS = {
    "base": "artifacts/sprints/sprint-2/story-2.1/fixture-generator-report.json",
    "paths": "artifacts/sprints/sprint-2/story-2.1/path-fixture-report.json",
    "adversarial": "artifacts/sprints/sprint-2/story-2.1/adversarial-fixture-report.json",
    "documents": "artifacts/sprints/sprint-2/story-2.1/document-fixture-report.json",
    "golden": "artifacts/sprints/sprint-2/story-2.1/expected-output-report.json",
}
SOURCE_ORDER = tuple(SOURCE_REPORTS)
CONTRACT_SPECS = (
    {
        "contract_id": "agentmage-fake-adapters-v1",
        "contract_kind": "fake-adapters",
        "profile": "fixtures/fake-adapter-contract.json",
        "implementation": "fixtures/fake_adapters.py",
        "report": "artifacts/sprints/sprint-2/story-2.1/fake-adapter-report.json",
        "node_suffix": "fake-adapters",
    },
    {
        "contract_id": "agentmage-platform-result-recorder-v1",
        "contract_kind": "platform-results",
        "profile": "fixtures/platform-result-profile.json",
        "implementation": "scripts/platform_result_recorder.py",
        "report": (
            "artifacts/sprints/sprint-2/story-2.1/"
            "platform-result-recorder-report.json"
        ),
        "node_suffix": "platform-results",
    },
    {
        "contract_id": "agentmage-test-result-bundle-v1",
        "contract_kind": "test-result-bundles",
        "profile": "fixtures/test-result-bundle-profile.json",
        "implementation": "scripts/test_result_bundle.py",
        "report": "artifacts/sprints/sprint-2/story-2.1/test-result-bundle-report.json",
        "node_suffix": "result-bundles",
    },
    {
        "contract_id": "agentmage-shared-acceptance-runner-v1",
        "contract_kind": "acceptance-runner",
        "profile": "fixtures/acceptance-runner-profile.json",
        "implementation": "scripts/shared_acceptance_runner.py",
        "report": (
            "artifacts/sprints/sprint-2/story-2.1/"
            "shared-acceptance-runner-report.json"
        ),
        "node_suffix": "acceptance-runner",
    },
    {
        "contract_id": "agentmage-fault-test-adapters-v1",
        "contract_kind": "fault-test-adapters",
        "profile": "fixtures/fault-test-adapter-profile.json",
        "implementation": "fixtures/fault_test_adapters.py",
        "report": "artifacts/sprints/sprint-2/story-2.1/fault-test-adapter-report.json",
        "node_suffix": "fault-test-adapters",
    },
)
EXPECTED_RELATIONS = {
    "configures",
    "produces",
    "contributes-to",
    "describes",
    "verifies",
    "consumes",
}
PRIVATE_PATH = re.compile(r"(?:^|/)(?:home|Users|var/home)/")
SECRET_VALUE = re.compile(
    r"(?:-----BEGIN [A-Z ]*PRIVATE KEY-----|\bAKIA[0-9A-Z]{16}\b|"
    r"\bgh[pousr]_[A-Za-z0-9]{20,}\b|\bBearer\s+[A-Za-z0-9._-]{12,})"
)


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def artifact(root: Path, relative_path: str) -> dict[str, str]:
    path = root / relative_path
    if not path.is_file():
        raise ValueError(f"provenance artifact is missing: {relative_path}")
    return {"path": relative_path, "sha256": sha256_file(path)}


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.tmp")
    temporary.write_bytes(content)
    temporary.replace(path)


def entry_set_sha256(entries: list[dict[str, Any]]) -> str:
    material = [
        {
            "archive_path": entry["archive_path"],
            "bytes": entry["bytes"],
            "kind": entry["kind"],
            "sha256": entry["sha256"],
            "source_relative_path": entry["source_relative_path"],
        }
        for entry in sorted(entries, key=lambda item: item["archive_path"])
    ]
    return sha256_bytes(canonical_json(material))


def node(node_id: str, role: str, value: dict[str, str]) -> dict[str, str]:
    return {
        "node_id": node_id,
        "artifact_role": role,
        "path": value["path"],
        "sha256": value["sha256"],
    }


def edge(
    edge_id: str, from_node: str, to_node: str, relation: str
) -> dict[str, str]:
    return {
        "edge_id": edge_id,
        "from_node": from_node,
        "to_node": to_node,
        "relation": relation,
    }


def root_fixture_contracts(root: Path) -> set[str]:
    return {
        path.relative_to(root).as_posix()
        for path in (root / "fixtures").glob("*.json")
        if path.is_file()
    }


def expected_root_fixture_contracts(manifest: dict[str, Any]) -> set[str]:
    paths = {"fixtures/corpus-profile.json"}
    paths.update(item["profile_path"] for item in manifest["source_provenance"])
    paths.update(spec["profile"] for spec in CONTRACT_SPECS)
    return paths


def build_ledger(root: Path = ROOT) -> dict[str, Any]:
    corpus_failures = check_checked_corpus(root)
    if corpus_failures:
        raise ValueError("; ".join(corpus_failures))

    manifest_path = root / CORPUS_MANIFEST_PATH.relative_to(ROOT)
    manifest = read_json(manifest_path)
    platform_report_path = root / PLATFORM_REPORT_PATH.relative_to(ROOT)
    platform_report = read_json(platform_report_path)
    platform_records = platform_report["synthetic_record_set"]["records"]
    platform_failures = [
        failure
        for record in platform_records
        for failure in validate_platform_record(record)
    ]
    if platform_failures:
        raise ValueError("; ".join(platform_failures))

    actual_contracts = root_fixture_contracts(root)
    expected_contracts = expected_root_fixture_contracts(manifest)
    if actual_contracts != expected_contracts:
        missing = sorted(expected_contracts - actual_contracts)
        unexpected = sorted(actual_contracts - expected_contracts)
        raise ValueError(
            f"root fixture contract closure failed; missing={missing}; "
            f"unexpected={unexpected}"
        )

    provenance_by_family = {
        item["family"]: item for item in manifest["source_provenance"]
    }
    if tuple(provenance_by_family) != SOURCE_ORDER:
        raise ValueError("corpus source provenance order or closure drifted")

    source_families: list[dict[str, Any]] = []
    nodes: list[dict[str, str]] = []
    edges: list[dict[str, str]] = []
    for family in SOURCE_ORDER:
        source = provenance_by_family[family]
        family_entries = [
            item for item in manifest["entries"] if item["family"] == family
        ]
        profile = artifact(root, source["profile_path"])
        generator = artifact(root, source["generator"])
        evidence_report = artifact(root, SOURCE_REPORTS[family])
        if profile["sha256"] != source["profile_sha256"]:
            raise ValueError(f"source profile hash drifted: {family}")
        if generator["sha256"] != source["generator_sha256"]:
            raise ValueError(f"source generator hash drifted: {family}")
        source_families.append(
            {
                "family": family,
                "namespace": source["namespace"],
                "profile": {"id": source["profile_id"], **profile},
                "generator": generator,
                "evidence_report": evidence_report,
                "archive_entries": {
                    "count": len(family_entries),
                    "set_sha256": entry_set_sha256(family_entries),
                },
            }
        )
        profile_node = f"profile-{family}"
        generator_node = f"generator-{family}"
        report_node = f"report-{family}"
        nodes.extend(
            (
                node(profile_node, "profile", profile),
                node(generator_node, "generator", generator),
                node(report_node, "evidence-report", evidence_report),
            )
        )
        edges.extend(
            (
                edge(
                    f"edge-{family}-profile-generator",
                    profile_node,
                    generator_node,
                    "configures",
                ),
                edge(
                    f"edge-{family}-generator-report",
                    generator_node,
                    report_node,
                    "produces",
                ),
                edge(
                    f"edge-{family}-generator-archive",
                    generator_node,
                    "archive-corpus",
                    "contributes-to",
                ),
            )
        )

    corpus_profile = artifact(root, "fixtures/corpus-profile.json")
    assembler = artifact(root, manifest["assembler"]["path"])
    archive = artifact(root, manifest["archive"]["path"])
    manifest_artifact = artifact(
        root, CORPUS_MANIFEST_PATH.relative_to(ROOT).as_posix()
    )
    corpus_report = artifact(
        root, "artifacts/sprints/sprint-2/story-2.1/versioned-corpus-report.json"
    )
    if corpus_profile["sha256"] != manifest["corpus_profile_sha256"]:
        raise ValueError("corpus profile hash drifted")
    if assembler["sha256"] != manifest["assembler"]["sha256"]:
        raise ValueError("corpus assembler hash drifted")
    if archive["sha256"] != manifest["archive"]["sha256"]:
        raise ValueError("corpus archive hash drifted")

    nodes.extend(
        (
            node("profile-corpus", "profile", corpus_profile),
            node("assembler-corpus", "implementation", assembler),
            node("archive-corpus", "archive", archive),
            node("manifest-corpus", "manifest", manifest_artifact),
            node("report-corpus", "evidence-report", corpus_report),
        )
    )
    edges.extend(
        (
            edge(
                "edge-corpus-profile-assembler",
                "profile-corpus",
                "assembler-corpus",
                "configures",
            ),
            edge(
                "edge-corpus-assembler-archive",
                "assembler-corpus",
                "archive-corpus",
                "produces",
            ),
            edge(
                "edge-corpus-manifest-archive",
                "manifest-corpus",
                "archive-corpus",
                "describes",
            ),
            edge(
                "edge-corpus-report-manifest",
                "report-corpus",
                "manifest-corpus",
                "verifies",
            ),
        )
    )

    contracts: list[dict[str, Any]] = []
    for spec in CONTRACT_SPECS:
        profile = artifact(root, spec["profile"])
        implementation = artifact(root, spec["implementation"])
        evidence_report = artifact(root, spec["report"])
        contracts.append(
            {
                "contract_id": spec["contract_id"],
                "contract_kind": spec["contract_kind"],
                "profile": profile,
                "implementation": implementation,
                "evidence_report": evidence_report,
            }
        )
        suffix = spec["node_suffix"]
        profile_node = f"profile-{suffix}"
        implementation_node = f"implementation-{suffix}"
        report_node = f"report-{suffix}"
        nodes.extend(
            (
                node(profile_node, "profile", profile),
                node(implementation_node, "implementation", implementation),
                node(report_node, "evidence-report", evidence_report),
            )
        )
        edges.extend(
            (
                edge(
                    f"edge-{suffix}-profile-implementation",
                    profile_node,
                    implementation_node,
                    "configures",
                ),
                edge(
                    f"edge-{suffix}-implementation-report",
                    implementation_node,
                    report_node,
                    "produces",
                ),
            )
        )

    acceptance_node = "implementation-acceptance-runner"
    edges.extend(
        (
            edge(
                "edge-acceptance-consumes-corpus",
                acceptance_node,
                "archive-corpus",
                "consumes",
            ),
            edge(
                "edge-acceptance-consumes-platform-results",
                acceptance_node,
                "report-platform-results",
                "consumes",
            ),
            edge(
                "edge-acceptance-consumes-result-bundles",
                acceptance_node,
                "report-result-bundles",
                "consumes",
            ),
            edge(
                "edge-acceptance-consumes-fake-adapters",
                acceptance_node,
                "report-fake-adapters",
                "consumes",
            ),
            edge(
                "edge-acceptance-consumes-fault-test-adapters",
                acceptance_node,
                "report-fault-test-adapters",
                "consumes",
            ),
        )
    )

    golden_manifests = [
        {
            "manifest_id": item["manifest_id"],
            "evidence_state": item["evidence_state"],
            "archive_path": item["path"],
            "sha256": item["sha256"],
        }
        for item in manifest["golden_manifests"]
    ]
    ledger = {
        "schema_version": 1,
        "ledger_id": "agentmage-fixture-provenance-ledger-v1",
        "ledger_version": "1.0.0",
        "status": "verified-synthetic-provenance",
        "generated_at": "2024-01-01T00:03:00Z",
        "scope": {
            "fixture_classification": "repository-controlled-synthetic-only",
            "root_fixture_contract_count": len(actual_contracts),
            "root_fixture_contracts_complete": True,
            "private_user_data": False,
            "real_credentials": False,
            "external_paths": False,
        },
        "corpus": {
            "corpus_id": manifest["corpus_id"],
            "corpus_version": manifest["corpus_version"],
            "profile": corpus_profile,
            "assembler": assembler,
            "archive": {
                **archive,
                "bytes": manifest["archive"]["bytes"],
                "entry_count": manifest["archive"]["entry_count"],
            },
            "manifest": {
                **manifest_artifact,
                "self_sha256": manifest["manifest_sha256"],
            },
            "evidence_report": corpus_report,
        },
        "source_families": source_families,
        "contracts": contracts,
        "golden_manifests": golden_manifests,
        "lineage": {
            "nodes": sorted(nodes, key=lambda item: item["node_id"]),
            "edges": sorted(edges, key=lambda item: item["edge_id"]),
        },
        "controls": {
            "all_artifact_hashes_verified": True,
            "unique_node_ids": True,
            "unique_artifact_paths": True,
            "unique_edge_ids": True,
            "no_dangling_edges": True,
            "acyclic": True,
            "platform_records_schema_valid": True,
            "raw_environment_values_retained": False,
        },
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }
    ledger["ledger_sha256"] = sha256_bytes(canonical_json(ledger))
    return ledger


def duplicate_values(values: list[str]) -> list[str]:
    seen: set[str] = set()
    duplicates: set[str] = set()
    for value in values:
        if value in seen:
            duplicates.add(value)
        seen.add(value)
    return sorted(duplicates)


def graph_has_cycle(nodes: set[str], edges: list[dict[str, str]]) -> bool:
    adjacency = {node_id: [] for node_id in nodes}
    for item in edges:
        if item.get("from_node") in adjacency and item.get("to_node") in adjacency:
            adjacency[item["from_node"]].append(item["to_node"])
    visiting: set[str] = set()
    visited: set[str] = set()

    def visit(node_id: str) -> bool:
        if node_id in visiting:
            return True
        if node_id in visited:
            return False
        visiting.add(node_id)
        if any(visit(neighbor) for neighbor in adjacency[node_id]):
            return True
        visiting.remove(node_id)
        visited.add(node_id)
        return False

    return any(visit(node_id) for node_id in sorted(nodes))


def string_values(value: Any) -> list[str]:
    if isinstance(value, str):
        return [value]
    if isinstance(value, dict):
        return [item for child in value.values() for item in string_values(child)]
    if isinstance(value, list):
        return [item for child in value for item in string_values(child)]
    return []


def valid_repository_path(value: Any) -> bool:
    if not isinstance(value, str) or not value or "\\" in value:
        return False
    path = PurePosixPath(value)
    return not path.is_absolute() and ".." not in path.parts


def validate_ledger(ledger: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(ledger, dict):
        return ["fixture provenance ledger must be an object"]
    failures: list[str] = []
    required = {
        "schema_version",
        "ledger_id",
        "ledger_version",
        "status",
        "generated_at",
        "scope",
        "corpus",
        "source_families",
        "contracts",
        "golden_manifests",
        "lineage",
        "controls",
        "macos_execution_status",
        "macos_support_claim",
        "ledger_sha256",
    }
    if set(ledger) != required:
        failures.append("fixture provenance ledger fields do not match the contract")
        return failures
    if (
        ledger["schema_version"] != 1
        or ledger["ledger_id"] != "agentmage-fixture-provenance-ledger-v1"
        or ledger["ledger_version"] != "1.0.0"
        or ledger["status"] != "verified-synthetic-provenance"
    ):
        failures.append("fixture provenance ledger identity is invalid")
    if ledger["macos_execution_status"] != "blocked-macos" or ledger[
        "macos_support_claim"
    ] != "none":
        failures.append("fixture provenance ledger made an invalid macOS claim")

    source_families = ledger.get("source_families")
    if not isinstance(source_families, list) or [
        item.get("family") for item in source_families if isinstance(item, dict)
    ] != list(SOURCE_ORDER):
        failures.append("fixture provenance source-family closure is invalid")
    contract_ids = [
        item.get("contract_id", "")
        for item in ledger.get("contracts", [])
        if isinstance(item, dict)
    ]
    expected_contract_ids = [spec["contract_id"] for spec in CONTRACT_SPECS]
    if contract_ids != expected_contract_ids or duplicate_values(contract_ids):
        failures.append("fixture provenance contract closure is invalid")
    golden_manifests = ledger.get("golden_manifests", [])
    golden_ids = [
        item.get("manifest_id", "")
        for item in golden_manifests
        if isinstance(item, dict)
    ]
    golden_states = [
        item.get("evidence_state", "")
        for item in golden_manifests
        if isinstance(item, dict)
    ]
    if len(golden_ids) != 4 or len(set(golden_ids)) != 4 or set(golden_states) != {
        "Observed",
        "Derived",
        "Inferred",
        "Unknown/Blocked",
    }:
        failures.append("fixture provenance golden-manifest closure is invalid")

    expected_controls = {
        "all_artifact_hashes_verified": True,
        "unique_node_ids": True,
        "unique_artifact_paths": True,
        "unique_edge_ids": True,
        "no_dangling_edges": True,
        "acyclic": True,
        "platform_records_schema_valid": True,
        "raw_environment_values_retained": False,
    }
    if ledger.get("controls") != expected_controls:
        failures.append("fixture provenance controls were weakened")

    unhashed = dict(ledger)
    recorded_hash = unhashed.pop("ledger_sha256", None)
    if recorded_hash != sha256_bytes(canonical_json(unhashed)):
        failures.append("fixture provenance ledger self-hash is invalid")

    lineage = ledger.get("lineage")
    if not isinstance(lineage, dict):
        failures.append("fixture provenance lineage is missing")
        return failures
    nodes = lineage.get("nodes")
    edges = lineage.get("edges")
    if not isinstance(nodes, list) or not isinstance(edges, list):
        failures.append("fixture provenance lineage collections are invalid")
        return failures

    node_ids = [item.get("node_id", "") for item in nodes if isinstance(item, dict)]
    node_paths = [item.get("path", "") for item in nodes if isinstance(item, dict)]
    edge_ids = [item.get("edge_id", "") for item in edges if isinstance(item, dict)]
    if duplicate_values(node_ids):
        failures.append("fixture provenance ledger contains duplicate node ids")
    if duplicate_values(node_paths):
        failures.append("fixture provenance ledger contains duplicate artifact paths")
    if duplicate_values(edge_ids):
        failures.append("fixture provenance ledger contains duplicate edge ids")

    node_id_set = set(node_ids)
    for item in nodes:
        if not isinstance(item, dict):
            failures.append("fixture provenance node is invalid")
            continue
        path = item.get("path")
        if not valid_repository_path(path):
            failures.append(f"fixture provenance path is invalid: {path}")
            continue
        absolute = root / path
        if not absolute.is_file():
            failures.append(f"fixture provenance artifact is missing: {path}")
        elif item.get("sha256") != sha256_file(absolute):
            failures.append(f"fixture provenance artifact hash is invalid: {path}")

    for item in edges:
        if not isinstance(item, dict):
            failures.append("fixture provenance edge is invalid")
            continue
        if item.get("from_node") not in node_id_set or item.get("to_node") not in node_id_set:
            failures.append(f"fixture provenance edge is dangling: {item.get('edge_id')}")
        if item.get("relation") not in EXPECTED_RELATIONS:
            failures.append(f"fixture provenance relation is invalid: {item.get('edge_id')}")
    if graph_has_cycle(node_id_set, [item for item in edges if isinstance(item, dict)]):
        failures.append("fixture provenance graph contains a cycle")

    for value in string_values(ledger):
        if PRIVATE_PATH.search(value):
            failures.append("fixture provenance ledger contains a private path")
            break
    for value in string_values(ledger):
        if SECRET_VALUE.search(value):
            failures.append("fixture provenance ledger contains secret-shaped material")
            break

    try:
        expected = build_ledger(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        failures.append(f"cannot rebuild fixture provenance ledger: {error}")
    else:
        if ledger != expected:
            failures.append("fixture provenance ledger is stale or non-deterministic")
    return failures


def build_report(root: Path = ROOT) -> dict[str, Any]:
    ledger_path = root / LEDGER_PATH.relative_to(ROOT)
    ledger = read_json(ledger_path)
    failures = validate_ledger(ledger, root)
    if failures:
        raise ValueError("; ".join(failures))
    platform_report = read_json(root / PLATFORM_REPORT_PATH.relative_to(ROOT))
    return {
        "schema_version": 1,
        "task_id": "2.1.2.3",
        "status": "pass",
        "schemas": {
            "fixture_provenance_ledger": artifact(
                root, LEDGER_SCHEMA_PATH.relative_to(ROOT).as_posix()
            ),
            "platform_result": artifact(
                root, PLATFORM_SCHEMA_PATH.relative_to(ROOT).as_posix()
            ),
        },
        "ledger": {
            "path": LEDGER_PATH.relative_to(ROOT).as_posix(),
            "sha256": sha256_file(ledger_path),
            "self_sha256": ledger["ledger_sha256"],
            "source_family_count": len(ledger["source_families"]),
            "contract_count": len(ledger["contracts"]),
            "node_count": len(ledger["lineage"]["nodes"]),
            "edge_count": len(ledger["lineage"]["edges"]),
        },
        "platform_records": {
            "count": len(platform_report["synthetic_record_set"]["records"]),
            "record_set_sha256": platform_report["synthetic_record_set"]["sha256"],
            "schema_valid": True,
            "raw_environment_values_retained": False,
        },
        "root_fixture_contracts_complete": True,
        "all_artifact_hashes_verified": True,
        "lineage_graph_acyclic": True,
        "fixture_classification": "repository-controlled-synthetic-only",
        "product_support_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }


def validate_report(report: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(report, dict):
        return ["fixture provenance report must be an object"]
    failures: list[str] = []
    if report.get("schema_version") != 1 or report.get("task_id") != "2.1.2.3":
        failures.append("fixture provenance report identity is invalid")
    if report.get("status") != "pass":
        failures.append("fixture provenance report did not pass")
    if report.get("product_support_claim") != "none":
        failures.append("fixture provenance report made a product support claim")
    if report.get("macos_execution_status") != "blocked-macos" or report.get(
        "macos_support_claim"
    ) != "none":
        failures.append("fixture provenance report made an invalid macOS claim")
    try:
        expected = build_report(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        failures.append(f"cannot rebuild fixture provenance report: {error}")
    else:
        if report != expected:
            failures.append("fixture provenance report is stale or non-deterministic")
    return failures


def write_artifacts(root: Path = ROOT) -> None:
    ledger_path = root / LEDGER_PATH.relative_to(ROOT)
    write_atomic(ledger_path, canonical_json(build_ledger(root)))
    write_atomic(root / REPORT_PATH.relative_to(ROOT), canonical_json(build_report(root)))


def check_artifacts(root: Path = ROOT) -> list[str]:
    try:
        ledger = read_json(root / LEDGER_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read fixture provenance ledger: {error}"]
    failures = validate_ledger(ledger, root)
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        failures.append(f"cannot read fixture provenance report: {error}")
    else:
        failures.extend(validate_report(report, root))
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_artifacts()
        failures = check_artifacts()
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"fixture provenance validation failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"fixture provenance validation failed: {failure}", file=sys.stderr)
        return 1
    print("platform result schemas and fixture provenance ledger validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
