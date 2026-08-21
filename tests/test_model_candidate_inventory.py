from __future__ import annotations

import copy
import unittest

from scripts import model_candidate_inventory as inventory


def _synthetic_snapshot() -> dict:
    snapshot = {
        "frozen_on": inventory.FREEZE_DATE,
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
    snapshot["snapshot_sha256"] = inventory.sha256_bytes(
        inventory.canonical_bytes(snapshot)
    )
    return snapshot


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

    def test_reference_machine_envelopes_reconcile_with_authoritative_profiles(self) -> None:
        envelopes = inventory.REFERENCE_MACHINE_ENVELOPES
        envelope_ids = [envelope["envelope_id"] for envelope in envelopes]
        self.assertEqual(len(envelope_ids), len(set(envelope_ids)))
        required_keys = {
            "envelope_id",
            "profile_source",
            "platform",
            "architecture",
            "acceleration",
            "runtime_family",
            "adapter_gate",
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
            self.assertNotIn("vllm", envelope["supported_runtime_families"])
        fedora_envelopes = [
            envelope
            for envelope in envelopes
            if envelope["platform"].startswith("Fedora")
        ]
        fedora_runtimes = {
            envelope["runtime_family"] for envelope in fedora_envelopes
        }
        self.assertEqual(fedora_runtimes, {"llama.cpp", "docker-model-runner"})
        for envelope in fedora_envelopes:
            self.assertEqual(len(envelope["supported_runtime_families"]), 1)
        windows_envelopes = [
            envelope
            for envelope in envelopes
            if envelope["platform"].startswith("Windows")
        ]
        self.assertEqual(len(windows_envelopes), 1)
        self.assertEqual(windows_envelopes[0]["runtime_family"], "llama.cpp")
        self.assertEqual(
            windows_envelopes[0]["supported_runtime_families"], ("llama.cpp",)
        )
        self.assertNotIn(
            "docker-model-runner", windows_envelopes[0]["supported_runtime_families"]
        )

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

    def test_runtime_binding_required_before_runtime_or_format_result(self) -> None:
        safetensors_only = {
            "id": "google/codegemma-1.1-2b",
            "revision": "1" * 40,
            "artifact_listing": ["README.md", "model.safetensors"],
            "pipeline_tag": "text-generation",
        }
        gguf_only = {
            "id": "google/gemma-2b-gguf",
            "revision": "2" * 40,
            "artifact_listing": ["README.md", "model.gguf"],
            "pipeline_tag": "text-generation",
        }
        fedora_native = next(
            envelope
            for envelope in inventory.REFERENCE_MACHINE_ENVELOPES
            if envelope["envelope_id"] == "fedora-kinoite-44-x86_64-cuda-llamacpp-native"
        )
        for source in (safetensors_only, gguf_only):
            result = inventory.reference_machine_preflight(source, fedora_native)
            self.assertEqual(result["dimensions"]["runtime"]["status"], "BLOCKED")
            self.assertEqual(result["dimensions"]["format"]["status"], "BLOCKED")

    def test_runtime_binding_produces_per_envelope_result(self) -> None:
        pinned_llamacpp = {
            "id": "google/gemma-2b-gguf",
            "revision": "3" * 40,
            "artifact_listing": ["README.md", "model.gguf"],
            "artifact_architectures": ["x86_64"],
            "artifact_runtime_bindings": [
                {"artifact_format": "GGUF", "runtime_family": "llama.cpp"}
            ],
            "pipeline_tag": "text-generation",
        }
        pinned_docker = {
            "id": "google/gemma-2b-dmr",
            "revision": "4" * 40,
            "artifact_listing": ["README.md", "model.gguf"],
            "artifact_architectures": ["x86_64"],
            "artifact_runtime_bindings": [
                {"artifact_format": "GGUF", "runtime_family": "docker-model-runner"}
            ],
            "pipeline_tag": "text-generation",
        }
        fedora_native = next(
            envelope
            for envelope in inventory.REFERENCE_MACHINE_ENVELOPES
            if envelope["envelope_id"] == "fedora-kinoite-44-x86_64-cuda-llamacpp-native"
        )
        fedora_docker = next(
            envelope
            for envelope in inventory.REFERENCE_MACHINE_ENVELOPES
            if envelope["envelope_id"]
            == "fedora-kinoite-44-x86_64-cuda-docker-model-runner"
        )
        native_result = inventory.reference_machine_preflight(
            pinned_llamacpp, fedora_native
        )
        docker_result = inventory.reference_machine_preflight(
            pinned_llamacpp, fedora_docker
        )
        self.assertEqual(native_result["dimensions"]["runtime"]["status"], "CANDIDATE")
        self.assertEqual(
            docker_result["dimensions"]["runtime"]["status"], "BLOCKED-HARDWARE"
        )
        native_docker = inventory.reference_machine_preflight(
            pinned_docker, fedora_native
        )
        docker_docker = inventory.reference_machine_preflight(
            pinned_docker, fedora_docker
        )
        self.assertEqual(
            native_docker["dimensions"]["runtime"]["status"], "BLOCKED-HARDWARE"
        )
        self.assertEqual(docker_docker["dimensions"]["runtime"]["status"], "CANDIDATE")

    def test_architecture_fixture_matrix(self) -> None:
        fedora_native = next(
            envelope
            for envelope in inventory.REFERENCE_MACHINE_ENVELOPES
            if envelope["envelope_id"] == "fedora-kinoite-44-x86_64-cuda-llamacpp-native"
        )
        binding = [{"artifact_format": "GGUF", "runtime_family": "llama.cpp"}]
        x86_only = {
            "id": "google/gemma-x86",
            "revision": "5" * 40,
            "artifact_architectures": ["x86_64"],
            "artifact_runtime_bindings": binding,
            "pipeline_tag": "text-generation",
        }
        arm_only = {
            "id": "google/gemma-arm",
            "revision": "6" * 40,
            "artifact_architectures": ["arm64"],
            "artifact_runtime_bindings": binding,
            "pipeline_tag": "text-generation",
        }
        neutral = {
            "id": "google/gemma-neutral",
            "revision": "7" * 40,
            "artifact_architectures": [inventory.ARCHITECTURE_NEUTRAL],
            "artifact_runtime_bindings": binding,
            "pipeline_tag": "text-generation",
        }
        unresolved = {
            "id": "google/gemma-unresolved",
            "revision": "8" * 40,
            "artifact_runtime_bindings": binding,
            "pipeline_tag": "text-generation",
        }
        self.assertEqual(
            inventory.reference_machine_preflight(x86_only, fedora_native)[
                "dimensions"
            ]["architecture"]["status"],
            "CANDIDATE",
        )
        self.assertEqual(
            inventory.reference_machine_preflight(arm_only, fedora_native)[
                "dimensions"
            ]["architecture"]["status"],
            "BLOCKED-HARDWARE",
        )
        self.assertEqual(
            inventory.reference_machine_preflight(neutral, fedora_native)[
                "dimensions"
            ]["architecture"]["status"],
            "CANDIDATE",
        )
        self.assertEqual(
            inventory.reference_machine_preflight(unresolved, fedora_native)[
                "dimensions"
            ]["architecture"]["status"],
            "BLOCKED",
        )

    def test_reference_machine_preflight_records_blocked_hardware_for_modality(self) -> None:
        multimodal_source = {
            "id": "google/paligemma-3b-mix-448",
            "revision": "d" * 40,
            "artifact_listing": ["README.md", "model.safetensors"],
            "artifact_architectures": ["x86_64"],
            "artifact_runtime_bindings": [
                {"artifact_format": "GGUF", "runtime_family": "llama.cpp"}
            ],
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

    def test_reference_machine_preflight_matrix_is_non_acquiring_and_complete(self) -> None:
        snapshot = _synthetic_snapshot()
        result = inventory.preflight_matrix(snapshot)
        self.assertEqual(result["acquisition_authorized"], False)
        self.assertEqual(len(result["entries"]), 2)
        self.assertEqual(
            len(result["envelopes"]), len(inventory.REFERENCE_MACHINE_ENVELOPES)
        )
        expected_result_count = 2 * len(inventory.REFERENCE_MACHINE_ENVELOPES)
        self.assertEqual(result["counts"]["results"], expected_result_count)
        for entry in result["entries"]:
            self.assertEqual(
                len(entry["results"]), len(inventory.REFERENCE_MACHINE_ENVELOPES)
            )
            envelope_ids = [item["envelope_id"] for item in entry["results"]]
            self.assertEqual(
                envelope_ids,
                [
                    envelope["envelope_id"]
                    for envelope in inventory.REFERENCE_MACHINE_ENVELOPES
                ],
            )
            for envelope_result in entry["results"]:
                self.assertIs(envelope_result["acquisition_started"], False)
                self.assertIn(envelope_result["status"], inventory.PREFLIGHT_STATUSES)
                self.assertEqual(
                    set(envelope_result["dimensions"].keys()),
                    set(inventory.PREFLIGHT_DIMENSIONS),
                )
        self.assertRegex(result["matrix_sha256"], r"^[0-9a-f]{64}$")

    def test_preflight_matrix_validation_and_drift(self) -> None:
        snapshot = _synthetic_snapshot()
        matrix = inventory.preflight_matrix(snapshot)
        self.assertEqual(inventory.validate_preflight_matrix(snapshot, matrix), [])

        omitted = copy.deepcopy(matrix)
        omitted["entries"].pop()
        omitted["matrix_sha256"] = inventory.sha256_bytes(
            inventory.canonical_bytes(
                {k: v for k, v in omitted.items() if k != "matrix_sha256"}
            )
        )
        self.assertTrue(inventory.validate_preflight_matrix(snapshot, omitted))

        duplicated = copy.deepcopy(matrix)
        duplicated["entries"].append(copy.deepcopy(duplicated["entries"][0]))
        duplicated["matrix_sha256"] = inventory.sha256_bytes(
            inventory.canonical_bytes(
                {k: v for k, v in duplicated.items() if k != "matrix_sha256"}
            )
        )
        self.assertTrue(inventory.validate_preflight_matrix(snapshot, duplicated))

        mutated = copy.deepcopy(matrix)
        mutated["entries"][0]["results"][0]["status"] = "not-a-status"
        mutated["matrix_sha256"] = inventory.sha256_bytes(
            inventory.canonical_bytes(
                {k: v for k, v in mutated.items() if k != "matrix_sha256"}
            )
        )
        self.assertTrue(inventory.validate_preflight_matrix(snapshot, mutated))

        drifted = copy.deepcopy(matrix)
        drifted["envelopes"].pop()
        drifted["matrix_sha256"] = inventory.sha256_bytes(
            inventory.canonical_bytes(
                {k: v for k, v in drifted.items() if k != "matrix_sha256"}
            )
        )
        self.assertTrue(inventory.validate_preflight_matrix(snapshot, drifted))

        acquired = copy.deepcopy(matrix)
        acquired["acquisition_authorized"] = True
        acquired["matrix_sha256"] = inventory.sha256_bytes(
            inventory.canonical_bytes(
                {k: v for k, v in acquired.items() if k != "matrix_sha256"}
            )
        )
        self.assertTrue(inventory.validate_preflight_matrix(snapshot, acquired))

        digest_broken = copy.deepcopy(matrix)
        digest_broken["matrix_sha256"] = "0" * 64
        self.assertTrue(
            inventory.validate_preflight_matrix(snapshot, digest_broken)
        )

    def test_preflight_matrix_result_dimensions_and_envelope_pairing(self) -> None:
        snapshot = _synthetic_snapshot()
        matrix = inventory.preflight_matrix(snapshot)
        expected_pairs = len(snapshot["entries"]) * len(
            inventory.REFERENCE_MACHINE_ENVELOPES
        )
        actual_pairs = sum(len(entry["results"]) for entry in matrix["entries"])
        self.assertEqual(actual_pairs, expected_pairs)
        seen = set()
        for entry in matrix["entries"]:
            for result in entry["results"]:
                key = (entry["entry_id"], result["envelope_id"])
                self.assertNotIn(key, seen)
                seen.add(key)
        self.assertEqual(len(seen), expected_pairs)


if __name__ == "__main__":
    unittest.main()
