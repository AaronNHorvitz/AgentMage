#!/usr/bin/env python3
"""Reproducible binding inventories, retained stage logs and verification records.

A verification record is assembled only from committed material, so a reviewer
can regenerate it and compare bytes:

- `inventory` lists direct whole-file hash bindings of the paths changed
  between two revisions. It reads both revisions and every JSON artifact from
  Git, so it gives the same result after later commits. It is not transitive
  and does not follow line spans.
- `retain` copies one private evidence stage into the repository. It replaces
  the checkout root, home directory, temporary directory and any declared
  private roots with placeholders, then refuses the copy if the user or host
  name remains. A name that is also public vocabulary, such as a distribution's
  default host name, is allowed only when named explicitly, and the stage
  records how many names were allowed, never the names.
- `record` builds a record from a committed specification, the commit range,
  the retained stages and a committed inventory. With `--check` it compares the
  result with the committed record instead of writing it.
"""

from __future__ import annotations

import argparse
import getpass
import hashlib
import json
import os
import re
import socket
import subprocess
import sys
import tempfile
from collections import Counter
from pathlib import Path
from typing import Any, Iterable

ROOT = Path(__file__).resolve().parents[1]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
STAGE_NAME = re.compile(r"^[a-z0-9][a-z0-9-]{0,63}$")
TEST_RESULT = re.compile(
    r"^test result: (?:ok|FAILED)\. (?P<passed>\d+) passed; (?P<failed>\d+) failed; "
    r"(?P<ignored>\d+) ignored;",
    re.M,
)
BINDING_KEYS = ("path", "input", "source", "file")
DIGEST_KEYS = ("sha256", "content_sha256", "input_sha256", "source_sha256")


class RecordError(ValueError):
    """Committed material does not reproduce the requested record."""


def git(*arguments: str) -> str:
    process = subprocess.run(
        ("git", *arguments), cwd=ROOT, check=True, capture_output=True, text=True, timeout=60
    )
    return process.stdout.strip()


def resolve(revision: str) -> str:
    resolved = git("rev-parse", "--verify", f"{revision}^{{commit}}")
    if not REVISION.fullmatch(resolved):
        raise RecordError(f"not a commit: {revision}")
    return resolved


def blobs(revision: str, paths: list[str]) -> dict[str, bytes | None]:
    """Read many files at one revision with a single `git cat-file --batch`."""
    request = "".join(f"{revision}:{path}\n" for path in paths).encode("utf-8")
    output = subprocess.run(
        ("git", "cat-file", "--batch"), cwd=ROOT, input=request, capture_output=True,
        check=True, timeout=600,
    ).stdout
    found: dict[str, bytes | None] = {}
    cursor = 0
    for path in paths:
        end = output.index(b"\n", cursor)
        header = output[cursor:end].split()
        cursor = end + 1
        if len(header) == 3 and header[1] == b"blob":
            size = int(header[2])
            found[path] = output[cursor:cursor + size]
            cursor += size + 1
        else:
            found[path] = None
    return found


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def is_sha256(value: Any) -> bool:
    return isinstance(value, str) and SHA256.fullmatch(value) is not None


def _bindings(document: Any, artifact: str, paths: set[str]) -> set[tuple[str, str, str]]:
    found: set[tuple[str, str, str]] = set()
    stack = [document]
    while stack:
        value = stack.pop()
        if isinstance(value, dict):
            for name, digest in value.items():
                if name in paths and is_sha256(digest):
                    found.add((artifact, name, digest))
            for key in BINDING_KEYS:
                name = value.get(key)
                if isinstance(name, str) and name in paths:
                    for digest_key in DIGEST_KEYS:
                        if is_sha256(value.get(digest_key)):
                            found.add((artifact, name, value[digest_key]))
            stack.extend(value.values())
        elif isinstance(value, list):
            stack.extend(value)
    return found


