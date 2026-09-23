#!/usr/bin/env python3
"""Inspect retained live-coding evidence; never dispatch tools or grant admission.

The CLI's verifier/artifact reports remain the authority. This read-only collector
cross-checks those reports with recorded native proposals and exact prompt feedback.
It does not turn a diagnostic success into production or campaign qualification.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def observations(prompt: str) -> list[dict]:
    """Read only host-rendered message envelopes, not arbitrary nested JSON."""
    decoder = json.JSONDecoder()
    found = []
    offset = 0
    marker = '{"message_id":'
    while (offset := prompt.find(marker, offset)) >= 0:
        envelope, length = decoder.raw_decode(prompt[offset:])
        offset += length
        content = envelope.get("content")
        if envelope.get("role") == "tool" and isinstance(content, dict):
            if "completed_call" not in content or "result" not in content:
                raise ValueError("unpaired native tool observation")
            found.append(content)
    # New native rendering uses ordinary message bodies. Frame delimiters cannot
    # be forged by repository strings: the codec escapes them inside JSON data.
    for frame in prompt.split("<|start|>")[1:]:
        if "<|message|>" not in frame:
            continue
        header, body = frame.split("<|message|>", 1)
        if header.startswith("tool "):
            if not body.startswith('<tool_output name="') or "\n" not in body:
                raise ValueError("invalid native ATEM feedback frame")
            body = body.split("\n", 1)[1]
        elif not (header.startswith("functions.") and
                  header.endswith(" to=assistant<|channel|>commentary")):
            continue
        content, _ = decoder.raw_decode(body)
        if "message_id" in content:
            continue  # Already collected the older envelope above.
        if not isinstance(content, dict) or content.get("untrusted_tool_observation") is not True:
            raise ValueError("invalid native feedback body")
        if "completed_call" not in content or "result" not in content:
            raise ValueError("unpaired native tool observation")
        found.append(content)
    return found


def ordered_feedback(events: list[dict], prompts: list[str]) -> list[dict]:
    by_call = {}
    for prompt in prompts:
        for item in observations(prompt):
            call_id = item["completed_call"]["tool_call_id"]
            if item["result"]["tool_call_id"] != call_id:
                raise ValueError("feedback call identity mismatch")
            if call_id in by_call and by_call[call_id] != item:
                raise ValueError("conflicting recorded tool observation")
            by_call[call_id] = item
    ordered = []
    for event in events:
        kind = event.get("kind", {})
        if kind.get("event") != "tool_completed":
            continue
        item = by_call.get(kind["tool_call_id"])
        if item is None or item["result_sha256"] != kind["result_sha256"]:
            raise ValueError("completed effect missing exact later prompt feedback")
        ordered.append(item)
    return ordered


def inspect(log_dir: Path, records: Path, workspace: Path, case: str) -> dict:
    run = json.loads((log_dir / "result.json").read_text())
    events = [json.loads(line) for line in (log_dir / "stdout.jsonl").read_text().splitlines()]
    outcomes = [row for row in events if "state" in row and "run_id" in row]
    outcome = outcomes[-1] if outcomes else {}
    prompts, profiles, results, rejections, record_hashes = [], [], [], [], []
    for path in sorted(records.glob("*.metadata.json")):
        metadata_bytes = path.read_bytes()
        metadata = json.loads(metadata_bytes)
        raw_path = path.with_name(path.name.removesuffix(".metadata.json") + ".response.bin")
        raw = raw_path.read_bytes()
        kind = metadata.get("kind")
        if kind == "rendered-prompt":
            if digest(raw) != metadata["preflight"]["rendered_prompt_sha256"]:
                raise ValueError("rendered prompt digest mismatch")
            prompts.append(raw.decode())
            profiles.append(metadata["profile"])
        elif kind == "model-response":
            if digest(raw) != metadata["result"]["response_sha256"]:
                raise ValueError("native response digest mismatch")
            results.append(metadata["result"])
        else:
            if digest(raw) != metadata["response_sha256"]:
                raise ValueError("rejection digest mismatch")
            rejections.append(metadata)
        record_hashes.append({"metadata": path.name, "sha256": digest(metadata_bytes),
                              "raw_sha256": digest(raw), "raw_bytes": len(raw)})
    feedback_error = None
    try:
        feedback = ordered_feedback(events, prompts)
    except ValueError as error:
        feedback, feedback_error = [], str(error)
    validation = []
    tools = []
    for index, item in enumerate(feedback):
        tool = item["completed_call"]["tool_id"]
        tools.append(tool)
        if tool == "agentmage.validation.run-template":
            receipt = item["result"]["output"]["content"]
            validation.append({"index": index, "status": receipt["status"],
                               "exit_code": receipt["exit_code"], "failed": receipt["failed"],
                               "passed": receipt["passed"], "coverage": receipt["coverage"],
                               "receipt_sha256": receipt["receipt_sha256"]})
    status = subprocess.run(
        ["git", "--no-optional-locks", "status", "--porcelain=v1", "--untracked-files=all"],
        cwd=workspace, check=True, capture_output=True, text=True,
    ).stdout.splitlines()
    diff = subprocess.run(["git", "--no-optional-locks", "diff", "--no-ext-diff", "--binary"],
                          cwd=workspace, check=True, capture_output=True).stdout
    expected = {"repair": [" M src/calc.py"], "new-file": ["?? src/calc.py"],
                "multi-file": [" M src/calc.py", " M src/subtract.py"], "stable": []}[case]
    edits = [index for index, tool in enumerate(tools)
             if tool in {"agentmage.code.patch-file", "agentmage.code.create-file"}]
    failed = [item["index"] for item in validation if item["exit_code"] != 0 and item["failed"] > 0]
    passed = [item["index"] for item in validation if item["exit_code"] == 0 and
              item["failed"] == 0 and item["passed"] > 0 and item["coverage"] == "complete"]
    resources = run.get("candidate_resources") or {}
    artifacts = [event for event in events if event.get("type") == "runtime_artifact_verified"]
    inspections = {item["completed_call"]["arguments"].get("operation") for item in feedback
                   if item["completed_call"]["tool_id"] == "agentmage.git.inspect"}
    checks = {
        "native-candidate": run["model"] in {"muse", "gpt-oss"},
        "recording-consent": run.get("record_session") is True,
        "terminal-verifier": run["exit_code"] == 0 and len(outcomes) == 1 and
            outcome.get("state") == ("NO_OP" if case == "stable" else "SUCCESS") and
            outcome.get("answer_evidence") is not None and not outcome.get("unresolved_codes"),
        "binaries-unchanged": run.get("binary_identity_unchanged") is True,
        "exact-profile": bool(profiles) and all(profile == profiles[0] for profile in profiles) and
            profiles[0]["context"]["max_context_tokens"] == 32768 and
            profiles[0]["decoding"]["max_output_tokens"] == 4096,
        "native-responses": bool(results) and not rejections and len(results) == len(prompts) and
            all(result["terminal_state"] == "proposed" and not result["failure"] for result in results),
        "feedback": feedback_error is None and bool(feedback),
        "filesystem": status == expected,
        "full-artifacts": bool(artifacts) and any("context-packet" in item["media_type"] for item in artifacts),
        "resource-guard": resources.get("resource_guard_error") is None and
            0 < resources.get("peak_total_gpu_used_mib_sampled", 0) <= 22528,
        "passing-native-validation": bool(passed),
        "failed-test-repair": case == "stable" or any(f < e < p for f in failed for e in edits for p in passed),
        "current-diff-status": {"diff", "status"}.issubset(inspections),
    }
    return {"schema_version": 1, "scope": "diagnostic-live-coding-not-model-admission",
            "case": case, "model": run["model"], "checks": checks, "passed": all(checks.values()),
            "feedback_error": feedback_error, "terminal": outcome.get("state"),
            "elapsed_seconds": run.get("elapsed_seconds"), "implementation": run.get("implementation"),
            "profile": profiles[0] if profiles else None, "candidate_resources": resources,
            "tool_order": tools, "validation_receipts": validation, "worktree_status": status,
            "diff_sha256": digest(diff), "verified_artifact_count": len(artifacts),
            "records": record_hashes,
            "logs": {name: digest((log_dir / name).read_bytes())
                     for name in ("stdout.jsonl", "stderr.log", "result.json")}}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--log-dir", type=Path, required=True)
    parser.add_argument("--records", type=Path, required=True)
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--case", choices=("repair", "new-file", "multi-file", "stable"), required=True)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    report = inspect(args.log_dir, args.records, args.workspace, args.case)
    serialized = json.dumps(report, sort_keys=True, indent=2) + "\n"
    if args.output:
        descriptor = os.open(args.output, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
        with os.fdopen(descriptor, "w") as stream:
            stream.write(serialized)
        print(json.dumps({"passed": report["passed"], "checks": report["checks"], "output": str(args.output)}))
    else:
        print(serialized, end="")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
