#!/usr/bin/env python3
"""Validate the structural kernel-to-effect mediation boundary."""

from __future__ import annotations

import re
import sys
import tomllib
from collections import Counter
from pathlib import Path

try:
    from scripts.rust_source_audit import production_source as _production_source
except ModuleNotFoundError:
    from rust_source_audit import production_source as _production_source

ROOT = Path(__file__).resolve().parents[1]

ENGINE = Path("kernel/engine/src/authority_transaction.rs")
COMMAND_RUNNER = Path("kernel/engine/src/command_runner.rs")
REPOSITORY_SAFETY = Path("kernel/engine/src/repository_safety.rs")
REPOSITORY_INSPECTION = Path("kernel/engine/src/repository_inspection.rs")
RESEARCH_EFFECT_BINDING = Path("kernel/engine/src/research_effect_binding.rs")
RESEARCH_DISPATCH = Path("kernel/engine/src/research_dispatch.rs")
LOCAL_COMMIT = Path("kernel/engine/src/local_commit.rs")
OPERATIONAL_STORE = Path("kernel/engine/src/operational_store.rs")
CONFIGURATION = Path("kernel/engine/src/configuration.rs")
LINUX_LIB = Path("platforms/linux/src/lib.rs")
LINUX_CONFIGURATION = Path("platforms/linux/src/configuration_store.rs")
LINUX_SANDBOX = Path("platforms/linux/src/sandbox.rs")
LINUX_RESEARCH = Path("platforms/linux/src/research_sandbox.rs")
LINUX_SANDBOX_SUPERVISION = Path("platforms/linux/src/sandbox_supervision.rs")
LINUX_SANDBOX_ACCOUNTING = Path("platforms/linux/src/sandbox_supervision/resource_usage.rs")
LINUX_SUPERVISION_USERS = {LINUX_SANDBOX, LINUX_SANDBOX_SUPERVISION, LINUX_RESEARCH,
                         Path("platforms/linux/src/command_runner.rs"),
                         Path("platforms/linux/src/repository_safety.rs")}

# Exact internal interface for the closed native owners, not a public
# process API. New entries require a deliberate boundary review and regressions.
SUPERVISION_DECLARATIONS = (
    "pub(crate) enum LaunchOwner {", "pub(crate) struct Launch {",
    "pub owner: LaunchOwner,", "pub projections: Vec<OwnedFd>,", "pub unit: String,",
    "pub deadline: Instant,", "pub stdout_limit: usize,", "pub stderr_limit: usize,",
    "pub filter_pipe: Option<FilterPipe>,", "pub(crate) struct FilterPipe {",
    "pub(crate) fn new() -> Result<Self, LinuxSandboxError> {",
    "pub(crate) fn read_descriptor(&self) -> RawFd {",
    "pub(crate) struct CapturedStream {", "pub retained: Vec<u8>,",
    "pub sha256: [u8; 32],", "pub total: usize,", "pub(crate) struct SupervisedResult {",
    "pub outcome: Result<OperationOutcome, LinuxSandboxError>,",
    "pub status: Option<ExitStatus>,", "pub stdout: CapturedStream,", "pub stderr: CapturedStream,",
    "pub resources: Option<agentmage_kernel_engine::command_runner::CommandResourceUsage>,",
    "pub output_complete: bool,",
    "pub(crate) fn admission_snapshot() -> Result<(), LinuxSandboxError> {",
    "pub(super) fn run(", "pub(crate) fn run_owned(",
)
ACCOUNTING_DECLARATIONS = (
    "pub(super) fn sample(directory: &OwnedFd) -> Option<CommandResourceUsage> {",
    "pub(super) struct Observation {",
    "pub(super) fn merge(&mut self, sample: Option<CommandResourceUsage>) {",
    "pub(super) fn finish(self) -> Option<CommandResourceUsage> {",
)
LINUX_SECRETS = Path("platforms/linux/src/secret_service.rs")
LINUX_IPC = Path("platforms/linux/src/ipc.rs")

