#!/usr/bin/env python3
"""Closed opt-in research worker inventories; never network or runtime admission.

This developer check retains the complete manifest/lock and SBOM boundaries. Cargo's
normal/build graph is inspected separately from dev/test feature unification. Nothing
in this module adds a global exemption to the strict-local source or package audit.
"""

from __future__ import annotations

import hashlib
import json
import re
import subprocess
import tomllib
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
POLICY_PATH = Path("security/public-research-dependency-policy.json")
MANIFEST_PATH = "platforms/linux/Cargo.toml"
FEATURE = "public-research-worker"
WORKER_NAME = "agentmage-public-research-worker"
WORKER_PATH = "src/bin/agentmage-public-research-worker.rs"
WORKER_SOURCE = "platforms/linux/" + WORKER_PATH
WORKER_MODULES = ("public_research_target", "public_research_transport")
TARGET = "x86_64-unknown-linux-gnu"
DEPENDENCIES = {
    "ureq": {
        "version": "=3.4.2", "default-features": False,
        "features": ["rustls"], "optional": True,
    },
    "url": {
        "version": "=2.5.8", "default-features": False,
        "features": ["std"], "optional": True,
    },
}
KEYS = {"schema_version", "target", "feature", "lock_sha256", "default", "connected"}
IDENTITY = re.compile(r"[A-Za-z0-9_-]+@[0-9]+\.[0-9]+\.[0-9]+(?:[-+][A-Za-z0-9.+-]+)?")
FEATURE_NAME = re.compile(r"[A-Za-z0-9_+.-]+")


class ResearchClosureError(ValueError):
    """A missing, ambiguous or drifted production closure is not admissible."""


def normalize_tree(text: str, root: Path) -> list[dict[str, Any]]:
    """Retain all package/feature contexts, removing only exact local source paths.

    Repeated normal/build contexts may have different features: do not union them.
    Unknown Git/path sources, dedup markers, new output syntax or malformed features
    fail closed. Registry identities remain fully covered by the locked Cargo input.
    """
    contexts: set[tuple[str, tuple[str, ...]]] = set()
    members = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["members"]
    local = {}
    for member in members:
        package = tomllib.loads((root / member / "Cargo.toml").read_text())["package"]
        local[package["name"]] = str((root / member).resolve())
    if len(text.encode()) > 8 * 1024 * 1024:
        raise ResearchClosureError("Cargo graph exceeds the diagnostic bound")
    for line in text.splitlines():
        if not line:
            continue
        match = re.fullmatch(r"([A-Za-z0-9_-]+) v([^ |]+)(.*?)\|\|(.*)", line)
        if match is None:
            raise ResearchClosureError("unrecognized Cargo production graph line")
        name, version, source, feature_text = match.groups()
        identity = f"{name}@{version}"
        if not IDENTITY.fullmatch(identity):
            raise ResearchClosureError("invalid Cargo package identity")
        if source.endswith(" (proc-macro)"):
            source = source.removesuffix(" (proc-macro)")
        expected_source = f" ({local[name]})" if name in local else ""
        if source != expected_source:
            raise ResearchClosureError("unreviewed Cargo source or graph annotation")
        features = feature_text.split(",") if feature_text else []
        if features != sorted(set(features)) or any(
            not FEATURE_NAME.fullmatch(feature) for feature in features
        ):
            raise ResearchClosureError("invalid Cargo feature context")
        contexts.add((identity, tuple(features)))
    if not contexts:
        raise ResearchClosureError("empty Cargo production graph")
    return [{"package": package, "features": list(features)}
            for package, features in sorted(contexts)]


def observe(root: Path, connected: bool) -> list[dict[str, Any]]:
    """Resolve the actual offline, locked normal/build graph without compiling."""
    command = [
        "cargo", "tree", "--locked", "--offline", "--workspace", "--target", TARGET,
        "--edges", "normal,build", "--prefix", "none", "--format", "{p}||{f}",
        "--no-dedupe", "--color", "never", "--charset", "ascii",
    ]
    if connected:
        command += ["--features", f"agentmage-platform-linux/{FEATURE}"]
    try:
        result = subprocess.run(command, cwd=root, capture_output=True, text=True,
                                check=False, timeout=60)
    except (OSError, subprocess.TimeoutExpired) as error:
        raise ResearchClosureError("Cargo production graph unavailable") from error
    if result.returncode != 0:
        # Cargo errors may contain paths or credential-bearing registry URLs.
        raise ResearchClosureError("Cargo production graph resolution failed")
    return normalize_tree(result.stdout, root)


