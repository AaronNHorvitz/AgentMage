from __future__ import annotations

import copy
import unittest

from scripts.supply_chain import ROOT, build_documents, check_outputs, validate_documents


class SupplyChainTests(unittest.TestCase):
    def setUp(self) -> None:
        self.provenance, self.bom, self.hash_text = build_documents()

    def test_checked_in_artifacts_are_current(self) -> None:
        self.assertEqual(check_outputs(), [])

    def test_generation_is_byte_deterministic(self) -> None:
        second = build_documents()
        self.assertEqual((self.provenance, self.bom, self.hash_text), second)

    def test_every_component_has_license_and_hash_disposition(self) -> None:
        for component in self.provenance["components"]:
            with self.subTest(component=component["component_id"]):
                self.assertIsInstance(component["license"], str)
                self.assertTrue(component["license"])
                self.assertTrue(component["hashes"])

    def test_missing_license_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.provenance)
        mutated["components"][0]["license"] = ""
        failures = validate_documents(mutated, self.bom, self.hash_text)
        self.assertTrue(any("missing license" in item for item in failures))

    def test_missing_npm_integrity_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.provenance)
        npm_component = next(
            item for item in mutated["components"] if item["ecosystem"] == "npm"
        )
        npm_component["integrity"] = None
        failures = validate_documents(mutated, self.bom, self.hash_text)
        self.assertTrue(any("missing lock integrity" in item for item in failures))

    def test_external_cargo_packages_use_registry_checksums(self) -> None:
        external = [
            item
            for item in self.provenance["components"]
            if item["ecosystem"] == "cargo" and item["source"]["type"] == "registry"
        ]
        self.assertEqual(len(external), 42)
        for component in external:
            self.assertTrue(component["source"]["url"].startswith("https://crates.io/crates/"))
            self.assertEqual(component["integrity"], f"sha256:{component['hashes'][0]['content']}")

    def test_versioned_cargo_dependencies_resolve_without_name_ambiguity(self) -> None:
        components = {item["component_id"] for item in self.provenance["components"]}
        self.assertIn("cargo:syn@2.0.119", components)
        self.assertIn("cargo:syn@3.0.3", components)
        dependencies = {
            item["component_id"]: item["depends_on"]
            for item in self.provenance["dependencies"]
        }
        self.assertIn(
            "cargo:syn@2.0.119",
            dependencies["cargo:curve25519-dalek-derive@0.1.1"],
        )
        self.assertIn(
            "cargo:syn@3.0.3",
            dependencies["cargo:serde_derive@1.0.229"],
        )

    def test_sbom_omission_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.bom)
        mutated["components"].pop()
        failures = validate_documents(self.provenance, mutated, self.hash_text)
        self.assertTrue(any("component closure" in item for item in failures))

    def test_hash_manifest_mutation_is_rejected(self) -> None:
        mutated = self.hash_text.replace("a", "b", 1)
        failures = validate_documents(self.provenance, self.bom, mutated)
        self.assertTrue(any("hash manifest" in item for item in failures))

    def test_macos_status_cannot_be_promoted(self) -> None:
        mutated = copy.deepcopy(self.provenance)
        mutated["platform_status"]["macos"] = "verified"
        failures = validate_documents(mutated, self.bom, self.hash_text)
        self.assertTrue(any("blocked macOS" in item for item in failures))

    def test_provenance_contains_no_absolute_repository_paths(self) -> None:
        self.assertNotIn(str(ROOT), str(self.provenance))


if __name__ == "__main__":
    unittest.main()
