import copy
import importlib.util
import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = ROOT / "scripts/macos_platform_manifest.py"
SPEC = importlib.util.spec_from_file_location("macos_platform_manifest", MODULE_PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class MacosPlatformManifestTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.baseline = json.loads(MODULE.FIELD_FREEZE_PATH.read_text(encoding="utf-8"))

    def test_committed_field_freeze_passes_and_canonicalizes(self):
        self.assertEqual(MODULE.validate_field_freeze(self.baseline), [])
        first = MODULE.canonical_json(self.baseline)
        second = MODULE.canonical_json(copy.deepcopy(self.baseline))
        self.assertEqual(first, second)

    def test_missing_or_extra_top_level_field_is_rejected(self):
        missing = copy.deepcopy(self.baseline)
        missing.pop("team_id")
        self.assertTrue(MODULE.validate_field_freeze(missing))
        extra = copy.deepcopy(self.baseline)
        extra["unexpected"] = "value"
        self.assertTrue(MODULE.validate_field_freeze(extra))

    def test_identity_and_platform_mutations_fail(self):
        for field, mutated in (
            ("schema_version", 1),
            ("record_type", "platform-release-manifest"),
            ("status", "signed-release"),
            ("platform_family", "fedora"),
            ("architecture", "x86_64"),
        ):
            candidate = copy.deepcopy(self.baseline)
            candidate[field] = mutated
            self.assertTrue(MODULE.validate_field_freeze(candidate), field)

    def test_hash_and_identity_object_mutations_fail(self):
        for path in (
            ("platform_build", "sha256"),
            ("product_toolchain", "sha256"),
            ("vscode", "sha256"),
            ("package", "sha256"),
        ):
            candidate = copy.deepcopy(self.baseline)
            candidate[path[0]][path[1]] = "not-a-hash"
            self.assertTrue(MODULE.validate_field_freeze(candidate), path)
        wrong_commit = copy.deepcopy(self.baseline)
        wrong_commit["vscode"]["commit"] = "shortcommit"
        self.assertTrue(MODULE.validate_field_freeze(wrong_commit))
        wrong_format = copy.deepcopy(self.baseline)
        wrong_format["package"]["format"] = "dmg"
        self.assertTrue(MODULE.validate_field_freeze(wrong_format))
        wrong_class = copy.deepcopy(self.baseline)
        wrong_class["package"]["identity_class"] = "released-package"
        self.assertTrue(MODULE.validate_field_freeze(wrong_class))

    def test_team_id_bundle_and_app_group_mutations_fail(self):
        wrong_team = copy.deepcopy(self.baseline)
        wrong_team["team_id"] = "shortid"
        self.assertTrue(MODULE.validate_field_freeze(wrong_team))
        lowercase = copy.deepcopy(self.baseline)
        lowercase["team_id"] = "agntmg0000"
        self.assertTrue(MODULE.validate_field_freeze(lowercase))
        wrong_group = copy.deepcopy(self.baseline)
        wrong_group["app_group"] = "OTHER00000.com.agentmage.shared"
        self.assertTrue(MODULE.validate_field_freeze(wrong_group))
        for helper in MODULE.HELPERS:
            candidate = copy.deepcopy(self.baseline)
            candidate["bundle_ids"][helper] = "not_a_reverse_dns"
            self.assertTrue(MODULE.validate_field_freeze(candidate), helper)

    def test_designated_requirements_must_bind_bundle_and_team(self):
        missing_helper = copy.deepcopy(self.baseline)
        missing_helper["designated_requirements"].pop("kernel_host")
        self.assertTrue(MODULE.validate_field_freeze(missing_helper))
        wrong_bundle = copy.deepcopy(self.baseline)
        wrong_bundle["designated_requirements"]["kernel_host"] = (
            'identifier "com.example.other" and anchor apple generic'
            ' and certificate leaf[subject.OU] = "AGNTMG0000"'
        )
        self.assertTrue(MODULE.validate_field_freeze(wrong_bundle))
        wrong_team = copy.deepcopy(self.baseline)
        wrong_team["designated_requirements"]["kernel_host"] = (
            'identifier "com.agentmage.kernel" and anchor apple generic'
            ' and certificate leaf[subject.OU] = "WRONGTEAM0"'
        )
        self.assertTrue(MODULE.validate_field_freeze(wrong_team))

    def test_entitlements_must_be_sorted_deduplicated_non_empty(self):
        empty = copy.deepcopy(self.baseline)
        empty["entitlements"]["metal_inference"] = []
        self.assertTrue(MODULE.validate_field_freeze(empty))
        duplicate = copy.deepcopy(self.baseline)
        duplicate["entitlements"]["metal_inference"] = [
            "com.apple.security.app-sandbox",
            "com.apple.security.app-sandbox",
        ]
        self.assertTrue(MODULE.validate_field_freeze(duplicate))
        unsorted = copy.deepcopy(self.baseline)
        unsorted["entitlements"]["kernel_host"] = list(
            reversed(sorted(self.baseline["entitlements"]["kernel_host"]))
        )
        self.assertTrue(MODULE.validate_field_freeze(unsorted))

    def test_helper_hashes_are_unique_and_hex(self):
        wrong_hash = copy.deepcopy(self.baseline)
        wrong_hash["helper_hashes"]["kernel_host"] = "not-a-hash"
        self.assertTrue(MODULE.validate_field_freeze(wrong_hash))
        duplicate = copy.deepcopy(self.baseline)
        duplicate["helper_hashes"]["kernel_host"] = duplicate["helper_hashes"]["vscode_bridge"]
        self.assertTrue(MODULE.validate_field_freeze(duplicate))
        missing_helper = copy.deepcopy(self.baseline)
        missing_helper["helper_hashes"].pop("metal_inference")
        self.assertTrue(MODULE.validate_field_freeze(missing_helper))

    def test_private_release_and_forbidden_field_overclaims_fail(self):
        for field in (
            "credential_values_present",
            "private_environment_values_present",
            "linux_evidence_substituted",
        ):
            candidate = copy.deepcopy(self.baseline)
            candidate[field] = True
            self.assertTrue(MODULE.validate_field_freeze(candidate), field)
        released = copy.deepcopy(self.baseline)
        released["release_claim"] = "supported"
        self.assertTrue(MODULE.validate_field_freeze(released))
        for key in ("hostname", "signature", "signer_identity", "notarization_ticket", "username"):
            candidate = copy.deepcopy(self.baseline)
            candidate["package"][key] = "value"
            self.assertTrue(MODULE.validate_field_freeze(candidate), key)


if __name__ == "__main__":
    unittest.main()
