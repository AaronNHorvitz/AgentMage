from __future__ import annotations

import copy
import unittest

from scripts import model_candidate_inventory as inventory


class ModelCandidateInventoryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.snapshot = inventory.read_json(inventory.SOURCE_SNAPSHOT)
        cls.normalized = inventory.read_json(inventory.INVENTORY)
        cls.matrix = inventory.read_json(inventory.ROLE_MATRIX)

    def test_frozen_sources_reconcile_without_omission_or_duplicate(self) -> None:
        self.assertEqual(
            inventory.validate(self.snapshot, self.normalized, self.matrix), []
        )
        self.assertGreaterEqual(self.snapshot["counts"]["google"], 300)
        self.assertEqual(self.snapshot["counts"]["meta"], 4)
        self.assertEqual(
            self.snapshot["counts"]["total"], len(self.normalized["entries"])
        )
        excluded = [
            item
            for item in self.normalized["entries"]
            if item["disposition"] == "INELIGIBLE"
        ]
        self.assertEqual(len(excluded), 1)
        self.assertIn("collection-adjacent-non-gemma-model", excluded[0]["blockers"])

    def test_omission_duplicate_and_matrix_drift_fail(self) -> None:
        for target, mutate in (
            ("inventory", lambda value: value["entries"].pop()),
            ("inventory", lambda value: value["entries"].append(value["entries"][0])),
            ("matrix", lambda value: value["entries"].pop()),
        ):
            normalized = copy.deepcopy(self.normalized)
            matrix = copy.deepcopy(self.matrix)
            mutate(normalized if target == "inventory" else matrix)
            self.assertTrue(inventory.validate(self.snapshot, normalized, matrix))

    def test_specialists_cannot_escalate_into_coding_planner(self) -> None:
        for marker in inventory.PROTECTED_SPECIALIST_MARKERS:
            matching = [
                item
                for item in self.normalized["entries"]
                if marker in item["repository"].lower()
            ]
            if not matching:
                continue
            for item in matching:
                if "coding_planner" in item["prohibited_roles"]:
                    self.assertNotIn("coding_planner", item["roles"])
        changed = copy.deepcopy(self.normalized)
        specialist = next(
            item for item in changed["entries"] if item["prohibited_roles"]
        )
        specialist["roles"].append("coding_planner")
        self.assertTrue(inventory.validate(self.snapshot, changed, self.matrix))

    def test_hardware_boundaries_are_exact_and_non_acquiring(self) -> None:
        required = {
            "required_disk": 20,
            "required_memory": 32,
            "required_accelerator": 16,
        }
        self.assertEqual(
            inventory.hardware_fit(
                **required,
                available_disk=19,
                available_memory=32,
                available_accelerator=16,
            ),
            "BLOCKED-HARDWARE",
        )
        self.assertEqual(
            inventory.hardware_fit(
                **required,
                available_disk=20,
                available_memory=32,
                available_accelerator=16,
            ),
            "CANDIDATE",
        )
        self.assertEqual(
            inventory.hardware_fit(
                **required,
                available_disk=21,
                available_memory=33,
                available_accelerator=17,
            ),
            "CANDIDATE",
        )
        self.assertTrue(
            all(
                item["hardware_preflight"]["acquisition_started"] is False
                for item in self.normalized["entries"]
            )
        )

    def test_intake_routes_first_party_and_prohibited_sources_exactly(self) -> None:
        self.assertEqual(
            inventory.intake_disposition(
                owner="google",
                revision="a" * 40,
                private=False,
                first_party=True,
            ),
            ("CANDIDATE", []),
        )
        cases = (
            {"owner": "mirror", "revision": "a" * 40, "private": False, "first_party": False},
            {"owner": "google", "revision": "main", "private": False, "first_party": True},
            {
                "owner": "google",
                "revision": "a" * 40,
                "private": False,
                "first_party": True,
                "mirrored": True,
            },
            {
                "owner": "google",
                "revision": "a" * 40,
                "private": False,
                "first_party": True,
                "community_converted": True,
            },
        )
        for case in cases:
            disposition, reasons = inventory.intake_disposition(**case)
            self.assertEqual(disposition, "INELIGIBLE")
            self.assertTrue(reasons)

    def test_activation_fallback_and_acquisition_mutations_fail(self) -> None:
        for field in ("enabled", "automatic_fallback", "acquisition_allowed"):
            changed = copy.deepcopy(self.normalized)
            changed["entries"][0][field] = True
            self.assertTrue(inventory.validate(self.snapshot, changed, self.matrix))

    def test_reference_machine_envelopes_declared_and_distinct(self) -> None:
        envelopes = inventory.REFERENCE_MACHINE_ENVELOPES
        self.assertGreaterEqual(len(envelopes), 2)
        envelope_ids = [envelope["envelope_id"] for envelope in envelopes]
        self.assertEqual(len(envelope_ids), len(set(envelope_ids)))
        required_keys = {
            "envelope_id",
            "platform",
            "architecture",
            "acceleration",
            "supported_runtime_families",
            "supported_artifact_formats",
            "supported_modalities",
            "available_disk_bytes",
            "available_memory_bytes",
            "available_accelerator_bytes",
            "maximum_context_tokens",
        }
        for envelope in envelopes:
            self.assertTrue(required_keys.issubset(envelope.keys()))

    def test_reference_machine_preflight_covers_every_declared_dimension(self) -> None:
        source_entry = {
            "id": "google/gemma-9b-it",
            "revision": "a" * 40,
            "artifact_listing": ["README.md", "model.safetensors", "tokenizer.json"],
            "pipeline_tag": "text-generation",
            "library_name": "transformers",
        }
        for envelope in inventory.REFERENCE_MACHINE_ENVELOPES:
            result = inventory.reference_machine_preflight(source_entry, envelope)
            self.assertEqual(result["envelope_id"], envelope["envelope_id"])
            self.assertIs(result["acquisition_started"], False)
            self.assertIn(result["status"], inventory.PREFLIGHT_STATUSES)
            self.assertEqual(
                set(result["dimensions"].keys()), set(inventory.PREFLIGHT_DIMENSIONS)
            )
            for dimension in result["dimensions"].values():
                self.assertIn(dimension["status"], inventory.PREFLIGHT_STATUSES)

    def test_reference_machine_preflight_is_exact_per_envelope_not_family(self) -> None:
        gguf_source = {
            "id": "google/gemma-2b-gguf",
            "revision": "b" * 40,
            "artifact_listing": ["README.md", "model.gguf"],
            "pipeline_tag": "text-generation",
        }
        safetensors_source = {
            "id": "google/gemma-2b-keras",
            "revision": "c" * 40,
            "artifact_listing": ["README.md", "model.safetensors"],
            "pipeline_tag": "text-generation",
        }
        cpu_envelope = next(
            envelope
            for envelope in inventory.REFERENCE_MACHINE_ENVELOPES
            if envelope["acceleration"] == "cpu"
        )
        gguf_result = inventory.reference_machine_preflight(gguf_source, cpu_envelope)
        safetensors_result = inventory.reference_machine_preflight(
            safetensors_source, cpu_envelope
        )
        self.assertEqual(gguf_result["dimensions"]["format"]["status"], "CANDIDATE")
        self.assertEqual(
            safetensors_result["dimensions"]["format"]["status"], "BLOCKED-HARDWARE"
        )
        self.assertEqual(gguf_result["dimensions"]["runtime"]["status"], "CANDIDATE")
        self.assertEqual(
            safetensors_result["dimensions"]["runtime"]["status"], "BLOCKED-HARDWARE"
        )

    def test_reference_machine_preflight_records_blocked_hardware_for_modality(self) -> None:
        multimodal_source = {
            "id": "google/paligemma-3b-mix-448",
            "revision": "d" * 40,
            "artifact_listing": ["README.md", "model.safetensors"],
            "pipeline_tag": "image-text-to-text",
        }
        cpu_envelope = next(
            envelope
            for envelope in inventory.REFERENCE_MACHINE_ENVELOPES
            if envelope["acceleration"] == "cpu"
        )
        result = inventory.reference_machine_preflight(multimodal_source, cpu_envelope)
        self.assertEqual(
            result["dimensions"]["modality"]["status"], "BLOCKED-HARDWARE"
        )
        self.assertEqual(result["status"], "BLOCKED-HARDWARE")
        self.assertIs(result["acquisition_started"], False)

    def test_reference_machine_preflight_blocks_when_format_unresolved(self) -> None:
        source = {
            "id": "google/gemma-mystery",
            "revision": "e" * 40,
            "artifact_listing": ["README.md", "config.json"],
            "pipeline_tag": "text-generation",
        }
        envelope = inventory.REFERENCE_MACHINE_ENVELOPES[0]
        result = inventory.reference_machine_preflight(source, envelope)
        self.assertEqual(result["dimensions"]["format"]["status"], "BLOCKED")
        self.assertEqual(result["dimensions"]["runtime"]["status"], "BLOCKED")
        self.assertEqual(result["status"], "BLOCKED")

    def test_reference_machine_preflight_matrix_is_non_acquiring_and_complete(self) -> None:
        synthetic_snapshot = {
            "frozen_on": inventory.FREEZE_DATE,
            "snapshot_sha256": "0" * 64,
            "entries": [
                {
                    "repository": "google/gemma-2-2b-it",
                    "revision": "f" * 40,
                    "artifact_listing": ["README.md", "model.safetensors"],
                    "pipeline_tag": "text-generation",
                    "tags": [],
                    "private": False,
                },
                {
                    "repository": "google/gemma-2-2b-gguf",
                    "revision": "a" * 40,
                    "artifact_listing": ["README.md", "model.gguf"],
                    "pipeline_tag": "text-generation",
                    "tags": [],
                    "private": False,
                },
            ],
        }
        result = inventory.preflight_matrix(synthetic_snapshot)
        self.assertEqual(result["acquisition_authorized"], False)
        self.assertEqual(len(result["entries"]), 2)
        self.assertEqual(
            len(result["envelopes"]), len(inventory.REFERENCE_MACHINE_ENVELOPES)
        )
        for entry in result["entries"]:
            self.assertEqual(
                len(entry["results"]), len(inventory.REFERENCE_MACHINE_ENVELOPES)
            )
            for envelope_result in entry["results"]:
                self.assertIs(envelope_result["acquisition_started"], False)
                self.assertIn(envelope_result["status"], inventory.PREFLIGHT_STATUSES)
        self.assertRegex(result["matrix_sha256"], r"^[0-9a-f]{64}$")

    def test_reference_machine_preflight_never_infers_family_result(self) -> None:
        specialist_source = {
            "id": "google/shieldgemma-2b",
            "revision": "1" * 40,
            "artifact_listing": ["README.md", "model.safetensors"],
            "pipeline_tag": "text-classification",
        }
        dialogue_source = {
            "id": "google/gemma-2b-it",
            "revision": "2" * 40,
            "artifact_listing": ["README.md", "model.safetensors"],
            "pipeline_tag": "text-generation",
        }
        envelope = inventory.REFERENCE_MACHINE_ENVELOPES[0]
        specialist_result = inventory.reference_machine_preflight(specialist_source, envelope)
        dialogue_result = inventory.reference_machine_preflight(dialogue_source, envelope)
        self.assertNotEqual(
            specialist_result["dimensions"]["modality"],
            dialogue_result["dimensions"]["modality"],
        )


if __name__ == "__main__":
    unittest.main()