def inventory(base: str, head: str) -> dict[str, Any]:
    """Direct bindings of paths changed from `base` to `head`, read from Git."""
    base, head = resolve(base), resolve(head)
    if subprocess.run(("git", "merge-base", "--is-ancestor", base, head), cwd=ROOT).returncode:
        raise RecordError("the base is not an ancestor of the head")
    changed = git("diff", "--name-only", "--no-renames", base, head).splitlines()
    after_blobs, before_blobs = blobs(head, changed), blobs(base, changed)
    current: dict[str, str] = {}
    prior: dict[str, str | None] = {}
    for path in changed:
        after, before = after_blobs[path], before_blobs[path]
        if after is None:
            continue
        current[path] = sha256_bytes(after)
        prior[path] = None if before is None else sha256_bytes(before)
    paths = set(current)
    bindings: set[tuple[str, str, str]] = set()
    artifacts = [name for name in git("ls-tree", "-r", "--name-only", head).splitlines()
                 if name.endswith(".json")]
    for artifact, raw in blobs(head, artifacts).items():
        try:
            document = json.loads(raw or b"")
        except (json.JSONDecodeError, UnicodeDecodeError):
            continue
        bindings |= _bindings(document, artifact, paths)
    rows = []
    for artifact, name, digest in sorted(bindings):
        if digest == current[name]:
            disposition = "current"
        elif digest == prior[name]:
            disposition = "newly-stale"
        else:
            disposition = "previously-stale-or-historical"
        rows.append({
            "artifact": artifact, "input": name, "retained_sha256": digest,
            "base_sha256": prior[name], "head_sha256": current[name], "disposition": disposition,
        })
    return {
        "schema_version": 1,
        "record_type": "agentmage-direct-binding-inventory",
        "scope": "direct whole-file hash records naming paths changed between the revisions; not transitive or line-span",
        "base_commit": base,
        "head_commit": head,
        "changed_paths": sorted(paths),
        "counts": dict(sorted(Counter(row["disposition"] for row in rows).items())),
        "newly_stale_artifacts": sorted({row["artifact"] for row in rows if row["disposition"] == "newly-stale"}),
        "bindings": rows,
    }


def private_roots(extra: Iterable[tuple[str, str]]) -> list[tuple[str, str]]:
    roots = [(str(ROOT), "<repo>"), (str(Path.home()), "<home>"),
             (tempfile.gettempdir(), "<tmp>"), *extra]
    for variable in ("TMPDIR", "CARGO_TARGET_DIR"):
        value = os.environ.get(variable)
        if value and os.path.isabs(value):
            roots.append((value.rstrip("/"), f"<{variable.lower()}>"))
    # Longest first, so a nested root is replaced before its parent.
    return sorted({(path.rstrip("/"), label) for path, label in roots if path not in ("", "/")},
                  key=lambda item: -len(item[0]))


def private_names(allowed: Iterable[str] = ()) -> list[str]:
    names = {getpass.getuser(), socket.gethostname(), socket.gethostname().split(".")[0]}
    if getpass.getuser() in set(allowed):
        raise RecordError("the user name cannot be allowed")
    return sorted(
        name for name in names - set(allowed) if len(name) >= 3 and name != "localhost"
    )


def redact(text: str, roots: list[tuple[str, str]], names: list[str]) -> str:
    for path, label in roots:
        text = text.replace(path, label)
    for name in names:
        if name in text:
            raise RecordError("a private user or host name remains after redaction")
    return text


