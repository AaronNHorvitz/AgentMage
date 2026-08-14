#!/usr/bin/env python3
"""Build and verify the closed Muse-capable llama.cpp runtime package."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import os
import stat
import tarfile
from pathlib import Path, PurePosixPath
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
PROFILE_PATH: Final = ROOT / "model-profiles/runtimes/llama-cpp-b10423-muse-linux-x86_64.json"
DEFAULT_OUTPUT: Final = ROOT / "release-output/agentmage-llama-cpp-b10423-muse-vulkan-linux-x86_64.tar.gz"
MAX_ARCHIVE_BYTES: Final = 64 * 1024 * 1024
MAX_EXPANDED_BYTES: Final = 128 * 1024 * 1024
MAX_MEMBERS: Final = 128


class MuseRuntimeError(ValueError):
    """The source, profile, or package violated the closed runtime contract."""


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def safe_path(value: str) -> PurePosixPath:
    path = PurePosixPath(value)
    if not value or value.startswith("/") or "\\" in value or any(part in {"", ".", ".."} for part in path.parts):
        raise MuseRuntimeError("muse-runtime.path-invalid")
    return path


def load_profile(path: Path = PROFILE_PATH) -> tuple[dict[str, Any], str]:
    raw = path.read_bytes()
    profile = json.loads(raw)
    validate_profile(profile)
    return profile, sha256_bytes(raw)


def validate_profile(profile: Any) -> None:
    expected = {
        "schema_version", "record_type", "project", "release", "source_commit", "platform",
        "architecture", "backend", "upstream_license", "authority", "listener", "model_store",
        "package", "resource_ceiling", "source_archive", "source_files", "decision",
    }
    if not isinstance(profile, dict) or set(profile) != expected:
        raise MuseRuntimeError("muse-runtime.profile-fields")
    if (
        profile["schema_version"] != 1
        or profile["record_type"] != "linux_muse_runtime_package_profile"
        or profile["release"] != "b10423"
        or profile["source_commit"] != "a94d563ed801d1da1b8c2432946de07d0231bb3d"
        or profile["backend"] != "vulkan-with-x86-cpu-fallback"
    ):
        raise MuseRuntimeError("muse-runtime.profile-identity")
    if profile["authority"] != {
        "credential": False, "egress": False, "grant": False, "shell": False,
        "tool": False, "workspace": False,
    }:
        raise MuseRuntimeError("muse-runtime.profile-authority")
    if profile["listener"] != {
        "kind": "private-loopback-behind-authenticated-agentmage-adapter",
        "non_loopback_bind": False,
        "public_reachability": False,
        "runtime_selectable_host": False,
        "runtime_selectable_port": False,
    }:
        raise MuseRuntimeError("muse-runtime.profile-listener")
    if profile["decision"] != {
        "enabled_models": 0,
        "inference_implemented": False,
        "release_approval": False,
        "status": "PACKAGE_INPUT_PINNED_NOT_ACTIVATED",
    }:
        raise MuseRuntimeError("muse-runtime.profile-decision")
    archive = profile["source_archive"]
    if archive != {
        "download_url": "https://github.com/ggml-org/llama.cpp/releases/download/b10423/llama-b10423-bin-ubuntu-vulkan-x64.tar.gz",
        "format": "tar-gzip",
        "name": "llama-b10423-bin-ubuntu-vulkan-x64.tar.gz",
        "root": "llama-b10423",
        "sha256": "3b1194ef38f4b02b6329d698e29532435a5a7c3567c84b8bb822459ca0893286",
        "size_bytes": 32_989_764,
    }:
        raise MuseRuntimeError("muse-runtime.profile-archive")
    records = profile["source_files"]
    if not isinstance(records, list) or len(records) != 20:
        raise MuseRuntimeError("muse-runtime.profile-file-count")
    destinations: set[str] = set()
    sources: set[str] = set()
    for record in records:
        if not isinstance(record, dict) or record.get("type") not in {"file", "symlink"}:
            raise MuseRuntimeError("muse-runtime.profile-file")
        destination = safe_path(str(record.get("destination", ""))).as_posix()
        source = safe_path(str(record.get("source", ""))).as_posix()
        if destination in destinations or source in sources or not source.startswith("llama-b10423/"):
            raise MuseRuntimeError("muse-runtime.profile-file-duplicate")
        destinations.add(destination)
        sources.add(source)
        if record["type"] == "file":
            if set(record) != {"destination", "mode", "sha256", "size_bytes", "source", "type"}:
                raise MuseRuntimeError("muse-runtime.profile-file-fields")
            if record["mode"] not in {0o444, 0o555} or len(str(record["sha256"])) != 64:
                raise MuseRuntimeError("muse-runtime.profile-file-identity")
        else:
            if set(record) != {"destination", "mode", "source", "target", "type"} or record["mode"] != 0o777:
                raise MuseRuntimeError("muse-runtime.profile-link-fields")
            target = str(record["target"])
            if "/" in target or "\\" in target or target in {"", ".", ".."}:
                raise MuseRuntimeError("muse-runtime.profile-link-target")
            if PurePosixPath(destination).parent.joinpath(target).as_posix() not in destinations | {
                item["destination"] for item in records
            }:
                raise MuseRuntimeError("muse-runtime.profile-link-closure")
    prohibited = ("rpc", "download", "quantize", "bench", "completion", "tokenize", "imatrix")
    if any(term in destination.lower() for term in prohibited for destination in destinations):
        raise MuseRuntimeError("muse-runtime.profile-prohibited-entrypoint")
    if "bin/llama-server" not in destinations or "lib/libggml-vulkan.so" not in destinations:
        raise MuseRuntimeError("muse-runtime.profile-required-file")


def read_payload(path: Path, profile: dict[str, Any]) -> dict[str, bytes | str]:
    archive_id = profile["source_archive"]
    metadata = path.stat(follow_symlinks=False)
    if not stat.S_ISREG(metadata.st_mode) or path.is_symlink():
        raise MuseRuntimeError("muse-runtime.archive-type")
    if metadata.st_size != archive_id["size_bytes"] or sha256_file(path) != archive_id["sha256"]:
        raise MuseRuntimeError("muse-runtime.archive-identity")
    expected = {record["source"]: record for record in profile["source_files"]}
    payload: dict[str, bytes | str] = {}
    expanded = 0
    with tarfile.open(path, mode="r:gz") as archive:
        members = archive.getmembers()
        if not 0 < len(members) <= MAX_MEMBERS:
            raise MuseRuntimeError("muse-runtime.archive-member-count")
        seen: set[str] = set()
        for member in members:
            name = safe_path(member.name.rstrip("/")).as_posix()
            if name in seen or not name.startswith(f"{archive_id['root']}/") and name != archive_id["root"]:
                raise MuseRuntimeError("muse-runtime.archive-member-name")
            seen.add(name)
            if member.isdir():
                continue
            if member.isreg():
                expanded += member.size
                if expanded > MAX_EXPANDED_BYTES:
                    raise MuseRuntimeError("muse-runtime.archive-expanded-limit")
                record = expected.get(name)
                if record is None:
                    continue
                if record["type"] != "file" or member.size != record["size_bytes"]:
                    raise MuseRuntimeError("muse-runtime.archive-file-identity")
                stream = archive.extractfile(member)
                content = stream.read() if stream is not None else b""
                if sha256_bytes(content) != record["sha256"]:
                    raise MuseRuntimeError("muse-runtime.archive-file-identity")
                payload[name] = content
            elif member.issym():
                if "/" in member.linkname or "\\" in member.linkname or member.linkname in {"", ".", ".."}:
                    raise MuseRuntimeError("muse-runtime.archive-link")
                record = expected.get(name)
                if record is not None:
                    if record["type"] != "symlink" or member.linkname != record["target"]:
                        raise MuseRuntimeError("muse-runtime.archive-link-identity")
                    payload[name] = member.linkname
            else:
                raise MuseRuntimeError("muse-runtime.archive-member-type")
    if set(payload) != set(expected):
        raise MuseRuntimeError("muse-runtime.archive-file-closure")
    return payload


def canonical(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def manifest(profile: dict[str, Any], profile_sha256: str) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "agentmage_muse_runtime_manifest",
        "package_id": profile["package"]["package_id"],
        "profile_sha256": profile_sha256,
        "source_commit": profile["source_commit"],
        "source_archive_sha256": profile["source_archive"]["sha256"],
        "authority": profile["authority"],
        "listener": profile["listener"],
        "resource_ceiling": profile["resource_ceiling"],
        "entrypoints": ["bin/llama-server"],
        "enabled_models": 0,
        "inference_implemented": False,
        "release_approval": False,
        "files": [
            {key: record[key] for key in (
                ("destination", "mode", "sha256", "size_bytes", "type")
                if record["type"] == "file"
                else ("destination", "mode", "target", "type")
            )}
            for record in profile["source_files"]
        ],
    }


def _info(name: str, mode: int, kind: bytes = tarfile.REGTYPE) -> tarfile.TarInfo:
    info = tarfile.TarInfo(name)
    info.type = kind
    info.mode = mode
    info.uid = info.gid = info.mtime = 0
    info.uname = info.gname = ""
    return info


def build_bytes(profile: dict[str, Any], profile_sha256: str, payload: dict[str, bytes | str]) -> bytes:
    root = profile["package"]["relative_install_root"]
    output = io.BytesIO()
    with tarfile.open(fileobj=output, mode="w", format=tarfile.USTAR_FORMAT) as archive:
        manifest_bytes = canonical(manifest(profile, profile_sha256))
        item = _info(f"{root}/runtime-manifest.json", 0o444)
        item.size = len(manifest_bytes)
        archive.addfile(item, io.BytesIO(manifest_bytes))
        for record in profile["source_files"]:
            value = payload[record["source"]]
            item = _info(f"{root}/{record['destination']}", record["mode"], tarfile.SYMTYPE if record["type"] == "symlink" else tarfile.REGTYPE)
            if record["type"] == "symlink":
                item.linkname = str(value)
                archive.addfile(item)
            else:
                content = bytes(value)
                item.size = len(content)
                archive.addfile(item, io.BytesIO(content))
        # Directory entries come last so an unprivileged general-purpose
        # extractor can create payload files before the tree becomes read-only.
        for directory, mode in (
            (f"{root}/bin", 0o555),
            (f"{root}/lib", 0o555),
            (root, 0o555),
            ("runtimes", 0o700),
        ):
            archive.addfile(_info(directory, mode, tarfile.DIRTYPE))
    compressed = io.BytesIO()
    with gzip.GzipFile(fileobj=compressed, mode="wb", filename="", mtime=0, compresslevel=9) as stream:
        stream.write(output.getvalue())
    return compressed.getvalue()


def verify_package(path: Path, profile: dict[str, Any], profile_sha256: str) -> dict[str, Any]:
    expected_root = profile["package"]["relative_install_root"]
    expected_manifest = manifest(profile, profile_sha256)
    files = {record["destination"]: record for record in profile["source_files"]}
    observed: set[str] = set()
    with tarfile.open(path, mode="r:gz") as archive:
        for item in archive.getmembers():
            name = safe_path(item.name).as_posix()
            if item.isdir():
                continue
            prefix = f"{expected_root}/"
            if not name.startswith(prefix):
                raise MuseRuntimeError("muse-runtime.package-root")
            relative = name[len(prefix):]
            if relative == "runtime-manifest.json":
                stream = archive.extractfile(item)
                if stream is None or item.mode != 0o444 or json.loads(stream.read()) != expected_manifest:
                    raise MuseRuntimeError("muse-runtime.package-manifest")
                observed.add(relative)
                continue
            record = files.get(relative)
            if record is None or relative in observed:
                raise MuseRuntimeError("muse-runtime.package-file-closure")
            observed.add(relative)
            if record["type"] == "symlink":
                if not item.issym() or item.mode != record["mode"] or item.linkname != record["target"]:
                    raise MuseRuntimeError("muse-runtime.package-link")
            else:
                stream = archive.extractfile(item)
                content = stream.read() if stream is not None else b""
                if (
                    not item.isreg()
                    or item.mode != record["mode"]
                    or len(content) != record["size_bytes"]
                    or sha256_bytes(content) != record["sha256"]
                ):
                    raise MuseRuntimeError("muse-runtime.package-file")
    if observed != set(files) | {"runtime-manifest.json"}:
        raise MuseRuntimeError("muse-runtime.package-file-closure")
    return {
        "package_id": profile["package"]["package_id"],
        "package_sha256": sha256_file(path),
        "package_bytes": path.stat().st_size,
        "source_profile_sha256": profile_sha256,
        "source_commit": profile["source_commit"],
        "entrypoints": ["bin/llama-server"],
        "enabled_models": 0,
        "inference_implemented": False,
        "release_approval": False,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--archive", type=Path)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--verify-package", type=Path)
    args = parser.parse_args()
    profile, profile_sha256 = load_profile()
    if args.verify_package:
        print(json.dumps(verify_package(args.verify_package, profile, profile_sha256), indent=2, sort_keys=True))
        return 0
    if args.archive is None:
        parser.error("--archive is required when building")
    payload = read_payload(args.archive, profile)
    package = build_bytes(profile, profile_sha256, payload)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(package)
    print(json.dumps(verify_package(args.output, profile, profile_sha256), indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
