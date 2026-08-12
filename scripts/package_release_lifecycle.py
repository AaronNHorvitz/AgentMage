#!/usr/bin/env python3
"""Exercise deterministic signed-package mechanics without retaining a key."""

from __future__ import annotations

import hashlib
import json
import shutil
import subprocess
import tempfile
from pathlib import Path

try:
    from scripts.package_candidate import ROOT, MANIFEST_PATH, build_release_bundle
    from scripts.package_lifecycle import extract_deb, extract_rpm
except ModuleNotFoundError:
    from package_candidate import ROOT, MANIFEST_PATH, build_release_bundle
    from package_lifecycle import extract_deb, extract_rpm


PRIVATE_KEY_DER_PREFIX = bytes.fromhex("302e020100300506032b657004220420")


class ReleaseLifecycleError(ValueError):
    """Raised when signed-package lifecycle behavior drifts."""


def run(arguments: list[str], *, input_bytes: bytes | None = None, success: bool = True) -> None:
    result = subprocess.run(
        arguments,
        cwd=ROOT,
        input=input_bytes,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    if (result.returncode == 0) != success:
        raise ReleaseLifecycleError(
            f"package.release.command_result:{Path(arguments[0]).name}:"
            f"exit={result.returncode}:expected_success={str(success).lower()}"
        )


def raw_public_key(seed: bytes) -> bytes:
    result = subprocess.run(
        ["openssl", "pkey", "-inform", "DER", "-pubout", "-outform", "DER"],
        input=PRIVATE_KEY_DER_PREFIX + seed,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0 or len(result.stdout) != 44:
        raise ReleaseLifecycleError("package.release.public_key_derivation_failed")
    return result.stdout[-32:]


def compare_artifacts(first: dict[str, Path], second: dict[str, Path]) -> None:
    if set(first) != {"manifest", "vsix", "deb", "rpm"} or set(first) != set(second):
        raise ReleaseLifecycleError("package.release.artifact_set")
    for name in sorted(first):
        if first[name].read_bytes() != second[name].read_bytes():
            raise ReleaseLifecycleError(f"package.release.nondeterministic:{name}")


def main() -> int:
    required = ("ar", "cargo", "cpio", "openssl", "rpm2cpio", "tar")
    if any(shutil.which(command) is None for command in required):
        raise ReleaseLifecycleError("package.release.tool_unavailable")
    run(["npm", "run", "build", "--workspace", "@agentmage/vscode-shell"])
    run(["cargo", "build", "--release", "-p", "agentmage-host", "--locked"])
    run(
        [
            "cargo",
            "build",
            "--release",
            "-p",
            "agentmage-platform-linux-inference",
            "--bin",
            "agentmage-native-inference",
            "--locked",
        ]
    )
    host = ROOT / "target/release/agentmage-host"
    seed = hashlib.sha256(b"agentmage synthetic release lifecycle v1").digest()
    wrong_seed = hashlib.sha256(b"agentmage synthetic wrong release key v1").digest()
    with tempfile.TemporaryDirectory(prefix="agentmage-signed-release-") as directory:
        temporary = Path(directory)
        first = build_release_bundle(temporary / "first", "0.1.0", 1)
        second = build_release_bundle(temporary / "second", "0.1.0", 1)
        compare_artifacts(first, second)
        public_key = temporary / "release.pub"
        wrong_key = temporary / "wrong.pub"
        signature = temporary / "release.sig"
        public_key.write_bytes(raw_public_key(seed))
        wrong_key.write_bytes(raw_public_key(wrong_seed))
        run(
            [
                "cargo",
                "run",
                "-p",
                "agentmage-xtask",
                "--locked",
                "--",
                "sign-package-manifest",
                "--manifest",
                str(first["manifest"]),
                "--public-key",
                str(public_key),
                "--signature",
                str(signature),
            ],
            input_bytes=seed,
        )
        roots = {"rpm": temporary / "rpm", "deb": temporary / "deb"}
        for root in roots.values():
            root.mkdir()
        extract_rpm(first["rpm"], roots["rpm"])
        extract_deb(first["deb"], roots["deb"])
        manifests = [root / MANIFEST_PATH for root in roots.values()]
        if any(path.read_bytes() != first["manifest"].read_bytes() for path in manifests):
            raise ReleaseLifecycleError("package.release.cross_format_manifest_drift")
        for root in roots.values():
            command = [
                str(host),
                "--verify-package-root",
                str(root),
                "--signature",
                str(signature),
                "--public-key",
                str(public_key),
            ]
            run(command)
            wrong = command[:-1] + [str(wrong_key)]
            run(wrong, success=False)
            run([str(host), "--verify-package-candidate-root", str(root)], success=False)
        deb_manifest = roots["deb"] / MANIFEST_PATH
        value = json.loads(deb_manifest.read_text(encoding="utf-8"))
        value["release_sequence"] = 2
        deb_manifest.write_text(json.dumps(value), encoding="utf-8")
        run(
            [
                str(host),
                "--verify-package-root",
                str(roots["deb"]),
                "--signature",
                str(signature),
                "--public-key",
                str(public_key),
            ],
            success=False,
        )
    print(
        json.dumps(
            {
                "artifact_determinism": "pass",
                "candidate_release_separation": "pass",
                "cross_format_identity": "pass",
                "detached_signature_verification": "pass",
                "key_material_retained": False,
                "manifest_mutation_refusal": "pass",
                "wrong_trust_root_refusal": "pass",
            },
            sort_keys=True,
            separators=(",", ":"),
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
