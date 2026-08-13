#!/usr/bin/env python3
"""Rebuild and execute AgentMage candidate package lifecycle gates."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Final

try:
    from scripts.package_candidate import (
        MANIFEST_PATH,
        PAYLOAD_FILES,
        ROOT,
        build_all,
        canonical_json,
    )
except ModuleNotFoundError:
    from package_candidate import (  # type: ignore[no-redef]
        MANIFEST_PATH,
        PAYLOAD_FILES,
        ROOT,
        build_all,
        canonical_json,
    )


STANDARD_USER: Final = "10001:10001"
ADMIN_USER: Final = "0:0"
CONTAINER_PACKAGE_ROOT: Final = "/packages"
SHA256_ID = re.compile(r"^(?:sha256:)?[0-9a-f]{64}$")
IMMUTABLE_IMAGE_REFERENCE = re.compile(r"^.+@sha256:[0-9a-f]{64}$")
CONTAINER_ID = re.compile(r"^[0-9a-f]{64}$")
EXPECTED_INFERENCE_DESCRIPTOR: Final = {
    "accepted_operation": "self-check-only",
    "authority_inputs": [],
    "component_id": "platform-linux-native-inference",
    "contract": "authenticated-local-endpoint-v1",
    "docker_compatibility_available": False,
    "docker_guard_profile_sha256": "88fb0d5a78829cbdfc34af5cbcbfe3ca2a80f550889947e6478fdb66bf7edb2a",
    "docker_model_artifact_digest": "sha256:08fa7b1d44f255be48cfc12359211725bfd659742612ed4b221cd5be90d14444",
    "docker_model_runner_image_digest": "sha256:bd94095bbc1ddc4266c3a88f582a92562c6b63eceb175572c9a60045663727c9",
    "docker_runtime_profile_sha256": "ab8cde6bc1440f8a0013390aa2e291a315cdebcfe44aa1d339f0aa0b1d70899c",
    "enabled_models": 0,
    "inference_available": False,
    "native_runtime_package": "agentmage-llama-cpp-b10333-cpu-linux-x86_64",
    "native_runtime_profile_sha256": "21346c06fb86b418706326b186609f8e1f690d6b57b53e73d02b4ad8e28e53ea",
    "network_listener": False,
    "process_boundary_version": 4,
}
INSTALLED_FILES: Final = tuple(f"/{path.as_posix()}" for path in PAYLOAD_FILES) + (
    f"/{MANIFEST_PATH.as_posix()}",
)
INSTALLED_DIRECTORIES: Final = (
    "/usr/libexec/agentmage",
    "/usr/share/agentmage",
    "/usr/share/licenses/agentmage",
)
EXPECTED_MODES: Final = {
    f"/{path.as_posix()}": ("755" if index < 2 else "644")
    for index, path in enumerate(PAYLOAD_FILES)
} | {f"/{MANIFEST_PATH.as_posix()}": "644"}
VERIFY_STATE_SUFFIXES: Final = (
    "package-version",
    "component-manifest",
    "file-authority",
    "host-launch",
    "inference-boundary-launch",
)
EXPECTED_STEP_IDS: Final = (
    "platform-identity",
    "standard-user-identity",
    "install-0.0.0",
    *(f"initial-{suffix}" for suffix in VERIFY_STATE_SUFFIXES),
    "corrupt-upgrade-refusal",
    *(f"post-refusal-{suffix}" for suffix in VERIFY_STATE_SUFFIXES),
    "upgrade-0.0.1",
    *(f"upgraded-{suffix}" for suffix in VERIFY_STATE_SUFFIXES),
    "rollback-0.0.0",
    *(f"rolled-back-{suffix}" for suffix in VERIFY_STATE_SUFFIXES),
    "uninstall-after-rollback",
    "first-uninstall-package-record-absent",
    "first-uninstall-filesystem-residue-absent",
    "reinstall-0.0.1-after-clean-removal",
    *(f"recovered-{suffix}" for suffix in VERIFY_STATE_SUFFIXES),
    "final-uninstall",
    "final-uninstall-package-record-absent",
    "final-uninstall-filesystem-residue-absent",
)
ADMIN_STEP_IDS: Final = {
    "install-0.0.0",
    "corrupt-upgrade-refusal",
    "upgrade-0.0.1",
    "rollback-0.0.0",
    "uninstall-after-rollback",
    "reinstall-0.0.1-after-clean-removal",
    "final-uninstall",
}
NONZERO_STEP_IDS: Final = {
    "corrupt-upgrade-refusal",
    "first-uninstall-package-record-absent",
    "final-uninstall-package-record-absent",
}


@dataclass(frozen=True)
class ContainerTarget:
    """A closed package-manager and operating-system lifecycle target."""

    platform_id: str
    image: str
    os_id: str
    version_id: str
    package_format: str


@dataclass(frozen=True)
class LifecycleStep:
    """A single container lifecycle operation with an explicit actor."""

    step_id: str
    actor: str
    argv: tuple[str, ...]
    expect_success: bool = True
    expected_stdout: str | None = None


class PackageLifecycleError(ValueError):
    """Raised when a lifecycle gate fails."""


def run_command(
    arguments: list[str],
    cwd: Path | None = None,
    expect_success: bool = True,
    timeout: int = 900,
) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        arguments,
        cwd=cwd,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=timeout,
        check=False,
    )
    if (result.returncode == 0) != expect_success:
        raise PackageLifecycleError(
            f"package.lifecycle.command_result:{Path(arguments[0]).name}:"
            f"exit={result.returncode}:expected_success={str(expect_success).lower()}"
        )
    return result


def command(
    arguments: list[str],
    cwd: Path | None = None,
    expect_success: bool = True,
    timeout: int = 900,
) -> None:
    run_command(arguments, cwd, expect_success, timeout)


def require_commands(names: tuple[str, ...]) -> None:
    if any(shutil.which(name) is None for name in names):
        raise PackageLifecycleError("package.lifecycle.tool_unavailable")


def extract_rpm(package: Path, root: Path) -> None:
    first = subprocess.Popen(
        ["rpm2cpio", str(package)], stdout=subprocess.PIPE, stderr=subprocess.PIPE
    )
    assert first.stdout is not None
    second = subprocess.run(
        ["cpio", "-idm", "--quiet"],
        cwd=root,
        stdin=first.stdout,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    first.stdout.close()
    _, first_error = first.communicate()
    if first.returncode != 0 or second.returncode != 0:
        raise PackageLifecycleError(
            f"package.lifecycle.rpm_extract:{first_error.decode(errors='replace')}"
        )


def extract_deb(package: Path, root: Path) -> None:
    command(["ar", "x", str(package)], cwd=root)
    command(["tar", "-xzf", "data.tar.gz"], cwd=root)


def verify_extracted_candidates(
    artifacts_v0: dict[str, Path], artifacts_v1: dict[str, Path]
) -> dict[str, str]:
    require_commands(("ar", "cpio", "rpm2cpio", "tar"))
    host = ROOT / "target/release/agentmage-host"
    with tempfile.TemporaryDirectory(prefix="agentmage-lifecycle-") as directory:
        temporary = Path(directory)
        roots = {"rpm": temporary / "rpm", "deb": temporary / "deb"}
        for root in roots.values():
            root.mkdir()
        extract_rpm(artifacts_v0["rpm"], roots["rpm"])
        extract_deb(artifacts_v0["deb"], roots["deb"])
        for root in roots.values():
            command([str(host), "--verify-package-candidate-root", str(root)])
            command(
                [str(host), "--verify-package-root", str(root)], expect_success=False
            )
        rpm_manifest = roots["rpm"] / "usr/share/agentmage/package-manifest.json"
        deb_manifest = roots["deb"] / "usr/share/agentmage/package-manifest.json"
        if rpm_manifest.read_bytes() != deb_manifest.read_bytes():
            raise PackageLifecycleError("package.lifecycle.cross_format_drift")
        vsix = roots["deb"] / "usr/share/agentmage/agentmage.vsix"
        vsix.write_bytes(vsix.read_bytes() + b"mutation")
        command(
            [str(host), "--verify-package-candidate-root", str(roots["deb"])],
            expect_success=False,
        )
        if artifacts_v0["vsix"].read_bytes() == artifacts_v1["vsix"].read_bytes():
            raise PackageLifecycleError("package.lifecycle.version_not_bound")
    return {"extracted_payloads": "pass", "mutation_refusal": "pass"}


def verify_vscode_lifecycle(
    artifacts_v0: dict[str, Path], artifacts_v1: dict[str, Path]
) -> dict[str, str]:
    require_commands(("code",))
    with tempfile.TemporaryDirectory(prefix="agentmage-vsix-lifecycle-") as directory:
        root = Path(directory)
        common = [
            "code",
            "--disable-gpu",
            "--user-data-dir",
            str(root / "user-data"),
            "--extensions-dir",
            str(root / "extensions"),
        ]
        extension = "agentmage-project.agentmage-vscode-shell"
        command([*common, "--install-extension", str(artifacts_v0["vsix"]), "--force"])
        assert_extension(common, f"{extension}@0.0.0")
        command([*common, "--install-extension", str(artifacts_v1["vsix"]), "--force"])
        assert_extension(common, f"{extension}@0.0.1")
        broken = root / "broken.vsix"
        broken.write_bytes(artifacts_v0["vsix"].read_bytes()[:512])
        command([*common, "--install-extension", str(broken), "--force"], expect_success=False)
        assert_extension(common, f"{extension}@0.0.1")
        command([*common, "--install-extension", str(artifacts_v0["vsix"]), "--force"])
        assert_extension(common, f"{extension}@0.0.0")
        command([*common, "--uninstall-extension", extension])
        listing = subprocess.run(
            [*common, "--list-extensions"],
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        ).stdout.splitlines()
        if extension in listing:
            raise PackageLifecycleError("package.lifecycle.vsix_residue")
    return {"vscode_install_upgrade_rollback_uninstall": "pass"}


def assert_extension(common: list[str], expected: str) -> None:
    listing = subprocess.run(
        [*common, "--list-extensions", "--show-versions"],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    ).stdout.splitlines()
    if expected not in listing:
        raise PackageLifecycleError("package.lifecycle.vsix_version")


def expected_managed_paths(package_format: str) -> list[str]:
    if package_format == "rpm":
        return sorted((*INSTALLED_FILES, *INSTALLED_DIRECTORIES))
    if package_format != "deb":
        raise PackageLifecycleError("package.lifecycle.package_format")
    directories: set[str] = set()
    for installed in INSTALLED_FILES:
        parent = Path(installed).parent
        while parent != Path("/"):
            directories.add(parent.as_posix())
            parent = parent.parent
    return sorted(directories | set(INSTALLED_FILES))


def container_create_argv(target: ContainerTarget, package_root: Path) -> list[str]:
    return [
        "podman",
        "create",
        "--pull=never",
        "--network=none",
        f"--user={ADMIN_USER}",
        "--cap-drop=all",
        "--security-opt=no-new-privileges",
        "--security-opt=label=disable",
        "--pids-limit=64",
        "--memory=512m",
        "--tmpfs=/tmp:rw,nosuid,nodev,size=67108864",
        "--mount",
        f"type=bind,src={package_root.resolve()},dst={CONTAINER_PACKAGE_ROOT},ro=true",
        target.image,
        "/usr/bin/sleep",
        "infinity",
    ]


def container_exec_argv(container_id: str, step: LifecycleStep) -> list[str]:
    user = ADMIN_USER if step.actor == "package-administrator" else STANDARD_USER
    return [
        "podman",
        "exec",
        f"--user={user}",
        "--workdir=/tmp",
        "--env=HOME=/tmp",
        "--env=LANG=C",
        "--env=LC_ALL=C",
        "--env=PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
        container_id,
        *step.argv,
    ]


def _step_record(
    container_id: str, step: LifecycleStep
) -> tuple[dict[str, Any], str]:
    if step.actor not in {"package-administrator", "standard-user"}:
        raise PackageLifecycleError("package.lifecycle.actor")
    try:
        result = run_command(
            container_exec_argv(container_id, step),
            expect_success=step.expect_success,
            timeout=120,
        )
    except PackageLifecycleError as error:
        raise PackageLifecycleError(
            f"package.lifecycle.step:{step.step_id}"
        ) from error
    if step.expected_stdout is not None and result.stdout != step.expected_stdout:
        raise PackageLifecycleError(f"package.lifecycle.output:{step.step_id}")
    return (
        {
            "id": step.step_id,
            "actor": step.actor,
            "uid_gid": ADMIN_USER if step.actor == "package-administrator" else STANDARD_USER,
            "expected_exit": "zero" if step.expect_success else "nonzero",
            "observed_exit": "zero" if result.returncode == 0 else "nonzero",
            "output_bytes": len(result.stdout.encode("utf-8")),
            "output_sha256": hashlib.sha256(result.stdout.encode("utf-8")).hexdigest(),
            "status": "pass",
        },
        result.stdout,
    )


def _package_arguments(
    target: ContainerTarget,
    action: str,
    package_path: str | None = None,
) -> tuple[str, ...]:
    if target.package_format == "rpm":
        operations = {
            "install": ("rpm", "-i", "--nosignature", str(package_path)),
            "upgrade": ("rpm", "-U", "--nosignature", str(package_path)),
            "rollback": (
                "rpm",
                "-U",
                "--oldpackage",
                "--nosignature",
                str(package_path),
            ),
            "remove": ("rpm", "-e", "agentmage"),
            "query": ("rpm", "-q", "--qf", "%{VERSION}\n", "agentmage"),
            "list": ("rpm", "-ql", "agentmage"),
            "absent": ("rpm", "-q", "agentmage"),
        }
    elif target.package_format == "deb":
        operations = {
            "install": ("dpkg", "-i", str(package_path)),
            "upgrade": ("dpkg", "-i", str(package_path)),
            "rollback": ("dpkg", "-i", str(package_path)),
            "remove": ("dpkg", "-r", "agentmage"),
            "query": ("dpkg-query", "--show", "--showformat=${Version}\n", "agentmage"),
            "list": ("dpkg", "-L", "agentmage"),
            "absent": ("dpkg", "--status", "agentmage"),
        }
    else:
        raise PackageLifecycleError("package.lifecycle.package_format")
    return operations[action]


def _file_state_script() -> str:
    records = " ".join(
        f"'{path}|{EXPECTED_MODES[path]}'" for path in sorted(EXPECTED_MODES)
    )
    return f"""set -eu
