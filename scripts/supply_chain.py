#!/usr/bin/env python3
"""Generate deterministic dependency provenance, hashes, and CycloneDX SBOM."""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import re
import sys
import tomllib
import uuid
from pathlib import Path
from typing import Any
from urllib.parse import quote


ROOT = Path(__file__).resolve().parents[1]
OUTPUT_DIR = ROOT / "supply-chain"
PROVENANCE_PATH = OUTPUT_DIR / "dependency-provenance.json"
SBOM_PATH = OUTPUT_DIR / "sbom.cdx.json"
HASH_PATH = OUTPUT_DIR / "dependency-hashes.sha256"
INPUT_PATHS = (
    "Cargo.lock",
    "Cargo.toml",
    "package-lock.json",
    "package.json",
    "platforms/macos/Package.resolved",
    "platforms/macos/Package.swift",
    "rust-toolchain.toml",
    "shells/vscode/package.json",
    "supply-chain/cargo-external-catalog.json",
)


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_toml(path: Path) -> dict[str, Any]:
    return tomllib.loads(path.read_text(encoding="utf-8"))


def canonical_json(value: Any) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def tree_hash(path: Path) -> str:
    digest = hashlib.sha256()
    excluded = {
        ".build",
        ".git",
        "__pycache__",
        "artifacts",
        "dist",
        "node_modules",
        "review-evidence",
        "supply-chain",
        "target",
    }
    files = sorted(
        candidate
        for candidate in path.rglob("*")
        if candidate.is_file()
        and not any(part in excluded for part in candidate.parts)
    )
    for candidate in files:
        relative = candidate.relative_to(path).as_posix().encode("utf-8")
        digest.update(len(relative).to_bytes(8, "big"))
        digest.update(relative)
        content = candidate.read_bytes()
        digest.update(len(content).to_bytes(8, "big"))
        digest.update(content)
    return digest.hexdigest()


def integrity_hash(integrity: str) -> tuple[str, str]:
    algorithm, encoded = integrity.split("-", 1)
    algorithm_names = {
        "sha256": "SHA-256",
        "sha384": "SHA-384",
        "sha512": "SHA-512",
    }
    if algorithm not in algorithm_names:
        raise ValueError(f"unsupported npm integrity algorithm: {algorithm}")
    return algorithm_names[algorithm], base64.b64decode(encoded, validate=True).hex()


def npm_name(lock_path: str) -> str:
    return lock_path.rsplit("node_modules/", 1)[1]


def cargo_components(root: Path) -> tuple[list[dict[str, Any]], dict[str, list[str]]]:
    cargo_lock = read_toml(root / "Cargo.lock")
    workspace = read_toml(root / "Cargo.toml")
    external_catalog = read_json(root / "supply-chain/cargo-external-catalog.json")
    members = workspace["workspace"]["members"]
    workspace_license = workspace["workspace"]["package"]["license"]
    member_by_name: dict[str, tuple[str, dict[str, Any]]] = {}
    for member in members:
        manifest = read_toml(root / member / "Cargo.toml")
        member_by_name[manifest["package"]["name"]] = (member, manifest)

    external_licenses = {
        (item["name"], item["version"]): item["license"]
        for item in external_catalog.get("packages", [])
    }
    lock_packages = cargo_lock.get("package", [])
    external_lock_keys = {
        (item["name"], item["version"])
        for item in lock_packages
        if item.get("source") is not None
    }
    if external_catalog.get("schema_version") != 1:
        raise ValueError("Cargo external license catalog schema version is invalid")
    if set(external_licenses) != external_lock_keys:
        raise ValueError("Cargo external license catalog does not match Cargo.lock")
    package_id_by_name = {
        item["name"]: f"cargo:{item['name']}@{item['version']}" for item in lock_packages
    }
    if len(package_id_by_name) != len(lock_packages):
        raise ValueError("Cargo.lock contains ambiguous package names")

    components: list[dict[str, Any]] = []
    edges: dict[str, list[str]] = {}
    for package in lock_packages:
        name = package["name"]
        version = package["version"]
        component_id = f"cargo:{name}@{version}"
        if name in member_by_name:
            member, manifest = member_by_name[name]
            component = {
                "component_id": component_id,
                "ecosystem": "cargo",
                "name": name,
                "version": version,
                "classification": "production",
                "scope": "required",
                "license": (
                    workspace_license
                    if manifest["package"].get("license") == {"workspace": True}
                    else manifest["package"].get("license", "NOASSERTION")
                ),
                "source": {"type": "repository-path", "path": member},
                "integrity": None,
                "hashes": [{"algorithm": "SHA-256", "content": tree_hash(root / member)}],
            }
        else:
            checksum = package.get("checksum")
            if package.get("source") != "registry+https://github.com/rust-lang/crates.io-index":
                raise ValueError(f"Cargo package has an unapproved source: {name}")
            if not isinstance(checksum, str) or not re.fullmatch(r"[a-f0-9]{64}", checksum):
                raise ValueError(f"Cargo package is missing its registry checksum: {name}")
            component = {
                "component_id": component_id,
                "ecosystem": "cargo",
                "name": name,
                "version": version,
                "classification": "production",
                "scope": "required",
                "license": external_licenses[(name, version)],
                "source": {
                    "type": "registry",
                    "url": f"https://crates.io/crates/{quote(name, safe='')}/{quote(version, safe='')}",
                },
                "integrity": f"sha256:{checksum}",
                "hashes": [{"algorithm": "SHA-256", "content": checksum}],
            }
        components.append(component)
        dependencies = []
        for dependency in package.get("dependencies", []):
            dependency_name = dependency.split(" ", 1)[0]
            dependency_id = package_id_by_name.get(dependency_name)
            if dependency_id is None:
                raise ValueError(f"Cargo dependency is absent from the lock: {dependency_name}")
            dependencies.append(dependency_id)
        edges[component_id] = sorted(dependencies)
    return components, edges


