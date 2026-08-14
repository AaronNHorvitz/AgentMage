#!/usr/bin/env python3
"""Task 12.2.3.3 agent-failure-corpus evidence specification."""

from pathlib import Path

try:
    from scripts.revision_evidence import RevisionEvidenceSpec, run_cli
except ModuleNotFoundError:
    from revision_evidence import RevisionEvidenceSpec, run_cli

ROOT = Path(__file__).resolve().parents[1]
SPEC = RevisionEvidenceSpec(
    root=ROOT,
    report_path=ROOT / "artifacts/sprints/sprint-12/story-12.2/agent-failure-corpus.json",
    artifact_id="task-12-2-3-3-agent-failure-corpus",
    case_pattern=r"`(APF-\d{3})`",
    case_ids=tuple(f"APF-{index:03}" for index in range(1, 7)),
    source_paths=(
        "docs/verification/task-12-2-3-3-agent-failure-corpus-results.md",
        "fixtures/agent-policy/v1/failure-corpus.json",
        "scripts/validate_agent_failure_corpus.py",
        "tests/test_agent_failure_corpus.py",
        "artifacts/sprints/sprint-12/story-12.2/restart-reconciliation.json",
        "artifacts/sprints/sprint-12/story-12.2/verifier-registry.json",
        "artifacts/sprints/sprint-12/story-12.2/classifier-failure.json",
        "artifacts/sprints/sprint-12/story-12.2/agent-ceilings.json",
        "scripts/revision_evidence.py",
        "scripts/agent_failure_corpus_evidence.py",
        "tests/test_agent_failure_corpus_evidence.py",
    ),
    document_fragments=(
        "Pass for the versioned, content-free agent and policy failure corpus.",
        "Lock transitions until exact reconciliation",
        "Block resume and duplicate effect",
        "Reject unreconciled grant replay",
        "Reject nondeterministic completion evidence",
        "Require an explicit user decision",
        "Enter the sticky `STALLED` terminal state",
    ),
    source_fragments={
        "fixtures/agent-policy/v1/failure-corpus.json": (
            '"category": "restart"',
            '"category": "uncertain_effect"',
            '"category": "consumed_grant"',
            '"category": "false_completion"',
            '"category": "classifier_disagreement"',
            '"category": "no_progress"',
        ),
        "scripts/validate_agent_failure_corpus.py": (
            "EXPECTED_DISPOSITIONS = {",
            '"success" not in prohibited',
            "sha256_file(evidence_path) != expected_sha",
        ),
        "tests/test_agent_failure_corpus.py": (
            "test_every_case_rejects_false_success",
        ),
    },
    command_specs=(
        (("python3", "scripts/validate_agent_failure_corpus.py"), "Agent failure corpus: pass (6 cases)"),
        (("python3", "-m", "unittest", "tests.test_agent_failure_corpus"), "Ran 4 tests"),
        (("cargo", "test", "-p", "agentmage-kernel-engine", "task_12_2_1_", "--locked"), "30 passed; 0 failed"),
        (("cargo", "test", "-p", "agentmage-kernel-engine", "task_12_2_2_", "--locked"), "24 passed; 0 failed"),
        (("python3", "-m", "unittest", "tests.test_agent_failure_corpus_evidence"), "Ran 4 tests"),
        (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
        (("npm", "run", "docs:mermaid"), "Validated 60 Mermaid block(s)."),
    ),
    claims={
        "focused_case_count": 6,
        "category_count": 6,
        "retained_source_artifact_count": 4,
        "false_success_expectations": 0,
        "unbound_cases": 0,
        "external_network_used": False,
        "manual_fuzzing_executed": False,
        "release_support": False,
    },
    limitations=(
        "The corpus indexes focused deterministic unit evidence; it is not the full crash-before-and-after transition campaign required by Task 12.2.4.4.",
        "The cases use no production worker, model, classifier, tool, provider, private user data, or external network operation.",
        "Encrypted persistence integration, cross-platform execution, packaging, release acceptance, and manual fuzzing remain later tasks and gates.",
    ),
    status="pass-agent-failure-corpus",
    task_ids=("12.2.3.3",),
    label="Task 12.2.3.3 agent failure corpus",
)


if __name__ == "__main__":
    raise SystemExit(run_cli(SPEC))
