#!/usr/bin/env python3
"""Reject private home paths in changed files; preserve exact historical blobs."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
from urllib.parse import unquote_to_bytes


ROOT = Path(__file__).resolve().parents[1]
POLICY = "architecture/private-path-exceptions.json"
POLICY_SHA256 = "6c1b07bb7b852d5c339a38910494e74d8960c4ec48bb70a25c47b84e9493ecbe"
# These are existing synthetic fixture identities, never identities detected
# from the machine. A literal private username is checked independently below.
SYNTHETIC_USERS = {b"agentmage", b"agentmage-test", b"user", b"private", b"example",
                   b"traversal", b"synthetic", b"modelrunner", b"person"}
HOME_USER = re.compile(rb"(?:[/](?:var[/])?home[/]|[/]Users[/]|[A-Za-z]:[\\/]Users[\\/])"
                       rb"([^\s/\\<>\"'`:;,)}\]]+)", re.IGNORECASE)
TOKEN = re.compile(rb"[A-Za-z0-9_-]+")
PUBLIC_REPOSITORY_URL = b"https://github.com/AaronNHorvitz/AgentMage"
PUBLIC_URL = re.compile(rb"(?<![A-Za-z0-9:/._%?&=+#@-])" + re.escape(PUBLIC_REPOSITORY_URL)
                        + rb"(?![A-Za-z0-9:/._%?&=+#@-])")
JSON_ESCAPE = re.compile(rb'\\(?:u[0-9a-fA-F]{4}|["\\/bfnrt])')


def git(root: Path, *args: str) -> bytes:
    return subprocess.check_output(["git", *args], cwd=root)


def contains_private(data: bytes, username_hash: str, username_bytes: int = 13) -> bool:
    views = [data]
    if b"\x00" in data or data.startswith((b"\xff\xfe", b"\xfe\xff")):
        views.extend(data.decode(codec, errors="ignore").encode("utf-8", errors="ignore")
                     for codec in ("utf-16-le", "utf-16-be", "utf-32-le", "utf-32-be"))
    for view in views:
        # Scan raw bytes as well as decoded text. Never let an encoded spelling
        # hide a value that was visible before decoding.
        for _ in range(8):
            safe = PUBLIC_URL.sub(b"<public-repository>", view).lower()
            for token in TOKEN.findall(safe):
                for start in range(len(token) - username_bytes + 1):
                    if hashlib.sha256(token[start:start + username_bytes]).hexdigest() == username_hash:
                        return True
            if any(match.lower() not in SYNTHETIC_USERS and match not in {b".", b"...", "…".encode()}
                   for match in HOME_USER.findall(safe)):
                return True
            decoded = unquote_to_bytes(JSON_ESCAPE.sub(
                lambda match: json.loads('"' + match.group().decode("ascii") + '"')
                .encode("utf-8", errors="ignore"), view))
            if decoded == view:
                break
            view = decoded
        else:
            # Refuse excessive encoding depth rather than silently accepting
            # bytes that the bounded decoder has not completely inspected.
            return True
    return False


def check(root: Path, base: str) -> list[str]:
    policy_bytes = (root / POLICY).read_bytes()
    if hashlib.sha256(policy_bytes).hexdigest() != POLICY_SHA256:
        raise ValueError("private-path exception list changed without a policy update")
    policy = json.loads(policy_bytes)
    if policy["schema_version"] != 1:
        raise ValueError("unsupported private-path policy")
    username_bytes = policy["username_bytes"]
    if type(username_bytes) is not int or not 1 <= username_bytes <= 255:
        raise ValueError("invalid private username length")
    def private(data: bytes) -> bool:
        return contains_private(data, policy["username_sha256"], username_bytes)
    failures = []
    preserved = policy["preserved_files"]
    tracked = set(git(root, "ls-files", "-z").split(b"\0"))
    if POLICY.encode() in tracked and hashlib.sha256(git(root, "show", ":" + POLICY)).hexdigest() != POLICY_SHA256:
        failures.append("staged private-path exception policy changed")
    # Pin the exception set to its recorded Git baseline. Editing this JSON
    # cannot silently exempt a new file or change an exempt blob's digest.
    baseline = git(root, "rev-parse", "--verify", "--end-of-options",
                   policy["baseline_commit"] + "^{commit}").decode().strip()
    names = git(root, "ls-tree", "-r", "--name-only", "-z", baseline).split(b"\0")
    # The explicit list is authoritative, with each blob verified against Git.
    for name, digest in preserved.items():
        if name.encode() not in names:
            raise ValueError("exception is absent from its baseline")
        data = git(root, "show", baseline + ":" + name)
        if hashlib.sha256(data).hexdigest() != digest or not private(data):
            raise ValueError("exception does not match its historical blob")
        path = root / name
        if path.is_symlink() or not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            failures.append(f"{name}: historical private-path file must remain byte-identical")
        if name.encode() not in tracked or hashlib.sha256(git(root, "show", ":" + name)).hexdigest() != digest:
            failures.append(f"{name}: staged historical file must remain byte-identical")
    revision = git(root, "rev-parse", "--verify", "--end-of-options", base + "^{commit}").decode().strip()
    changed = git(root, "diff", "--name-only", "--no-renames", "-z", revision, "--")
    staged = git(root, "diff", "--cached", "--name-only", "--no-renames", "-z", revision, "--")
    new = git(root, "ls-files", "--others", "--exclude-standard", "-z")
    for raw in sorted(set((changed + staged + new).split(b"\0")) - {b""}):
        name = raw.decode("utf-8")
        path = root / name
        if name in preserved:
            continue
        private_name = private(raw)
        safe_name = "<private filename>" if private_name else name
        if private_name:
            failures.append("changed filename contains a private host identifier")
        if raw in tracked and private(git(root, "show", ":" + name)):
            failures.append(f"{safe_name}: staged private home path or local username")
        if path.is_symlink():
            data = str(path.readlink()).encode()
        elif path.is_file():
            data = path.read_bytes()
        else:
            continue
        if private(data):
            # Never echo the matched private value into a public log.
            failures.append(f"{safe_name}: private home path or local username")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default="HEAD")
    args = parser.parse_args()
    try:
        failures = check(ROOT, args.base)
    except (KeyError, ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"PRIVATE PATH CHECK: FAIL ({type(error).__name__})", file=sys.stderr)
        return 1
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print("PRIVATE PATH CHECK: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
