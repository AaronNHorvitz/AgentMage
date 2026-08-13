#!/usr/bin/env python3
"""Run and validate clean Fedora and Ubuntu graphical package acceptance."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import stat
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Final

try:
    from scripts.package_candidate import build_all
except ModuleNotFoundError:
    from package_candidate import build_all


ROOT: Final = Path(__file__).resolve().parents[1]
REPORT_PATH: Final = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-9"
    / "story-9.1"
    / "linux-clean-image-acceptance.json"
)
CONTAINERFILE: Final = ROOT / "release/acceptance/Containerfile.linux-vscode"
PROBE_ROOT: Final = ROOT / "release/acceptance/vscode-probe"
PROCEDURE_PATH: Final = ROOT / "docs/support/linux-clean-image-acceptance.md"
VSCODE_VERSION: Final = "1.132.0"
VSCODE_COMMIT: Final = "df53daabb18cd157bdb08c7f01c34df936cf12f4"
VSCODE_ARCHIVE_SHA256: Final = (
    "acdaf0fa557bda1720956ff65ca0de0965e92d68f97e2db22341984400937aed"
)
VSCODE_ARCHIVE_URL: Final = (
    f"https://update.code.visualstudio.com/{VSCODE_VERSION}/linux-x64/stable"
)
STANDARD_USER: Final = "10001:10001"
PACKAGE_ADMIN: Final = "0:0"
EXTENSION_ID: Final = "agentmage-project.agentmage-vscode-shell"
EXPECTED_PROBE: Final = {
    "schemaVersion": 1,
    "status": "pass",
    "observed": {
        "id": "secure-local-read",
        "vendor": "agentmage",
        "family": "agentmage-secure-read",
        "version": "0.0.0-phase9",
        "tokenCount": 4,
    },
}
EXPECTED_INFERENCE_DESCRIPTOR: Final = {
    "accepted_operation": "self-check-only",
    "authority_inputs": [],
    "component_id": "platform-linux-native-inference",
    "contract": "authenticated-local-endpoint-v1",
    "docker_compatibility_available": False,
    "docker_model_artifact_digest": "sha256:08fa7b1d44f255be48cfc12359211725bfd659742612ed4b221cd5be90d14444",
    "docker_model_runner_image_digest": "sha256:bd94095bbc1ddc4266c3a88f582a92562c6b63eceb175572c9a60045663727c9",
    "docker_runtime_profile_sha256": "ab8cde6bc1440f8a0013390aa2e291a315cdebcfe44aa1d339f0aa0b1d70899c",
    "enabled_models": 0,
    "inference_available": False,
    "native_runtime_package": "agentmage-llama-cpp-b10333-cpu-linux-x86_64",
    "native_runtime_profile_sha256": "21346c06fb86b418706326b186609f8e1f690d6b57b53e73d02b4ad8e28e53ea",
    "network_listener": False,
    "process_boundary_version": 3,
}
CONTAINER_CONTROLS: Final = {
    "runtime": "rootless-podman",
    "network": "none",
    "privileged": False,
    "capabilities_added": [],
    "capabilities_dropped": [
        "CAP_CHOWN",
        "CAP_DAC_OVERRIDE",
        "CAP_FOWNER",
        "CAP_FSETID",
        "CAP_KILL",
        "CAP_NET_BIND_SERVICE",
        "CAP_SETFCAP",
        "CAP_SETGID",
        "CAP_SETPCAP",
        "CAP_SETUID",
        "CAP_SYS_CHROOT",
    ],
    "no_new_privileges": True,
    "seccomp_mode": 2,
    "selinux_label_disabled": True,
    "init": True,
    "pid_limit": 1024,
    "memory_limit_bytes": 2_147_483_648,
    "shared_memory_bytes": 268_435_456,
    "package_mount": "read-only",
    "probe_mount": "read-only",
}
STEP_IDS: Final = (
    "platform-identity",
    "standard-user-identity",
    "system-package-install",
    "package-version",
    "package-files",
    "native-host-launch",
    "inactive-inference-launch",
    "vscode-version",
    "extension-install",
    "extension-registration",
    "vscode-launch-and-provider-exercise",
    "standard-user-process-boundary",
    "vscode-shutdown",
    "extension-uninstall",
    "extension-registration-absent",
    "system-package-uninstall",
    "package-record-absent",
    "isolated-profile-cleanup",
    "final-residue-scan",
)
ADMIN_STEPS: Final = {"system-package-install", "system-package-uninstall"}
EXPECTED_PACKAGE_SCRIPTS: Final = {
    "evidence:story9.1-linux-acceptance:bootstrap": (
        "python3 scripts/linux_clean_image_acceptance.py --bootstrap-images"
    ),
    "evidence:story9.1-linux-acceptance:build": (
        "python3 scripts/linux_clean_image_acceptance.py --write "
        "--source-revision HEAD"
    ),
    "evidence:story9.1-linux-acceptance:check": (
        "python3 scripts/linux_clean_image_acceptance.py"
    ),
}
SOURCE_EXACT: Final = {
    ".cargo/config.toml",
    "Cargo.lock",
    "Cargo.toml",
    "LICENSE",
    "docs/support/linux-clean-image-acceptance.md",
    "package-lock.json",
    "package.json",
    "packaging/linux/README.md",
    "release/acceptance/Containerfile.linux-vscode",
    "release/acceptance/vscode-probe/extension.js",
    "release/acceptance/vscode-probe/package.json",
    "rust-toolchain.toml",
    "scripts/linux_clean_image_acceptance.py",
    "scripts/package_candidate.py",
    "tests/test_linux_clean_image_acceptance.py",
}
SOURCE_PREFIXES: Final = (
    "capabilities/read-only/",
    "kernel/contracts/",
    "kernel/engine/",
    "packaging/linux/",
    "platforms/linux/",
    "platforms/linux-inference/",
    "shells/host/",
    "shells/vscode/",
)
LIMITATIONS: Final = [
    "These are clean rootless container-image acceptance runs, not native physical-host certification.",
    "The graphical harness disables Chromium's nested sandbox only inside an outer rootless, capability-free, no-new-privileges, networkless test container; this is not a supported production launch option.",
    "Container root performs only RPM or DEB package-manager installation and removal; all AgentMage execution, Visual Studio Code lifecycle actions, provider exercise, and residue inspection use numeric UID/GID 10001:10001.",
    "The probe exercises provider discovery and token counting without granting workspace reads, tools, models, credentials, or network authority.",
    "The generated empty workspace is treated as trusted only inside the disposable acceptance container; no repository or user file is mounted into it.",
    "Candidates are unsigned, no model is enabled, no inference is performed, and no supported-release or macOS claim is made.",
]
SHA256 = re.compile(r"^[0-9a-f]{64}$")
OBJECT_ID = re.compile(r"^[0-9a-f]{40,64}$")
IMAGE_ID = re.compile(r"^sha256:[0-9a-f]{64}$")


@dataclass(frozen=True)
class Target:
    """One exact clean-image package and desktop target."""

    platform_id: str
    distribution: str
    version: str
    package_format: str
    base_image: str
    image: str


TARGETS: Final = (
    Target(
        "fedora-x86_64",
        "fedora",
        "44",
        "rpm",
        "docker.io/library/fedora@sha256:89f61a124414261868224666aa7fb8df1b78397a53623774bdfb105d1612b48b",
        "localhost/agentmage-vscode-acceptance:fedora-44",
    ),
    Target(
        "ubuntu-x86_64",
        "ubuntu",
        "26.04",
        "deb",
        "docker.io/library/ubuntu@sha256:7b202b0e2e0028c6250f5fcf41d04df492d145a1654c6995a6553f0c1f6f1960",
        "localhost/agentmage-vscode-acceptance:ubuntu-26.04",
    ),
)


class CleanImageAcceptanceError(ValueError):
    """Raised when clean-image acceptance cannot support its claim."""


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def run_command(
    arguments: list[str], timeout: int = 900
) -> subprocess.CompletedProcess[str]:
    environment = os.environ.copy()
    environment["CARGO_NET_OFFLINE"] = "true"
    return subprocess.run(
        arguments,
        cwd=ROOT,
        env=environment,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=timeout,
        check=False,
    )


def require_command(arguments: list[str], timeout: int = 900) -> str:
    result = run_command(arguments, timeout)
    if result.returncode != 0:
        raise CleanImageAcceptanceError(
            f"verification command failed: {Path(arguments[0]).name}"
        )
    return result.stdout


def git_revision(candidate: str) -> str:
    revision = require_command(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"], 30
    ).strip()
    if re.fullmatch(r"[0-9a-f]{40}", revision) is None:
        raise CleanImageAcceptanceError("source revision is unavailable")
    return revision


def committed_file(revision: str, relative: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{relative}"],
        cwd=ROOT,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=30,
        check=False,
    )
    if result.returncode != 0:
        raise CleanImageAcceptanceError(f"committed source absent: {relative}")
    return result.stdout


def source_identity(revision: str) -> dict[str, Any]:
    paths = tuple(
        path
        for path in require_command(
            ["git", "ls-tree", "-r", "--name-only", revision], 30
        ).splitlines()
        if path in SOURCE_EXACT or path.startswith(SOURCE_PREFIXES)
    )
    if set(SOURCE_EXACT) - set(paths):
        raise CleanImageAcceptanceError("acceptance source closure is incomplete")
    records = []
    for relative in paths:
        committed = committed_file(revision, relative)
        current = ROOT / relative
        if not current.is_file() or current.read_bytes() != committed:
            raise CleanImageAcceptanceError(
                f"acceptance source differs from revision: {relative}"
            )
        records.append(
            {
                "path": relative,
                "sha256": sha256_bytes(committed),
                "bytes": len(committed),
            }
        )
    tree = require_command(["git", "rev-parse", f"{revision}^{{tree}}"], 30).strip()
    if OBJECT_ID.fullmatch(tree) is None:
        raise CleanImageAcceptanceError("source tree identity is unavailable")
    return {"revision": revision, "tree": tree, "files": records}


def validate_published_procedure(root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    try:
        package = json.loads((root / "package.json").read_text(encoding="utf-8"))
        procedure = (root / PROCEDURE_PATH.relative_to(ROOT)).read_text(
            encoding="utf-8"
        )
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        return [f"cannot read published acceptance procedure: {error}"]
    scripts = package.get("scripts", {}) if isinstance(package, dict) else {}
    for name, command in EXPECTED_PACKAGE_SCRIPTS.items():
        if scripts.get(name) != command:
            failures.append(f"published package script drifted: {name}")
        if f"npm run {name}" not in procedure:
            failures.append(f"published invocation is absent: {name}")
    for literal in (VSCODE_ARCHIVE_URL, VSCODE_ARCHIVE_SHA256, "--network=none"):
        if literal not in procedure:
            failures.append(f"published acceptance literal is absent: {literal}")
    return failures


def verify_archive(path: Path) -> None:
    try:
        metadata = path.lstat()
    except OSError as error:
        raise CleanImageAcceptanceError("VS Code archive is unavailable") from error
    if (
        not stat.S_ISREG(metadata.st_mode)
        or metadata.st_size <= 0
        or metadata.st_size > 512 * 1024 * 1024
        or sha256_file(path) != VSCODE_ARCHIVE_SHA256
    ):
        raise CleanImageAcceptanceError("VS Code archive identity is invalid")


def require_rootless_podman() -> None:
    value = require_command(
        ["podman", "info", "--format", "{{.Host.Security.Rootless}}"], 30
    ).strip()
    if value != "true":
        raise CleanImageAcceptanceError("rootless Podman is required")


def bootstrap_images(archive: Path) -> None:
    verify_archive(archive)
    require_rootless_podman()
    with tempfile.TemporaryDirectory(prefix="agentmage-vscode-image-") as directory:
        context = Path(directory)
        shutil.copyfile(archive, context / "vscode-linux-x64.tar.gz")
        for target in TARGETS:
            require_command(
                [
                    "podman",
                    "build",
                    "--pull=missing",
                    "--build-arg",
                    f"BASE_IMAGE={target.base_image}",
                    "--build-arg",
                    f"DISTRIBUTION={target.distribution}",
                    "--build-arg",
                    f"VSCODE_SHA256={VSCODE_ARCHIVE_SHA256}",
                    "--tag",
                    target.image,
                    "--file",
                    str(CONTAINERFILE),
                    str(context),
                ],
                1800,
            )


def image_record(target: Target) -> dict[str, Any]:
    raw = require_command(
        ["podman", "image", "inspect", target.image, "--format", "json"], 30
    )
    try:
        value = json.loads(raw)[0]
        raw_id = value["Id"]
        labels = value["Config"]["Labels"]
    except (IndexError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise CleanImageAcceptanceError("acceptance image metadata is invalid") from error
    image_id = raw_id if str(raw_id).startswith("sha256:") else f"sha256:{raw_id}"
    expected_labels = {
        "org.agentmage.acceptance.base-image": target.base_image,
        "org.agentmage.acceptance.distribution": target.distribution,
        "org.agentmage.acceptance.vscode-version": VSCODE_VERSION,
        "org.agentmage.acceptance.vscode-sha256": VSCODE_ARCHIVE_SHA256,
    }
    if IMAGE_ID.fullmatch(image_id) is None or any(
        labels.get(key) != expected for key, expected in expected_labels.items()
    ):
        raise CleanImageAcceptanceError(
            f"acceptance image identity is invalid: {target.platform_id}"
        )
    return {"tag": target.image, "id": image_id, "labels": expected_labels}


def inspect_container_controls(container: str) -> dict[str, Any]:
    try:
        value = json.loads(require_command(["podman", "inspect", container], 30))[0]
        host = value["HostConfig"]
        mounts = value["Mounts"]
    except (IndexError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise CleanImageAcceptanceError("container control inspection failed") from error
    destinations = {
        item.get("Destination"): not item.get("RW", True)
        for item in mounts
        if isinstance(item, dict)
    }
    security = set(host.get("SecurityOpt") or [])
    status = require_command(
        [
            "podman",
            "exec",
            f"--user={STANDARD_USER}",
            container,
            "sh",
            "-c",
            "awk '$1 == \"Seccomp:\" { print $2 }' /proc/self/status",
        ],
        30,
    ).strip()
    controls = {
        "runtime": "rootless-podman",
        "network": host.get("NetworkMode"),
        "privileged": host.get("Privileged"),
        "capabilities_added": sorted(host.get("CapAdd") or []),
        "capabilities_dropped": sorted(host.get("CapDrop") or []),
        "no_new_privileges": "no-new-privileges" in security,
        "seccomp_mode": int(status) if status.isdigit() else -1,
        "selinux_label_disabled": "label=disable" in security,
        "init": host.get("Init"),
        "pid_limit": host.get("PidsLimit"),
        "memory_limit_bytes": host.get("Memory"),
        "shared_memory_bytes": host.get("ShmSize"),
        "package_mount": "read-only" if destinations.get("/packages") else "invalid",
        "probe_mount": (
            "read-only" if destinations.get("/acceptance-probe") else "invalid"
        ),
    }
    if controls != CONTAINER_CONTROLS:
        raise CleanImageAcceptanceError("container controls differ from policy")
    return controls


def actor_user(actor: str) -> str:
    return PACKAGE_ADMIN if actor == "package-administrator" else STANDARD_USER


def container_exec(
    container: str,
    actor: str,
    arguments: tuple[str, ...],
    *,
    timeout: int = 120,
    expect_success: bool | None = True,
) -> subprocess.CompletedProcess[str]:
    user = actor_user(actor)
    home = "/tmp" if actor == "package-administrator" else "/home/agentmage-test"
    result = run_command(
        [
            "podman",
            "exec",
            f"--user={user}",
            "--workdir=/tmp",
            f"--env=HOME={home}",
            "--env=LANG=C.UTF-8",
            "--env=LC_ALL=C.UTF-8",
            "--env=XDG_RUNTIME_DIR=/run/user/10001",
            container,
            *arguments,
        ],
        timeout,
    )
    if expect_success is not None and (result.returncode == 0) is not expect_success:
        raise CleanImageAcceptanceError("container acceptance command failed")
    return result


def record_step(
    container: str,
    step_id: str,
    actor: str,
    arguments: tuple[str, ...],
    assertion: Callable[[str], bool] | None = None,
    *,
    expect_success: bool = True,
    timeout: int = 120,
) -> tuple[dict[str, str], str]:
    try:
        result = container_exec(
            container,
            actor,
            arguments,
            timeout=timeout,
            expect_success=expect_success,
        )
    except CleanImageAcceptanceError as error:
        raise CleanImageAcceptanceError(f"acceptance step failed: {step_id}") from error
    if assertion is not None and not assertion(result.stdout):
        raise CleanImageAcceptanceError(f"acceptance assertion failed: {step_id}")
    return (
        {
            "id": step_id,
            "actor": actor,
            "uid_gid": actor_user(actor),
            "expected_exit": "zero" if expect_success else "nonzero",
            "observed_exit": "zero" if result.returncode == 0 else "nonzero",
            "status": "pass",
        },
        result.stdout,
    )


def package_arguments(
    target: Target, action: str, path: str | None = None
) -> tuple[str, ...]:
    if target.package_format == "rpm":
        values = {
            "install": ("rpm", "-i", "--nosignature", str(path)),
            "version": ("rpm", "-q", "--qf", "%{VERSION}\n", "agentmage"),
            "list": ("rpm", "-ql", "agentmage"),
            "remove": ("rpm", "-e", "agentmage"),
            "absent": ("rpm", "-q", "agentmage"),
        }
    else:
        values = {
            "install": ("dpkg", "-i", str(path)),
            "version": (
                "dpkg-query",
                "--show",
                "--showformat=${Version}\n",
                "agentmage",
            ),
            "list": ("dpkg", "-L", "agentmage"),
            "remove": ("dpkg", "-r", "agentmage"),
            "absent": ("dpkg", "--status", "agentmage"),
        }
    return values[action]


def launch_vscode(container: str) -> None:
    script = r"""set -eu