def npm_components(root: Path) -> tuple[list[dict[str, Any]], dict[str, list[str]]]:
    lock = read_json(root / "package-lock.json")
    packages = lock["packages"]
    components: list[dict[str, Any]] = []
    path_to_id: dict[str, str] = {}

    workspace_specs = (
        ("", "agentmage-planning-docs", "development", "excluded"),
        ("shells/vscode", "@agentmage/vscode-shell", "production", "required"),
    )
    for path, expected_name, classification, scope in workspace_specs:
        package = packages[path]
        component_id = f"npm-workspace:{expected_name}@{package['version']}"
        path_to_id[path] = component_id
        source_path = "." if path == "" else path
        components.append(
            {
                "component_id": component_id,
                "ecosystem": "npm-workspace",
                "name": expected_name,
                "version": package["version"],
                "classification": classification,
                "scope": scope,
                "license": package.get("license", "NOASSERTION"),
                "source": {"type": "repository-path", "path": source_path},
                "integrity": None,
                "hashes": [
                    {
                        "algorithm": "SHA-256",
                        "content": (
                            sha256_bytes((root / "package.json").read_bytes())
                            if path == ""
                            else tree_hash(root / source_path)
                        ),
                    }
                ],
            }
        )

    for lock_path, package in sorted(packages.items()):
        if not lock_path.startswith("node_modules/") or package.get("link") is True:
            continue
        name = npm_name(lock_path)
        version = package["version"]
        component_id = f"npm:{lock_path}@{version}"
        path_to_id[lock_path] = component_id
        algorithm, content = integrity_hash(package["integrity"])
        components.append(
            {
                "component_id": component_id,
                "ecosystem": "npm",
                "name": name,
                "version": version,
                "classification": "development" if package.get("dev") else "production",
                "scope": "excluded" if package.get("dev") else "required",
                "license": package.get("license", "NOASSERTION"),
                "source": {"type": "registry", "url": package["resolved"]},
                "integrity": package["integrity"],
                "hashes": [{"algorithm": algorithm, "content": content}],
            }
        )

    def resolve_dependency(package_path: str, dependency: str) -> str | None:
        nested = f"{package_path}/node_modules/{dependency}" if package_path else ""
        root_path = f"node_modules/{dependency}"
        if nested and nested in path_to_id:
            return path_to_id[nested]
        return path_to_id.get(root_path)

    edges: dict[str, list[str]] = {}
    for lock_path, package in sorted(packages.items()):
        source_id = path_to_id.get(lock_path)
        if source_id is None:
            continue
        dependency_names: set[str] = set()
        for field in ("dependencies", "optionalDependencies", "peerDependencies"):
            dependency_names.update(package.get(field, {}))
        resolved = {
            target
            for dependency in dependency_names
            if (target := resolve_dependency(lock_path, dependency)) is not None
        }
        edges[source_id] = sorted(resolved)
    return components, edges


