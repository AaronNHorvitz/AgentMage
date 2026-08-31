from __future__ import annotations

import copy
import json
import unittest
from pathlib import Path

from scripts.macos_hosted_compatibility import (
    CHECKOUT_SHA,
    COMMANDS,
    LIMITATIONS,
    ROOT,
    UPLOAD_SHA,
    check_source_contract,
    git,
    sha256,
    validate_result,
)


class MacOSHostedCompatibilityTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.commit = git("rev-parse", "HEAD")
        cls.tree = git("rev-parse", "HEAD^{tree}")
        cls.record = {
            "schema_version": 1,
            "task_id": "8.1.4.2",
            "artifact_id": "hosted-apple-silicon-source-compatibility",
            "status": "pass-preliminary-compatibility",
            "disposition": "preliminary-apple-silicon-source-compatibility-only",
            "source": {
                "repository": "owner/AgentMage",
                "commit": cls.commit,
                "tree": cls.tree,
                "workflow_path": ".github/workflows/macos.yml",
                "workflow_sha256": sha256((ROOT / ".github/workflows/macos.yml").read_bytes()),
            },
            "workflow": {
                "workflow_ref": "owner/AgentMage/.github/workflows/macos.yml@refs/heads/build/agentmage-ga",
                "run_id": "1234",
                "run_attempt": "1",
                "trigger": "workflow_dispatch",
                "budget_confirmed": True,
                "checkout_action_sha": CHECKOUT_SHA,
                "upload_action_sha": UPLOAD_SHA,
            },
            "runner": {
                "requested_label": "macos-15",
                "image_os": "macos15",
                "image_version": "20260824.1",
                "architecture": "arm64",
                "macos_product_version": "15.7",
                "macos_build_version": "24G222",
                "xcode_version": "Xcode 16.4\nBuild version 16F6",
                "swift_version": "Apple Swift version 6.1.2",
            },
            "commands": [
                {"argv": list(COMMANDS[0]), "outcome": "success"},
                {"argv": list(COMMANDS[1]), "outcome": "success"},
            ],
            "preliminary_compatibility": True,
            "native_operations": {
                "signing": False,
                "notarization": False,
                "installation": False,
                "support_or_release_promotion": False,
            },
            "limitations": list(LIMITATIONS),
        }

    def test_checked_in_source_contract_is_current(self) -> None:
        self.assertEqual(check_source_contract(), [])

    def test_exact_passing_record_validates(self) -> None:
        self.assertEqual(validate_result(self.record), [])

    def test_failure_record_cannot_claim_compatibility(self) -> None:
        value = copy.deepcopy(self.record)
        value["commands"][1]["outcome"] = "failure"
        self.assertTrue(validate_result(value))
        value["status"] = "fail-preliminary-compatibility"
        value["preliminary_compatibility"] = False
        self.assertEqual(validate_result(value), [])

    def test_source_and_runner_identity_mutations_fail(self) -> None:
        for path, replacement in (
            (("source", "workflow_sha256"), "0" * 64),
            (("runner", "architecture"), "x86_64"),
            (("runner", "image_version"), ""),
            (("workflow", "budget_confirmed"), False),
        ):
            value = copy.deepcopy(self.record)
            value[path[0]][path[1]] = replacement
            self.assertTrue(validate_result(value), path)

    def test_command_and_action_mutations_fail(self) -> None:
        value = copy.deepcopy(self.record)
        value["commands"][0]["argv"].append("--configuration=release")
        self.assertTrue(validate_result(value))
        value = copy.deepcopy(self.record)
        value["workflow"]["upload_action_sha"] = "0" * 40
        self.assertTrue(validate_result(value))

    def test_release_and_native_operation_overclaims_fail(self) -> None:
        value = copy.deepcopy(self.record)
        value["disposition"] = "macos-supported"
        self.assertTrue(validate_result(value))
        value = copy.deepcopy(self.record)
        value["native_operations"]["signing"] = True
        self.assertTrue(validate_result(value))

    def test_unknown_fields_fail_closed(self) -> None:
        value = copy.deepcopy(self.record)
        value["unexpected"] = True
        self.assertTrue(validate_result(value))


if __name__ == "__main__":
    unittest.main()
