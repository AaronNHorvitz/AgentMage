#!/usr/bin/env python3
"""Build deterministic unsigned AgentMage RPM, DEB, and VSIX candidates."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import os
import shutil
import stat
import struct
import subprocess
import tarfile
import tempfile
import zipfile
from pathlib import Path, PurePosixPath
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
VERSION: Final = "0.0.0"
MANIFEST_PATH: Final = PurePosixPath("usr/share/agentmage/package-manifest.json")
PAYLOAD_FILES: Final = (
    PurePosixPath("usr/libexec/agentmage/agentmage-host"),
    PurePosixPath("usr/share/agentmage/agentmage.vsix"),
    PurePosixPath("usr/share/licenses/agentmage/LICENSE"),
)


class PackageCandidateError(ValueError):
    """Raised when candidate inputs or package output fail closed."""


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def require_regular(path: Path, maximum: int = 512 * 1024 * 1024) -> None:
    try:
        metadata = path.lstat()
    except OSError as error:
        raise PackageCandidateError("package.input_unavailable") from error
    if not stat.S_ISREG(metadata.st_mode) or metadata.st_size > maximum:
        raise PackageCandidateError("package.input_denied")


def zip_entry(archive: zipfile.ZipFile, name: str, data: bytes, mode: int) -> None:
    info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
    info.compress_type = zipfile.ZIP_DEFLATED
    info.create_system = 3
    info.external_attr = (stat.S_IFREG | mode) << 16
    archive.writestr(info, data)


def package_id(version: str) -> str:
    return f"agentmage-linux-x86_64-{version}-candidate"


def valid_version(version: str) -> bool:
    parts = version.split(".")
    return len(parts) == 3 and all(part.isdigit() and str(int(part)) == part for part in parts)


def build_vsix(
    extension_root: Path, license_path: Path, output: Path, version: str = VERSION
) -> None:
    package_path = extension_root / "package.json"
    require_regular(package_path, 1024 * 1024)
    package = json.loads(package_path.read_text(encoding="utf-8"))
    required = {
        "name": "@agentmage/vscode-shell",
        "publisher": "agentmage-project",
        "main": "./dist/src/extension.js",
    }
    if any(package.get(key) != value for key, value in required.items()):
        raise PackageCandidateError("package.vsix_manifest_invalid")
    if not valid_version(version):
        raise PackageCandidateError("package.version_invalid")
    package["name"] = "agentmage-vscode-shell"
    package["version"] = version
    files: list[tuple[str, bytes, int]] = [
        ("extension/package.json", canonical_json(package), 0o644),
        ("extension/LICENSE", license_path.read_bytes(), 0o644),
    ]
    dist_root = extension_root / "dist/src"
    for path in sorted(dist_root.rglob("*.js")):
        if path.is_file() and not path.is_symlink():
            relative = path.relative_to(extension_root).as_posix()
            files.append((f"extension/{relative}", path.read_bytes(), 0o644))
    if not any(name == "extension/dist/src/extension.js" for name, _, _ in files):
        raise PackageCandidateError("package.vsix_entrypoint_missing")
    identity = "agentmage-project.agentmage-vscode-shell"
    manifest = f"""<?xml version="1.0" encoding="utf-8"?>
<PackageManifest Version="2.0.0" xmlns="http://schemas.microsoft.com/developer/vsx-schema/2011">
  <Metadata>
    <Identity Language="en-US" Id="{identity}" Version="{version}" Publisher="agentmage-project" />
    <DisplayName>AgentMage</DisplayName>
    <Description xml:space="preserve">Strict-local AgentMage provider candidate.</Description>
  </Metadata>
  <Installation><InstallationTarget Id="Microsoft.VisualStudio.Code" Version="[1.125.0,)" /></Installation>
  <Dependencies />
  <Assets><Asset Type="Microsoft.VisualStudio.Code.Manifest" Path="extension/package.json" Addressable="true" /></Assets>