PERMIT_USERS = {
    ENGINE,
    COMMAND_RUNNER,
    REPOSITORY_INSPECTION,
    RESEARCH_EFFECT_BINDING,
    RESEARCH_DISPATCH,
    REPOSITORY_SAFETY,
    LOCAL_COMMIT,
    LINUX_CONFIGURATION,
    LINUX_SANDBOX,
    LINUX_RESEARCH,
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

TEST_ONLY_BINARIES = {
    Path("capabilities/read-only/src/bin/agentmage-read-only-lifecycle-fixture.rs"): (
        Path("capabilities/read-only/Cargo.toml"),
        "agentmage-read-only-lifecycle-fixture",
        "lifecycle-fixture",
    ),
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


def _verified_test_only_binaries(
    root: Path,
    overrides: dict[Path, str],
) -> set[Path]:
    """Return exact binary paths whose required test feature remains sealed."""

    verified: set[Path] = set()
    for source_path, (manifest_path, binary_name, feature_name) in TEST_ONLY_BINARIES.items():
        manifest = tomllib.loads(_read(manifest_path, root, overrides))
        features = manifest.get("features", {})
        targets = manifest.get("bin", [])
        if not isinstance(features, dict) or feature_name not in features:
            continue
        if not isinstance(targets, list):
            continue
        for target in targets:
            if not isinstance(target, dict):
                continue
            if (
                target.get("name") == binary_name
                and target.get("path") == source_path.relative_to(manifest_path.parent).as_posix()
                and target.get("required-features") == [feature_name]
            ):
                verified.add(source_path)
                break
    return verified


def validate_effect_boundary(
    root: Path = ROOT,
    overrides: dict[Path, str] | None = None,
) -> list[str]:
    """Return every structural effect-boundary violation."""

    replacements = overrides or {}
    failures: list[str] = []
    engine = _read(ENGINE, root, replacements)
    operational_store = _read(OPERATIONAL_STORE, root, replacements)
    research_dispatch = _read(RESEARCH_DISPATCH, root, replacements)
    configuration = _read(CONFIGURATION, root, replacements)
    linux_lib = _read(LINUX_LIB, root, replacements)
    linux_configuration = _read(LINUX_CONFIGURATION, root, replacements)
    linux_sandbox = _read(LINUX_SANDBOX, root, replacements)
    linux_research = _production_source(_read(LINUX_RESEARCH, root, replacements))
    linux_secrets = _read(LINUX_SECRETS, root, replacements)
    linux_ipc = _read(LINUX_IPC, root, replacements)
    test_only_binaries = _verified_test_only_binaries(root, replacements)

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
    research_proof = re.search(
        r"pub struct ResearchDispatch<'a>\s*\{(?P<body>.*?)\n\}",
        research_dispatch,
        re.DOTALL,
    )
    if research_proof is None or re.search(r"(?m)^\s*pub(?:\([^)]*\))?\s+", research_proof.group("body")):
        failures.append("research dispatch proof fields must remain private")
    if research_proof is not None:
        prefix = research_dispatch[max(0, research_proof.start() - 160):research_proof.start()]
        if re.search(r"derive\([^)]*\b(Clone|Copy|Serialize|Deserialize)\b", prefix):
            failures.append("research dispatch proof must not be duplicable or serializable")
    if re.search(r"impl\s+(Clone|Copy|serde::Serialize|serde::Deserialize).*ResearchDispatch", research_dispatch):
        failures.append("research dispatch proof has a prohibited trait implementation")
    if re.search(r"(?m)^pub\s+struct\s+(FreshResearchDispatch|ResearchDispatchAdapter)\b", research_dispatch):
        failures.append("research dispatch owner material exceeds crate visibility")
    if re.search(
        r"\bpub\s+(?:const\s+)?fn\s+\w+\s*(?:<[^{};]*>)?\s*\([^{};]*\)\s*->[^{};]*(?:ResearchDispatch|FreshResearchDispatch)",
        research_dispatch,
    ):
        failures.append("research dispatch proof exposes a public constructor")
    if not re.search(r"impl\s*<D:\s*ResearchEffectDriver>\s+EffectDriver\s+for\s+ResearchDispatchAdapter", research_dispatch):
        failures.append("research dispatch must bridge the existing effect coordinator")
    if re.search(r"(?m)^\s*pub(?:\([^)]*\))?\s+fn\s+begin_effect_with_preflight\s*(?:<|\()", operational_store):
        failures.append("research preflight callback must remain owner-private")
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
    if re.search(r"(?m)^\s*pub(?:\([^)]*\))?\s+fn\s+run\s*\(", linux_research):
        failures.append("raw Linux research execution must remain module-private")
    if ('#[cfg(feature = "public-research-worker")]\n#[path = "research_sandbox.rs"]\npub(crate) mod research;'
            not in linux_sandbox):
        failures.append("Linux research adapter must remain feature-gated and crate-internal")
    if ('#[cfg(feature = "public-research-worker")]\npub use sandbox::research::{' not in linux_lib):
        failures.append("Linux research exports must remain feature-gated")
    if re.search(r"impl\s+(?:[^\s]+::)?EffectDriver\s+for\s+LinuxPublicResearchEffectDriver", linux_research):
        failures.append("Linux research driver cannot bypass the dual-proof interface")
    if not re.search(r"fn\s+execute_research\s*\(\s*&mut\s+self,\s*authorization:\s*EffectAuthorization<'_>,\s*dispatch:\s*ResearchDispatch<'_>,?\s*\)", linux_research):
        failures.append("Linux research driver must consume both existing proofs")
    supervision = _production_source(_read(LINUX_SANDBOX_SUPERVISION, root, replacements))
    accounting = _production_source(_read(LINUX_SANDBOX_ACCOUNTING, root, replacements))
    for source, declarations, label in (
        (supervision, SUPERVISION_DECLARATIONS, "supervisor"),
        (accounting, ACCOUNTING_DECLARATIONS, "accounting"),
    ):
        public_lines = [line.strip() for line in source.splitlines()
                        if re.match(r"\s*pub(?:\([^)]*\))?\s+", line)]
        if Counter(public_lines) != Counter(declarations):
            failures.append(f"Linux sandbox {label} exceeds its closed internal boundary")
    owners = re.search(r"pub\(crate\) enum LaunchOwner\s*\{([^}]+)\}", supervision)
    if owners is None or " ".join(owners[1].split()) != (
        "Read(Arc<LinuxSandboxManifest>), "
        "Command(Arc<crate::command_runner::LinuxCommandManifest>), "
        "Git(Arc<crate::repository_safety::LinuxRepositoryInspectionManifest>),"
        ' #[cfg(feature = "public-research-worker")] '
        "Research(Arc<super::research::LinuxPublicResearchManifest>),"
    ):
        failures.append("Linux sandbox supervisor owner set is not closed")
    if supervision.count("static OWNED_ATTEMPT: Mutex<Option<Attempt>>") != 1:
        failures.append("Linux sandbox supervisor must retain its single attempt owner")
    if re.findall(r"(?m)^\s*pub(?:\([^)]*\))?\s+mod\s+supervision\s*;", linux_sandbox) != ["pub(crate) mod supervision;"]:
        failures.append("Linux sandbox supervisor module must remain crate-internal")
    if re.search(r"\b(?:Command|TcpStream|UnixStream)::|\bstd::process\b|\bthread::spawn\b", accounting):
        failures.append("Linux sandbox accounting cannot own processes or sockets")
    for path, source in ((LINUX_SANDBOX, _production_source(linux_sandbox)), (LINUX_SANDBOX_SUPERVISION, supervision), (LINUX_SANDBOX_ACCOUNTING, accounting), (LINUX_RESEARCH, linux_research)):
        # Thread joins take no arguments. Path and string joins are ordinary
        # bounded data operations; they must not be mistaken for process waits.
        blocking_wait = re.search(r"\.(?:wait|wait_with_output|status|output)\s*\(", source)
        thread_join = re.search(r"\.join\s*\((?:\s|/\*.*?\*/|//[^\n]*\n)*\)", source, re.DOTALL)
        if blocking_wait or thread_join:
            failures.append(f"unbounded blocking worker/control wait reintroduced: {path}")
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
        production = _production_source(source)
        if (re.search(r"\bsupervision\s*(?:::|[;{])|\bsandbox_supervision\b", production)
                and relative not in LINUX_SUPERVISION_USERS):
            failures.append(f"unregistered native supervisor consumer: {relative}")

    for base in (Path("shells"), Path("capabilities")):
        for path in sorted((root / base).rglob("*.rs")):
            relative = path.relative_to(root)
            if relative in test_only_binaries:
                continue
            source = _production_source(_read(relative, root, replacements))
            for pattern, label in (
                (
                    r"\bstd::process::Command\b|\bCommand::new\s*\(",
                    "process launch",
                ),
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
