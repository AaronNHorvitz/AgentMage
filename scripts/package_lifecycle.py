#!/usr/bin/env python3
"""Rebuild and execute AgentMage candidate package lifecycle gates."""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import tempfile
from pathlib import Path

try:
    from scripts.package_candidate import ROOT, build_all, canonical_json
except ModuleNotFoundError:
    from package_candidate import ROOT, build_all, canonical_json  # type: ignore[no-redef]


class PackageLifecycleError(ValueError):
    """Raised when a lifecycle gate fails."""


def command(arguments: list[str], cwd: Path | None = None, expect_success: bool = True) -> None:
    result = subprocess.run(
        arguments,
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        check=False,
    )
    if (result.returncode == 0) != expect_success:
        raise PackageLifecycleError(
            f"package.lifecycle.command_result:{Path(arguments[0]).name}:"
            f"exit={result.returncode}:expected_success={str(expect_success).lower()}"
        )


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


def verify_container_lifecycle(
    artifacts: dict[str, dict[str, Path]], fedora_image: str, ubuntu_image: str
) -> dict[str, str]:
    require_commands(("podman",))
    output = artifacts["0.0.0"]["rpm"].parent.resolve()
    mount = f"{output}:/packages:ro,Z"
    fedora_script = """set -e
rpm -i --nosignature /packages/agentmage-0.0.0-1.*.x86_64.rpm
/usr/libexec/agentmage/agentmage-host --verify-package-candidate-root /
rpm -U --nosignature /packages/agentmage-0.0.1-1.*.x86_64.rpm
test "$(rpm -q --qf '%{VERSION}' agentmage)" = 0.0.1
/usr/libexec/agentmage/agentmage-host --verify-package-candidate-root /
cp /packages/agentmage-0.0.0-1.*.x86_64.rpm /tmp/broken.rpm
truncate -s 1024 /tmp/broken.rpm
! rpm -U --nosignature /tmp/broken.rpm
test "$(rpm -q --qf '%{VERSION}' agentmage)" = 0.0.1
/usr/libexec/agentmage/agentmage-host --verify-package-candidate-root /
rpm -U --oldpackage --nosignature /packages/agentmage-0.0.0-1.*.x86_64.rpm
test "$(rpm -q --qf '%{VERSION}' agentmage)" = 0.0.0
rpm -e agentmage
test ! -e /usr/libexec/agentmage/agentmage-host
test ! -e /usr/libexec/agentmage/agentmage-native-inference
test ! -e /usr/share/agentmage/package-manifest.json
"""
    ubuntu_script = """set -e
dpkg -i /packages/agentmage_0.0.0_amd64.deb
/usr/libexec/agentmage/agentmage-host --verify-package-candidate-root /
dpkg -i /packages/agentmage_0.0.1_amd64.deb
dpkg -s agentmage | grep -qx 'Version: 0.0.1'
/usr/libexec/agentmage/agentmage-host --verify-package-candidate-root /
cp /packages/agentmage_0.0.0_amd64.deb /tmp/broken.deb
truncate -s 1024 /tmp/broken.deb
! dpkg -i /tmp/broken.deb
dpkg -s agentmage | grep -qx 'Version: 0.0.1'
/usr/libexec/agentmage/agentmage-host --verify-package-candidate-root /
dpkg -i /packages/agentmage_0.0.0_amd64.deb
dpkg -s agentmage | grep -qx 'Version: 0.0.0'
dpkg -r agentmage
test ! -e /usr/libexec/agentmage/agentmage-host
test ! -e /usr/libexec/agentmage/agentmage-native-inference
test ! -e /usr/share/agentmage/package-manifest.json
"""
    for image, script in ((fedora_image, fedora_script), (ubuntu_image, ubuntu_script)):
        command(
            ["podman", "run", "--rm", "--network=none", "-v", mount, image, "sh", "-lc", script]
        )
    return {"fedora_native_container": "pass", "ubuntu_native_container": "pass"}


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
