#!/usr/bin/env python3
"""Scan AgentMage source, build, package, and SBOM surfaces."""

from __future__ import annotations

import argparse
import copy
import json
import re
import sys
import tempfile
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
POLICY_PATH = ROOT / "architecture" / "artifact-scan-policy.json"
REPORT_PATH = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-1"
    / "story-1.1"
    / "artifact-scan-report.json"
)
EXPECTED_SOURCE_ROOTS = ("capabilities", "kernel", "platforms", "release", "shells")
EXPECTED_EXTENSIONS = (".js", ".mjs", ".rs", ".swift", ".ts")
EXPECTED_SEEDS = (
    "dynamic-loader",
    "privileged-assumption",
    "secret-pattern",
    "undeclared-binary",
    "undeclared-license",
)
EXPECTED_PRODUCTION_LICENSES = (
    "0BSD OR MIT OR Apache-2.0",
    "(MIT OR Apache-2.0) AND Unicode-3.0",
    "(Apache-2.0 OR MIT) AND BSD-3-Clause",
    "Apache-2.0",
    "Apache-2.0 OR BSD-3-Clause",
    "Apache-2.0 OR MIT",
    "Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT",
    "BSD-3-Clause",
    "MIT",
    "MIT AND BSD-3-Clause",
    "MIT OR Apache-2.0 OR LGPL-2.1-or-later",
    "MIT OR Apache-2.0 OR BSD-1-Clause",
    "MIT OR Apache-2.0 OR Zlib",
    "MIT OR Apache-2.0",
    "MIT OR Zlib OR Apache-2.0",
    "MIT/Apache-2.0",
    "Unlicense OR MIT",
    "Zlib OR Apache-2.0 OR MIT",
    "Zlib",
)
EXPECTED_PLANNING_ROOTS = (
    "architecture",
    "configuration",
    "docs",
    "requirements",
    "schemas",
    "scripts",
    "tests",
)
EXPECTED_PLANNING_DISPOSITION = {
    "production_source_roots_unchanged": True,
    "runtime_source_scan_expansion": "blocked-until-source-exists",
    "validation_owners": [
        "build-contract",
        "documentation",
        "schema",
        "secret-scan",
    ],
}
SECRET_ASSIGNMENT = re.compile(
    r"(?i)\b(?:api[_-]?key|password|token)\s*=\s*[\"']"
    r"(?=[^,\r\n\"'])[^\r\n\"']+[\"']"
)
DYNAMIC_LOADERS = (
    re.compile(r"\bdlopen\s*\("),
    re.compile(r"\blibloading::"),
    re.compile(r"\bLoadLibrary(?:A|W)?\s*\("),
)
PRIVILEGED_ASSUMPTIONS = (
    re.compile(r"\bsudo\s+"),
    re.compile(r"\bpkexec\b"),
    re.compile(r"\bCAP_SYS_ADMIN\b"),
    re.compile(r'(?<!== )"/var/run/docker\.sock"'),
    re.compile(r"\bgeteuid\s*\(\s*\)\s*==\s*0"),
)
BINARY_MAGICS = (
    bytes((0x7F, 0x45, 0x4C, 0x46)),
    bytes((0xCF, 0xFA, 0xED, 0xFE)),
    bytes((0xFE, 0xED, 0xFA, 0xCF)),
    bytes((0x4D, 0x5A)),
)