root=/home/agentmage-test/acceptance
mkdir -p "$root/workspace" "$root/logs"
printf '%s\n' "$$" > "$root/launcher.pid"
xvfb_pid=
session_pid=
cleanup() {
  trap - EXIT HUP INT TERM
  pkill -TERM -f '^/usr/share/code/code --disable-gpu' 2>/dev/null || true
  if test -n "$session_pid"; then
    kill -TERM "$session_pid" 2>/dev/null || true
    wait "$session_pid" 2>/dev/null || true
  fi
  pkill -TERM -x dbus-daemon 2>/dev/null || true
  pkill -TERM -f '^/usr/libexec/at-spi-bus-launcher' 2>/dev/null || true
  if test -n "$xvfb_pid"; then
    kill -TERM "$xvfb_pid" 2>/dev/null || true
    wait "$xvfb_pid" 2>/dev/null || true
  fi
  exit 0
}
trap cleanup EXIT HUP INT TERM
Xvfb :99 -screen 0 1280x800x24 >"$root/logs/xvfb.log" 2>&1 &
xvfb_pid=$!
export DISPLAY=:99
dbus-run-session -- /usr/share/code/code \
  --disable-gpu \
  --no-sandbox \
  --disable-updates \
  --disable-telemetry \
  --skip-welcome \
  --skip-release-notes \
  --disable-workspace-trust \
  --user-data-dir "$root/user-data" \
  --extensions-dir "$root/extensions" \
  --extensionDevelopmentPath=/acceptance-probe \
  --new-window "$root/workspace" >"$root/logs/code.log" 2>&1 &
