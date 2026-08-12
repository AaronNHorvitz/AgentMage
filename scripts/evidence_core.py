#!/usr/bin/env python3
"""Shared fail-closed primitives for current and historical evidence tooling."""

from __future__ import annotations

import hashlib
import json
import os
import stat
import subprocess
import tempfile
from pathlib import Path, PurePosixPath
from typing import Any, Final


DEFAULT_MAXIMUM_BYTES: Final = 8 * 1024 * 1024
SHA256_LENGTH: Final = 64


class EvidenceError(ValueError):
    """Raised when evidence input cannot be admitted safely."""


def canonical_json_bytes(value: Any) -> bytes:
    """Return deterministic, ASCII-only, newline-terminated JSON bytes."""

    return (
        json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n"
    ).encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    """Return a lowercase SHA-256 digest."""

    return hashlib.sha256(value).hexdigest()


def valid_sha256(value: Any) -> bool:
    """Return whether a value is one exact lowercase SHA-256 digest."""

    return (
        isinstance(value, str)
        and len(value) == SHA256_LENGTH
        and all(character in "0123456789abcdef" for character in value)
    )


def safe_relative_path(value: Any) -> bool:
    """Return whether a string is one normalized repository-relative path."""

    if not isinstance(value, str) or not value or "\\" in value or "\x00" in value:
        return False
    path = PurePosixPath(value)
    return (
        not path.is_absolute()
        and "." not in path.parts
        and ".." not in path.parts
        and str(path) == value
    )


def repository_path(root: Path, relative: Any) -> Path:
    """Resolve a normalized path beneath root without following a final link."""

    if not safe_relative_path(relative):
        raise EvidenceError("evidence.path.invalid")
    resolved_root = root.resolve()
    candidate = resolved_root / str(relative)
    try:
        candidate.parent.resolve(strict=True).relative_to(resolved_root)
    except (FileNotFoundError, ValueError) as error:
        raise EvidenceError("evidence.path.outside_root") from error
    return candidate


def bounded_read(
    root: Path,
    relative: Any,
    *,
    maximum_bytes: int = DEFAULT_MAXIMUM_BYTES,
) -> bytes:
    """Read one in-root regular file without following a final symbolic link."""

    if maximum_bytes <= 0:
        raise EvidenceError("evidence.read.invalid_limit")
    candidate = repository_path(root, relative)
    flags = os.O_RDONLY | getattr(os, "O_CLOEXEC", 0) | getattr(os, "O_NOFOLLOW", 0)
    try:
        descriptor = os.open(candidate, flags)
    except OSError as error:
        raise EvidenceError("evidence.read.unavailable") from error
    try:
        metadata = os.fstat(descriptor)
        if not stat.S_ISREG(metadata.st_mode):
            raise EvidenceError("evidence.read.not_regular")
        if metadata.st_size > maximum_bytes:
            raise EvidenceError("evidence.read.size_exceeded")
        chunks: list[bytes] = []
        total = 0
        while True:
            chunk = os.read(descriptor, min(64 * 1024, maximum_bytes + 1 - total))
            if not chunk:
                break
            chunks.append(chunk)
            total += len(chunk)
            if total > maximum_bytes:
                raise EvidenceError("evidence.read.size_exceeded")
        return b"".join(chunks)
    finally:
        os.close(descriptor)


def read_json_object(
    root: Path,
    relative: Any,
    *,
    maximum_bytes: int = DEFAULT_MAXIMUM_BYTES,
) -> dict[str, Any]:
    """Read one bounded JSON object from a repository-relative path."""

    try:
        value = json.loads(
            bounded_read(root, relative, maximum_bytes=maximum_bytes).decode("utf-8")
        )
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise EvidenceError("evidence.json.malformed") from error
    if not isinstance(value, dict):
        raise EvidenceError("evidence.json.not_object")
    return value


def sha256_file(
    root: Path,
    relative: Any,
    *,
    maximum_bytes: int = DEFAULT_MAXIMUM_BYTES,
) -> str:
    """Hash one bounded repository-relative regular file."""

    return sha256_bytes(bounded_read(root, relative, maximum_bytes=maximum_bytes))


def atomic_write(path: Path, content: bytes, *, mode: int = 0o644) -> None:
    """Synchronize and atomically replace one regular file in its directory."""

    path.parent.mkdir(parents=True, exist_ok=True)
    directory = path.parent.resolve(strict=True)
    descriptor, temporary_name = tempfile.mkstemp(prefix=".agentmage-evidence-", dir=directory)
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        temporary.chmod(mode)
        if path.exists() and path.is_symlink():
            raise EvidenceError("evidence.write.symbolic_link")
        os.replace(temporary, path)
        directory_descriptor = os.open(directory, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
        try:
            os.fsync(directory_descriptor)
        finally:
            os.close(directory_descriptor)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def _run_git(root: Path, arguments: list[str], *, maximum_bytes: int) -> bytes:
    result = subprocess.run(
        ["git", *arguments],
        cwd=root,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=30,
        check=False,
    )
    if result.returncode != 0:
        raise EvidenceError("evidence.git.unavailable")
    if len(result.stdout) > maximum_bytes:
        raise EvidenceError("evidence.git.size_exceeded")
    return result.stdout


def git_source_identity(root: Path, revision: str) -> dict[str, str]:
    """Resolve one commit and tree without consulting the working tree."""

    if not isinstance(revision, str) or not revision:
        raise EvidenceError("evidence.git.invalid_revision")
    commit = _run_git(root, ["rev-parse", f"{revision}^{{commit}}"], maximum_bytes=256)
    tree = _run_git(root, ["rev-parse", f"{revision}^{{tree}}"], maximum_bytes=256)
    commit_text = commit.decode("ascii").strip()
    tree_text = tree.decode("ascii").strip()
    if not all(
        len(value) in (40, 64)
        and all(character in "0123456789abcdef" for character in value)
        for value in (commit_text, tree_text)
    ):
        raise EvidenceError("evidence.git.invalid_identity")
    return {"revision": commit_text, "tree": tree_text}


def git_blob(
    root: Path,
    revision: str,
    relative: Any,
    *,
    maximum_bytes: int = DEFAULT_MAXIMUM_BYTES,
) -> bytes:
    """Read one bounded regular blob from an exact historical revision."""

    if not safe_relative_path(relative):
        raise EvidenceError("evidence.git.invalid_path")
    object_type = _run_git(
        root,
        ["cat-file", "-t", f"{revision}:{relative}"],
        maximum_bytes=64,
    ).decode("ascii").strip()
    if object_type != "blob":
        raise EvidenceError("evidence.git.not_blob")
    return _run_git(
        root,
        ["show", f"{revision}:{relative}"],
        maximum_bytes=maximum_bytes,
    )


def redacted_diagnostic(code: str, *, subject: str | None = None) -> dict[str, str]:
    """Build a content-free diagnostic with an optional stable subject identity."""

    result = {"code": code}
    if subject is not None:
        result["subject"] = subject
    return result