def swift_component(root: Path) -> dict[str, Any]:
    path = root / "platforms/macos"
    return {
        "component_id": "swift-workspace:AgentMageMacOSPlatform@0.0.0",
        "ecosystem": "swift-workspace",
        "name": "AgentMageMacOSPlatform",
        "version": "0.0.0",
        "classification": "production",
        "scope": "required",
        "license": "Apache-2.0",
        "source": {"type": "repository-path", "path": "platforms/macos"},
        "integrity": None,
        "hashes": [{"algorithm": "SHA-256", "content": tree_hash(path)}],
        "platform_status": "blocked-macos",
    }


def purl(component: dict[str, Any]) -> str:
    ecosystem = component["ecosystem"]
    name = component["name"]
    version = component["version"]
    if ecosystem == "cargo":
        return f"pkg:cargo/{quote(name, safe='')}@{quote(version, safe='')}"
    if ecosystem in {"npm", "npm-workspace"}:
        return f"pkg:npm/{quote(name, safe='/')}@{quote(version, safe='')}"
    return f"pkg:swift/{quote(name, safe='')}@{quote(version, safe='')}"


def build_documents(root: Path = ROOT) -> tuple[dict[str, Any], dict[str, Any], str]:
    cargo, cargo_edges = cargo_components(root)
    npm, npm_edges = npm_components(root)
    swift = swift_component(root)
    components = sorted([*cargo, *npm, swift], key=lambda item: item["component_id"])
    edges = {**cargo_edges, **npm_edges, swift["component_id"]: []}

    input_hashes = {path: sha256_bytes((root / path).read_bytes()) for path in INPUT_PATHS}
    provenance = {
        "schema_version": 1,
        "document_id": "agentmage-dependency-provenance",
        "generated_from": input_hashes,
        "platform_status": {"shared_linux": "verified-local", "macos": "blocked-macos"},
        "components": components,
        "dependencies": [
            {"component_id": component_id, "depends_on": edges.get(component_id, [])}
            for component_id in sorted(edges)
        ],
    }

    seed = b"".join((root / path).read_bytes() for path in INPUT_PATHS)
    serial = uuid.UUID(bytes=hashlib.sha256(seed).digest()[:16], version=5)
    bom_components = []
    for component in components:
        license_value = component["license"]
        if license_value == "NOASSERTION":
            license_choice = {"license": {"name": "NOASSERTION"}}
        elif re.fullmatch(r"[A-Za-z0-9.+-]+", license_value):
            license_choice = {"license": {"id": license_value}}
        else:
            license_choice = {"expression": license_value}
        bom_components.append(
            {
                "type": "library",
                "bom-ref": component["component_id"],
                "name": component["name"],
                "version": component["version"],
                "scope": component["scope"],
                "purl": purl(component),
                "licenses": [license_choice],
                "hashes": [
                    {"alg": item["algorithm"], "content": item["content"]}
                    for item in component["hashes"]
                ],
                "properties": [
                    {
                        "name": "agentmage:dependency-class",
                        "value": component["classification"],
                    },
                    {"name": "agentmage:ecosystem", "value": component["ecosystem"]},
                ],
            }
        )
    bom = {
        "bomFormat": "CycloneDX",
        "specVersion": "1.6",
        "serialNumber": f"urn:uuid:{serial}",
        "version": 1,
        "metadata": {
            "component": {
                "type": "application",
                "bom-ref": "application:agentmage@0.0.0",
                "name": "AgentMage",
                "version": "0.0.0",
                "licenses": [{"license": {"id": "Apache-2.0"}}],
            },
            "properties": [
                {"name": "agentmage:macos-status", "value": "blocked-macos"}
            ],
        },
        "components": bom_components,
        "dependencies": [
            {"ref": item["component_id"], "dependsOn": item["depends_on"]}
            for item in provenance["dependencies"]
        ],
    }

    provenance_text = canonical_json(provenance)
    bom_text = canonical_json(bom)
    hashed_content = {
        **{path: (root / path).read_bytes() for path in INPUT_PATHS},
        "supply-chain/dependency-provenance.json": provenance_text.encode("utf-8"),
        "supply-chain/sbom.cdx.json": bom_text.encode("utf-8"),
    }
    hash_text = "".join(
        f"{sha256_bytes(content)}  {path}\n"
        for path, content in sorted(hashed_content.items())
    )
    return provenance, bom, hash_text