session_pid=$!
wait "$session_pid"
"""
    result = run_command(
        [
            "podman",
            "exec",
            "--detach",
            f"--user={STANDARD_USER}",
            "--workdir=/home/agentmage-test",
            "--env=HOME=/home/agentmage-test",
            "--env=LANG=C.UTF-8",
            "--env=LC_ALL=C.UTF-8",
            "--env=XDG_RUNTIME_DIR=/run/user/10001",
            container,
            "bash",
            "-lc",
            script,
        ],
        30,
    )
    if result.returncode != 0:
        raise CleanImageAcceptanceError("Visual Studio Code launch failed")


def wait_for_probe(container: str, timeout: float = 45.0) -> dict[str, Any]:
    deadline = time.monotonic() + timeout
    path = "/home/agentmage-test/acceptance/provider-result.json"
    while time.monotonic() < deadline:
        result = container_exec(
            container,
            "standard-user",
            ("test", "-f", path),
            timeout=10,
            expect_success=None,
        )
        if result.returncode == 0:
            output = container_exec(
                container, "standard-user", ("cat", path), timeout=10
            ).stdout
            try:
                return json.loads(output)
            except json.JSONDecodeError as error:
                raise CleanImageAcceptanceError("provider probe is malformed") from error
        time.sleep(0.25)
    raise CleanImageAcceptanceError("provider probe timed out")


def extension_log(container: str) -> tuple[str, str]:
    activation = f"ExtensionService#_doActivateExtension {EXTENSION_ID}"
    deadline = time.monotonic() + 10.0
    while time.monotonic() < deadline:
        logs = container_exec(
            container,
            "standard-user",
            (
                "find",
                "/home/agentmage-test/acceptance/user-data/logs",
                "-path",
                "*/exthost/exthost.log",
                "-type",
                "f",
                "-print",
            ),
        ).stdout.splitlines()
        for path in logs:
            content = container_exec(
                container, "standard-user", ("cat", path), timeout=30
            ).stdout
            related = [
                line for line in content.splitlines() if EXTENSION_ID in line
            ]
            if any(activation in line for line in related):
                if any(
                    "[error]" in line.lower() or "failed" in line.lower()
                    for line in related
                ):
                    raise CleanImageAcceptanceError(
                        "AgentMage extension activation failed"
                    )
                return activation, sha256_bytes(content.encode("utf-8"))
        time.sleep(0.25)
    raise CleanImageAcceptanceError("AgentMage extension activation was not logged")


def process_snapshot(container: str) -> dict[str, Any]:
    output = container_exec(
        container, "standard-user", ("ps", "-eo", "uid=,comm=")
    ).stdout
    watched = {
        "code",
        "agentmage-host",
        "agentmage-native-inference",
        "at-spi-bus-launcher",
        "Xvfb",
        "dbus-daemon",
    }
    records = []
    for line in output.splitlines():
        fields = line.split(None, 1)
        if len(fields) != 2:
            continue
        command = fields[1]
        if command == "agentmage-nativ":
            command = "agentmage-native-inference"
        elif command == "at-spi-bus-laun":
            command = "at-spi-bus-launcher"
        if command in watched:
            records.append((fields[0], command))
    counts = {
        name: sum(command == name for _, command in records)
        for name in sorted(watched)
    }
    if counts["code"] < 3 or counts["Xvfb"] != 1 or counts["dbus-daemon"] < 1:
        raise CleanImageAcceptanceError("graphical process topology is incomplete")
    if any(uid != "10001" for uid, _ in records):
        raise CleanImageAcceptanceError("graphical process escaped the standard user")
    return {"standard_user_only": True, "process_counts": counts}


def stop_vscode(container: str) -> None:
    pid = container_exec(
        container,
        "standard-user",
        ("cat", "/home/agentmage-test/acceptance/launcher.pid"),
    ).stdout.strip()
    if not pid.isdigit() or int(pid) <= 1:
        raise CleanImageAcceptanceError("Visual Studio Code launcher identity is invalid")
    container_exec(container, "standard-user", ("kill", "-TERM", pid))
    watched = re.compile(
        r"^(?:code|Xvfb|dbus-daemon|at-spi-bus-laun|agentmage-host|agentmage-nativ.*)$"
    )
    deadline = time.monotonic() + 15.0
    while time.monotonic() < deadline:
        output = container_exec(
            container, "standard-user", ("ps", "-eo", "comm=")
        ).stdout
        if not any(watched.fullmatch(line.strip()) for line in output.splitlines()):
            return
        time.sleep(0.25)
    raise CleanImageAcceptanceError("Visual Studio Code process residue remained")


def create_container(target: Target, package_root: Path) -> str:
    container = require_command(
        [
            "podman",
            "run",
            "--detach",
            "--pull=never",
            "--network=none",
            "--cap-drop=all",
            "--security-opt=no-new-privileges",
            "--security-opt=label=disable",
            "--init",
            "--pids-limit=1024",
            "--memory=2g",
            "--shm-size=268435456",
            "--tmpfs=/tmp:rw,nosuid,nodev,size=268435456",
            "--mount",
            f"type=bind,src={package_root.resolve()},dst=/packages,ro=true",
            "--mount",
            f"type=bind,src={PROBE_ROOT.resolve()},dst=/acceptance-probe,ro=true",
            target.image,
        ],
        30,
    ).strip()
    if re.fullmatch(r"[0-9a-f]{64}", container) is None:
        raise CleanImageAcceptanceError("acceptance container identity is invalid")
    return container


def run_target(target: Target, package: Path, package_root: Path) -> dict[str, Any]:
    image = image_record(target)
    container = create_container(target, package_root)
    steps: list[dict[str, str]] = []
    result: dict[str, Any] | None = None
    try:
        controls = inspect_container_controls(container)
        step, _ = record_step(
            container,
            "platform-identity",
            "standard-user",
            (
                "sh",
                "-c",
                ". /etc/os-release; printf '%s\\n%s\\n' \"$ID\" \"$VERSION_ID\"",
            ),
            lambda value: value == f"{target.distribution}\n{target.version}\n",
        )
        steps.append(step)
        step, _ = record_step(
            container,
            "standard-user-identity",
            "standard-user",
            ("sh", "-c", "printf '%s:%s\\n' \"$(id -u)\" \"$(id -g)\""),
            lambda value: value == f"{STANDARD_USER}\n",
        )
        steps.append(step)
        step, _ = record_step(
            container,
            "system-package-install",
            "package-administrator",
            package_arguments(target, "install", f"/packages/{package.name}"),
        )
        steps.append(step)
        step, _ = record_step(
            container,
            "package-version",
            "standard-user",
            package_arguments(target, "version"),
            lambda value: value == "0.0.0\n",
        )
        steps.append(step)
        step, package_paths = record_step(
            container,
            "package-files",
            "standard-user",
            package_arguments(target, "list"),
            lambda value: all(
                path in value.splitlines()
                for path in (
                    "/usr/libexec/agentmage/agentmage-host",
                    "/usr/libexec/agentmage/agentmage-native-inference",
                    "/usr/share/agentmage/agentmage.vsix",
                    "/usr/share/agentmage/package-manifest.json",
                )
            ),
        )
        steps.append(step)
        step, _ = record_step(
            container,
            "native-host-launch",
            "standard-user",
            (
                "/usr/libexec/agentmage/agentmage-host",
                "--verify-package-candidate-root",
                "/",
            ),
            lambda value: value == "agentmage.package.valid\n",
        )
        steps.append(step)
        step, adapter = record_step(
            container,
            "inactive-inference-launch",
            "standard-user",
            ("/usr/libexec/agentmage/agentmage-native-inference", "--self-check"),
            lambda value: json.loads(value) == EXPECTED_INFERENCE_DESCRIPTOR,
        )
        steps.append(step)
        step, vscode = record_step(
            container,
            "vscode-version",
            "standard-user",
            ("code", "--version"),
            lambda value: value.splitlines()[:2]
            == [VSCODE_VERSION, VSCODE_COMMIT],
        )
        steps.append(step)

        profile = "/home/agentmage-test/acceptance"
        common = (
            "code",
            "--disable-gpu",
            "--user-data-dir",
            f"{profile}/user-data",
            "--extensions-dir",
            f"{profile}/extensions",
        )
        step, _ = record_step(
            container,
            "extension-install",
            "standard-user",
            (
                *common,
                "--install-extension",
                "/usr/share/agentmage/agentmage.vsix",
                "--force",
            ),
            lambda value: "successfully installed" in value,
        )
        steps.append(step)
        step, _ = record_step(
            container,
            "extension-registration",
            "standard-user",
            (*common, "--list-extensions", "--show-versions"),
            lambda value: value.splitlines() == [f"{EXTENSION_ID}@0.0.0"],
        )
        steps.append(step)

        launch_vscode(container)
        probe = wait_for_probe(container)
        activation, log_sha256 = extension_log(container)
        if probe != EXPECTED_PROBE:
            raise CleanImageAcceptanceError("provider exercise result changed")
        steps.append(passive_step("vscode-launch-and-provider-exercise"))
        processes = process_snapshot(container)
        steps.append(passive_step("standard-user-process-boundary"))
        stop_vscode(container)
        steps.append(passive_step("vscode-shutdown"))

        step, _ = record_step(
            container,
            "extension-uninstall",
            "standard-user",
            (*common, "--uninstall-extension", EXTENSION_ID),
            lambda value: "successfully uninstalled" in value,
        )
        steps.append(step)
        step, _ = record_step(
            container,
            "extension-registration-absent",
            "standard-user",
            (*common, "--list-extensions", "--show-versions"),
            lambda value: value.strip() == "",
        )
        steps.append(step)
        step, _ = record_step(
            container,
            "system-package-uninstall",
            "package-administrator",
            package_arguments(target, "remove"),
        )
        steps.append(step)
        step, _ = record_step(
            container,
            "package-record-absent",
            "standard-user",
            package_arguments(target, "absent"),
            expect_success=False,
        )
        steps.append(step)
        step, _ = record_step(
            container,
            "isolated-profile-cleanup",
            "standard-user",
            ("rm", "-rf", profile),
        )
        steps.append(step)

        residue_script = r"""set -eu
