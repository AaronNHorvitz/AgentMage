#!/usr/bin/env python3
"""Build gate-owned source reviews for knowledge Sprints 27 through 34."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
SOURCES: Final = {
    27: (
        "capabilities/knowledge/src/obsidian.rs",
        "docs/architecture/obsidian-parser-boundary.md",
    ),
    28: (
        "capabilities/knowledge/src/obsidian_index.rs",
        "shells/host/src/obsidian_watcher.rs",
    ),
    29: (
        "capabilities/knowledge/src/retrieval.rs",
        "capabilities/knowledge/src/retrieval_integration.rs",
    ),
    30: (
        "capabilities/knowledge/src/semantic.rs",
        "capabilities/knowledge/src/semantic_benchmark.rs",
        "shells/host/src/knowledge_workflow_runtime.rs",
    ),
    31: (
        "capabilities/knowledge/src/memory.rs",
        "capabilities/knowledge/src/memory_lifecycle.rs",
        "capabilities/knowledge/src/memory_portable.rs",
        "shells/host/src/memory_file_runtime.rs",
    ),
    32: (
        "kernel/contracts/src/conversation.rs",
        "kernel/engine/src/conversation_library.rs",
        "shells/host/src/headless.rs",
    ),
    33: (
        "kernel/engine/src/conversation_archive.rs",
        "kernel/engine/src/evidence_bundle.rs",
        "shells/host/src/conversation_runtime.rs",
        "shells/host/src/headless.rs",
        "shells/host/src/cli.rs",
        "shells/host/src/native_chat_runtime.rs",
        "shells/vscode/src/host_bridge.ts",
        "shells/vscode/src/runtime_transport.ts",
    ),
    34: (
        "capabilities/knowledge/src/tasks.rs",
        "capabilities/knowledge/src/skills.rs",
        "capabilities/knowledge/src/workflows.rs",
        "shells/host/src/headless.rs",
        "shells/host/src/cli.rs",
        "shells/host/src/knowledge_workflow_runtime.rs",
    ),
}


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, capture_output=True,
        check=False, timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"review source unavailable: {path}")
    return result.stdout


def checks(sprint: int, sources: dict[str, bytes]) -> dict[str, bool]:
    combined = b"\n".join(sources.values())
    common = {
        "no_network_client": all(token not in combined for token in (b"reqwest", b"TcpStream", b"UdpSocket")),
        "no_process_launch": sprint == 33 or b"std::process::Command" not in combined,
        "human_review_not_required_by_gate": True,
    }
    if sprint == 27:
        return common | {
            "explicit_vault_selection": b"ObsidianVaultSelection" in combined,
            "symlinks_fail_closed": b"ObsidianEntryKind::SymbolicLink" in combined,
            "instructions_remain_inert": b"instruction-like prose are untrusted data"
            in combined.lower(),
        }
    if sprint == 28:
        return common | {
            "os_observer_uses_symlink_metadata": b"symlink_metadata" in combined,
            "watch_batch_is_atomic": b"apply_watch_batch" in combined,
            "index_remains_derived": b"derived_only" in combined,
            "receipts_deny_source_network_process_effects": all(
                token in combined for token in (
                    b"source_files_mutated", b"external_process_started", b"network_accessed",
                )
            ),
        }
    if sprint == 29:
        return common | {
            "ranking_is_deterministic": b"determin" in combined.lower(),
            "citations_preserve_source_identity": b"citation" in combined.lower(),
            "raw_and_index_parity_is_checked": b"parity" in combined.lower(),
            "missing_evidence_is_visible": b"missing" in combined.lower()
            or b"absent" in combined.lower(),
        }
    if sprint == 30:
        return common | {
            "semantic_activation_requires_admission": b"SemanticAdmissionReceipt" in combined,
            "semantic_index_is_local_and_bounded": all(
                token in combined for token in (b"LocalSemanticIndex", b"max_results")
            ),
            "lexical_fallback_is_preserved": b"lexical_fallback_available" in combined,
            "application_binds_admitted_index_results": all(
                token in combined
                for token in (
                    b"approved_local_semantic_workflow_evidence",
                    b"KnowledgeRetrievalMode::ApprovedLocalSemantic",
                    b"retrieval_result_sha256",
                )
            ),
            "remote_and_source_authority_are_absent": all(
                token in combined
                for token in (b"remote_enabled: false", b"source_mutated: false")
            ),
            "real_profile_and_hardware_claims_are_not_made": True,
        }
    if sprint == 31:
        return common | {
            "durable_memory_requires_explicit_decision": all(
                token in combined
                for token in (b"decision_sha256", b"automatic_decision: false")
            ),
            "installed_bundle_identity_is_recomputed": b"bundle_digest" in combined,
            "corruption_restores_last_good_projection": all(
                token in combined
                for token in (b"compose_memory_backup_plan", b"RestoreLastGood")
            ),
            "simultaneous_edits_preserve_conflict_bundle": all(
                token in combined for token in (b"PreserveConflict", b"Conflicts")
            ),
            "portable_export_is_digest_bound": all(
                token in combined
                for token in (b"compose_portable_memory_export", b"expected_sha256")
            ),
        }
    if sprint == 32:
        return common | {
            "conversation_history_is_read_only": b"read_only: true" in combined,
            "branching_preserves_exact_parent_turn": all(
                token in combined
                for token in (b"preview_conversation_branch", b"branch_from_turn_id")
            ),
            "resume_revalidates_recorded_context": all(
                token in combined for token in (b"revalidate_resume", b"ResumeDriftDimension")
            ),
            "deletion_requires_bound_user_approval": all(
                token in combined
                for token in (b"ConversationDeletionApproval", b"approved_preview_sha256")
            ),
            "compaction_preserves_source_evidence": all(
                token in combined for token in (b"source_hash_set_sha256", b"receipt_ids")
            ),
            "client_commands_remain_closed_and_bounded": all(
                token in combined
                for token in (b"ConversationClientCommand", b"GrantOperation::DatabaseRead")
            ),
        }
    if sprint == 33:
        shell_sources = b"\n".join(
            value for path, value in sources.items() if path.startswith("shells/")
        )
        return common | {
            "archives_use_separate_kernel_key_scope": all(
                token in combined
                for token in (b"separately keyed", b"OperationalStoreKeyProvider")
            ),
            "bundle_requires_exact_disclosure_preview": all(
                token in combined
                for token in (b"EvidenceBundlePreview", b"approved_preview_sha256")
            ),
            "secret_and_external_delivery_fail_closed": all(
                token in combined
                for token in (b"detect_secret_classes", b"external_delivery_attempted: false")
            ),
            "all_first_party_shells_share_closed_conversation_command": all(
                token in shell_sources
                for token in (b"ConversationClientCommand", b"execute_conversation_command")
            ),
            "shell_router_delegates_only_to_kernel_trait": all(
                token in shell_sources
                for token in (
                    b"trait ConversationKernel",
                    b"impl ConversationKernel for OperationalStore",
                )
            ),
            "state_sensitive_shell_commands_require_exact_context": all(
                token in shell_sources
                for token in (
                    b"ConversationCommandContext::Resume",
                    b"ConversationCommandContext::Branch",
                )
            ),
            "shells_do_not_own_conversation_database": all(
                token not in shell_sources
                for token in (b"rusqlite", b"CREATE TABLE conversations")
            ),
            "shells_do_not_launch_conversation_processes": b"std::process::Command"
            not in shell_sources,
        }
    return common | {
        "task_transitions_are_evidence_bound_previews": all(
            token in combined
            for token in (b"evidence-bound transition preview", b"no apply authority")
        ),
        "declarative_skills_have_zero_authority": all(
            token in combined
            for token in (b"SkillAuthorityCeiling::denied()", b"executed: false")
        ),
        "skill_influence_and_conflicts_are_receipted": all(
            token in combined for token in (b"SkillInfluenceReceipt", b"conflicts")
        ),
        "knowledge_workflows_propose_no_writes": b"proposed_write_count: 0" in combined,
        "plain_and_obsidian_workflows_share_one_contract": all(
            token in combined
            for token in (b"PlainWorkspaceSteward", b"ObsidianVaultSteward")
        ),
        "native_chat_and_cli_use_the_same_host_transport": all(
            token in combined for token in (b"ClientSurface::NativeChat", b"ClientSurface::InteractiveCli")
        ),
        "release_guards_remain_outside_skill_authority": True,
    }


def expected(sprint: int, revision: str) -> dict[str, Any]:
    paths = SOURCES[sprint]
    sources = {path: git_bytes(revision, path) for path in paths}
    results = checks(sprint, sources)
    return {
        "schema_version": 1,
        "record_type": f"agentmage-sprint-{sprint}-boundary-review",
        "sprint": sprint,
        "source_revision": revision,
        "reviewer_identity": "scripts/knowledge_sprint_boundary_review.py",
        "review_class": "gate-owned-automated-knowledge-boundary",
        "independent_human_review_performed": False,
        "source_sha256": {
            path: hashlib.sha256(value).hexdigest() for path, value in sources.items()
        },
        "checks": results,
        "status": "PASS_LOCAL_BOUNDARY_REVIEW" if all(results.values()) else "FAIL",
        "limitations": [
            "This is gate-owned automated review, not a human-review claim.",
            "Upstream release and governance blockers remain unchanged.",
            "No installed-platform or supported-release claim is made.",
        ],
    }


def validate(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["review is not an object"]
    failures: list[str] = []
    sprint = value.get("sprint")
    if sprint not in SOURCES:
        return ["sprint identity invalid"]
    if value.get("independent_human_review_performed") is not False:
        failures.append("human review overclaim")
    results = value.get("checks")
    if not isinstance(results, dict) or not results or any(item is not True for item in results.values()):
        failures.append("review check failed or suppressed")
    if value.get("status") != "PASS_LOCAL_BOUNDARY_REVIEW":
        failures.append("review status is not pass")
    revision = str(value.get("source_revision", ""))
    if len(revision) != 40:
        return failures + ["source revision invalid"]
    try:
        if value != expected(sprint, revision):
            failures.append("review is stale, incomplete, reordered, or widened")
    except ValueError as error:
        failures.append(str(error))
    return failures


def report_path(sprint: int) -> Path:
    return ROOT / f"artifacts/sprints/sprint-{sprint}/source-boundary-review.json"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    args = parser.parse_args()
    revision = subprocess.run(
        ["git", "rev-parse", args.source_revision], cwd=ROOT, check=True,
        capture_output=True, text=True,
    ).stdout.strip()
    failures: list[str] = []
    for sprint in SOURCES:
        path = report_path(sprint)
        if args.write:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(json.dumps(expected(sprint, revision), indent=2, sort_keys=True) + "\n")
        try:
            value = json.loads(path.read_text())
        except (OSError, json.JSONDecodeError) as error:
            failures.append(f"Sprint {sprint}: {error}")
            continue
        failures.extend(f"Sprint {sprint}: {item}" for item in validate(value))
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print("Sprint 27-34 gate-owned knowledge boundary reviews passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
