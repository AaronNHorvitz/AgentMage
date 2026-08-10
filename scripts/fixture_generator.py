#!/usr/bin/env python3
"""Generate deterministic, synthetic AgentMage test fixtures."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import struct
import sys
import tempfile
import zlib
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
PROFILE_PATH = ROOT / "fixtures" / "generator-profile.json"
REPORT_PATH = (
    ROOT
    / "artifacts"
    / "sprints"
    / "sprint-2"
    / "story-2.1"
    / "fixture-generator-report.json"
)
EXPECTED_CATEGORIES = (
    "markdown-workspaces",
    "obsidian-vaults",
    "git-repositories",
    "supported-parser-languages",
    "unsupported-languages",
    "malformed-inputs",
)
EXPECTED_SUPPORTED = (
    ("go", ".go"),
    ("javascript", ".js"),
    ("python", ".py"),
    ("rust", ".rs"),
    ("shell", ".sh"),
    ("sql", ".sql"),
    ("typescript", ".ts"),
)
EXPECTED_UNSUPPORTED = (
    ("cobol", ".cob"),
    ("fortran", ".f90"),
    ("unknown", ".xyz"),
)
EXPECTED_MALFORMED = (
    "invalid-json",
    "invalid-utf8",
    "nul-containing-text",
    "truncated-source",
    "unterminated-frontmatter",
)
EXPECTED_SIDE_EFFECTS = {
    "executes_external_commands": False,
    "uses_network": False,
    "writes_outside_requested_destination": False,
    "overwrites_existing_destination": False,
    "creates_executable_files": False,
}
EXPECTED_PRIVACY = {
    "private_user_data": False,
    "real_credentials": False,
    "real_person_identity": False,
    "remote_references": False,
}


@dataclass(frozen=True)
class FixtureFile:
    category: str
    content: bytes


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def safe_path(value: str) -> bool:
    path = PurePosixPath(value)
    return bool(value) and not path.is_absolute() and ".." not in path.parts


def _pairs(records: Any) -> tuple[tuple[str, str], ...]:
    if not isinstance(records, list):
        return ()
    return tuple(
        (item.get("id"), item.get("extension"))
        for item in records
        if isinstance(item, dict)
    )


def validate_profile(profile: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(profile, dict):
        return ["fixture generator profile must be an object"]
    if (
        profile.get("schema_version") != 1
        or profile.get("profile_id") != "agentmage-synthetic-corpus-v1"
        or profile.get("status") != "fixture-contract-not-product-admission"
    ):
        failures.append("fixture generator profile identity is invalid")
    if profile.get("seed") != "agentmage-sprint-2-synthetic-v1":
        failures.append("fixture generator seed is not pinned")
    if profile.get("fixed_timestamp_epoch") != 1704067200:
        failures.append("fixture generator timestamp is not pinned")
    if tuple(profile.get("categories", [])) != EXPECTED_CATEGORIES:
        failures.append("fixture generator category closure drifted")
    if _pairs(profile.get("supported_parser_fixture_languages")) != EXPECTED_SUPPORTED:
        failures.append("supported parser fixture language closure drifted")
    if _pairs(profile.get("unsupported_language_fixtures")) != EXPECTED_UNSUPPORTED:
        failures.append("unsupported language fixture closure drifted")
    if tuple(profile.get("malformed_cases", [])) != EXPECTED_MALFORMED:
        failures.append("malformed fixture closure drifted")
    if profile.get("side_effect_contract") != EXPECTED_SIDE_EFFECTS:
        failures.append("fixture generator side-effect contract was weakened")
    if profile.get("privacy_contract") != EXPECTED_PRIVACY:
        failures.append("fixture generator privacy contract was weakened")
    if profile.get("product_parser_support_claim") != "none":
        failures.append("fixture profile cannot make a product parser support claim")
    return failures


def fixture_marker(seed: str, category: str) -> str:
    return hashlib.sha256(f"{seed}:{category}".encode("utf-8")).hexdigest()[:12]


def add_file(
    files: dict[str, FixtureFile],
    path: str,
    category: str,
    content: str | bytes,
) -> None:
    if not safe_path(path) or category not in EXPECTED_CATEGORIES:
        raise ValueError(f"invalid fixture file declaration: {path}")
    if path in files:
        raise ValueError(f"duplicate fixture path: {path}")
    encoded = content.encode("utf-8") if isinstance(content, str) else content
    files[path] = FixtureFile(category=category, content=encoded)


def markdown_workspace(files: dict[str, FixtureFile], seed: str) -> None:
    marker = fixture_marker(seed, "markdown-workspaces")
    add_file(
        files,
        "markdown-workspace/README.md",
        "markdown-workspaces",
        f"# Northwind Notes\n\nSynthetic workspace `{marker}`.\n",
    )
    add_file(
        files,
        "markdown-workspace/projects/alpha.md",
        "markdown-workspaces",
        "# Project Alpha\n\nStatus: active\n\n- [ ] Verify the fictional report.\n",
    )
    add_file(
        files,
        "markdown-workspace/meetings/2024-01-02.md",
        "markdown-workspaces",
        "# Planning Meeting\n\nDecision: use the bounded synthetic dataset.\n",
    )


def obsidian_vault(files: dict[str, FixtureFile], seed: str) -> None:
    marker = fixture_marker(seed, "obsidian-vaults")
    add_file(
        files,
        "obsidian-vault/Home.md",
        "obsidian-vaults",
        "---\ntags: [synthetic, home]\n---\n# Home\n\n"
        "See [[Projects/Orchid]] and [[Missing Note]].\n",
    )
    add_file(
        files,
        "obsidian-vault/Projects/Orchid.md",
        "obsidian-vaults",
        f"---\nid: orchid-{marker}\nstatus: active\n---\n# Orchid\n\n"
        "- [ ] Reconcile the synthetic ledger.\n",
    )
    add_file(
        files,
        "obsidian-vault/Handoffs/current.md",
        "obsidian-vaults",
        "# Current Handoff\n\nSupersedes [[Handoffs/old]].\n",
    )
    add_file(
        files,
        "obsidian-vault/Handoffs/old.md",
        "obsidian-vaults",
        "---\nstatus: superseded\n---\n# Old Handoff\n",
    )
    add_file(
        files,
        "obsidian-vault/People/Casey Example.md",
        "obsidian-vaults",
        "# Casey Example\n\nFictional fixture person.\n",
    )
    add_file(
        files,
        "obsidian-vault/Meetings/Review.md",
        "obsidian-vaults",
        "# Review\n\nRelated: [[Projects/Orchid]] and [[People/Casey Example]].\n",
    )
    add_file(
        files,
        "obsidian-vault/Raw Notes/inbox.md",
        "obsidian-vaults",
        "# Inbox\n\nUnprocessed synthetic observation.\n",
    )


def git_object(kind: str, content: bytes) -> tuple[str, bytes]:
    payload = f"{kind} {len(content)}\0".encode("ascii") + content
    object_id = hashlib.sha1(payload, usedforsecurity=False).hexdigest()
    return object_id, zlib.compress(payload, level=0)


def git_tree_objects(
    worktree: dict[str, bytes],
) -> tuple[str, dict[str, bytes], dict[str, str]]:
    objects: dict[str, bytes] = {}
    blob_ids: dict[str, str] = {}
    nested: dict[str, Any] = {}
    for path, content in sorted(worktree.items()):
        blob_id, compressed = git_object("blob", content)
        objects[blob_id] = compressed
        blob_ids[path] = blob_id
        cursor = nested
        parts = PurePosixPath(path).parts
        for part in parts[:-1]:
            cursor = cursor.setdefault(part, {})
        cursor[parts[-1]] = blob_id

    def build_tree(node: dict[str, Any]) -> str:
        entries = bytearray()
        for name, value in sorted(node.items(), key=lambda item: item[0].encode("utf-8")):
            if isinstance(value, dict):
                object_id = build_tree(value)
                mode = b"40000"
            else:
                object_id = value
                mode = b"100644"
            entries.extend(mode + b" " + name.encode("utf-8") + b"\0")
            entries.extend(bytes.fromhex(object_id))
        tree_id, compressed = git_object("tree", bytes(entries))
        objects[tree_id] = compressed
        return tree_id

    return build_tree(nested), objects, blob_ids


def git_index(worktree: dict[str, bytes], blob_ids: dict[str, str], timestamp: int) -> bytes:
    entries = bytearray()
    for path, content in sorted(worktree.items()):
        encoded_path = path.encode("utf-8")
        fixed_fields = struct.pack(
            ">10I",
            timestamp,
            0,
            timestamp,
            0,
            0,
            0,
            0o100644,
            10001,
            10001,
            len(content),
        )
        flags = min(len(encoded_path), 0xFFF)
        entry = bytearray(
            fixed_fields
            + bytes.fromhex(blob_ids[path])
            + struct.pack(">H", flags)
            + encoded_path
            + b"\0"
        )
        entry.extend(b"\0" * ((8 - len(entry) % 8) % 8))
        entries.extend(entry)
    body = b"DIRC" + struct.pack(">II", 2, len(worktree)) + bytes(entries)
    return body + hashlib.sha1(body, usedforsecurity=False).digest()


def git_repository(files: dict[str, FixtureFile], seed: str, timestamp: int) -> None:
    marker = fixture_marker(seed, "git-repositories")
    worktree = {
        "README.md": f"# Calculator Fixture\n\nIdentity `{marker}`.\n".encode(),
        "src/calculator.py": (
            b"def add(left: int, right: int) -> int:\n"
            b"    return left - right  # deliberate synthetic defect\n"
        ),
        "tests/test_calculator.py": (
            b"from src.calculator import add\n\n"
            b"def test_add() -> None:\n"
            b"    assert add(2, 3) == 5\n"
        ),
    }
    tree_id, objects, blob_ids = git_tree_objects(worktree)
    commit_content = (
        f"tree {tree_id}\n"
        f"author Fixture Author <fixture@example.invalid> {timestamp} +0000\n"
        f"committer Fixture Author <fixture@example.invalid> {timestamp} +0000\n\n"
        "Initial synthetic fixture\n"
    ).encode("utf-8")
    commit_id, commit_object = git_object("commit", commit_content)
    objects[commit_id] = commit_object
    prefix = "git/repository"
    for path, content in worktree.items():
        add_file(files, f"{prefix}/{path}", "git-repositories", content)
    for object_id, content in objects.items():
        add_file(
            files,
            f"{prefix}/.git/objects/{object_id[:2]}/{object_id[2:]}",
            "git-repositories",
            content,
        )
    add_file(files, f"{prefix}/.git/HEAD", "git-repositories", "ref: refs/heads/main\n")
    add_file(
        files,
        f"{prefix}/.git/refs/heads/main",
        "git-repositories",
        f"{commit_id}\n",
    )
    add_file(
        files,
        f"{prefix}/.git/config",
        "git-repositories",
        "[core]\n\trepositoryformatversion = 0\n\tbare = false\n",
    )
    add_file(
        files,
        f"{prefix}/.git/index",
        "git-repositories",
        git_index(worktree, blob_ids, timestamp),
    )


def supported_languages(files: dict[str, FixtureFile], seed: str) -> None:
    marker = fixture_marker(seed, "supported-parser-languages")
    samples = {
        "go/main.go": 'package main\n\nimport "fmt"\n\nfunc main() { fmt.Println("fixture") }\n',
        "javascript/index.js": "export function fixture() { return 'javascript'; }\n",
        "python/main.py": "def fixture() -> str:\n    return 'python'\n",
        "rust/main.rs": 'fn main() { println!("fixture"); }\n',
        "shell/main.sh": "#!/bin/sh\nprintf '%s\\n' fixture\n",
        "sql/schema.sql": "CREATE TABLE fixture_record (id INTEGER PRIMARY KEY);\n",
        "typescript/index.ts": "export const fixture = (): string => 'typescript';\n",
    }
    for relative, content in samples.items():
        add_file(
            files,
            f"parser-supported/{relative}",
            "supported-parser-languages",
            f"/* fixture {marker} */\n{content}" if not relative.endswith((".py", ".sh")) else content,
        )


def unsupported_languages(files: dict[str, FixtureFile], seed: str) -> None:
    marker = fixture_marker(seed, "unsupported-languages")
    samples = {
        "cobol/main.cob": "IDENTIFICATION DIVISION.\nPROGRAM-ID. FIXTURE.\nSTOP RUN.\n",
        "fortran/main.f90": "program fixture\n  print *, 'fixture'\nend program fixture\n",
        "unknown/main.xyz": f"opaque fixture syntax {marker}\n",
    }
    for relative, content in samples.items():
        add_file(
            files,
            f"parser-unsupported/{relative}",
            "unsupported-languages",
            content,
        )


def malformed_inputs(files: dict[str, FixtureFile]) -> None:
    add_file(files, "malformed/invalid.json", "malformed-inputs", b'{"open": [1, 2}')
    add_file(files, "malformed/invalid-utf8.txt", "malformed-inputs", b"text\xff\xfeend")
    add_file(files, "malformed/nul.txt", "malformed-inputs", b"before\x00after")
    add_file(files, "malformed/truncated.py", "malformed-inputs", "def unfinished(\n")
    add_file(
        files,
        "malformed/frontmatter.md",
        "malformed-inputs",
        "---\ntitle: unfinished\n# Missing closing delimiter\n",
    )


def materialize_specs(profile: dict[str, Any]) -> dict[str, FixtureFile]:
    files: dict[str, FixtureFile] = {}
    seed = profile["seed"]
    markdown_workspace(files, seed)
    obsidian_vault(files, seed)
    git_repository(files, seed, profile["fixed_timestamp_epoch"])
    supported_languages(files, seed)
    unsupported_languages(files, seed)
    malformed_inputs(files)
    return dict(sorted(files.items()))


def corpus_identity(files: dict[str, FixtureFile]) -> str:
    digest = hashlib.sha256()
    for path, fixture in sorted(files.items()):
        encoded_path = path.encode("utf-8")
        encoded_category = fixture.category.encode("utf-8")
        digest.update(len(encoded_path).to_bytes(8, "big"))
        digest.update(encoded_path)
        digest.update(len(encoded_category).to_bytes(8, "big"))
        digest.update(encoded_category)
        digest.update(len(fixture.content).to_bytes(8, "big"))
        digest.update(fixture.content)
    return digest.hexdigest()


def category_counts(files: dict[str, FixtureFile]) -> dict[str, int]:
    return {
        category: sum(item.category == category for item in files.values())
        for category in EXPECTED_CATEGORIES
    }


def generate(profile: dict[str, Any], destination: Path) -> dict[str, Any]:
    failures = validate_profile(profile)
    if failures:
        raise ValueError("; ".join(failures))
    if destination.exists():
        raise FileExistsError("fixture destination already exists")
    destination.parent.mkdir(parents=True, exist_ok=True)
    staging = Path(tempfile.mkdtemp(prefix=".agentmage-fixtures-", dir=destination.parent))
    files = materialize_specs(profile)
    try:
        for relative, fixture in files.items():
            target = staging / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(fixture.content)
            target.chmod(0o644)
            os.utime(target, (profile["fixed_timestamp_epoch"],) * 2)
        for directory in sorted(
            (path for path in staging.rglob("*") if path.is_dir()), reverse=True
        ):
            os.utime(directory, (profile["fixed_timestamp_epoch"],) * 2)
        os.replace(staging, destination)
    except BaseException:
        shutil.rmtree(staging, ignore_errors=True)
        raise
    return {
        "corpus_sha256": corpus_identity(files),
        "file_count": len(files),
        "category_counts": category_counts(files),
    }


def build_report(root: Path = ROOT) -> dict[str, Any]:
    profile_path = root / "fixtures/generator-profile.json"
    profile = read_json(profile_path)
    failures = validate_profile(profile)
    if failures:
        raise ValueError("; ".join(failures))
    files = materialize_specs(profile)
    return {
        "schema_version": 1,
        "task_id": "2.1.1.1",
        "status": "pass",
        "profile": {
            "id": profile["profile_id"],
            "sha256": sha256_bytes(profile_path.read_bytes()),
            "seed_sha256": sha256_bytes(profile["seed"].encode("utf-8")),
        },
        "corpus_preview": {
            "persisted": False,
            "sha256": corpus_identity(files),
            "file_count": len(files),
            "category_counts": category_counts(files),
        },
        "side_effect_contract": profile["side_effect_contract"],
        "privacy_contract": profile["privacy_contract"],
        "product_parser_support_claim": "none",
        "versioned_corpus_status": "reserved-for-sub-task-2.1.2.1",
        "macos_support_claim": "none",
    }


def validate_report(report: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(report, dict):
        return ["fixture generator report must be an object"]
    failures = []
    if report.get("schema_version") != 1 or report.get("task_id") != "2.1.1.1":
        failures.append("fixture generator report identity is invalid")
    if report.get("status") != "pass":
        failures.append("fixture generator did not pass")
    if report.get("product_parser_support_claim") != "none":
        failures.append("fixture generator report made a parser support claim")
    if report.get("versioned_corpus_status") != "reserved-for-sub-task-2.1.2.1":
        failures.append("fixture generator prematurely claimed the versioned corpus artifact")
    if report.get("macos_support_claim") != "none":
        failures.append("fixture generator report made a macOS support claim")
    expected = build_report(root)
    if report != expected:
        failures.append("fixture generator report is stale or non-deterministic")
    return failures


def write_report(root: Path = ROOT) -> None:
    output = root / REPORT_PATH.relative_to(ROOT)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(canonical_json(build_report(root)))


def check_report(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read fixture generator report: {error}"]
    return validate_report(report, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path)
    parser.add_argument("--write-report", action="store_true")
    args = parser.parse_args()
    try:
        profile = read_json(PROFILE_PATH)
        if args.output is not None:
            result = generate(profile, args.output)
            print(json.dumps(result, indent=2, sort_keys=True))
        if args.write_report:
            write_report()
        failures = check_report()
    except (OSError, ValueError) as error:
        print(f"fixture generator failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"fixture generator failed: {failure}", file=sys.stderr)
        return 1
    if args.output is None:
        print("deterministic synthetic fixture generators validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