for record in {records}; do
  path=${{record%|*}}
  mode=${{record##*|}}
  test "$(stat -c '%u:%g:%a' "$path")" = "0:0:$mode"
  test -r "$path"
  test ! -w "$path"
done
printf 'package-files-root-owned-read-only\\n'
"""


def _residue_script() -> str:
    paths = " ".join(f"'{path}'" for path in (*INSTALLED_FILES, *INSTALLED_DIRECTORIES))
    return f"""set -eu
for path in {paths}; do
  test ! -e "$path"
  test ! -L "$path"
done
printf 'agentmage-package-residue-absent\\n'
"""


def _verify_installed_state(
    container_id: str,
    target: ContainerTarget,
    version: str,
    phase: str,
) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    version_record, _ = _step_record(
        container_id,
        LifecycleStep(
            f"{phase}-package-version",
            "standard-user",
            _package_arguments(target, "query"),
            expected_stdout=f"{version}\n",
        ),
    )
    records.append(version_record)
    paths_record, paths_output = _step_record(
        container_id,
        LifecycleStep(
            f"{phase}-component-manifest",
            "standard-user",
            _package_arguments(target, "list"),
        ),
    )
    if sorted(set(paths_output.splitlines())) != expected_managed_paths(
        target.package_format
    ):
        raise PackageLifecycleError(f"package.lifecycle.managed_paths:{target.platform_id}")
    records.append(paths_record)
    file_record, _ = _step_record(
        container_id,
        LifecycleStep(
            f"{phase}-file-authority",
            "standard-user",
            ("sh", "-c", _file_state_script()),
            expected_stdout="package-files-root-owned-read-only\n",
        ),
    )
    records.append(file_record)
    host_record, _ = _step_record(
        container_id,
        LifecycleStep(
            f"{phase}-host-launch",
            "standard-user",
            (
                "/usr/libexec/agentmage/agentmage-host",
                "--verify-package-candidate-root",
                "/",
            ),
            expected_stdout="agentmage.package.valid\n",
        ),
    )
    records.append(host_record)
    adapter_record, adapter_output = _step_record(
        container_id,
        LifecycleStep(
            f"{phase}-inference-boundary-launch",
            "standard-user",
            ("/usr/libexec/agentmage/agentmage-native-inference", "--self-check"),
        ),
    )
    try:
        descriptor = json.loads(adapter_output)
    except json.JSONDecodeError as error:
        raise PackageLifecycleError("package.lifecycle.inference_descriptor") from error
    if descriptor != EXPECTED_INFERENCE_DESCRIPTOR:
        raise PackageLifecycleError("package.lifecycle.inference_descriptor")
    records.append(adapter_record)
    return records


def _verify_removed_state(
    container_id: str, target: ContainerTarget, phase: str
) -> list[dict[str, Any]]:
    absent_record, _ = _step_record(
        container_id,
        LifecycleStep(
            f"{phase}-package-record-absent",
            "standard-user",
            _package_arguments(target, "absent"),
            expect_success=False,
        ),
    )
    residue_record, _ = _step_record(
        container_id,
        LifecycleStep(
            f"{phase}-filesystem-residue-absent",
            "standard-user",
            ("sh", "-c", _residue_script()),
            expected_stdout="agentmage-package-residue-absent\n",
        ),
    )
    return [absent_record, residue_record]


def _inspect_container_controls(container_id: str) -> dict[str, Any]:
    inspected = run_command(["podman", "inspect", container_id], timeout=30)
    try:
        value = json.loads(inspected.stdout)[0]
        host = value["HostConfig"]
    except (IndexError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise PackageLifecycleError("package.lifecycle.container_inspection") from error
    security = set(host.get("SecurityOpt") or [])
    controls = {
        "runtime": "rootless-podman",
        "network": host.get("NetworkMode"),
        "privileged": host.get("Privileged"),
        "capabilities_added": host.get("CapAdd") or [],
        "capabilities_dropped": sorted(host.get("CapDrop") or []),
        "no_new_privileges": "no-new-privileges" in security,
        "selinux_label_isolated_mount": "label=disable" in security,
        "pids_limit": host.get("PidsLimit"),
        "memory_limit_bytes": host.get("Memory"),
        "root_filesystem": "ephemeral-writable-for-package-manager",
        "package_mount": "read-only",
    }
    if (
        controls["network"] != "none"
        or controls["privileged"] is not False
        or controls["capabilities_added"] != []
        or not controls["capabilities_dropped"]
        or controls["no_new_privileges"] is not True
        or controls["selinux_label_isolated_mount"] is not True
        or controls["pids_limit"] != 64
        or controls["memory_limit_bytes"] != 512 * 1024 * 1024
    ):
        raise PackageLifecycleError("package.lifecycle.container_controls")
    return controls


def _image_identity(image: str) -> dict[str, Any]:
    if IMMUTABLE_IMAGE_REFERENCE.fullmatch(image) is None:
        raise PackageLifecycleError("package.lifecycle.image_reference_not_pinned")
    inspected = run_command(["podman", "image", "inspect", image], timeout=30)
    try:
        value = json.loads(inspected.stdout)[0]
        image_id = value["Id"]
    except (IndexError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise PackageLifecycleError("package.lifecycle.image_identity") from error
    if not isinstance(image_id, str) or SHA256_ID.fullmatch(image_id) is None:
        raise PackageLifecycleError("package.lifecycle.image_identity")
    repo_digests = sorted(value.get("RepoDigests") or [])
    if image not in repo_digests:
        raise PackageLifecycleError("package.lifecycle.image_digest_unavailable")
    return {
        "reference": image,
        "id": image_id if image_id.startswith("sha256:") else f"sha256:{image_id}",
        "architecture": value.get("Architecture"),
        "os": value.get("Os"),
        "repo_digests": repo_digests,
    }


def _container_platform_identity(
    container_id: str, target: ContainerTarget
) -> tuple[dict[str, Any], dict[str, str]]:
    record, output = _step_record(
        container_id,
        LifecycleStep(
            "platform-identity",
            "standard-user",
            (
                "sh",
                "-c",
                '. /etc/os-release; printf "%s\\n%s\\n" "$ID" "$VERSION_ID"; uname -m',
            ),
        ),
    )
    lines = output.splitlines()
    if lines != [target.os_id, target.version_id, "x86_64"]:
        raise PackageLifecycleError(f"package.lifecycle.platform:{target.platform_id}")
    return record, {"os_id": lines[0], "version_id": lines[1], "architecture": lines[2]}


def _run_container_target(
    artifacts: dict[str, dict[str, Path]],
    target: ContainerTarget,
    package_root: Path,
) -> dict[str, Any]:
    image = _image_identity(target.image)
    if image["architecture"] != "amd64" or image["os"] != "linux":
        raise PackageLifecycleError(f"package.lifecycle.image_platform:{target.platform_id}")
    created = run_command(container_create_argv(target, package_root), timeout=60)
    container_id = created.stdout.strip()
    if CONTAINER_ID.fullmatch(container_id) is None:
        raise PackageLifecycleError("package.lifecycle.container_identity")
    records: list[dict[str, Any]] = []
    try:
        command(["podman", "start", container_id], timeout=30)
        controls = _inspect_container_controls(container_id)
        platform_record, platform = _container_platform_identity(container_id, target)
        records.append(platform_record)
        user_record, _ = _step_record(
            container_id,
            LifecycleStep(
                "standard-user-identity",
                "standard-user",
                ("sh", "-c", "printf '%s\\n%s\\n' \"$(id -u)\" \"$(id -g)\""),
                expected_stdout="10001\n10001\n",
            ),
        )
        records.append(user_record)

        extension = "rpm" if target.package_format == "rpm" else "deb"
        package_v0 = f"{CONTAINER_PACKAGE_ROOT}/{artifacts['0.0.0'][extension].name}"
        package_v1 = f"{CONTAINER_PACKAGE_ROOT}/{artifacts['0.0.1'][extension].name}"
        broken_v1 = f"{CONTAINER_PACKAGE_ROOT}/broken-{artifacts['0.0.1'][extension].name}"
        install_record, _ = _step_record(
            container_id,
            LifecycleStep(
                "install-0.0.0",
                "package-administrator",
                _package_arguments(target, "install", package_v0),
            ),
        )
        records.append(install_record)
        records.extend(_verify_installed_state(container_id, target, "0.0.0", "initial"))

        corrupt_record, _ = _step_record(
            container_id,
            LifecycleStep(
                "corrupt-upgrade-refusal",
                "package-administrator",
                _package_arguments(target, "upgrade", broken_v1),
                expect_success=False,
            ),
        )
        records.append(corrupt_record)
        records.extend(
            _verify_installed_state(container_id, target, "0.0.0", "post-refusal")
        )

        upgrade_record, _ = _step_record(
            container_id,
            LifecycleStep(
                "upgrade-0.0.1",
                "package-administrator",
                _package_arguments(target, "upgrade", package_v1),
            ),
        )
        records.append(upgrade_record)
        records.extend(_verify_installed_state(container_id, target, "0.0.1", "upgraded"))

        rollback_record, _ = _step_record(
            container_id,
            LifecycleStep(
                "rollback-0.0.0",
                "package-administrator",
                _package_arguments(target, "rollback", package_v0),
            ),
        )
        records.append(rollback_record)
        records.extend(
            _verify_installed_state(container_id, target, "0.0.0", "rolled-back")
        )

        remove_record, _ = _step_record(
            container_id,
            LifecycleStep(
                "uninstall-after-rollback",
                "package-administrator",
                _package_arguments(target, "remove"),
            ),
        )
        records.append(remove_record)
        records.extend(_verify_removed_state(container_id, target, "first-uninstall"))

        reinstall_record, _ = _step_record(
            container_id,
            LifecycleStep(
                "reinstall-0.0.1-after-clean-removal",
                "package-administrator",
                _package_arguments(target, "install", package_v1),
            ),
        )
        records.append(reinstall_record)
        records.extend(
            _verify_installed_state(container_id, target, "0.0.1", "recovered")
        )

        final_remove_record, _ = _step_record(
            container_id,
            LifecycleStep(
                "final-uninstall",
                "package-administrator",
                _package_arguments(target, "remove"),
            ),
        )
        records.append(final_remove_record)
        records.extend(_verify_removed_state(container_id, target, "final-uninstall"))
    finally:
        command(["podman", "rm", "--force", container_id], timeout=30)
    return {
        "platform_id": target.platform_id,
        "status": "pass",
        "image": image,
        "platform": platform,
        "controls": controls,
        "package_format": target.package_format,
        "package_administrator": ADMIN_USER,
        "runtime_user": STANDARD_USER,
        "managed_paths": expected_managed_paths(target.package_format),
        "steps": records,
        "checks": {
            "clean_install": True,
            "standard_user_launch": True,
            "component_manifest_exact": True,
            "root_owned_runtime_read_only": True,
            "corrupt_upgrade_refused": True,
            "prior_valid_state_preserved": True,
            "upgrade": True,
            "rollback": True,
            "uninstall": True,
            "reinstall_recovery": True,
            "final_package_record_absent": True,
            "final_filesystem_residue_absent": True,
            "network_disabled": True,
        },
    }


def _prepare_container_packages(
    artifacts: dict[str, dict[str, Path]], package_root: Path
) -> None:
    for version in ("0.0.0", "0.0.1"):
        for package_format in ("rpm", "deb"):
            source = artifacts[version][package_format]
            if not source.is_file() or source.is_symlink():
                raise PackageLifecycleError("package.lifecycle.artifact")
            shutil.copyfile(source, package_root / source.name)
    for package_format in ("rpm", "deb"):
        source = artifacts["0.0.1"][package_format]
        broken = package_root / f"broken-{source.name}"
        content = source.read_bytes()
        if len(content) <= 1024:
            raise PackageLifecycleError("package.lifecycle.artifact_size")
        broken.write_bytes(content[:1024])


def validate_container_lifecycle(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["container lifecycle must be an object"]
    failures: list[str] = []
    if value.get("schema_version") != 1 or value.get("status") != "pass":
        failures.append("container lifecycle identity changed")
    if value.get("rootless_runtime") is not True or value.get("network_used") is not False:
        failures.append("container lifecycle isolation changed")
    platforms = value.get("platforms")
    if (
        not isinstance(platforms, list)
        or any(not isinstance(item, dict) for item in platforms)
        or [item.get("platform_id") for item in platforms]
        != ["fedora-x86_64", "ubuntu-x86_64"]
    ):
        failures.append("container lifecycle platforms changed")
        return failures
    required_checks = {
        "clean_install",
        "standard_user_launch",
        "component_manifest_exact",
        "root_owned_runtime_read_only",
        "corrupt_upgrade_refused",
        "prior_valid_state_preserved",
        "upgrade",
        "rollback",
        "uninstall",
        "reinstall_recovery",
        "final_package_record_absent",
        "final_filesystem_residue_absent",
        "network_disabled",
    }
    for platform in platforms:
        image = platform.get("image", {})
        controls = platform.get("controls", {})
        steps = platform.get("steps")
        checks = platform.get("checks")
        if not isinstance(image, dict) or not isinstance(controls, dict):
            failures.append(f"{platform.get('platform_id')} structure changed")
            continue
        repo_digests = image.get("repo_digests")
        if (
            platform.get("status") != "pass"
            or platform.get("package_administrator") != ADMIN_USER
            or platform.get("runtime_user") != STANDARD_USER
            or not isinstance(image.get("id"), str)
            or SHA256_ID.fullmatch(image["id"]) is None
            or not isinstance(image.get("reference"), str)
            or IMMUTABLE_IMAGE_REFERENCE.fullmatch(image["reference"]) is None
            or not isinstance(repo_digests, list)
            or image["reference"] not in repo_digests
            or image.get("architecture") != "amd64"
            or image.get("os") != "linux"
        ):
            failures.append(f"{platform.get('platform_id')} identity changed")
        if (
            controls.get("runtime") != "rootless-podman"
            or controls.get("network") != "none"
            or controls.get("privileged") is not False
            or controls.get("capabilities_added") != []
            or not controls.get("capabilities_dropped")
            or controls.get("no_new_privileges") is not True
            or controls.get("selinux_label_isolated_mount") is not True
            or controls.get("pids_limit") != 64
            or controls.get("memory_limit_bytes") != 512 * 1024 * 1024
            or controls.get("root_filesystem")
            != "ephemeral-writable-for-package-manager"
            or controls.get("package_mount") != "read-only"
        ):
            failures.append(f"{platform.get('platform_id')} controls changed")
        if (
            not isinstance(checks, dict)
            or set(checks) != required_checks
            or any(value is not True for value in checks.values())
        ):
            failures.append(f"{platform.get('platform_id')} checks changed")
        if (
            not isinstance(steps, list)
            or any(not isinstance(step, dict) for step in steps)
            or [step.get("id") for step in steps] != list(EXPECTED_STEP_IDS)
            or any(step.get("status") != "pass" for step in steps)
            or any(
                step.get("actor")
                != (
                    "package-administrator"
                    if step.get("id") in ADMIN_STEP_IDS
                    else "standard-user"
                )
                for step in steps
            )
            or any(
                step.get("uid_gid")
                != (
                    ADMIN_USER
                    if step.get("id") in ADMIN_STEP_IDS
                    else STANDARD_USER
                )
                for step in steps
            )
            or any(
                step.get("expected_exit")
                != ("nonzero" if step.get("id") in NONZERO_STEP_IDS else "zero")
                or step.get("observed_exit") != step.get("expected_exit")
                for step in steps
            )
            or any(
                not isinstance(step.get("output_bytes"), int)
                or step.get("output_bytes", -1) < 0
                or SHA256_ID.fullmatch(str(step.get("output_sha256"))) is None
                for step in steps
            )
        ):
            failures.append(f"{platform.get('platform_id')} step closure changed")
    return failures


def verify_container_lifecycle(
    artifacts: dict[str, dict[str, Path]], fedora_image: str, ubuntu_image: str
) -> dict[str, Any]:
    require_commands(("podman",))
    rootless = run_command(
        ["podman", "info", "--format", "{{.Host.Security.Rootless}}"], timeout=30
    )
    if rootless.stdout.strip() != "true":
        raise PackageLifecycleError("package.lifecycle.rootless_required")
    targets = (
        ContainerTarget(
            "fedora-x86_64", fedora_image, "fedora", "44", "rpm"
        ),
        ContainerTarget(
            "ubuntu-x86_64", ubuntu_image, "ubuntu", "26.04", "deb"
        ),
    )
    with tempfile.TemporaryDirectory(prefix="agentmage-container-packages-") as directory:
        package_root = Path(directory)
        _prepare_container_packages(artifacts, package_root)
        platforms = [
            _run_container_target(artifacts, target, package_root) for target in targets
        ]
    lifecycle = {
        "schema_version": 1,
        "status": "pass",
        "rootless_runtime": True,
        "network_used": False,
        "platforms": platforms,
    }
    failures = validate_container_lifecycle(lifecycle)
    if failures:
        raise PackageLifecycleError("; ".join(failures))
    return {
        "fedora_native_container": "pass",
        "ubuntu_native_container": "pass",
        "container_lifecycle": lifecycle,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "release-output")
    parser.add_argument("--with-vscode", action="store_true")
    parser.add_argument("--with-containers", action="store_true")
    parser.add_argument("--fedora-image")
    parser.add_argument("--ubuntu-image")
    args = parser.parse_args(argv)
    try:
        command(["npm", "run", "build", "--workspace", "@agentmage/vscode-shell"], cwd=ROOT)
        command(["cargo", "build", "-p", "agentmage-host", "--release", "--locked"], cwd=ROOT)
        command(
            [
                "cargo",
                "build",
                "-p",
                "agentmage-platform-linux-inference",
                "--bin",
                "agentmage-native-inference",
                "--release",
                "--locked",
            ],
            cwd=ROOT,
        )
        artifacts = {
            version: build_all(args.output.resolve(), version) for version in ("0.0.0", "0.0.1")
        }
        with tempfile.TemporaryDirectory(prefix="agentmage-rebuild-") as directory:
            repeated = build_all(Path(directory), "0.0.0")
            if any(
                artifacts["0.0.0"][kind].read_bytes() != repeated[kind].read_bytes()
                for kind in ("rpm", "deb", "vsix")
            ):
                raise PackageLifecycleError("package.lifecycle.rebuild_drift")
        results = verify_extracted_candidates(artifacts["0.0.0"], artifacts["0.0.1"])
        results["deterministic_rebuild"] = "pass"
        if args.with_vscode:
            results.update(verify_vscode_lifecycle(artifacts["0.0.0"], artifacts["0.0.1"]))
        if args.with_containers:
            if not args.fedora_image or not args.ubuntu_image:
                raise PackageLifecycleError("package.lifecycle.container_identity_required")
            results.update(
                verify_container_lifecycle(artifacts, args.fedora_image, args.ubuntu_image)
            )
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"package lifecycle failed: {error}", file=os.sys.stderr)
        return 1
    results.update(
        {
            "release_signature": "blocked-external-signing-identity-unavailable",
            "trusted_host_bootstrap": (
                "source-implemented-production-trust-and-platform-activation-blocked"
            ),
            "release_claim": "none-pre-release",
        }
    )
    print(canonical_json({"schema_version": 1, "results": results}).decode(), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