def retain(plan_path: Path, results_path: Path, destination: Path,
           extra: list[tuple[str, str]], allowed_public_names: tuple[str, ...] = (),
           attachments: tuple[Path, ...] = ()) -> dict[str, Any]:
    plan = json.loads(plan_path.read_text(encoding="utf-8"))
    results = json.loads(results_path.read_text(encoding="utf-8"))
    name = plan["name"]
    if not STAGE_NAME.fullmatch(name) or not REVISION.fullmatch(plan["source_revision"]):
        raise RecordError("stage name or source revision is invalid")
    resolve(plan["source_revision"])
    if len(results) > len(plan["commands"]):
        raise RecordError("more results than planned commands")
    target = destination / name
    if target.exists():
        raise RecordError(f"stage already retained: {target}")
    roots = private_roots(extra)
    names = private_names(allowed_public_names)
    retained = []
    staged: list[tuple[str, str]] = []
    for index, row in enumerate(results, start=1):
        if row["index"] != index or row["command"] != plan["commands"][index - 1]:
            raise RecordError("results do not follow the plan")
        raw = Path(row["log"]).read_bytes()
        if sha256_bytes(raw) != row["sha256"]:
            raise RecordError(f"private log changed after the stage: {row['log']}")
        text = redact(raw.decode("utf-8", errors="replace"), roots, names)
        log_name = f"{index:02d}.log"
        staged.append((log_name, text))
        retained.append({
            "index": index,
            "argv": [redact(part, roots, names) for part in row["command"]],
            "exit_code": row["exit_code"],
            "seconds": row["seconds"],
            "log": log_name,
            "log_sha256": sha256_bytes(text.encode("utf-8")),
            "private_log_sha256": row["sha256"],
        })
    attached = []
    for path in attachments:
        if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]{0,127}", path.name) or any(
            path.name == row["name"] for row in attached
        ):
            raise RecordError(f"attachment name is invalid or repeated: {path.name}")
        raw = path.read_bytes()
        text = redact(raw.decode("utf-8"), roots, names)
        staged.append((f"attachments/{path.name}", text))
        attached.append({"name": path.name, "sha256": sha256_bytes(text.encode("utf-8")),
                         "private_sha256": sha256_bytes(raw)})
    stage = {
        "schema_version": 1,
        "stage": name,
        "source_revision": plan["source_revision"],
        "scope": redact(plan["scope"], roots, names),
        "planned_commands": [[redact(part, roots, names) for part in command]
                             for command in plan["commands"]],
        "continue_after_failure_indexes": plan.get("continue_after_failure_indexes", []),
        "sources": plan["sources"],
        # Count only: naming an allowed host name here would disclose it.
        "allowed_public_name_count": len(set(allowed_public_names)),
        "attachments": attached,
        "results": retained,
    }
    target.mkdir(parents=True)
    for log_name, text in staged:
        (target / log_name).parent.mkdir(parents=True, exist_ok=True)
        (target / log_name).write_text(text, encoding="utf-8")
    (target / "stage.json").write_text(json.dumps(stage, indent=2) + "\n", encoding="utf-8")
    return stage


def load_stage(directory: Path) -> dict[str, Any]:
    stage = json.loads((directory / "stage.json").read_text(encoding="utf-8"))
    if stage.get("schema_version") != 1 or stage.get("stage") != directory.name:
        raise RecordError(f"stage identity differs: {directory}")
    resolve(stage["source_revision"])
    for row in stage["results"]:
        log = directory / row["log"]
        if sha256_bytes(log.read_bytes()) != row["log_sha256"]:
            raise RecordError(f"retained log digest differs: {log}")
    for row in stage.get("attachments", []):
        attachment = directory / "attachments" / row["name"]
        if sha256_bytes(attachment.read_bytes()) != row["sha256"]:
            raise RecordError(f"retained attachment digest differs: {attachment}")
    return stage


def stage_summary(stage: dict[str, Any]) -> dict[str, Any]:
    return {
        "stage": stage["stage"],
        "source_revision": stage["source_revision"],
        "scope": stage["scope"],
        "planned_commands": len(stage["planned_commands"]),
        "ran": len(stage["results"]),
        "results": [
            {"argv": row["argv"], "exit_code": row["exit_code"], "log_sha256": row["log_sha256"]}
            for row in stage["results"]
        ],
    }


def test_counts(text: str) -> dict[str, int]:
    """Sum libtest result lines; counts are only as complete as the retained log."""
    totals = Counter()
    for match in TEST_RESULT.finditer(text):
        for key in ("passed", "failed", "ignored"):
            totals[key] += int(match[key])
    totals["result_lines"] = len(TEST_RESULT.findall(text))
    return dict(totals)


def commits(base: str, head: str) -> list[dict[str, str]]:
    base, head = resolve(base), resolve(head)
    if subprocess.run(("git", "merge-base", "--is-ancestor", base, head), cwd=ROOT).returncode:
        raise RecordError("the commit range base is not an ancestor of its head")
    lines = git("log", "--reverse", "--format=%H %s", f"{base}..{head}").splitlines()
    return [{"commit": line[:40], "subject": line[41:]} for line in lines]


