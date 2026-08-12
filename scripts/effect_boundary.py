#!/usr/bin/env python3
"""Validate the structural kernel-to-effect mediation boundary."""

from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]

ENGINE = Path("kernel/engine/src/authority_transaction.rs")
OPERATIONAL_STORE = Path("kernel/engine/src/operational_store.rs")
CONFIGURATION = Path("kernel/engine/src/configuration.rs")
LINUX_LIB = Path("platforms/linux/src/lib.rs")
LINUX_CONFIGURATION = Path("platforms/linux/src/configuration_store.rs")
LINUX_SANDBOX = Path("platforms/linux/src/sandbox.rs")
LINUX_SECRETS = Path("platforms/linux/src/secret_service.rs")
LINUX_IPC = Path("platforms/linux/src/ipc.rs")

PERMIT_USERS = {
    ENGINE,
    LINUX_CONFIGURATION,
    LINUX_SANDBOX,
    LINUX_SECRETS,
}

MANIFEST_MODULES = {
    "kernel-engine": Path("kernel/engine/Cargo.toml"),
    "platform-linux": Path("platforms/linux/Cargo.toml"),
    "capability-read-only": Path("capabilities/read-only/Cargo.toml"),
    "shell-host": Path("shells/host/Cargo.toml"),
}

PACKAGE_MODULES = {
    "agentmage-kernel-contracts": "kernel-contracts",
    "agentmage-kernel-engine": "kernel-engine",
    "agentmage-platform-linux": "platform-linux",
    "agentmage-capability-read-only": "capability-read-only",
}

EXPECTED_INTERNAL_IMPORTS = {
    "kernel-engine": {"kernel-contracts"},
    "platform-linux": {"kernel-contracts", "kernel-engine"},
    "capability-read-only": {"kernel-contracts"},
    "shell-host": {
        "capability-read-only",
        "kernel-contracts",
        "kernel-engine",
        "platform-linux",
    },
}


def _read(
    relative: Path,
    root: Path,
    overrides: dict[Path, str],
) -> str:
    return overrides.get(relative, (root / relative).read_text(encoding="utf-8"))


def _manifest_imports(text: str) -> set[str]:
    manifest = tomllib.loads(text)
    packages: set[str] = set()
    for section in ("dependencies", "dev-dependencies", "build-dependencies"):
        values = manifest.get(section, {})
        if isinstance(values, dict):
            packages.update(values)
    for target in manifest.get("target", {}).values():
        if not isinstance(target, dict):
            continue
        for section in ("dependencies", "dev-dependencies", "build-dependencies"):
            values = target.get(section, {})
            if isinstance(values, dict):
                packages.update(values)
    return {
        PACKAGE_MODULES[package]
        for package in packages
        if package in PACKAGE_MODULES
    }


def _public_function(source: str, name: str) -> bool:
    return re.search(rf"(?m)^\s*pub\s+(?:const\s+)?fn\s+{re.escape(name)}\s*\(", source) is not None