for path in \
  /usr/libexec/agentmage \
  /usr/share/agentmage \
  /usr/share/licenses/agentmage \
  /run/user/10001/agentmage \
  /home/agentmage-test/acceptance; do
  test ! -e "$path"
  test ! -L "$path"
done
if ps -eo comm= | grep -Eq \
  '^(code|Xvfb|dbus-daemon|at-spi-bus-laun|agentmage-host|agentmage-nativ.*)$'; then
  exit 1
fi
printf 'agentmage-runtime-and-profile-residue-absent\n'
"""
        step, _ = record_step(
            container,
            "final-residue-scan",
            "standard-user",
            ("sh", "-c", residue_script),
            lambda value: value
            == "agentmage-runtime-and-profile-residue-absent\n",
        )
        steps.append(step)
        if tuple(item["id"] for item in steps) != STEP_IDS:
            raise CleanImageAcceptanceError("acceptance step closure changed")
        result = {
            "platform_id": target.platform_id,
            "distribution": target.distribution,
            "version": target.version,
            "architecture": "x86_64",
            "package_format": target.package_format,
            "base_image": target.base_image,
            "acceptance_image": image,
            "container_controls": controls,
            "actors": {
                "package_administrator": PACKAGE_ADMIN,
                "agentmage_and_vscode_user": STANDARD_USER,
            },
            "steps": steps,
            "vscode": {
                "version": vscode.splitlines()[0],
                "commit": vscode.splitlines()[1],
                "archive_sha256": VSCODE_ARCHIVE_SHA256,
                "extension_id": EXTENSION_ID,
                "extension_version": "0.0.0",
                "activation_event": activation,
                "extension_host_log_sha256": log_sha256,
                "provider_probe": probe,
                "chromium_sandbox": (
                    "disabled-inside-outer-test-container-only"
                ),
            },
            "process_boundary": processes,
            "package_file_count": len(set(package_paths.splitlines())),
            "inference_descriptor": json.loads(adapter),
            "residue": {
                "active_extension_registration": False,
                "package_record": False,
                "package_paths": False,
                "runtime_processes": False,
                "runtime_socket": False,
                "isolated_user_profile": False,
            },
        }
    finally:
        run_command(["podman", "rm", "--force", container], 30)
    if result is None:
        raise CleanImageAcceptanceError("acceptance target did not produce a result")
    absent = run_command(["podman", "container", "inspect", container], 10)
    if absent.returncode == 0:
        raise CleanImageAcceptanceError("acceptance container residue remained")
    result["container_removed"] = True
    return result


def passive_step(step_id: str) -> dict[str, str]:
    return {
        "id": step_id,
        "actor": "standard-user",
        "uid_gid": STANDARD_USER,
        "expected_exit": "zero",
        "observed_exit": "zero",
        "status": "pass",
    }


def build_report(revision: str) -> dict[str, Any]:
    procedure_failures = validate_published_procedure()
    if procedure_failures:
        raise CleanImageAcceptanceError("; ".join(procedure_failures))
    require_rootless_podman()
    source = source_identity(revision)
    require_command(["npm", "run", "build", "--workspace", "@agentmage/vscode-shell"])
    require_command(
        ["cargo", "build", "-p", "agentmage-host", "--release", "--locked"]
    )
    require_command(
        [
            "cargo",
            "build",
            "-p",
            "agentmage-platform-linux-inference",
            "--bin",
            "agentmage-native-inference",
            "--release",
            "--locked",
        ]
    )
    with tempfile.TemporaryDirectory(prefix="agentmage-clean-acceptance-") as directory:
        package_root = Path(directory)
        artifacts = build_all(package_root, "0.0.0")
        packages = {
            kind: {
                "filename": path.name,
                "sha256": sha256_file(path),
                "bytes": path.stat().st_size,
            }
            for kind, path in sorted(artifacts.items())
        }
        platforms = [
            run_target(target, artifacts[target.package_format], package_root)
            for target in TARGETS
        ]
    return {
        "schema_version": 1,
        "artifact_id": "linux-clean-image-graphical-acceptance",
        "task_ids": ["9.1.3.4"],
        "status": "pass-clean-fedora-ubuntu-images",
        "source": source,
        "published_procedure": {
            "path": PROCEDURE_PATH.relative_to(ROOT).as_posix(),
            "package_scripts": EXPECTED_PACKAGE_SCRIPTS,
            "vscode_archive_url": VSCODE_ARCHIVE_URL,
            "vscode_archive_sha256": VSCODE_ARCHIVE_SHA256,
        },
        "packages": packages,
        "rootless_runtime": True,
        "platforms": platforms,
        "bootstrap_network_separate": True,
        "acceptance_network_used": False,
        "private_values_present": False,
        "enabled_models": 0,
        "inference_performed": False,
        "release_claim": "none",
        "limitations": LIMITATIONS,
    }


def validate_platform(value: Any, target: Target) -> list[str]:
    if not isinstance(value, dict):
        return [f"platform result is malformed: {target.platform_id}"]
    failures: list[str] = []
    image = value.get("acceptance_image", {})
    expected_labels = {
        "org.agentmage.acceptance.base-image": target.base_image,
        "org.agentmage.acceptance.distribution": target.distribution,
        "org.agentmage.acceptance.vscode-version": VSCODE_VERSION,
        "org.agentmage.acceptance.vscode-sha256": VSCODE_ARCHIVE_SHA256,
    }
    if (
        value.get("platform_id") != target.platform_id
        or value.get("distribution") != target.distribution
        or value.get("version") != target.version
        or value.get("architecture") != "x86_64"
        or value.get("package_format") != target.package_format
        or value.get("base_image") != target.base_image
        or not isinstance(image, dict)
        or image.get("tag") != target.image
        or IMAGE_ID.fullmatch(str(image.get("id"))) is None
        or image.get("labels") != expected_labels
    ):
        failures.append(f"platform or image identity changed: {target.platform_id}")
    if value.get("container_controls") != CONTAINER_CONTROLS:
        failures.append(f"container controls weakened: {target.platform_id}")
    if value.get("actors") != {
        "package_administrator": PACKAGE_ADMIN,
        "agentmage_and_vscode_user": STANDARD_USER,
    }:
        failures.append(f"acceptance actor boundary changed: {target.platform_id}")
    steps = value.get("steps")
    if (
        not isinstance(steps, list)
        or any(not isinstance(item, dict) for item in steps)
        or [item.get("id") for item in steps] != list(STEP_IDS)
        or any(
            item.get("actor")
            != (
                "package-administrator"
                if item.get("id") in ADMIN_STEPS
                else "standard-user"
            )
            or item.get("uid_gid")
            != (PACKAGE_ADMIN if item.get("id") in ADMIN_STEPS else STANDARD_USER)
            or item.get("status") != "pass"
            or item.get("expected_exit") != item.get("observed_exit")
            for item in steps or []
        )
    ):
        failures.append(f"acceptance step closure changed: {target.platform_id}")
    vscode = value.get("vscode", {})
    if (
        not isinstance(vscode, dict)
        or vscode.get("version") != VSCODE_VERSION
        or vscode.get("commit") != VSCODE_COMMIT
        or vscode.get("archive_sha256") != VSCODE_ARCHIVE_SHA256
        or vscode.get("extension_id") != EXTENSION_ID
        or vscode.get("extension_version") != "0.0.0"
        or vscode.get("activation_event")
        != f"ExtensionService#_doActivateExtension {EXTENSION_ID}"
        or SHA256.fullmatch(str(vscode.get("extension_host_log_sha256"))) is None
        or vscode.get("provider_probe") != EXPECTED_PROBE
        or vscode.get("chromium_sandbox")
        != "disabled-inside-outer-test-container-only"
    ):
        failures.append(f"VS Code acceptance closure changed: {target.platform_id}")
    process = value.get("process_boundary", {})
    counts = process.get("process_counts", {}) if isinstance(process, dict) else {}
    if (
        not isinstance(process, dict)
        or process.get("standard_user_only") is not True
        or not isinstance(counts, dict)
        or counts.get("code", 0) < 3
        or counts.get("Xvfb") != 1
        or counts.get("dbus-daemon", 0) < 1
    ):
        failures.append(f"standard-user process boundary changed: {target.platform_id}")
    if (
        value.get("inference_descriptor") != EXPECTED_INFERENCE_DESCRIPTOR
        or not isinstance(value.get("package_file_count"), int)
        or value.get("package_file_count", 0) < 7
    ):
        failures.append(f"package execution closure changed: {target.platform_id}")
    if value.get("residue") != {
        "active_extension_registration": False,
        "package_record": False,
        "package_paths": False,
        "runtime_processes": False,
        "runtime_socket": False,
        "isolated_user_profile": False,
    } or value.get("container_removed") is not True:
        failures.append(f"residue closure changed: {target.platform_id}")
    return failures


def validate_report(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["clean-image acceptance report must be an object"]
    failures: list[str] = []
    source_value = value.get("source", {})
    source = source_value if isinstance(source_value, dict) else {}
    if (
        value.get("schema_version") != 1
        or value.get("artifact_id") != "linux-clean-image-graphical-acceptance"
        or value.get("task_ids") != ["9.1.3.4"]
        or value.get("status") != "pass-clean-fedora-ubuntu-images"
        or re.fullmatch(r"[0-9a-f]{40}", str(source.get("revision"))) is None
        or OBJECT_ID.fullmatch(str(source.get("tree"))) is None
    ):
        failures.append("clean-image acceptance report identity changed")
    files = source.get("files")
    if (
        not isinstance(files, list)
        or not files
        or any(not isinstance(item, dict) for item in files)
        or [item.get("path") for item in files]
        != sorted(item.get("path") for item in files)
        or any(
            SHA256.fullmatch(str(item.get("sha256"))) is None
            or not isinstance(item.get("bytes"), int)
            or item.get("bytes", -1) < 0
            for item in files
        )
    ):
        failures.append("acceptance source closure is invalid")
    if value.get("published_procedure") != {
        "path": PROCEDURE_PATH.relative_to(ROOT).as_posix(),
        "package_scripts": EXPECTED_PACKAGE_SCRIPTS,
        "vscode_archive_url": VSCODE_ARCHIVE_URL,
        "vscode_archive_sha256": VSCODE_ARCHIVE_SHA256,
    }:
        failures.append("published procedure identity changed")
    packages = value.get("packages")
    if (
        not isinstance(packages, dict)
        or set(packages) != {"deb", "rpm", "vsix"}
        or any(not isinstance(item, dict) for item in packages.values())
        or any(
            SHA256.fullmatch(str(item.get("sha256"))) is None
            or not isinstance(item.get("bytes"), int)
            or item.get("bytes", 0) <= 0
            or not isinstance(item.get("filename"), str)
            for item in packages.values()
        )
    ):
        failures.append("acceptance package identity closure is invalid")
    platforms = value.get("platforms")
    if not isinstance(platforms, list) or len(platforms) != len(TARGETS):
        failures.append("acceptance platform closure is invalid")
    else:
        for platform, target in zip(platforms, TARGETS, strict=True):
            failures.extend(validate_platform(platform, target))
    if (
        value.get("rootless_runtime") is not True
        or value.get("bootstrap_network_separate") is not True
        or value.get("acceptance_network_used") is not False
        or value.get("private_values_present") is not False
        or value.get("enabled_models") != 0
        or value.get("inference_performed") is not False
        or value.get("release_claim") != "none"
    ):
        failures.append("authority, network, privacy, model, or release state changed")
    if value.get("limitations") != LIMITATIONS:
        failures.append("clean-image acceptance limitations changed")
    return failures


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(
        prefix=".agentmage-clean-acceptance-", dir=path.parent
    )
    temporary = Path(name)
    try:
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(content)
            stream.flush()
            os.fsync(stream.fileno())
        temporary.chmod(0o644)
        os.replace(temporary, path)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def check_report() -> None:
    try:
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise CleanImageAcceptanceError(
            "clean-image acceptance report is unavailable"
        ) from error
    failures = validate_report(report)
    source = report.get("source")
    revision = source.get("revision") if isinstance(source, dict) else None
    if isinstance(revision, str):
        try:
            if source != source_identity(revision):
                failures.append("clean-image acceptance source identity is stale")
        except CleanImageAcceptanceError as error:
            failures.append(str(error))
    if failures:
        raise CleanImageAcceptanceError("; ".join(failures))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bootstrap-images", action="store_true")
    parser.add_argument("--vscode-archive", type=Path)
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    try:
        if arguments.bootstrap_images:
            if arguments.vscode_archive is None:
                raise CleanImageAcceptanceError(
                    "--vscode-archive is required while bootstrapping images"
                )
            bootstrap_images(arguments.vscode_archive.resolve())
            print("clean Fedora and Ubuntu VS Code acceptance images bootstrapped")
            return 0
        if arguments.vscode_archive is not None:
            raise CleanImageAcceptanceError(
                "--vscode-archive is accepted only with --bootstrap-images"
            )
        if arguments.write:
            revision = git_revision(arguments.source_revision)
            write_atomic(REPORT_PATH, canonical_json(build_report(revision)))
        check_report()
    except (
        CleanImageAcceptanceError,
        OSError,
        UnicodeError,
        json.JSONDecodeError,
        subprocess.SubprocessError,
    ) as error:
        print(f"clean-image acceptance failed: {error}", file=sys.stderr)
        return 1
    print("clean Fedora and Ubuntu graphical package acceptance validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