def build_record(spec_path: Path) -> dict[str, Any]:
    spec = json.loads(spec_path.read_text(encoding="utf-8"))
    logs = ROOT / spec["retained_logs"]
    ordered = [load_stage(logs / name) for name in spec["stages"]]
    commit_rows = commits(spec["base_commit"], spec["head_commit"])
    listed = {row["commit"] for row in commit_rows}
    if resolve(spec["source_commit"]) not in listed:
        raise RecordError("the source commit is outside the commit range")
    for stage in ordered:
        if stage["source_revision"] not in listed:
            raise RecordError(f"stage {stage['stage']} ran outside the commit range")
    inventory_path = ROOT / spec["inventory"]
    recorded = json.loads(inventory_path.read_text(encoding="utf-8"))
    if recorded != inventory(recorded["base_commit"], recorded["head_commit"]):
        raise RecordError("the committed inventory does not reproduce")
    checks = {}
    for name, relative in spec.get("check_logs", {}).items():
        text = (logs / relative).read_text(encoding="utf-8")
        checks[name] = {"log": relative, "log_sha256": sha256_bytes(text.encode("utf-8")),
                        **test_counts(text)}
    record = {
        "schema_version": 1,
        "record_type": spec["record_type"],
        "date": spec["date"],
        "base_commit": resolve(spec["base_commit"]),
        "head_commit": resolve(spec["head_commit"]),
        "source_commit": resolve(spec["source_commit"]),
        "generator": "scripts/verification_batch.py",
        "specification": spec_path.relative_to(ROOT).as_posix(),
        "commits": commit_rows,
        "check_logs": checks,
        "evidence_stages": [stage_summary(stage) for stage in ordered],
        "binding_inventory": {
            "path": spec["inventory"],
            "scope": recorded["scope"],
            "counts": recorded["counts"],
            "newly_stale_artifacts": recorded["newly_stale_artifacts"],
        },
    }
    for key, value in spec["fields"].items():
        if key in record:
            raise RecordError(f"specification field would replace a derived field: {key}")
        record[key] = value
    return record


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    commands = parser.add_subparsers(dest="command", required=True)
    inv = commands.add_parser("inventory")
    inv.add_argument("--base", required=True)
    inv.add_argument("--head", required=True)
    output = inv.add_mutually_exclusive_group(required=True)
    output.add_argument("--output", type=Path)
    output.add_argument("--check", type=Path)
    keep = commands.add_parser("retain")
    keep.add_argument("--plan", type=Path, required=True)
    keep.add_argument("--results", type=Path, required=True)
    keep.add_argument("--destination", type=Path, required=True)
    keep.add_argument("--redact", action="append", default=[], metavar="PATH=LABEL")
    keep.add_argument("--allow-public-name", action="append", default=[], metavar="NAME")
    keep.add_argument("--attach", action="append", default=[], type=Path, metavar="FILE")
    rec = commands.add_parser("record")
    rec.add_argument("--spec", type=Path, required=True)
    target = rec.add_mutually_exclusive_group(required=True)
    target.add_argument("--output", type=Path)
    target.add_argument("--check", type=Path)
    args = parser.parse_args(argv)
    try:
        if args.command == "inventory":
            value = inventory(args.base, args.head)
            text = json.dumps(value, indent=2) + "\n"
            if args.check:
                if args.check.read_text(encoding="utf-8") != text:
                    print("inventory does not reproduce", file=sys.stderr)
                    return 1
            else:
                args.output.write_text(text, encoding="utf-8")
            print(json.dumps({"counts": value["counts"], "newly_stale_artifacts": value["newly_stale_artifacts"]}))
        elif args.command == "retain":
            extra = []
            for item in args.redact:
                path, separator, label = item.partition("=")
                if not separator or not os.path.isabs(path) or not re.fullmatch(r"<[a-z-]+>", label):
                    parser.error(f"invalid --redact value: {item}")
                extra.append((path, label))
            stage = retain(args.plan, args.results, args.destination, extra,
                           tuple(args.allow_public_name), tuple(args.attach))
            print(json.dumps({"stage": stage["stage"], "logs": len(stage["results"])}))
        else:
            record = build_record(args.spec.resolve())
            text = json.dumps(record, indent=2) + "\n"
            if args.check:
                if args.check.read_text(encoding="utf-8") != text:
                    print("record does not reproduce", file=sys.stderr)
                    return 1
            else:
                args.output.write_text(text, encoding="utf-8")
            print(json.dumps({"record": record["record_type"], "stages": len(record["evidence_stages"])}))
    except (RecordError, OSError, KeyError, subprocess.SubprocessError) as error:
        print(f"verification batch refused: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
