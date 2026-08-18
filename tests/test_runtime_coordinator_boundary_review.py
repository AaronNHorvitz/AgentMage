from __future__ import annotations

import copy
import subprocess
import unittest
from pathlib import Path

from scripts.runtime_coordinator_boundary_review import (
    CODING_CLIENT,
    EXPECTED_CHECK_IDS,
    LIMITATIONS,
    READ_MANIFEST,
    REPORT_PATH,
    ROOT,
    RUNTIME_LOOP,
    RUNTIME_LOG,
    SOURCE_PATHS,
    VSCODE_PROVIDER,
    build_report,
    git_revision,
    read_report,
    review_checks,
    seal_report,
    validate_report,
)


SCRIPT_COMMITTED = subprocess.run(
    ["git", "cat-file", "-e", "HEAD:scripts/runtime_coordinator_boundary_review.py"],
    cwd=ROOT,
    stdout=subprocess.DEVNULL,
    stderr=subprocess.DEVNULL,
    check=False,
).returncode == 0


def worktree_sources() -> dict[str, str]:
    return {
        path: (Path(ROOT) / path).read_text(encoding="utf-8")
        for path in SOURCE_PATHS
        if path != RUNTIME_LOG
    }


class RuntimeCoordinatorBoundaryReviewTests(unittest.TestCase):
    def test_current_committed_boundary_passes_every_automated_check(self) -> None:
        checks = review_checks(worktree_sources())
        self.assertEqual([item["check_id"] for item in checks], list(EXPECTED_CHECK_IDS))
        self.assertTrue(all(item["passed"] for item in checks))

    def test_representative_shortcuts_fail_their_named_boundary(self) -> None:
        sources = worktree_sources()
        mutations = (
            (RUNTIME_LOOP, "\nuse std::process::Command;\n", "coordinator-no-direct-native-effect"),
            (RUNTIME_LOOP, "\nuse crate::mcp_registry::McpRegistry;\n", "coordinator-no-mcp-shortcut"),
            (RUNTIME_LOOP, "\nuse crate::operational_store::OperationalStore;\n", "coordinator-no-direct-storage"),
            (CODING_CLIENT, "\nuse agentmage_kernel_engine::tooling::ToolRegistry;\n", "shell-is-presentation-only"),
            (VSCODE_PROVIDER, '\nimport fs from "node:fs";\n', "shell-is-presentation-only"),
            (READ_MANIFEST, "\nagentmage-platform-linux.workspace = true\n", "capability-pack-has-no-platform-edge"),
        )
        for path, addition, check_id in mutations:
            changed = dict(sources)
            changed[path] += addition
            checks = {item["check_id"]: item["passed"] for item in review_checks(changed)}
            self.assertFalse(checks[check_id], check_id)

    @unittest.skipUnless(SCRIPT_COMMITTED, "report source is committed before seal testing")
    def test_report_seal_and_every_overclaim_are_rejected(self) -> None:
        report = seal_report(build_report(git_revision("HEAD")))
        self.assertEqual(validate_report(report), [])
        for field in (
            "independent_human_review_performed",
            "installed_runtime_observed",
            "manual_fuzzing_executed",
            "release_approved",
        ):
            changed = copy.deepcopy(report)
            changed[field] = True
            self.assertTrue(validate_report(changed), field)

        changed = copy.deepcopy(report)
        changed["checks"][0]["passed"] = False
        self.assertTrue(validate_report(changed))
        changed = copy.deepcopy(report)
        changed["report_sha256"] = "f" * 64
        self.assertTrue(validate_report(changed))

    def test_limitations_preserve_external_boundaries(self) -> None:
        combined = " ".join(LIMITATIONS)
        self.assertIn("not an independent human review", combined)
        self.assertIn("installed native client", combined)
        self.assertIn("Manual fuzzing", combined)
        self.assertIn("No release approval", combined)

    @unittest.skipUnless(REPORT_PATH.is_file(), "retained review follows source commit")
    def test_retained_review_matches_its_committed_source(self) -> None:
        self.assertEqual(validate_report(read_report()), [])


if __name__ == "__main__":
    unittest.main()