def validate_topology(manifest: dict[str, Any]) -> list[str]:
    """Admit only the exact optional dependency feature and required-feature binary."""
    failures = []
    features = manifest.get("features", {})
    if (not isinstance(features, dict)
            or any(not isinstance(enabled, list)
                   or any(not isinstance(item, str) for item in enabled)
                   for enabled in features.values())
            or features.get(FEATURE) != ["dep:ureq", "dep:url"]):
        failures.append("research worker feature topology changed")
    elif any(
        item in {FEATURE, "dep:ureq", "dep:url", "ureq", "url"}
        or item.startswith(("ureq/", "ureq?/", "url/", "url?/"))
        for name, enabled in features.items() if name != FEATURE
        for item in enabled
    ):
        failures.append("research dependencies activated outside the worker feature")
    dependencies = manifest.get("dependencies", {})
    if any(dependencies.get(name) != expected for name, expected in DEPENDENCIES.items()):
        failures.append("research worker dependency declarations changed")
    # A build/dev/target table cannot introduce another path to the transport crates.
    extra_tables = [manifest.get(key, {}) for key in ("build-dependencies", "dev-dependencies")]
    for target in manifest.get("target", {}).values():
        extra_tables.extend(target.get(key, {}) for key in
                            ("dependencies", "build-dependencies", "dev-dependencies"))
    if any(set(table) & set(DEPENDENCIES) for table in extra_tables):
        failures.append("research dependency duplicated outside its optional declaration")
    workers = [binary for binary in manifest.get("bin", [])
               if binary.get("name") == WORKER_NAME or binary.get("path") == WORKER_PATH]
    if workers != [{"name": WORKER_NAME, "path": WORKER_PATH,
                    "required-features": [FEATURE]}]:
        failures.append("research binary required-feature boundary changed")
    return failures


def validate_policy(policy: Any) -> None:
    """Reject partial, duplicate, malformed or unversioned graph inventories."""
    if (not isinstance(policy, dict) or set(policy) != KEYS
            or type(policy["schema_version"]) is not int
            or policy["schema_version"] != 1 or policy["target"] != TARGET
            or policy["feature"] != FEATURE
            or not isinstance(policy["lock_sha256"], str)
            or not re.fullmatch(r"[0-9a-f]{64}", policy["lock_sha256"])):
        raise ResearchClosureError("research dependency policy identity is invalid")
    for name in ("default", "connected"):
        contexts = policy[name]
        if not isinstance(contexts, list) or not contexts:
            raise ResearchClosureError("research dependency inventory is empty")
        keys = []
        for record in contexts:
            if (not isinstance(record, dict) or set(record) != {"package", "features"}
                    or not isinstance(record["package"], str)
                    or not IDENTITY.fullmatch(record["package"])
                    or not isinstance(record["features"], list)
                    or any(not isinstance(f, str) or not FEATURE_NAME.fullmatch(f)
                           for f in record["features"])
                    or record["features"] != sorted(set(record["features"]))):
                raise ResearchClosureError("research dependency inventory is malformed")
            keys.append((record["package"], tuple(record["features"])))
        if keys != sorted(set(keys)):
            raise ResearchClosureError("research dependency contexts are duplicated or unsorted")


def validate_worker_sources(sources: dict[str, str]) -> list[str]:
    """The raw modules belong to the explicit binary, not the platform library."""
    failures = []
    worker = sources.get(WORKER_SOURCE, "")
    for module in WORKER_MODULES:
        if worker.count(f'#[path = "../{module}.rs"]') != 1:
            failures.append("research worker module inclusion changed")
        for path, source in sources.items():
            if path != WORKER_SOURCE and path.endswith(".rs") and module in source:
                failures.append(f"research worker module referenced outside its binary: {path}")
    return sorted(set(failures))


def check(
    root: Path = ROOT, *, policy: Any = None,
    observations: dict[str, list[dict[str, Any]]] | None = None,
) -> list[str]:
    """Check both exact closures; this never edits inventories or admits effects."""
    if policy is None:
        policy = json.loads((root / POLICY_PATH).read_text())
    validate_policy(policy)
    failures = validate_topology(tomllib.loads((root / MANIFEST_PATH).read_text()))
    if hashlib.sha256((root / "Cargo.lock").read_bytes()).hexdigest() != policy["lock_sha256"]:
        failures.append("research dependency complete lockfile binding changed")
    if observations is None:
        observations = {"default": observe(root, False), "connected": observe(root, True)}
    if set(observations) != {"default", "connected"}:
        return [*failures, "research dependency observations are incomplete"]
    for name in ("default", "connected"):
        if observations[name] != policy[name]:
            failures.append(f"research {name} production closure changed")
    default_packages = {entry["package"].split("@", 1)[0] for entry in policy["default"]}
    if default_packages & set(DEPENDENCIES) or any(
        FEATURE in entry["features"] for entry in policy["default"]
    ):
        failures.append("default production graph contains the research worker")
    connected_packages = {entry["package"] for entry in policy["connected"]}
    if not {"ureq@3.4.2", "url@2.5.8"} <= connected_packages:
        failures.append("connected production graph omits exact research dependencies")
    # Full lock/manifests and strict source scans remain independently mandatory.
    return failures
