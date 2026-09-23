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


def accounted_native_responses(records: list[dict], rejections: list[dict],
                               events: list[dict], prompts: list[str], prompt_run_ids: list[str]) -> bool:
    """A rejected frame stays rejected; only a bound, budgeted later turn may correct it."""
    if not records or len(records) != len(prompts) or len(prompts) != len(prompt_run_ids):
        return False
    requests = [event for event in events if event.get("kind", {}).get("event") == "model_requested"]
    request_ids = [event["kind"]["model_run_id"] for event in requests]
    response_ids = [record["result"].get("model_run_id") for record in records]
    if len(request_ids) != len(records) or len(set(request_ids)) != len(request_ids) or \
            set(prompt_run_ids) != set(request_ids) or set(response_ids) != set(request_ids):
        return False
    request_sequences = {event["kind"]["model_run_id"]: event["sequence"] for event in requests}
    rejected = [record for record in records if record["result"]["terminal_state"] == "rejected"]
    argument_rejections = sum(event.get("kind", {}).get("event") == "tool_rejected" and
                              event["kind"].get("reason") == "arguments_invalid" for event in events)
    if len(rejected) != len(rejections) or len(rejected) + argument_rejections > 1:
        return False
    if any(record["result"]["terminal_state"] != "proposed" or record["result"]["failure"]
           for record in records if record not in rejected):
        return False
    decoder = json.JSONDecoder()
    notices = []
    for prompt_run_id, prompt in zip(prompt_run_ids, prompts, strict=True):
        for frame in prompt.split("<|start|>user<|message|>")[1:]:
            try:
                value, _ = decoder.raw_decode(frame)
            except ValueError:
                continue
            if isinstance(value, dict) and value.get("model_protocol_rejection") is True:
                notices.append((prompt_run_id, value))
    for record in rejected:
        result = record["result"]
        canonical = record.get("canonical_result", "")
        try:
            if json.loads(canonical) != result:
                return False
        except ValueError:
            return False
        result_digest = digest(canonical.encode())
        if result_digest != record.get("result_sha256") or result.get("proposal") is not None:
            return False
        if result.get("finish_reason") not in {"end_of_sequence", "configured_stop"} or result.get("failure") != {
            "code": "runtime.model.protocol_rejected", "retryable_after_correction": True,
            "dependency_recovery_required": False, "contract_error": None,
        }:
            return False
        originals = [item for item in rejections if item.get("model_run_id") == result["model_run_id"]
                     and item["response_sha256"] == result["response_sha256"]]
        if len(originals) != 1:
            return False
        original = originals[0].get("validated_result", {})
        if not isinstance(original, dict) or not isinstance(original.get("failure"), dict):
            return False
        codec_code = originals[0]["codec_failure_code"]
        if codec_code not in {
            "model.muse-codec.native-channel-invalid", "model.muse-codec.native-json-invalid",
            "model.muse-codec.final-channel-invalid", "model.gpt-oss-codec.tool-channel-invalid",
            "model.gpt-oss-codec.tool-arguments-invalid", "model.gpt-oss-codec.final-channel-invalid",
        } or original.get("failure", {}).get("code") != codec_code:
            return False
        if original.get("failure", {}).get("dependency_recovery_required") is not False or \
                original.get("failure", {}).get("contract_error") is not None:
            return False
        if {key: value for key, value in original.items() if key != "failure"} != \
                {key: value for key, value in result.items() if key != "failure"}:
            return False
        failures = [event for event in events if event.get("kind", {}).get("event") == "model_failed"
                    and event["kind"].get("model_run_id") == result["model_run_id"]
                    and event["kind"].get("failure_code") == "runtime.model.protocol_rejected"]
        if len(failures) != 1:
            return False
        failure = failures[0]
        if not any(event.get("turn_id") == failure["turn_id"] and event["sequence"] > failure["sequence"]
                   and event.get("kind", {}).get("event") == "turn_completed"
                   and event["kind"].get("outcome_sha256") == result_digest for event in events):
            return False
        if any(event.get("turn_id") == failure["turn_id"] and event.get("kind", {}).get("event") in {
            "proposal_observed", "tool_requested", "permission_requested", "tool_started", "tool_completed",
        } for event in events):
            return False
        if not any(request_sequences[prompt_run_id] > failure["sequence"] and
                   notice.get("model_run_id") == result["model_run_id"] and
                   notice.get("response_sha256") == result["response_sha256"] and
                   notice.get("result_sha256") == result_digest and notice.get("effect_occurred") is False
                   for prompt_run_id, notice in notices):
            return False
    return True


def inspect(log_dir: Path, records: Path, workspace: Path, case: str) -> dict:
    run = json.loads((log_dir / "result.json").read_text())
    events = [json.loads(line) for line in (log_dir / "stdout.jsonl").read_text().splitlines()]
    outcomes = [row for row in events if "state" in row and "run_id" in row]
    outcome = outcomes[-1] if outcomes else {}
    prompts, profiles, rejections, record_hashes = [], [], [], []
    response_records = []
    prompt_run_ids = []
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
            prompt_run_ids.append(metadata["request"]["model_run_id"])
            profiles.append(metadata["profile"])
        elif kind == "model-response":
            if digest(raw) != metadata["result"]["response_sha256"]:
                raise ValueError("native response digest mismatch")
            response_records.append(metadata)
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
        "native-responses": accounted_native_responses(response_records, rejections, events, prompts, prompt_run_ids),
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
            "protocol_rejection_count": len(rejections),
            "tool_rejections": [event["kind"] for event in events if event.get("kind", {}).get("event") == "tool_rejected"],
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