def validate_documents(
    provenance: Any, bom: Any, hash_text: str, root: Path = ROOT
) -> list[str]:
    failures: list[str] = []
    if not isinstance(provenance, dict) or not isinstance(bom, dict):
        return ["provenance and SBOM must be objects"]
    if provenance.get("schema_version") != 1:
        failures.append("provenance schema_version must equal 1")
    if provenance.get("platform_status") != {
        "shared_linux": "verified-local",
        "macos": "blocked-macos",
    }:
        failures.append("provenance must retain blocked macOS status")
    components = provenance.get("components", [])
    ids = [item.get("component_id") for item in components if isinstance(item, dict)]
    if len(ids) != len(set(ids)) or len(ids) != len(components):
        failures.append("provenance component ids must be unique and present")
    for component in components:
        component_id = component.get("component_id", "unknown")
        if not isinstance(component.get("license"), str) or not component["license"]:
            failures.append(f"component missing license disposition: {component_id}")
        if not component.get("hashes"):
            failures.append(f"component missing hash: {component_id}")
        if component.get("ecosystem") == "npm":
            source = component.get("source", {}).get("url", "")
            if not source.startswith("https://registry.npmjs.org/"):
                failures.append(f"npm component has unapproved source: {component_id}")
            if not component.get("integrity"):
                failures.append(f"npm component missing lock integrity: {component_id}")
        if component.get("ecosystem") == "cargo" and component.get("source", {}).get("type") == "registry":
            source = component.get("source", {}).get("url", "")
            if not source.startswith("https://crates.io/crates/"):
                failures.append(f"Cargo component has unapproved source: {component_id}")
            if not str(component.get("integrity", "")).startswith("sha256:"):
                failures.append(f"Cargo component missing lock integrity: {component_id}")

    if bom.get("bomFormat") != "CycloneDX" or bom.get("specVersion") != "1.6":
        failures.append("SBOM must use CycloneDX 1.6")
    if not str(bom.get("serialNumber", "")).startswith("urn:uuid:"):
        failures.append("SBOM serial number must be a deterministic UUID URN")
    bom_ids = [item.get("bom-ref") for item in bom.get("components", [])]
    if set(bom_ids) != set(ids) or len(bom_ids) != len(ids):
        failures.append("SBOM component closure must equal provenance component closure")
    properties = bom.get("metadata", {}).get("properties", [])
    if {item.get("name"): item.get("value") for item in properties}.get(
        "agentmage:macos-status"
    ) != "blocked-macos":
        failures.append("SBOM must retain blocked macOS status")

    expected_provenance, expected_bom, expected_hash_text = build_documents(root)
    if provenance != expected_provenance:
        failures.append("dependency provenance is stale or non-deterministic")
    if bom != expected_bom:
        failures.append("CycloneDX SBOM is stale or non-deterministic")
    if hash_text != expected_hash_text:
        failures.append("dependency hash manifest is stale or incorrect")
    return failures


def write_outputs(root: Path = ROOT) -> None:
    provenance, bom, hash_text = build_documents(root)
    output = root / "supply-chain"
    output.mkdir(parents=True, exist_ok=True)
    (output / "dependency-provenance.json").write_text(
        canonical_json(provenance), encoding="utf-8"
    )
    (output / "sbom.cdx.json").write_text(canonical_json(bom), encoding="utf-8")
    (output / "dependency-hashes.sha256").write_text(hash_text, encoding="utf-8")


def check_outputs(root: Path = ROOT) -> list[str]:
    try:
        provenance = read_json(root / "supply-chain/dependency-provenance.json")
        bom = read_json(root / "supply-chain/sbom.cdx.json")
        hash_text = (root / "supply-chain/dependency-hashes.sha256").read_text(
            encoding="utf-8"
        )
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read supply-chain output: {error}"]
    return validate_documents(provenance, bom, hash_text, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_outputs()
        failures = check_outputs()
    except (OSError, ValueError, KeyError, tomllib.TOMLDecodeError) as error:
        print(f"supply-chain validation failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"supply-chain validation failed: {failure}", file=sys.stderr)
        return 1
    print("supply-chain artifacts validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
