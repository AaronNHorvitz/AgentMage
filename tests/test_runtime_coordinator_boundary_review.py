from __future__ import annotations

import copy
import re
import subprocess
import unittest
from pathlib import Path

from scripts.runtime_coordinator_boundary_review import (
    ARTIFACT_PREPARATION,
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
    def test_model_dispatch_keeps_exact_origin_registry_and_call_across_line_wrapping(self) -> None:
        sources = worktree_sources()
        dispatch = "ToolDispatcher::new(&self.registry).dispatch(requested.origin, &call);"
        assignment = r"let\s+receipt\s*=\s*" + re.escape(dispatch)
        self.assertEqual(len(re.findall(assignment, sources[RUNTIME_LOOP])), 1)
        for spacing in (" ", "\n                "):
            changed = dict(sources)
            changed[RUNTIME_LOOP] = re.sub(
                assignment, "let receipt =" + spacing + dispatch, sources[RUNTIME_LOOP],
            )
            checks = {item["check_id"]: item["passed"] for item in review_checks(changed)}
            self.assertTrue(checks["model-proposal-is-inert"])
        model = "origin: ProposalOrigin::Model,"
        shell = "origin: ProposalOrigin::Shell,"
        destructure = "            closing_sha256,\n            ..\n        } = requested;"
        self.assertEqual(sources[RUNTIME_LOOP].count(model), 1)
        self.assertEqual(sources[RUNTIME_LOOP].count(shell), 1)
        self.assertEqual(sources[RUNTIME_LOOP].count(destructure), 1)
        for old, new in (
            # A model call dispatched under another class, registry or call.
            (dispatch, dispatch.replace("requested.origin", "ProposalOrigin::Shell")),
            (dispatch, dispatch.replace("&self.registry", "&another_registry")),
            (dispatch, dispatch.replace("&call", "&another_call")),
            # The model path claims the shell's class, or the derived path the model's.
            (model, shell),
            (shell, model),
            # A second dispatch site or another proposal class.
            (dispatch, dispatch + "\n        let other = ToolDispatcher::new(&self.registry);"),
            (shell, "origin: ProposalOrigin::Tool,"),
            # The class is rebound before the dispatch site (review V2).
            (
                "let receipt = " + dispatch,
                "let origin = ProposalOrigin::Shell;\n            let receipt = "
                + dispatch.replace("requested.origin", "origin"),
            ),
            (
                destructure,
                "            closing_sha256,\n            origin: _,\n        } = requested;"
                "\n        let origin = ProposalOrigin::Shell;",
            ),
            (
                "let receipt = " + dispatch,
                "let origin = if arguments_valid { ProposalOrigin::Shell } else "
                "{ requested.origin };\n            let receipt = "
                + dispatch.replace("requested.origin", "origin"),
            ),
            (
                "let receipt = " + dispatch,
                "let requested = RequestedToolCall { origin: ProposalOrigin::Shell, "
                "..requested };\n            let receipt = " + dispatch,
            ),
            (
                "requested: RequestedToolCall,",
                "mut requested: RequestedToolCall,",
            ),
            (
                "let receipt = " + dispatch,
                "requested.origin = ProposalOrigin::Shell;\n            let receipt = " + dispatch,
            ),
        ):
            changed = dict(sources)
            self.assertIn(old, changed[RUNTIME_LOOP])
            changed[RUNTIME_LOOP] = changed[RUNTIME_LOOP].replace(old, new, 1)
            checks = {item["check_id"]: item["passed"] for item in review_checks(changed)}
            self.assertFalse(checks["model-proposal-is-inert"], new)

    def test_a_pattern_rebinding_or_a_reclassifying_helper_fails_the_inert_check(self) -> None:
        # Review V2 of 8fbd2bc6: both probe mutants passed the Decision 0115
        # check, since a helper assigned `origin =` and the rebinding was a
        # pattern rather than `let requested`.
        sources = worktree_sources()
        dispatch = "let receipt = ToolDispatcher::new(&self.registry).dispatch(requested.origin, &call);"
        helper = (
            "\nfn reclass(mut request: RequestedToolCall) -> RequestedToolCall {\n"
            "    request.origin = ProposalOrigin::Shell;\n    request\n}\n"
        )
        identity = "\nfn reclass(request: RequestedToolCall) -> RequestedToolCall {\n    request\n}\n"
        mutants = []
        for rebinding in (
            "let (requested, _) = (reclass(requested), ());\n            " + dispatch,
            "if let Some(requested) = Some(reclass(requested)) {\n                "
            + dispatch + "\n            }",
            "let (requested, _) = (requested, ());\n            " + dispatch,
            "let receipt = [requested].map(|requested| ToolDispatcher::new(&self.registry)"
            ".dispatch(requested.origin, &call))[0];\n            " + dispatch,
        ):
            for appended in (helper, identity):
                mutants.append((dispatch, rebinding, appended))
        # The reclassifying helper alone, and any other assignment of an origin.
        mutants.append((dispatch, dispatch, helper))
        mutants.append((dispatch, dispatch, "\nfn classify(origin: ProposalOrigin) { let origin = origin; }\n"))
        for old, new, appended in mutants:
            changed = dict(sources)
            self.assertIn(old, changed[RUNTIME_LOOP])
            changed[RUNTIME_LOOP] = changed[RUNTIME_LOOP].replace(old, new, 1) + appended
            checks = {item["check_id"]: item["passed"] for item in review_checks(changed)}
            self.assertFalse(checks["model-proposal-is-inert"], (new, appended))
        # The unchanged source, and comparisons of the class, still pass.
        compared = dict(sources)
        compared[RUNTIME_LOOP] += "\nfn same(origin: ProposalOrigin) -> bool { origin == ProposalOrigin::Model }\n"
        checks = {item["check_id"]: item["passed"] for item in review_checks(compared)}
        self.assertTrue(checks["model-proposal-is-inert"])

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

    def test_preparation_child_cannot_hide_a_forbidden_owner_or_transport(self) -> None:
        sources = worktree_sources()
        self.assertIn(ARTIFACT_PREPARATION, SOURCE_PATHS)
        mutations = (
            ("use std::process::Command;", "coordinator-no-direct-native-effect"),
            ("use std::fs;", "coordinator-no-direct-native-effect"),
            ("use std::net;", "coordinator-no-direct-native-effect"),
            ("use crate::operational_store::OperationalStore;", "coordinator-no-direct-storage"),
            ("use crate::mcp_registry::McpRegistry;", "coordinator-no-mcp-shortcut"),
            ("struct RuntimeHostResponse;", "coordinator-transport-independent"),
            ("struct CodingClient;", "coordinator-client-independent"),
            ("struct CapabilityGrant;", "tool-dispatch-is-grant-gated"),
        )
        for addition, check_id in mutations:
            with self.subTest(check_id=check_id, addition=addition):
                changed = dict(sources)
                changed[ARTIFACT_PREPARATION] += "\n" + addition + "\n"
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
