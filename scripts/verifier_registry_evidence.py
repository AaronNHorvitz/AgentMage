#!/usr/bin/env python3
"""Task 12.2.1.4 deterministic-verifier evidence specification."""

from pathlib import Path

try:
    from scripts.revision_evidence import RevisionEvidenceSpec, run_cli
except ModuleNotFoundError:
    from revision_evidence import RevisionEvidenceSpec, run_cli

ROOT = Path(__file__).resolve().parents[1]
SPEC = RevisionEvidenceSpec(
    root=ROOT,
    report_path=ROOT / "artifacts/sprints/sprint-12/story-12.2/verifier-registry.json",
    artifact_id="task-12-2-1-4-verifier-registry",
    case_pattern=r"`(VRF-\d{2})`",
    case_ids=tuple(f"VRF-{index:02}" for index in range(1, 7)),
    source_paths=(
        "docs/verification/task-12-2-1-4-verifier-registry-results.md",
        "kernel/contracts/src/agent_verifier.rs",
        "kernel/contracts/src/ids.rs",
        "kernel/contracts/src/lib.rs",
        "kernel/contracts/src/serialization.rs",
        "kernel/engine/src/agent_verifier.rs",
        "kernel/engine/src/agent_state.rs",
        "kernel/engine/src/lib.rs",
        "scripts/revision_evidence.py",
        "scripts/verifier_registry_evidence.py",
        "tests/test_verifier_registry_evidence.py",
    ),
    document_fragments=(
        "Pass for deterministic typed completion proofs and fail-closed advisory completion claims.",
        "Only the registry can construct `VerifiedCompletion`",
        "all five prohibited advisory sources",
        "authenticated production verifier-worker provenance remains a later integration gate",
        "manual fuzzing",
    ),
    source_fragments={
        "kernel/contracts/src/agent_verifier.rs": (
            "pub enum VerifierSource",
            "pub struct PostconditionResult",
            "pub struct VerifierCandidate",
        ),
        "kernel/engine/src/agent_verifier.rs": (
            "pub struct VerifierRegistry",
            "pub struct VerifiedCompletion",
            "pub fn verify(",
        ),
        "kernel/engine/src/agent_state.rs": (
            "pub fn complete(",
            "return Err(AgentStateError::VerifierRequired);",
        ),
    },
    command_specs=(
        (("cargo", "test", "-p", "agentmage-kernel-engine", "task_12_2_1_4", "--locked"), "6 passed; 0 failed"),
        (("cargo", "test", "-p", "agentmage-kernel-contracts", "serialization", "--locked"), "5 passed; 0 failed"),
        (("cargo", "clippy", "-p", "agentmage-kernel-contracts", "-p", "agentmage-kernel-engine", "--all-targets", "--locked", "--", "-D", "warnings"), "Finished `dev` profile"),
        (("python3", "-m", "unittest", "tests.test_verifier_registry_evidence"), "Ran 4 tests"),
        (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
    ),
    claims={
        "focused_case_count": 6,
        "completion_route_count": 2,
        "rejected_advisory_source_count": 5,
        "bound_context_identity_count": 5,
        "maximum_postconditions": 128,
        "maximum_evidence_per_postcondition": 32,
        "public_completion_constructors": 0,
        "public_completion_deserializers": 0,
        "model_executions": 0,
        "classifier_executions": 0,
        "tool_executions": 0,
        "platform_executions": 0,
        "external_network_used": False,
        "manual_fuzzing_executed": False,
        "release_support": False,
    },
    limitations=(
        "This proves the in-process typed registry and state-controller boundary; authenticated production verifier-worker provenance remains a later integration gate.",
        "Evidence is synthetic and content-addressed to one repository snapshot; durable state, restart reconciliation, and receipt-chain integration remain later tasks.",
        "No model, classifier, tool, grant, worker, platform effect, private user data, or external network operation is used.",
        "Cross-platform execution, packaging, release acceptance, and manual fuzzing remain later gates.",
    ),
    status="pass-deterministic-verifier-registry",
    task_ids=("12.2.1.4",),
    label="Task 12.2.1.4 verifier registry",
)


if __name__ == "__main__":
    raise SystemExit(run_cli(SPEC))