def load_policy(path: Path = POLICY_PATH) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def validate_policy(policy: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(policy, dict):
        return ["artifact scan policy must be an object"]
    if policy.get("schema_version") != 1 or policy.get("status") != "enforced":
        failures.append("artifact scan policy identity is invalid")
    if policy.get("planning_decision_ids") != ["ADR-0043", "ADR-0044"]:
        failures.append("artifact scan planning decisions are incomplete")
    if tuple(policy.get("planning_contract_roots", [])) != EXPECTED_PLANNING_ROOTS:
        failures.append("artifact scan planning contract roots drifted")
    if policy.get("planning_contract_disposition") != EXPECTED_PLANNING_DISPOSITION:
        failures.append("artifact scan planning disposition drifted")
    if tuple(policy.get("source_roots", [])) != EXPECTED_SOURCE_ROOTS:
        failures.append("artifact scan source roots drifted")
    if tuple(policy.get("source_extensions", [])) != EXPECTED_EXTENSIONS:
        failures.append("artifact scan source extensions drifted")
    if tuple(policy.get("approved_production_licenses", [])) != EXPECTED_PRODUCTION_LICENSES:
        failures.append("production license allowlist drifted")
    if policy.get("excluded_development_noassertion_disposition") != "open-review":
        failures.append("missing development license metadata must remain open review")
    if tuple(sorted(policy.get("required_seeded_findings", []))) != EXPECTED_SEEDS:
        failures.append("seeded finding closure is incomplete")
    if policy.get("platform_status") != {
        "macos_binary_scan": "blocked-macos",
        "macos_package_scan": "blocked-macos",
    }:
        failures.append("artifact scan must retain blocked macOS status")
    return failures


def finding(
    category: str,
    surface: str,
    relative_path: str,
    severity: str = "blocking",
) -> dict[str, str]:
    return {
        "category": category,
        "surface": surface,
        "path": relative_path,
        "severity": severity,
    }


def scan_text(text: str, surface: str, relative_path: str) -> list[dict[str, str]]:
    findings = []
    if SECRET_ASSIGNMENT.search(text):
        findings.append(finding("secret-pattern", surface, relative_path))
    if any(pattern.search(text) for pattern in DYNAMIC_LOADERS):
        findings.append(finding("dynamic-loader", surface, relative_path))
    if any(pattern.search(text) for pattern in PRIVILEGED_ASSUMPTIONS):
        findings.append(finding("privileged-assumption", surface, relative_path))
    return findings


def product_source_files(root: Path, policy: dict[str, Any]) -> list[Path]:
    extensions = set(policy["source_extensions"])
    return sorted(
        path
        for source_root in policy["source_roots"]
        for path in (root / source_root).rglob("*")
        if path.is_file()
        and path.suffix in extensions
        and not any(part in {"dist", "target", ".build"} for part in path.parts)
    )


def scan_source(root: Path, policy: dict[str, Any]) -> dict[str, Any]:
    files = product_source_files(root, policy)
    findings = []
    for path in files:
        findings.extend(
            scan_text(
                path.read_text(encoding="utf-8"),
                "source",
                path.relative_to(root).as_posix(),
            )
        )
    return {
        "surface": "source",
        "files_scanned": len(files),
        "findings": sorted(
            findings, key=lambda item: (item["category"], item["path"])
        ),
    }


def is_binary(content: bytes) -> bool:
    return any(content.startswith(magic) for magic in BINARY_MAGICS)


def scan_binary_surface(
    root: Path,
    surface: str,
    declared_binaries: set[str],
) -> dict[str, Any]:
    files = sorted(path for path in root.rglob("*") if path.is_file())
    findings = []
    for path in files:
        relative = path.relative_to(root).as_posix()
        content = path.read_bytes()
        if is_binary(content) and relative not in declared_binaries:
            findings.append(finding("undeclared-binary", surface, relative))
        if any(marker in content for marker in (b"/lib64/ld-linux", b"libdl.so")):
            findings.append(finding("dynamic-loader", surface, relative))
        try:
            text = content.decode("utf-8")
        except UnicodeDecodeError:
            continue
        findings.extend(scan_text(text, surface, relative))
    return {
        "surface": surface,
        "files_scanned": len(files),
        "findings": sorted(
            findings, key=lambda item: (item["category"], item["path"])
        ),
    }


def _license_value(component: dict[str, Any]) -> str:
    licenses = component.get("licenses", [])
    if not licenses:
        return "NOASSERTION"
    choice = licenses[0]
    if "expression" in choice:
        return choice["expression"]
    license_record = choice.get("license", {})
    return license_record.get("id") or license_record.get("name") or "NOASSERTION"


def scan_sbom(bom: dict[str, Any], policy: dict[str, Any]) -> dict[str, Any]:
    findings = []
    approved = set(policy["approved_production_licenses"])
    for component in bom.get("components", []):
        properties = {
            item.get("name"): item.get("value")
            for item in component.get("properties", [])
        }
        dependency_class = properties.get("agentmage:dependency-class")
        license_value = _license_value(component)
        component_id = component.get("bom-ref", "unknown")
        if dependency_class == "production" and license_value not in approved:
            findings.append(finding("undeclared-license", "sbom", component_id))
        elif (
            dependency_class == "development"
            and component.get("scope") == "excluded"
            and license_value == "NOASSERTION"
        ):
            findings.append(
                finding("license-review", "sbom", component_id, severity="open-review")
            )
    return {
        "surface": "sbom",
        "files_scanned": 1,
        "components_scanned": len(bom.get("components", [])),
        "findings": sorted(
            findings, key=lambda item: (item["category"], item["path"])
        ),
    }


def clean_binary_surfaces() -> tuple[dict[str, Any], dict[str, Any]]:
    with tempfile.TemporaryDirectory(prefix="agentmage-artifact-scan-") as temporary:
        root = Path(temporary)
        build = root / "build-output"
        package = root / "package"
        build.mkdir()
        package.mkdir()
        clean_binary = bytes((0x7F, 0x45, 0x4C, 0x46)) + b"agentmage-static-fixture"
        (build / "agentmage").write_bytes(clean_binary)
        (package / "agentmage").write_bytes(clean_binary)
        (package / "LICENSE").write_text("Apache-2.0 fixture", encoding="utf-8")
        return (
            scan_binary_surface(build, "build-output", {"agentmage"}),
            scan_binary_surface(package, "package", {"agentmage"}),
        )


def seeded_cases(policy: dict[str, Any], bom: dict[str, Any]) -> list[dict[str, Any]]:
    cases = []
    with tempfile.TemporaryDirectory(prefix="agentmage-artifact-seeds-") as temporary:
        root = Path(temporary)
        binary_root = root / "binary"
        binary_root.mkdir()
        (binary_root / "undeclared-helper").write_bytes(
            bytes((0x7F, 0x45, 0x4C, 0x46)) + b"fixture"
        )
        binary_result = scan_binary_surface(binary_root, "build-output", set())
        cases.append(
            {
                "seed": "undeclared-binary",
                "observed_categories": sorted(
                    {item["category"] for item in binary_result["findings"]}
                ),
            }
        )

        secret_text = "const api" + "_key = 'synthetic-fixture';"
        dynamic_text = "unsafe_call = dlo" + "pen(handle);"
        privilege_text = "su" + "do run-fixture"
        for seed, text in (
            ("secret-pattern", secret_text),
            ("dynamic-loader", dynamic_text),
            ("privileged-assumption", privilege_text),
        ):
            result = scan_text(text, "source", f"synthetic/{seed}.txt")
            cases.append(
                {
                    "seed": seed,
                    "observed_categories": sorted(
                        {item["category"] for item in result}
                    ),
                }
            )

        mutated_bom = copy.deepcopy(bom)
        production = next(
            item
            for item in mutated_bom["components"]
            if {
                prop.get("name"): prop.get("value")
                for prop in item.get("properties", [])
            }.get("agentmage:dependency-class")
            == "production"
        )
        production["licenses"] = [{"license": {"name": "Synthetic-Unapproved"}}]
        license_result = scan_sbom(mutated_bom, policy)
        cases.append(
            {
                "seed": "undeclared-license",
                "observed_categories": sorted(
                    {item["category"] for item in license_result["findings"]}
                ),
            }
        )
    for case in cases:
        case["status"] = (
            "pass" if case["seed"] in case["observed_categories"] else "fail"
        )
    return sorted(cases, key=lambda item: item["seed"])


def build_report(root: Path = ROOT) -> dict[str, Any]:
    policy = load_policy(root / "architecture/artifact-scan-policy.json")
    bom = json.loads((root / "supply-chain/sbom.cdx.json").read_text(encoding="utf-8"))
    source = scan_source(root, policy)
    sbom = scan_sbom(bom, policy)
    build_output, package = clean_binary_surfaces()
    seeds = seeded_cases(policy, bom)
    blocking = sum(
        item["severity"] == "blocking"
        for surface in (source, sbom, build_output, package)
        for item in surface["findings"]
    )
    open_review = sum(
        item["severity"] == "open-review"
        for surface in (source, sbom, build_output, package)
        for item in surface["findings"]
    )
    return {
        "schema_version": 1,
        "test_id": "S-001-ST01",
        "status": (
            "pass-with-open-review"
            if blocking == 0 and all(item["status"] == "pass" for item in seeds)
            else "fail"
        ),
        "platform_status": {
            "shared_source_and_sbom": "scanned",
            "build_and_package": "synthetic-clean-surfaces",
            "macos_binary_scan": "blocked-macos",
            "macos_package_scan": "blocked-macos",
        },
        "surfaces": [source, build_output, package, sbom],
        "seeded_cases": seeds,
        "summary": {
            "blocking_findings": blocking,
            "open_review_findings": open_review,
            "seeded_findings_detected": sum(item["status"] == "pass" for item in seeds),
        },
        "macos_support_claim": "none",
    }


def validate_report(report: Any, root: Path = ROOT) -> list[str]:
    failures = validate_policy(load_policy(root / "architecture/artifact-scan-policy.json"))
    if not isinstance(report, dict):
        return [*failures, "artifact scan report must be an object"]
    if report.get("schema_version") != 1 or report.get("test_id") != "S-001-ST01":
        failures.append("artifact scan report identity is invalid")
    if report.get("status") != "pass-with-open-review":
        failures.append("artifact scanner did not pass with visible open review")
    if report.get("platform_status") != {
        "shared_source_and_sbom": "scanned",
        "build_and_package": "synthetic-clean-surfaces",
        "macos_binary_scan": "blocked-macos",
        "macos_package_scan": "blocked-macos",
    }:
        failures.append("artifact scan platform status is invalid")
    if report.get("macos_support_claim") != "none":
        failures.append("artifact scanner cannot make a macOS support claim")
    if report.get("summary", {}).get("blocking_findings") != 0:
        failures.append("canonical artifact surfaces contain a blocking finding")
    seeds = report.get("seeded_cases", [])
    if {item.get("seed") for item in seeds} != set(EXPECTED_SEEDS) or len(seeds) != 5:
        failures.append("seeded artifact finding closure is incomplete")
    for case in seeds:
        if case.get("status") != "pass" or case.get("seed") not in case.get(
            "observed_categories", []
        ):
            failures.append(f"seeded artifact violation was not detected: {case.get('seed')}")
    expected = build_report(root)
    if report != expected:
        failures.append("artifact scan report is stale or non-deterministic")
    return failures


def write_report(root: Path = ROOT) -> None:
    REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
    REPORT_PATH.write_text(
        json.dumps(build_report(root), indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def check_report(root: Path = ROOT) -> list[str]:
    try:
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read artifact scan report: {error}"]
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
            print(f"artifact scan validation failed: {failure}", file=sys.stderr)
        return 1
    print("source, build, package, SBOM, and seeded artifact scans validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
