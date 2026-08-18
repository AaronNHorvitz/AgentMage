#!/usr/bin/env python3
"""Run the promoted AgentMage matrix inside one offline Linux KVM guest."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import tempfile
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
LANE_ORDER: Final = (
    "product",
    "documentation",
    "package",
    "lifecycle",
    "security",
    "accessibility",
    "recovery",
    "removal",
    "native-runtime",
    "docker-compatibility",
)
RUNNER_DIGEST: Final = (
    "sha256:bd94095bbc1ddc4266c3a88f582a92562c6b63eceb175572c9a60045663727c9"
)
MODEL_DIGEST: Final = (
    "08fa7b1d44f255be48cfc12359211725bfd659742612ed4b221cd5be90d14444"
)
MODEL_FILES: Final = {
    "gemma-4-E4B-it-Q4_K_M.gguf": "85a896a047553e842f25297ee5b031d64ff30147d9c4af17b1e4b394cd1fab87",
    "mmproj-F16.gguf": "ddf46c21d7078e95338cfc22306b19b276a29a5ad089023449dd54d4b6170a51",
}
MANAGED_PATHS: Final = (
    "/usr/libexec/agentmage/agentmage-host",
    "/usr/libexec/agentmage/agentmage-native-inference",
    "/usr/libexec/agentmage/agentmage-model-installer",
    "/usr/libexec/agentmage/agentmage-docker-guard",
    "/usr/libexec/agentmage/agentmage-docker-topology-collector",
    "/usr/share/agentmage/agentmage.vsix",
    "/usr/share/agentmage/package-manifest.json",
    "/usr/share/licenses/agentmage/LICENSE",
)


class PromotedGuestError(ValueError):
    """Raised when a promoted guest lane does not pass exactly."""


def bounded_failure(output: str) -> str:
    sanitized = output.replace(str(ROOT), "<GUEST_SOURCE>")
    sanitized = sanitized.replace("/home/agentmage/source", "<GUEST_SOURCE>")
    sanitized = sanitized.replace("/home/agentmage", "<GUEST_HOME>")
    return " | ".join(line[-240:] for line in sanitized.splitlines()[-12:])[:2400] or "no-output"


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        while block := stream.read(1024 * 1024):
            digest.update(block)
    return digest.hexdigest()


def run(
    lane: str,
    command: list[str],
    receipts: list[dict[str, Any]],
    *,
    expected_success: bool = True,
    timeout: int = 3600,
) -> str:
    environment = {
        **os.environ,
        "CARGO_NET_OFFLINE": "true",
        "NPM_CONFIG_AUDIT": "false",
        "NPM_CONFIG_FUND": "false",
        "NPM_CONFIG_OFFLINE": "true",
        "NPM_CONFIG_UPDATE_NOTIFIER": "false",
        "RUSTUP_NO_UPDATE_CHECK": "1",
    }
    completed = subprocess.run(
        command,
        cwd=ROOT,
        env=environment,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=timeout,
        check=False,
    )
    success = completed.returncode == 0
    output = completed.stdout.replace(str(ROOT), "<GUEST_SOURCE>")
    receipts.append(
        {
            "lane": lane,
            "command_id": f"{lane}-{len(receipts) + 1}",
            "executable": Path(command[0]).name,
            "expected_exit": "zero" if expected_success else "nonzero",
            "observed_exit": "zero" if success else "nonzero",
            "output_bytes": len(output.encode()),
            "output_sha256": hashlib.sha256(output.encode()).hexdigest(),
        }
    )
    if success != expected_success:
        raise PromotedGuestError(
            f"promoted guest command failed: {lane}:{Path(command[0]).name}:"
            f"{bounded_failure(completed.stdout)}"
        )
    return completed.stdout


def package_arguments(distribution: str, action: str, package: Path | None = None) -> list[str]:
    if distribution == "fedora":
        operations = {
            "install": ["sudo", "rpm", "-i", "--nosignature", str(package)],
            "upgrade": ["sudo", "rpm", "-U", "--nosignature", str(package)],
            "rollback": ["sudo", "rpm", "-U", "--oldpackage", "--nosignature", str(package)],
            "remove": ["sudo", "rpm", "-e", "agentmage"],
            "query": ["rpm", "-q", "--qf", "%{VERSION}\\n", "agentmage"],
            "absent": ["rpm", "-q", "agentmage"],
        }
    elif distribution == "ubuntu":
        operations = {
            "install": ["sudo", "dpkg", "-i", str(package)],
            "upgrade": ["sudo", "dpkg", "-i", str(package)],
            "rollback": ["sudo", "dpkg", "-i", str(package)],
            "remove": ["sudo", "dpkg", "-r", "agentmage"],
            "query": ["dpkg-query", "--show", "--showformat=${Version}\\n", "agentmage"],
            "absent": ["dpkg", "--status", "agentmage"],
        }
    else:
        raise PromotedGuestError("unsupported promoted guest distribution")
    return operations[action]


def verify_installed(
    distribution: str,
    version: str,
    lane: str,
    receipts: list[dict[str, Any]],
) -> None:
    observed = run(lane, package_arguments(distribution, "query"), receipts)
    if observed != f"{version}\n":
        raise PromotedGuestError("installed package version drifted")
    run(
        lane,
        ["/usr/libexec/agentmage/agentmage-host", "--verify-package-candidate-root", "/"],
        receipts,
    )
    for executable in (
        "/usr/libexec/agentmage/agentmage-native-inference",
        "/usr/libexec/agentmage/agentmage-model-installer",
        "/usr/libexec/agentmage/agentmage-docker-guard",
        "/usr/libexec/agentmage/agentmage-docker-topology-collector",
    ):
        json.loads(run(lane, [executable, "--self-check"], receipts))
    script = "set -eu; " + "; ".join(
        f'test "$(stat -c %u:%g:%a {path})" = "0:0:{"755" if "/usr/libexec/" in path else "644"}"'
        for path in MANAGED_PATHS
    )
    run(lane, ["bash", "-c", script], receipts)


def verify_removed(lane: str, receipts: list[dict[str, Any]]) -> None:
    run(
        lane,
        ["bash", "-c", "set -eu; " + "; ".join(f"test ! -e {path}" for path in MANAGED_PATHS)],
        receipts,
    )


def execute(distribution: str, native_archive: Path) -> dict[str, Any]:
    receipts: list[dict[str, Any]] = []
    lanes: list[dict[str, Any]] = []

    def lane(name: str, start: int, **details: Any) -> None:
        lane_receipts = receipts[start:]
        lanes.append(
            {
                "id": name,
                "status": "pass",
                "command_count": len(lane_receipts),
                "receipt_sha256": hashlib.sha256(
                    json.dumps(lane_receipts, sort_keys=True, separators=(",", ":")).encode()
                ).hexdigest(),
                **details,
            }
        )

    start = len(receipts)
    run("product", ["npm", "run", "product:check"], receipts, timeout=7200)
    lane("product", start)

    start = len(receipts)
    run("documentation", ["npm", "run", "docs:check"], receipts, timeout=7200)
    lane("documentation", start)

    package_root = ROOT / "release-output/promoted-guest"
    start = len(receipts)
    run("package", ["cargo", "build", "--workspace", "--release", "--locked", "--offline"], receipts, timeout=7200)
    run("package", ["npm", "run", "build", "--workspace", "@agentmage/vscode-shell"], receipts)
    for version in ("0.0.0", "0.0.1"):
        run(
            "package",
            ["python3", "scripts/package_candidate.py", "--output", str(package_root / version), "--version", version],
            receipts,
        )
    extension = "rpm" if distribution == "fedora" else "deb"
    package_v0 = next((package_root / "0.0.0").glob(f"*.{extension}"))
    package_v1 = next((package_root / "0.0.1").glob(f"*.{extension}"))
    lane(
        "package",
        start,
        package_format=extension,
        candidate_sha256=[sha256_file(package_v0), sha256_file(package_v1)],
    )

    broken = package_root / f"broken.{extension}"
    broken.write_bytes(package_v1.read_bytes()[:1024])
    start = len(receipts)
    run("lifecycle", package_arguments(distribution, "install", package_v0), receipts)
    verify_installed(distribution, "0.0.0", "lifecycle", receipts)
    run("lifecycle", package_arguments(distribution, "upgrade", broken), receipts, expected_success=False)
    verify_installed(distribution, "0.0.0", "lifecycle", receipts)
    run("lifecycle", package_arguments(distribution, "upgrade", package_v1), receipts)
    verify_installed(distribution, "0.0.1", "lifecycle", receipts)
    run("lifecycle", package_arguments(distribution, "rollback", package_v0), receipts)
    verify_installed(distribution, "0.0.0", "lifecycle", receipts)
    lane("lifecycle", start, corrupt_upgrade_refused=True, prior_state_preserved=True)

    start = len(receipts)
    run("security", ["npm", "run", "strict-local-source:check"], receipts)
    run("security", ["npm", "run", "hostile-network:check"], receipts)
    run("security", ["cargo", "test", "-p", "agentmage-platform-linux", "--locked", "--offline"], receipts, timeout=3600)
    lane("security", start)

    start = len(receipts)
    run("accessibility", ["npm", "run", "test", "--workspace", "@agentmage/vscode-shell"], receipts)
    run(
        "accessibility",
        ["cargo", "test", "-p", "agentmage-kernel-engine", "document_control", "--locked", "--offline"],
        receipts,
    )
    lane("accessibility", start, scope="automated-contracts-only", human_review=False)

    start = len(receipts)
    run("recovery", package_arguments(distribution, "remove"), receipts)
    verify_removed("recovery", receipts)
    run("recovery", package_arguments(distribution, "install", package_v1), receipts)
    verify_installed(distribution, "0.0.1", "recovery", receipts)
    lane("recovery", start, clean_reinstall=True)

    start = len(receipts)
    run("removal", package_arguments(distribution, "remove"), receipts)
    run("removal", package_arguments(distribution, "absent"), receipts, expected_success=False)
    verify_removed("removal", receipts)
    lane("removal", start, package_record_absent=True, filesystem_residue_absent=True)

    start = len(receipts)
    run(
        "native-runtime",
        ["python3", "scripts/native_llama_runtime.py", "--archive", str(native_archive)],
        receipts,
    )
    native_package = ROOT / "release-output/agentmage-llama-cpp-b10333-cpu-linux-x86_64.tar.gz"
    run(
        "native-runtime",
        ["python3", "scripts/native_llama_runtime.py", "--verify-package", str(native_package)],
        receipts,
    )
    lane("native-runtime", start, adapter="native-llama-cpp", package_sha256=sha256_file(native_package))

    start = len(receipts)
    run("docker-compatibility", ["sudo", "docker", "version", "--format", "{{.Server.Version}}"], receipts)
    run(
        "docker-compatibility",
        ["sudo", "docker", "image", "inspect", f"docker.io/docker/model-runner@{RUNNER_DIGEST}"],
        receipts,
    )
    model_script = (
        "set -eu; root=$(sudo docker volume inspect agentmage-models --format '{{.Mountpoint}}'); "
        f"base=$root/bundles/sha256/{MODEL_DIGEST}/model; "
        + "; ".join(
            f'test "$(sudo sha256sum $base/{name} | cut -d" " -f1)" = "{digest}"'
            for name, digest in MODEL_FILES.items()
        )
    )
    run("docker-compatibility", ["bash", "-c", model_script], receipts)
    run(
        "docker-compatibility",
        ["cargo", "test", "-p", "agentmage-platform-linux", "docker_", "--locked", "--offline"],
        receipts,
    )
    lane(
        "docker-compatibility",
        start,
        adapter="docker-model-runner-compatibility",
        model_inference=False,
        separate_from_native=True,
    )

    if [item["id"] for item in lanes] != list(LANE_ORDER):
        raise PromotedGuestError("promoted guest lane order drifted")
    return {
        "schema_version": 1,
        "record_type": "agentmage-linux-promoted-guest-result",
        "distribution": distribution,
        "status": "pass",
        "network_used": False,
        "lanes": lanes,
        "command_receipts": receipts,
        "native_and_docker_results_distinct": True,
        "model_inference_executed": False,
        "human_accessibility_review_claim": False,
        "release_claim": False,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--distribution", choices=("fedora", "ubuntu"), required=True)
    parser.add_argument("--native-archive", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()
    try:
        result = execute(arguments.distribution, arguments.native_archive.resolve())
        arguments.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"promoted Linux guest matrix failed: {error}", file=os.sys.stderr)
        return 1
    print("promoted Linux guest matrix passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