</PackageManifest>
""".encode()
    content_types = b"""<?xml version="1.0" encoding="utf-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="json" ContentType="application/json" />
  <Default Extension="js" ContentType="application/javascript" />
  <Default Extension="ts" ContentType="text/plain" />
  <Default Extension="md" ContentType="text/markdown" />
  <Default Extension="vsixmanifest" ContentType="text/xml" />
</Types>
"""
    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(output, "w") as archive:
        zip_entry(archive, "extension.vsixmanifest", manifest, 0o644)
        zip_entry(archive, "[Content_Types].xml", content_types, 0o644)
        for name, data, mode in sorted(files):
            zip_entry(archive, name, data, mode)


def build_payload(
    host: Path,
    vsix: Path,
    license_path: Path,
    root: Path,
    version: str = VERSION,
) -> dict[str, Any]:
    for path in (host, vsix, license_path):
        require_regular(path)
    destinations = {
        PAYLOAD_FILES[0]: host,
        PAYLOAD_FILES[1]: vsix,
        PAYLOAD_FILES[2]: license_path,
    }
    records = []
    for relative, source in destinations.items():
        destination = root / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, destination)
        mode = 0o755 if relative == PAYLOAD_FILES[0] else 0o644
        destination.chmod(mode)
        records.append(
            {
                "path": relative.as_posix(),
                "sha256": sha256_file(destination),
                "size": destination.stat().st_size,
                "mode": mode,
            }
        )
    manifest = {
        "schema_version": 1,
        "record_type": "agentmage-package-manifest",
        "status": "unsigned-candidate",
        "package_id": package_id(version),
        "files": sorted(records, key=lambda item: item["path"]),
    }
    manifest_path = root / MANIFEST_PATH
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    manifest_path.write_bytes(canonical_json(manifest))
    manifest_path.chmod(0o644)
    return manifest


def tar_bytes(root: Path, members: list[PurePosixPath]) -> bytes:
    output = io.BytesIO()
    with tarfile.open(fileobj=output, mode="w", format=tarfile.PAX_FORMAT) as archive:
        for relative in sorted(members):
            path = root / relative
            info = archive.gettarinfo(str(path), arcname=relative.as_posix())
            info.uid = info.gid = 0
            info.uname = info.gname = "root"
            info.mtime = 0
            with path.open("rb") if path.is_file() else io.BytesIO() as stream:
                archive.addfile(info, stream if path.is_file() else None)
    return output.getvalue()


def gzip_bytes(data: bytes) -> bytes:
    output = io.BytesIO()
    with gzip.GzipFile(fileobj=output, mode="wb", filename="", mtime=0) as stream:
        stream.write(data)
    return output.getvalue()


def payload_members(root: Path) -> list[PurePosixPath]:
    return [
        PurePosixPath(path.relative_to(root).as_posix())
        for path in root.rglob("*")
        if path.is_dir() or path.is_file()
    ]


def ar_member(name: str, data: bytes) -> bytes:
    if len(name) > 15:
        raise PackageCandidateError("package.deb_member_name")
    header = f"{name + '/':<16}{0:<12}{0:<6}{0:<6}{0o100644:<8o}{len(data):<10}`\n".encode()
    if len(header) != 60:
        raise PackageCandidateError("package.deb_header")
    return header + data + (b"\n" if len(data) % 2 else b"")


def build_deb(
    payload_root: Path, template: Path, output: Path, version: str = VERSION
) -> None:
    control = template.read_text(encoding="utf-8").replace("@VERSION@", version).encode()
    with tempfile.TemporaryDirectory(prefix="agentmage-control-") as directory:
        control_root = Path(directory)
        (control_root / "control").write_bytes(control)
        control_tar = gzip_bytes(tar_bytes(control_root, [PurePosixPath("control")]))
    data_tar = gzip_bytes(tar_bytes(payload_root, payload_members(payload_root)))
    output.write_bytes(
        b"!<arch>\n"
        + ar_member("debian-binary", b"2.0\n")
        + ar_member("control.tar.gz", control_tar)
        + ar_member("data.tar.gz", data_tar)
    )


def build_rpm(
    payload_root: Path,
    template: Path,
    output_dir: Path,
    version: str = VERSION,
) -> Path:
    with tempfile.TemporaryDirectory(prefix="agentmage-rpmbuild-") as directory:
        top = Path(directory)
        for name in ("BUILD", "BUILDROOT", "RPMS", "SOURCES", "SPECS", "SRPMS"):
            (top / name).mkdir()
        source = top / "SOURCES" / f"agentmage-{version}-payload.tar.gz"
        source.write_bytes(gzip_bytes(tar_bytes(payload_root, payload_members(payload_root))))
        spec = template.read_text(encoding="utf-8").replace("@VERSION@", version)
        spec_path = top / "SPECS" / "agentmage.spec"
        spec_path.write_text(spec, encoding="utf-8")
        subprocess.run(
            [
                "rpmbuild",
                "--define",
                f"_topdir {top}",
                "--define",
                "_buildhost builder.agentmage.invalid",
                "--define",
                "use_source_date_epoch_as_buildtime 1",
                "--define",
                "clamp_mtime_to_source_date_epoch 1",
                "-bb",
                str(spec_path),
            ],
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            env={**os.environ, "SOURCE_DATE_EPOCH": "1786492800"},
        )
        candidates = sorted((top / "RPMS").rglob("*.rpm"))
        if len(candidates) != 1:
            raise PackageCandidateError("package.rpm_output")
        output = output_dir / candidates[0].name
        shutil.copyfile(candidates[0], output)
        return output


def verify_payload(
    root: Path,
    expected_status: str = "unsigned-candidate",
    version: str = VERSION,
) -> None:
    manifest_path = root / MANIFEST_PATH
    require_regular(manifest_path, 1024 * 1024)
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if set(manifest) != {"schema_version", "record_type", "status", "package_id", "files"}:
        raise PackageCandidateError("package.manifest_fields")
    if (
        manifest["schema_version"] != 1
        or manifest["record_type"] != "agentmage-package-manifest"
        or manifest["status"] != expected_status
        or manifest["package_id"] != package_id(version)
    ):
        raise PackageCandidateError("package.manifest_identity")
    paths = [record.get("path") for record in manifest["files"]]
    if paths != sorted(set(paths)) or paths != [path.as_posix() for path in PAYLOAD_FILES]:
        raise PackageCandidateError("package.manifest_file_set")
    for record in manifest["files"]:
        path = root / PurePosixPath(record["path"])
        require_regular(path)
        metadata = path.stat()
        if (
            metadata.st_size != record.get("size")
            or stat.S_IMODE(metadata.st_mode) != record.get("mode")
            or sha256_file(path) != record.get("sha256")
        ):
            raise PackageCandidateError("package.file_mismatch")


def build_all(output: Path, version: str = VERSION) -> dict[str, Path]:
    if not valid_version(version):
        raise PackageCandidateError("package.version_invalid")
    host = ROOT / "target/release/agentmage-host"
    extension = ROOT / "shells/vscode"
    license_path = ROOT / "LICENSE"
    output.mkdir(parents=True, exist_ok=True)
    vsix = output / f"agentmage-vscode-{version}.vsix"
    build_vsix(extension, license_path, vsix, version)
    with tempfile.TemporaryDirectory(prefix="agentmage-payload-") as directory:
        payload_root = Path(directory)
        build_payload(host, vsix, license_path, payload_root, version)
        verify_payload(payload_root, version=version)
        deb = output / f"agentmage_{version}_amd64.deb"
        build_deb(payload_root, ROOT / "packaging/linux/debian-control.in", deb, version)
        rpm = build_rpm(
            payload_root, ROOT / "packaging/linux/agentmage.spec.in", output, version
        )
    return {"vsix": vsix, "deb": deb, "rpm": rpm}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "release-output")
    parser.add_argument("--version", default=VERSION)
    args = parser.parse_args(argv)
    try:
        artifacts = build_all(args.output.resolve(), args.version)
    except (OSError, ValueError, subprocess.SubprocessError, zipfile.BadZipFile) as error:
        print(f"package candidate failed: {error}", file=os.sys.stderr)
        return 1
    print(canonical_json({name: str(path) for name, path in sorted(artifacts.items())}).decode(), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
