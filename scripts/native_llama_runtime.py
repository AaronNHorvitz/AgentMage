#!/usr/bin/env python3
"""Build and verify the closed Linux llama.cpp runtime-library package."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import os
import re
import stat
import sys
import tarfile
import tempfile
from pathlib import Path, PurePosixPath
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
PROFILE_PATH: Final = (
    ROOT / "model-profiles/runtimes/llama-cpp-b10333-linux-x86_64.json"
)
DEFAULT_OUTPUT: Final = (
    ROOT
    / "release-output"
    / "agentmage-llama-cpp-b10333-cpu-linux-x86_64.tar.gz"
)
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
MAX_ARCHIVE_BYTES: Final = 64 * 1024 * 1024
MAX_EXPANDED_BYTES: Final = 128 * 1024 * 1024
MAX_ARCHIVE_MEMBERS: Final = 128
EXPECTED_DESTINATIONS: Final = (
    "LICENSE.llama.cpp",
    "lib/libggml-base.so",
    "lib/libggml-base.so.0",
    "lib/libggml-base.so.0.19.0",
    "lib/libggml-cpu-alderlake.so",
    "lib/libggml-cpu-cannonlake.so",
    "lib/libggml-cpu-cascadelake.so",
    "lib/libggml-cpu-cooperlake.so",
    "lib/libggml-cpu-haswell.so",
    "lib/libggml-cpu-icelake.so",
    "lib/libggml-cpu-ivybridge.so",
    "lib/libggml-cpu-piledriver.so",
    "lib/libggml-cpu-sandybridge.so",
    "lib/libggml-cpu-sapphirerapids.so",
    "lib/libggml-cpu-skylakex.so",
    "lib/libggml-cpu-sse42.so",
    "lib/libggml-cpu-x64.so",
    "lib/libggml-cpu-zen4.so",
    "lib/libggml.so",
    "lib/libggml.so.0",
    "lib/libggml.so.0.19.0",
    "lib/libllama.so",
    "lib/libllama.so.0",
    "lib/libllama.so.0.0.10333",
)
PROHIBITED_OUTPUT_TERMS: Final = (
    "server",
    "rpc",
    "cli",
    "bench",
    "completion",
    "quantize",
    "vulkan",
    "mtmd",
    "download",
)


class NativeLlamaRuntimeError(ValueError):
    """Raised when the profile, source archive, or runtime package is unsafe."""


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


def require_regular(path: Path, maximum_bytes: int) -> os.stat_result:
    try:
        metadata = path.lstat()
    except OSError as error:
        raise NativeLlamaRuntimeError("native-runtime.input-unavailable") from error
    if (
        not stat.S_ISREG(metadata.st_mode)
        or metadata.st_nlink < 1
        or metadata.st_size <= 0
        or metadata.st_size > maximum_bytes
    ):
        raise NativeLlamaRuntimeError("native-runtime.input-denied")
    return metadata


def safe_relative(value: str) -> PurePosixPath:
    path = PurePosixPath(value)
    if (
        not value
        or value.startswith("/")
        or "\\" in value
        or any(part in {"", ".", ".."} for part in path.parts)
    ):
        raise NativeLlamaRuntimeError("native-runtime.path-invalid")
    return path


def _validate_source_file(record: Any, root: str) -> None:
    if not isinstance(record, dict):
        raise NativeLlamaRuntimeError("native-runtime.profile-file-invalid")
    kind = record.get("type")
    expected_fields = (
        {"destination", "mode", "sha256", "size_bytes", "source", "type"}
        if kind == "file"
        else {"destination", "mode", "source", "target", "type"}
    )
    if set(record) != expected_fields:
        raise NativeLlamaRuntimeError("native-runtime.profile-file-fields")
    source = safe_relative(str(record.get("source", "")))
    destination = safe_relative(str(record.get("destination", "")))
    if source.parts[0] != root or len(source.parts) < 2:
        raise NativeLlamaRuntimeError("native-runtime.profile-source-root")
    if destination.as_posix() not in EXPECTED_DESTINATIONS:
        raise NativeLlamaRuntimeError("native-runtime.profile-file-closure")
    if kind == "file":
        if (
            record.get("mode") != 0o444
            or not isinstance(record.get("size_bytes"), int)
            or isinstance(record.get("size_bytes"), bool)
            or not 0 < record["size_bytes"] <= MAX_EXPANDED_BYTES
            or SHA256.fullmatch(str(record.get("sha256"))) is None
        ):
            raise NativeLlamaRuntimeError("native-runtime.profile-file-identity")
    elif kind == "symlink":
        target = record.get("target")
        if (
            record.get("mode") != 0o777
            or not isinstance(target, str)
            or not target
            or "/" in target
            or "\\" in target
            or target in {".", ".."}
            or destination.parent.joinpath(target).as_posix()
            not in EXPECTED_DESTINATIONS
        ):
            raise NativeLlamaRuntimeError("native-runtime.profile-link-identity")
    else:
        raise NativeLlamaRuntimeError("native-runtime.profile-file-type")


def validate_profile(profile: Any) -> None:
    expected_fields = {
        "architecture",
        "authority",
        "backend",
        "decision",
        "ipc",
        "model_store",
        "package",
        "platform",
        "project",
        "record_type",
        "release",
        "resource_ceiling",
        "schema_version",
        "source_archive",
        "source_commit",
        "source_files",
        "upstream_license",
    }
    if not isinstance(profile, dict) or set(profile) != expected_fields:
        raise NativeLlamaRuntimeError("native-runtime.profile-fields")
    if (
        profile.get("schema_version") != 1
        or profile.get("record_type") != "linux_native_runtime_package_profile"
        or profile.get("project") != "ggml-org/llama.cpp"
        or profile.get("release") != "b10333"
        or profile.get("source_commit") != "08659901c43b51de735740f1cf61bb82fbe0c4e4"
        or REVISION.fullmatch(str(profile.get("source_commit"))) is None
        or profile.get("platform") != "linux"
        or profile.get("architecture") != "x86_64"
        or profile.get("backend") != "cpu"
        or profile.get("upstream_license") != "MIT"
    ):
        raise NativeLlamaRuntimeError("native-runtime.profile-identity")
    if profile.get("authority") != {
        "credential": False,
        "grant": False,
        "network": False,
        "tool": False,
        "workspace": False,
    }:
        raise NativeLlamaRuntimeError("native-runtime.profile-authority")
    if profile.get("decision") != {
        "enabled_models": 0,
        "inference_implemented": False,
        "release_approval": False,
        "status": "PACKAGE_INPUT_PINNED_NOT_ACTIVATED",
    }:
        raise NativeLlamaRuntimeError("native-runtime.profile-decision")
    if profile.get("ipc") != {
        "client": "kernel-native-inference-adapter",
        "listener": "agentmage-native-inference-only",
        "non_local_bind": False,
        "transport": "authenticated-unix-socket",
    }:
        raise NativeLlamaRuntimeError("native-runtime.profile-ipc")
    if profile.get("model_store") != {
        "directory_enumeration": False,
        "directory_mode": 0o700,
        "input": "kernel-held-read-only-model-descriptors",
        "path_input": False,
        "replacement": "refuse-existing-or-drifted-identity",
        "required_owner": "invoking-standard-user",
        "runtime_write": False,
    }:
        raise NativeLlamaRuntimeError("native-runtime.profile-model-store")
    package = profile.get("package")
    if package != {
        "file_mode": 0o444,
        "format": "deterministic-tar-gzip",
        "license_destination": "LICENSE.llama.cpp",
        "manifest_destination": "runtime-manifest.json",
        "package_id": "agentmage-llama-cpp-b10333-cpu-linux-x86_64",
        "relative_install_root": "runtimes/llama-cpp-b10333-cpu-linux-x86_64",
        "replacement": "no-overwrite",
        "symlink_mode": 0o777,
    }:
        raise NativeLlamaRuntimeError("native-runtime.profile-package")
    if profile.get("resource_ceiling") != {
        "cpu_percent": 3200,
        "memory_bytes": 64 * 1024 * 1024 * 1024,
        "output_bytes": 16 * 1024 * 1024,
        "parallel_slots": 1,
        "runtime_seconds": 3600,
        "swap_bytes": 0,
        "tasks": 64,
    }:
        raise NativeLlamaRuntimeError("native-runtime.profile-resources")
    source_archive = profile.get("source_archive")
    if not isinstance(source_archive, dict) or set(source_archive) != {
        "download_url",
        "format",
        "name",
        "root",
        "sha256",
        "size_bytes",
    }:
        raise NativeLlamaRuntimeError("native-runtime.profile-archive-fields")
    if (
        source_archive.get("name")
        != "llama-b10333-bin-ubuntu-vulkan-x64.tar.gz"
        or source_archive.get("format") != "tar-gzip"
        or source_archive.get("root") != "llama-b10333"
        or source_archive.get("sha256")
        != "f14e312fbee33ce60d2eed7036de5debe31c1d7f4d8f0e37920eb0a2de0854a5"
        or source_archive.get("size_bytes") != 32521550
        or source_archive.get("download_url")
        != "https://github.com/ggml-org/llama.cpp/releases/download/b10333/llama-b10333-bin-ubuntu-vulkan-x64.tar.gz"
    ):
        raise NativeLlamaRuntimeError("native-runtime.profile-archive-identity")
    source_files = profile.get("source_files")
    if not isinstance(source_files, list) or len(source_files) != len(
        EXPECTED_DESTINATIONS
    ):
        raise NativeLlamaRuntimeError("native-runtime.profile-file-count")
    for record in source_files:
        _validate_source_file(record, source_archive["root"])
    destinations = [record["destination"] for record in source_files]
    sources = [record["source"] for record in source_files]
    if (
        destinations != sorted(EXPECTED_DESTINATIONS)
        or len(sources) != len(set(sources))
        or any(term in destination.lower() for term in PROHIBITED_OUTPUT_TERMS for destination in destinations)
    ):
        raise NativeLlamaRuntimeError("native-runtime.profile-file-closure")


def load_profile(path: Path = PROFILE_PATH) -> tuple[dict[str, Any], str]:
    require_regular(path, 1024 * 1024)
    raw = path.read_bytes()
    try:
        profile = json.loads(raw)
    except (UnicodeError, json.JSONDecodeError) as error:
        raise NativeLlamaRuntimeError("native-runtime.profile-invalid-json") from error
    validate_profile(profile)
    return profile, sha256_bytes(raw)


def _safe_archive_name(name: str, root: str) -> str:
    normalized = name.rstrip("/")
    path = safe_relative(normalized)
    if path.parts[0] != root:
        raise NativeLlamaRuntimeError("native-runtime.archive-root")
    return path.as_posix()


def read_source_payload(archive_path: Path, profile: dict[str, Any]) -> dict[str, bytes | str]:
    archive_identity = profile["source_archive"]
    metadata = require_regular(archive_path, MAX_ARCHIVE_BYTES)
    if (
        metadata.st_size != archive_identity["size_bytes"]
        or sha256_file(archive_path) != archive_identity["sha256"]
    ):
        raise NativeLlamaRuntimeError("native-runtime.archive-identity")
    expected = {record["source"]: record for record in profile["source_files"]}
    payload: dict[str, bytes | str] = {}
    seen: set[str] = set()
    expanded = 0
    try:
        with tarfile.open(archive_path, mode="r:gz") as archive:
            members = archive.getmembers()
            if not 0 < len(members) <= MAX_ARCHIVE_MEMBERS:
                raise NativeLlamaRuntimeError("native-runtime.archive-member-count")
            for member in members:
                name = _safe_archive_name(member.name, archive_identity["root"])
                if name in seen:
                    raise NativeLlamaRuntimeError("native-runtime.archive-duplicate")
                seen.add(name)
                if member.isdir():
                    continue
                if member.isreg():
                    expanded += member.size
                    if expanded > MAX_EXPANDED_BYTES:
                        raise NativeLlamaRuntimeError("native-runtime.archive-expanded-limit")
                    record = expected.get(name)
                    if record is None:
                        continue
                    if record["type"] != "file" or member.size != record["size_bytes"]:
                        raise NativeLlamaRuntimeError("native-runtime.archive-file-identity")
                    stream = archive.extractfile(member)
                    if stream is None:
                        raise NativeLlamaRuntimeError("native-runtime.archive-file-unreadable")
                    content = stream.read(record["size_bytes"] + 1)
                    if (
                        len(content) != record["size_bytes"]
                        or sha256_bytes(content) != record["sha256"]
                    ):
                        raise NativeLlamaRuntimeError("native-runtime.archive-file-identity")
                    payload[name] = content
                    continue
                if member.issym():
                    record = expected.get(name)
                    if record is None:
                        if (
                            not member.linkname
                            or "/" in member.linkname
                            or "\\" in member.linkname
                            or member.linkname in {".", ".."}
                        ):
                            raise NativeLlamaRuntimeError("native-runtime.archive-link")
                        continue
                    if (
                        record["type"] != "symlink"
                        or member.linkname != record["target"]
                    ):
                        raise NativeLlamaRuntimeError("native-runtime.archive-link")
                    payload[name] = member.linkname
                    continue
                raise NativeLlamaRuntimeError("native-runtime.archive-member-type")
    except (OSError, tarfile.TarError) as error:
        raise NativeLlamaRuntimeError("native-runtime.archive-invalid") from error
    if set(payload) != set(expected):
        raise NativeLlamaRuntimeError("native-runtime.archive-file-closure")
    return payload


def runtime_manifest(profile: dict[str, Any], profile_sha256: str) -> dict[str, Any]:
    return {
        "architecture": profile["architecture"],
        "authority": profile["authority"],
        "backend": profile["backend"],
        "enabled_models": 0,
        "files": [
            {
                key: record[key]
                for key in (
                    ["destination", "mode", "sha256", "size_bytes", "type"]
                    if record["type"] == "file"
                    else ["destination", "mode", "target", "type"]
                )
            }
            for record in profile["source_files"]
        ],
        "inference_implemented": False,
        "ipc": profile["ipc"],
        "model_store": profile["model_store"],
        "package_id": profile["package"]["package_id"],
        "profile_sha256": profile_sha256,
        "record_type": "agentmage_native_runtime_manifest",
        "release": profile["release"],
        "release_approval": False,
        "resource_ceiling": profile["resource_ceiling"],
        "schema_version": 1,
        "source_archive_sha256": profile["source_archive"]["sha256"],
        "source_commit": profile["source_commit"],
        "upstream_entrypoints_included": [],
        "upstream_license": profile["upstream_license"],
    }


def _tar_info(name: str, mode: int, kind: bytes = tarfile.REGTYPE) -> tarfile.TarInfo:
    info = tarfile.TarInfo(name)
    info.type = kind
    info.mode = mode
    info.uid = 0
    info.gid = 0
    info.uname = ""
    info.gname = ""
    info.mtime = 0
    return info


def build_package_bytes(
    profile: dict[str, Any], profile_sha256: str, payload: dict[str, bytes | str]
) -> bytes:
    package_root = profile["package"]["relative_install_root"]
    output = io.BytesIO()
    with tarfile.open(fileobj=output, mode="w", format=tarfile.USTAR_FORMAT) as archive:
        for directory, mode in (
            ("runtimes", 0o700),
            (package_root, 0o555),
            (f"{package_root}/lib", 0o555),
        ):
            archive.addfile(_tar_info(directory, mode, tarfile.DIRTYPE))
        manifest_content = canonical_json(runtime_manifest(profile, profile_sha256))
        manifest_info = _tar_info(
            f"{package_root}/{profile['package']['manifest_destination']}", 0o444
        )
        manifest_info.size = len(manifest_content)
        archive.addfile(manifest_info, io.BytesIO(manifest_content))
        for record in profile["source_files"]:
            destination = f"{package_root}/{record['destination']}"
            value = payload[record["source"]]
            if record["type"] == "file":
                if not isinstance(value, bytes):
                    raise NativeLlamaRuntimeError("native-runtime.payload-type")
                info = _tar_info(destination, record["mode"])
                info.size = len(value)
                archive.addfile(info, io.BytesIO(value))
            else:
                if not isinstance(value, str):
                    raise NativeLlamaRuntimeError("native-runtime.payload-type")
                info = _tar_info(destination, record["mode"], tarfile.SYMTYPE)
                info.linkname = value
                archive.addfile(info)
    compressed = io.BytesIO()
    with gzip.GzipFile(fileobj=compressed, mode="wb", filename="", mtime=0) as stream:
        stream.write(output.getvalue())
    return compressed.getvalue()


def publish_no_replace(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=".agentmage-native-runtime-", dir=path.parent)
    temporary = Path(name)
    try:
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(content)
            stream.flush()
            os.fsync(stream.fileno())
        temporary.chmod(0o444)
        try:
            os.link(temporary, path)
        except FileExistsError as error:
            raise NativeLlamaRuntimeError("native-runtime.output-exists") from error
        directory = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        temporary.unlink(missing_ok=True)


def build_runtime_package(
    archive_path: Path,
    output_path: Path,
    profile_path: Path = PROFILE_PATH,
) -> dict[str, Any]:
    profile, profile_sha256 = load_profile(profile_path)
    payload = read_source_payload(archive_path, profile)
    package = build_package_bytes(profile, profile_sha256, payload)
    publish_no_replace(output_path, package)
    verify_runtime_package(output_path, profile_path)
    return {
        "bytes": len(package),
        "package_id": profile["package"]["package_id"],
        "path": str(output_path),
        "profile_sha256": profile_sha256,
        "sha256": sha256_bytes(package),
    }


def verify_runtime_package(
    package_path: Path, profile_path: Path = PROFILE_PATH
) -> dict[str, Any]:
    profile, profile_sha256 = load_profile(profile_path)
    return _verify_runtime_package(package_path, profile, profile_sha256)


def _verify_runtime_package(
    package_path: Path, profile: dict[str, Any], profile_sha256: str
) -> dict[str, Any]:
    metadata = require_regular(package_path, MAX_EXPANDED_BYTES)
    package_root = profile["package"]["relative_install_root"]
    expected_names = {
        "runtimes",
        package_root,
        f"{package_root}/lib",
        f"{package_root}/{profile['package']['manifest_destination']}",
        *(
            f"{package_root}/{record['destination']}"
            for record in profile["source_files"]
        ),
    }
    observed: set[str] = set()
    try:
        with tarfile.open(package_path, mode="r:gz") as archive:
            for member in archive.getmembers():
                name = safe_relative(member.name.rstrip("/")).as_posix()
                if name in observed or name not in expected_names:
                    raise NativeLlamaRuntimeError("native-runtime.package-file-closure")
                observed.add(name)
                if member.uid != 0 or member.gid != 0 or member.mtime != 0:
                    raise NativeLlamaRuntimeError("native-runtime.package-metadata")
                if name in {"runtimes", package_root, f"{package_root}/lib"}:
                    expected_mode = 0o700 if name == "runtimes" else 0o555
                    if not member.isdir() or member.mode != expected_mode:
                        raise NativeLlamaRuntimeError("native-runtime.package-directory")
                    continue
                relative = name.removeprefix(f"{package_root}/")
                if relative == profile["package"]["manifest_destination"]:
                    stream = archive.extractfile(member)
                    expected = canonical_json(runtime_manifest(profile, profile_sha256))
                    if (
                        not member.isreg()
                        or member.mode != 0o444
                        or stream is None
                        or stream.read(len(expected) + 1) != expected
                    ):
                        raise NativeLlamaRuntimeError("native-runtime.package-manifest")
                    continue
                record = next(
                    item for item in profile["source_files"] if item["destination"] == relative
                )
                if record["type"] == "file":
                    stream = archive.extractfile(member)
                    if (
                        not member.isreg()
                        or member.mode != record["mode"]
                        or member.size != record["size_bytes"]
                        or stream is None
                    ):
                        raise NativeLlamaRuntimeError("native-runtime.package-file")
                    content = stream.read(record["size_bytes"] + 1)
                    if (
                        len(content) != record["size_bytes"]
                        or sha256_bytes(content) != record["sha256"]
                    ):
                        raise NativeLlamaRuntimeError("native-runtime.package-file")
                elif (
                    not member.issym()
                    or member.mode != record["mode"]
                    or member.linkname != record["target"]
                ):
                    raise NativeLlamaRuntimeError("native-runtime.package-link")
    except (OSError, tarfile.TarError) as error:
        raise NativeLlamaRuntimeError("native-runtime.package-invalid") from error
    if observed != expected_names:
        raise NativeLlamaRuntimeError("native-runtime.package-file-closure")
    if any(term in name.lower() for term in PROHIBITED_OUTPUT_TERMS for name in observed):
        raise NativeLlamaRuntimeError("native-runtime.package-prohibited-surface")
    return {
        "bytes": metadata.st_size,
        "file_count": len(observed),
        "package_id": profile["package"]["package_id"],
        "profile_sha256": profile_sha256,
        "sha256": sha256_file(package_path),
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--profile", type=Path, default=PROFILE_PATH)
    parser.add_argument("--verify-package", type=Path)
    arguments = parser.parse_args(argv)
    try:
        if arguments.verify_package is not None:
            result = verify_runtime_package(
                arguments.verify_package.resolve(), arguments.profile.resolve()
            )
        else:
            if arguments.archive is None:
                parser.error("--archive is required when building a package")
            result = build_runtime_package(
                arguments.archive.resolve(),
                arguments.output.resolve(),
                arguments.profile.resolve(),
            )
    except (NativeLlamaRuntimeError, OSError) as error:
        print(f"native llama runtime package failed: {error}", file=sys.stderr)
        return 1
    print(canonical_json(result).decode("utf-8"), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