def validate_effect_boundary(
    root: Path = ROOT,
    overrides: dict[Path, str] | None = None,
) -> list[str]:
    """Return every structural effect-boundary violation."""

    replacements = overrides or {}
    failures: list[str] = []
    engine = _read(ENGINE, root, replacements)
    operational_store = _read(OPERATIONAL_STORE, root, replacements)
    configuration = _read(CONFIGURATION, root, replacements)
    linux_lib = _read(LINUX_LIB, root, replacements)
    linux_configuration = _read(LINUX_CONFIGURATION, root, replacements)
    linux_sandbox = _read(LINUX_SANDBOX, root, replacements)
    linux_secrets = _read(LINUX_SECRETS, root, replacements)
    linux_ipc = _read(LINUX_IPC, root, replacements)

    if "pub struct EffectAuthorization<'transaction>" not in engine:
        failures.append("kernel effect authorization type is missing")
    permit_match = re.search(
        r"pub struct EffectAuthorization<'transaction>\s*\{(?P<body>.*?)\n\}",
        engine,
        re.DOTALL,
    )
    if permit_match is None or re.search(r"(?m)^\s*pub\s+", permit_match.group("body")):
        failures.append("effect authorization fields must remain private")
    permit_prefix = engine[max(0, engine.find("pub struct EffectAuthorization") - 160) :]
    permit_prefix = permit_prefix[: permit_prefix.find("pub struct EffectAuthorization")]
    if re.search(r"derive\([^)]*\b(Clone|Copy|Serialize|Deserialize)\b", permit_prefix):
        failures.append("effect authorization must not be duplicable or serializable")
    if re.search(r"impl\s+(Clone|Copy|serde::Serialize|serde::Deserialize).*EffectAuthorization", engine):
        failures.append("effect authorization has a prohibited trait implementation")
    if re.search(r"EffectAuthorization[^\n]*::new\s*\(", engine.replace("let _ = EffectAuthorization::new();", "")):
        failures.append("effect authorization exposes a constructor")
    if not re.search(
        r"fn\s+execute\s*\(\s*&mut\s+self,\s*authorization:\s*EffectAuthorization<'_>\s*\)",
        engine,
    ):
        failures.append("effect driver must consume one authorization by value")
    if "pub fn execute_effect<D: EffectDriver>" not in operational_store:
        failures.append("durable authority mediated entry point is missing")
    if re.search(
        r"(?m)^\s*pub\s+fn\s+execute_effect\s*<D:\s*EffectDriver>", engine
    ):
        failures.append("in-memory authority coordinator exposes an effect entry point")

    if re.search(r"\bstd::fs\b|\bstd::path\b", configuration):
        failures.append("kernel configuration contains a native filesystem dependency")
    for name in ("migrate_v0", "rollback_migration", "apply", "rollback", "publish"):
        if _public_function(linux_configuration, name):
            failures.append(f"raw Linux configuration effect is public: {name}")
    if _public_function(linux_sandbox, "run"):
        failures.append("raw Linux sandbox execution is public")
    for name in ("probe", "store", "lookup", "clear"):
        if _public_function(linux_secrets, name):
            failures.append(f"raw Linux Secret Service effect is public: {name}")
    if "PrivateUnixListener" in linux_lib:
        failures.append("private Unix listener is exported from the Linux adapter")
    if re.search(
        r"(?m)^\s*pub(?:\([^)]*\))?\s+struct\s+PrivateUnixListener", linux_ipc
    ):
        failures.append("private Unix listener exceeds module visibility")

    for required in (
        "impl EffectDriver for LinuxConfigurationEffectDriver",
        "impl EffectDriver for LinuxSandboxEffectDriver",
        "impl EffectDriver for LinuxSecretEffectDriver",
    ):
        if required not in "\n".join((
            linux_configuration,
            linux_sandbox,
            linux_secrets,
        )):
            failures.append(f"missing mediated driver: {required}")

    rust_sources = sorted(
        path.relative_to(root)
        for base in (root / "kernel", root / "platforms", root / "capabilities", root / "shells")
        for path in base.rglob("*.rs")
    )
    for relative in rust_sources:
        source = _read(relative, root, replacements)
        if "EffectAuthorization" in source and relative not in PERMIT_USERS:
            failures.append(f"unregistered effect-authorization consumer: {relative}")

    for base in (Path("shells"), Path("capabilities")):
        for path in sorted((root / base).rglob("*.rs")):
            relative = path.relative_to(root)
            source = _read(relative, root, replacements)
            for pattern, label in (
                (r"\bstd::process\b|\bCommand::new\s*\(", "process launch"),
                (r"\b(?:TcpListener|TcpStream|UnixListener)::", "socket creation"),
                (
                    r"\b(?:std::)?fs::(?:write|remove_file|remove_dir|remove_dir_all|rename|create_dir|create_dir_all)\s*\(",
                    "filesystem mutation",
                ),
            ):
                if re.search(pattern, source):
                    failures.append(f"{relative} contains direct {label}")

    for module, relative in MANIFEST_MODULES.items():
        imports = _manifest_imports(_read(relative, root, replacements))
        if imports != EXPECTED_INTERNAL_IMPORTS[module]:
            failures.append(
                f"{module} internal imports {sorted(imports)} do not match "
                f"the effect boundary {sorted(EXPECTED_INTERNAL_IMPORTS[module])}"
            )

    return failures


def main() -> int:
    try:
        failures = validate_effect_boundary()
    except (OSError, tomllib.TOMLDecodeError) as error:
        print(f"effect boundary validation failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"effect boundary validation failed: {failure}", file=sys.stderr)
        return 1
    print("Structural effect mediation boundary validated.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
