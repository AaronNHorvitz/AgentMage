#!/usr/bin/env python3
"""Build and verify the Story 4.1 kernel architecture dependency report."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
RULES_PATH = ROOT / "architecture" / "dependency-rules.json"
INVENTORY_PATH = ROOT / "architecture" / "module-inventory.json"
DOC_PATH = ROOT / "docs" / "architecture" / "kernel-dependency-report.md"
REPORT_PATH = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-4"
    / "story-4.1"
    / "kernel-architecture-dependency-report.json"
)

CARGO_MANIFESTS = {
    "capability-read-only": "capabilities/read-only/Cargo.toml",
    "kernel-contracts": "kernel/contracts/Cargo.toml",
    "kernel-engine": "kernel/engine/Cargo.toml",
    "platform-linux-native-inference": "platforms/linux-inference/Cargo.toml",
    "platform-linux": "platforms/linux/Cargo.toml",
    "platform-windows": "platforms/windows/Cargo.toml",
    "release-xtask": "release/xtask/Cargo.toml",
    "shell-host": "shells/host/Cargo.toml",
}
SOURCE_PATHS = (
    "Cargo.lock",
    "Cargo.toml",
    "architecture/dependency-rules.json",
    "architecture/module-inventory.json",
    "capabilities/read-only/Cargo.toml",
    "docs/architecture/kernel-dependency-report.md",
    "kernel/contracts/Cargo.toml",
    "kernel/engine/Cargo.toml",
    "package-lock.json",
    "platforms/linux/Cargo.toml",
    "platforms/linux-inference/Cargo.toml",
    "platforms/macos/Package.resolved",
    "platforms/macos/Package.swift",
    "release/xtask/Cargo.toml",
    "scripts/kernel_architecture_report.py",
    "shells/host/Cargo.toml",
    "shells/vscode/package.json",
    "tests/test_kernel_architecture_report.py",
    "platforms/windows/Cargo.toml",
)
EXPECTED_UNMATERIALIZED = {
    ("platform-macos", "kernel-contracts"): "blocked-macos",
    ("platform-macos", "kernel-engine"): "blocked-macos",
    ("shell-host", "platform-macos"): "blocked-macos",
    ("shell-vscode", "kernel-contracts"): "protocol-not-yet-generated",
}
REQUIRED_HEADINGS = (
    "Evidence Boundary",
    "Materialized Product Graph",
    "Declared but Unmaterialized Edges",
    "External Dependencies",
    "Authority Findings",
    "Scope Limits",
)


class ArchitectureReportError(ValueError):
    """Raised when dependency evidence is incomplete or inconsistent."""


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_toml(path: Path) -> dict[str, Any]:
    return tomllib.loads(path.read_text(encoding="utf-8"))


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def git_revision(root: Path = ROOT) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout.strip()


def canonical_json(value: Any) -> str:
    return json.dumps(value, indent=2, sort_keys=True) + "\n"


def _dependency_sections(manifest: dict[str, Any]) -> list[tuple[str, str, dict[str, Any]]]:
    sections: list[tuple[str, str, dict[str, Any]]] = []
    for section, kind in (
        ("dependencies", "product"),
        ("dev-dependencies", "development"),
        ("build-dependencies", "build"),
    ):
        values = manifest.get(section, {})
        if isinstance(values, dict):
            sections.append((kind, "all", values))
    targets = manifest.get("target", {})
    if isinstance(targets, dict):
        for condition, target in sorted(targets.items()):
            if not isinstance(target, dict):
                continue
            for section, kind in (
                ("dependencies", "product"),
                ("dev-dependencies", "development"),
                ("build-dependencies", "build"),
            ):
                values = target.get(section, {})
                if isinstance(values, dict):
                    sections.append((kind, condition, values))
    return sections


def cargo_dependency_records(root: Path = ROOT) -> tuple[list[dict[str, str]], list[dict[str, str]]]:
    manifests = {
        module_id: read_toml(root / relative)
        for module_id, relative in CARGO_MANIFESTS.items()
    }
    package_to_module = {
        manifest["package"]["name"]: module_id
        for module_id, manifest in manifests.items()
    }
    internal: list[dict[str, str]] = []
    external: list[dict[str, str]] = []
    for source, manifest in sorted(manifests.items()):
        for kind, condition, dependencies in _dependency_sections(manifest):
            for package in sorted(dependencies):
                record = {
                    "source": source,
                    "target": package_to_module.get(package, package),
                    "dependency_kind": kind,
                    "condition": condition,
                    "manifest": CARGO_MANIFESTS[source],
                }
                if package in package_to_module:
                    internal.append(record)
                else:
                    external.append(record)
    return internal, external


def declared_edges(rules: dict[str, Any]) -> list[dict[str, str]]:
    return sorted(
        (
            {"source": record["id"], "target": target}
            for record in rules["module_rules"]
            for target in record["declared_imports"]
        ),
        key=lambda item: (item["source"], item["target"]),
    )


def find_cycle(edges: list[dict[str, str]]) -> list[str] | None:
    graph: dict[str, set[str]] = {}
    for edge in edges:
        graph.setdefault(edge["source"], set()).add(edge["target"])
        graph.setdefault(edge["target"], set())
    visited: set[str] = set()
    active: list[str] = []

    def visit(node: str) -> list[str] | None:
        if node in active:
            index = active.index(node)
            return [*active[index:], node]
        if node in visited:
            return None
        active.append(node)
        for target in sorted(graph[node]):
            cycle = visit(target)
            if cycle is not None:
                return cycle
        active.pop()
        visited.add(node)
        return None

    for node in sorted(graph):
        cycle = visit(node)
        if cycle is not None:
            return cycle
    return None


def graph_findings(
    rules: dict[str, Any], observed: list[dict[str, str]]
) -> dict[str, Any]:
    policies = {record["id"]: record for record in rules["module_rules"]}
    declared = declared_edges(rules)
    declared_pairs = {(edge["source"], edge["target"]) for edge in declared}
    observed_product = [
        edge for edge in observed if edge["dependency_kind"] == "product"
    ]
    observed_pairs = {(edge["source"], edge["target"]) for edge in observed_product}
    prohibited = [
        edge
        for edge in observed_product
        if edge["target"] not in policies.get(edge["source"], {}).get("allowed_imports", [])
    ]
    missing_pairs = declared_pairs - observed_pairs
    unmaterialized = [
        {
            "source": source,
            "target": target,
            "reason": EXPECTED_UNMATERIALIZED.get((source, target), "unexplained"),
        }
        for source, target in sorted(missing_pairs)
    ]
    return {
        "declared_edges": declared,
        "observed_product_edges": observed_product,
        "declared_edge_count": len(declared),
        "observed_product_edge_count": len(observed_product),
        "prohibited_observed_edges": prohibited,
        "declared_but_unmaterialized_edges": unmaterialized,
        "declared_cycle": find_cycle(declared),
        "observed_cycle": find_cycle(observed_product),
    }


def _cargo_lock_versions(root: Path) -> dict[str, dict[str, str]]:
    result: dict[str, dict[str, str]] = {}
    for package in read_toml(root / "Cargo.lock").get("package", []):
        if not isinstance(package, dict) or "source" not in package:
            continue
        record = {"version": str(package["version"]), "source": str(package["source"])}
        if "checksum" in package:
            record["checksum"] = str(package["checksum"])
        result[str(package["name"])] = record
    return result


def validate_document(text: str, graph: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    for heading in REQUIRED_HEADINGS:
        if f"## {heading}" not in text:
            failures.append(f"missing report heading: {heading}")
    for edge in graph["observed_product_edges"]:
        marker = f"`{edge['source']}` -> `{edge['target']}`"
        if marker not in text:
            failures.append(f"missing materialized edge in report: {marker}")
    for edge in graph["declared_but_unmaterialized_edges"]:
        marker = f"`{edge['source']}` -> `{edge['target']}`"
        if marker not in text or f"`{edge['reason']}`" not in text:
            failures.append(f"missing unmaterialized edge disposition: {marker}")
    forbidden_claims = (
        "macOS implementation is complete",
        "macOS verification passed",
        "all declared edges are implemented",
    )
    for claim in forbidden_claims:
        if claim.casefold() in text.casefold():
            failures.append(f"unsupported architecture claim: {claim}")
    return failures


def build_report(source_revision: str, root: Path = ROOT) -> dict[str, Any]:
    rules = read_json(root / "architecture/dependency-rules.json")
    inventory = read_json(root / "architecture/module-inventory.json")
    internal, external = cargo_dependency_records(root)
    graph = graph_findings(rules, internal)
    document_failures = validate_document(
        (root / "docs/architecture/kernel-dependency-report.md").read_text(
            encoding="utf-8"
        ),
        graph,
    )
    lock_versions = _cargo_lock_versions(root)
    direct_external = []
    for edge in external:
        direct_external.append({**edge, "lock": lock_versions.get(edge["target"])})
    vscode = read_json(root / "shells/vscode/package.json")
    swift_manifest = (root / "platforms/macos/Package.swift").read_text(
        encoding="utf-8"
    )
    source_records = [
        {"path": relative, "sha256": sha256_file(root / relative)}
        for relative in SOURCE_PATHS
    ]
    unmaterialized_pairs = {
        (edge["source"], edge["target"]): edge["reason"]
        for edge in graph["declared_but_unmaterialized_edges"]
    }
    checks = {
        "declared_graph_acyclic": graph["declared_cycle"] is None,
        "observed_graph_acyclic": graph["observed_cycle"] is None,
        "observed_edges_allowed": not graph["prohibited_observed_edges"],
        "unmaterialized_edges_explained": unmaterialized_pairs
        == EXPECTED_UNMATERIALIZED,
        "document_complete": not document_failures,
        "vscode_product_dependencies_absent": all(
            vscode.get(section) in (None, {}, [])
            for section in ("dependencies", "optionalDependencies", "bundledDependencies")
        ),
        "swift_external_dependencies_absent": ".package(" not in swift_manifest,
    }
    report = {
        "schema_version": 1,
        "task_id": "4.1.2.4",
        "artifact_id": "kernel-architecture-dependency-report",
        "status": "pass-linux-architecture" if all(checks.values()) else "fail",
        "source_revision": source_revision,
        "source_records": source_records,
        "decision_id": rules["decision_id"],
        "inventory_status": inventory["status"],
        "module_count": len(inventory["modules"]),
        "graph": graph,
        "external_dependencies": {
            "direct_cargo": direct_external,
            "direct_cargo_count": len(direct_external),
            "locked_external_cargo_package_count": len(lock_versions),
            "vscode_runtime": [],
            "vscode_development": sorted(vscode.get("devDependencies", {})),
            "swift_external": [],
        },
        "checks": checks,
        "document_failures": document_failures,
        "authority_findings": {
            "kernel_contracts_internal_dependencies": [
                edge
                for edge in graph["observed_product_edges"]
                if edge["source"] == "kernel-contracts"
            ],
            "kernel_engine_reverse_dependencies": [
                edge
                for edge in graph["observed_product_edges"]
                if edge["source"] == "kernel-engine"
                and edge["target"] != "kernel-contracts"
            ],
            "lower_layer_shell_dependencies": [
                edge
                for edge in graph["observed_product_edges"]
                if edge["source"]
                in {"kernel-contracts", "kernel-engine", "platform-linux", "capability-read-only"}
                and edge["target"] in {"shell-host", "shell-vscode"}
            ],
        },
        "platform_status": {
            "linux_manifest_graph": "verified-local",
            "macos_manifest": "interface-only",
            "macos_implementation": "blocked-macos",
            "macos_execution": "blocked-macos",
        },
        "limitations": [
            "The report verifies repository manifests and declared module policy, not runtime process or data flow.",
            "The VS Code generated contract protocol is not yet materialized.",
            "The macOS adapter and host composition edges remain blocked-macos.",
            "External package security and provenance are owned by separate supply-chain evidence.",
        ],
    }
    return report


def validate_report(report: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(report, dict):
        return ["kernel architecture report must be an object"]
    revision = report.get("source_revision")
    if not isinstance(revision, str) or len(revision) != 40:
        return ["kernel architecture report source revision is invalid"]
    try:
        expected = build_report(revision, root)
    except (OSError, KeyError, TypeError, ValueError, tomllib.TOMLDecodeError) as error:
        return [f"cannot rebuild kernel architecture report: {error}"]
    failures: list[str] = []
    if report != expected:
        failures.append("kernel architecture report is stale or malformed")
    if report.get("status") != "pass-linux-architecture":
        failures.append("kernel architecture report is not passing")
    return failures


def write_report(root: Path = ROOT) -> None:
    REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
    REPORT_PATH.write_text(
        canonical_json(build_report(git_revision(root), root)), encoding="utf-8"
    )


def check_report(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(REPORT_PATH)
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read kernel architecture report: {error}"]
    return validate_report(report, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write:
        write_report()
    failures = check_report()
    if failures:
        for failure in failures:
            print(f"kernel architecture report validation failed: {failure}", file=sys.stderr)
        return 1
    report = read_json(REPORT_PATH)
    print(
        "Kernel architecture dependency report validated: "
        f"{report['graph']['observed_product_edge_count']} materialized edges"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
