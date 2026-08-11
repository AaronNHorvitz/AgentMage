import copy
import importlib.util
import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = ROOT / "scripts/platform_manifest_artifact.py"
SPEC = importlib.util.spec_from_file_location("platform_manifest_artifact", MODULE_PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class PlatformManifestArtifactTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifests = {
            path.name: json.loads(path.read_text(encoding="utf-8"))
            for path in (ROOT / "release/platform-manifests/v1").glob("*.json")
        }

    def test_all_declared_manifests_pass_and_repeat_canonically(self):
        self.assertEqual(set(self.manifests), set(MODULE.EXPECTED_PLATFORMS))
        for filename, manifest in self.manifests.items():
            self.assertEqual(MODULE.validate_manifest(manifest, filename), [])
            self.assertEqual(
                MODULE.canonical_json(manifest),
                MODULE.canonical_json(copy.deepcopy(manifest)),
            )

    def test_capability_removal_reordering_and_package_drift_fail(self):
        filename = "fedora-x86_64.json"
        baseline = self.manifests[filename]
        for mutation in ("remove", "reorder", "package"):
            candidate = copy.deepcopy(baseline)
            if mutation == "remove":
                candidate["capabilities"].pop()
            elif mutation == "reorder":
                candidate["capabilities"][0], candidate["capabilities"][1] = (
                    candidate["capabilities"][1],
                    candidate["capabilities"][0],
                )
            else:
                candidate["runtime_identities"]["package"]["sha256"] = "0" * 64
            self.assertTrue(MODULE.validate_manifest(candidate, filename), mutation)

    def test_platform_dependency_private_data_and_release_overclaims_fail(self):
        filename = "ubuntu-x86_64.json"
        baseline = self.manifests[filename]
        mutations = []
        wrong_platform = copy.deepcopy(baseline)
        wrong_platform["platform"]["family"] = "fedora"
        mutations.append(wrong_platform)
        overlap = copy.deepcopy(baseline)
        overlap["dependency_classes"]["end_user_runtime"].append("cargo")
        overlap["dependency_classes"]["end_user_runtime"].sort()
        mutations.append(overlap)
        private = copy.deepcopy(baseline)
        private["platform"]["hostname"] = "private-host"
        mutations.append(private)
        release = copy.deepcopy(baseline)
        release["release_claim"] = "supported"
        mutations.append(release)
        for candidate in mutations:
            self.assertTrue(MODULE.validate_manifest(candidate, filename))

    def test_contract_source_check_rejects_capability_or_os_branch_drift(self):
        contract = (ROOT / "kernel/contracts/src/platform.rs").read_text(encoding="utf-8")
        selector = (ROOT / "kernel/engine/src/platform_startup.rs").read_text(
            encoding="utf-8"
        )
        self.assertEqual(MODULE.validate_contract_sources(contract, selector), [])
        self.assertTrue(
            MODULE.validate_contract_sources(
                contract.replace("PlatformCapability::Packaging", "PlatformCapability::Updates"),
                selector,
            )
        )
        self.assertTrue(
            MODULE.validate_contract_sources(
                contract,
                selector.replace(
                    "pub fn activate_platform", "// target_os\npub fn activate_platform", 1
                ),
            )
        )

    def test_report_validation_rejects_macos_or_release_overclaim(self):
        report = {
            "schema_version": 1,
            "task_id": "7.1.2",
            "artifact_id": "platform-adapter-contract-and-manifest-evidence",
            "status": "pass-all-available-non-macos-contract-scope",
            "reference_revision": "a" * 40,
            "api": {
                "version": 1,
                "required_capability_count": 10,
                "startup_failure_class_count": 11,
                "operating_system_branches_in_kernel_selector": 0,
            },
            "manifests": [
                {
                    "platform": platform,
                    "capability_count": 10,
                    "manifest_sha256": "a" * 64,
                    "repeat_manifest_sha256": "a" * 64,
                    "release_claim": "none",
                }
                for platform in ("fedora", "ubuntu")
            ],
            "conformance": {
                "deterministic_fake": "verified",
                "fedora_contract": "verified-local",
                "ubuntu_contract": "verified-no-network-container",
                "macos": "blocked-macos",
                "equivalent_available_contract_result_count": 3,
                "networked_test_count": 0,
            },
            "maintainer_credentials_recorded": False,
            "private_environment_values_recorded": False,
            "macos_evidence_substituted": False,
            "release_claim": "none",
            "limitations": ["linux-native-open", "macos-blocked"],
        }
        self.assertEqual(MODULE.validate_report(report), [])
        for field, value in (
            ("macos_evidence_substituted", True),
            ("release_claim", "supported"),
        ):
            candidate = copy.deepcopy(report)
            candidate[field] = value
            self.assertTrue(MODULE.validate_report(candidate), field)


if __name__ == "__main__":
    unittest.main()
