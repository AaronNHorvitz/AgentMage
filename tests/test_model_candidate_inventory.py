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


def _make_profile(**overrides) -> dict:
    base = {
        "profile_id": "google/fixture@" + "a" * 40 + "#model.gguf",
        "repository": "google/fixture",
        "revision": "a" * 40,
        "artifact_path": "model.gguf",
        "architectures": (inventory.ARCHITECTURE_NEUTRAL,),
        "runtime_bindings": (
            {"artifact_format": "GGUF", "runtime_family": "llama.cpp"},
        ),
        "supported_accelerations": ("cpu", "cuda", "metal"),
        "artifact_size_bytes": 2_000_000_000,
        "required_disk_bytes": 2_500_000_000,
        "required_memory_bytes": 3_000_000_000,
        "required_accelerator_bytes": 2_000_000_000,
        "expected_working_set_bytes": 4_000_000_000,
        "context_tokens": 4096,
        "modality": "text-generation",
    }
    base.update(overrides)
    return base


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
        for profile in inventory.EXACT_ARTIFACT_PROFILES:
            for envelope in inventory.REFERENCE_MACHINE_ENVELOPES:
                result = inventory.reference_machine_preflight(profile, envelope)
                self.assertEqual(result["envelope_id"], envelope["envelope_id"])
                self.assertEqual(result["profile_id"], profile["profile_id"])
                self.assertIs(result["acquisition_started"], False)
                self.assertIn(result["status"], {"CANDIDATE", "BLOCKED-HARDWARE"})
                self.assertEqual(
                    set(result["dimensions"].keys()),
                    set(inventory.PREFLIGHT_DIMENSIONS),
                )
                for dimension in result["dimensions"].values():
                    self.assertIn(
                        dimension["status"], {"CANDIDATE", "BLOCKED-HARDWARE"}
                    )

    def test_runtime_format_binding_is_tuple_intersected(self) -> None:
        fedora_native = next(
            envelope
            for envelope in inventory.REFERENCE_MACHINE_ENVELOPES
            if envelope["envelope_id"] == "fedora-kinoite-44-x86_64-cuda-llamacpp-native"
        )
        crossed = _make_profile(
            runtime_bindings=(
                {"artifact_format": "safetensors", "runtime_family": "llama.cpp"},
                {"artifact_format": "GGUF", "runtime_family": "docker-model-runner"},
            ),
        )
        crossed_result = inventory.reference_machine_preflight(crossed, fedora_native)
        self.assertEqual(
            crossed_result["dimensions"]["runtime"]["status"], "BLOCKED-HARDWARE"
        )
        self.assertEqual(
            crossed_result["dimensions"]["format"]["status"], "BLOCKED-HARDWARE"
        )
        self.assertIsNone(crossed_result["dimensions"]["runtime"]["matched_binding"])
        self.assertEqual(crossed_result["status"], "BLOCKED-HARDWARE")

        exact_match = _make_profile(
            runtime_bindings=(
                {"artifact_format": "GGUF", "runtime_family": "llama.cpp"},
            ),
        )
        match_result = inventory.reference_machine_preflight(exact_match, fedora_native)
        self.assertEqual(match_result["dimensions"]["runtime"]["status"], "CANDIDATE")
        self.assertEqual(match_result["dimensions"]["format"]["status"], "CANDIDATE")
        self.assertEqual(
            match_result["dimensions"]["runtime"]["matched_binding"],
            {"artifact_format": "GGUF", "runtime_family": "llama.cpp"},
        )
        self.assertEqual(match_result["status"], "CANDIDATE")

    def test_runtime_binding_produces_per_envelope_result(self) -> None:
        pinned_llamacpp = _make_profile(
            profile_id="google/gemma-2b-gguf@" + "3" * 40 + "#model.gguf",
            repository="google/gemma-2b-gguf",
            revision="3" * 40,
            architectures=("x86_64",),
            runtime_bindings=(
                {"artifact_format": "GGUF", "runtime_family": "llama.cpp"},
            ),
        )
        pinned_docker = _make_profile(
            profile_id="google/gemma-2b-dmr@" + "4" * 40 + "#model.gguf",
            repository="google/gemma-2b-dmr",
            revision="4" * 40,
            architectures=("x86_64",),
            runtime_bindings=(
                {"artifact_format": "GGUF", "runtime_family": "docker-model-runner"},
            ),
        )
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
        mac_metal = next(
            envelope
            for envelope in inventory.REFERENCE_MACHINE_ENVELOPES
            if envelope["envelope_id"] == "macbook-pro-m5-arm64-metal-llamacpp-native"
        )
        x86_only = _make_profile(architectures=("x86_64",))
        arm_only = _make_profile(architectures=("arm64",))
        neutral = _make_profile(architectures=(inventory.ARCHITECTURE_NEUTRAL,))
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
            inventory.reference_machine_preflight(x86_only, mac_metal)[
                "dimensions"
            ]["architecture"]["status"],
            "BLOCKED-HARDWARE",
        )
        self.assertEqual(
            inventory.reference_machine_preflight(arm_only, mac_metal)[
                "dimensions"
            ]["architecture"]["status"],
            "CANDIDATE",
        )

    def test_reference_machine_preflight_records_blocked_hardware_for_modality(self) -> None:
        multimodal_profile = _make_profile(
            profile_id="google/paligemma-3b-mix-448@" + "d" * 40 + "#model.safetensors",
            repository="google/paligemma-3b-mix-448",
            revision="d" * 40,
            architectures=("x86_64",),
            modality="image-text-to-text",
        )
        cpu_envelope = next(
            envelope
            for envelope in inventory.REFERENCE_MACHINE_ENVELOPES
            if envelope["acceleration"] == "cpu"
        )
        result = inventory.reference_machine_preflight(multimodal_profile, cpu_envelope)
        self.assertEqual(
            result["dimensions"]["modality"]["status"], "BLOCKED-HARDWARE"
        )
        self.assertEqual(result["status"], "BLOCKED-HARDWARE")
        self.assertIs(result["acquisition_started"], False)

    def test_dimension_boundary_matrix_produces_real_dispositions(self) -> None:
        cpu_envelope = next(
            envelope
            for envelope in inventory.REFERENCE_MACHINE_ENVELOPES
            if envelope["acceleration"] == "cpu"
        )
        base_disk = cpu_envelope["available_disk_bytes"]
        base_memory = cpu_envelope["available_memory_bytes"]
        base_context = cpu_envelope["maximum_context_tokens"]
        for offset, expected in ((-1, "CANDIDATE"), (0, "CANDIDATE"), (1, "BLOCKED-HARDWARE")):
            profile = _make_profile(
                required_disk_bytes=base_disk + offset,
                required_memory_bytes=1,
                required_accelerator_bytes=0,
                expected_working_set_bytes=1,
                context_tokens=1,
            )
            result = inventory.reference_machine_preflight(profile, cpu_envelope)
            self.assertEqual(result["dimensions"]["disk"]["status"], expected)
        for offset, expected in ((-1, "CANDIDATE"), (0, "CANDIDATE"), (1, "BLOCKED-HARDWARE")):
            profile = _make_profile(
                required_disk_bytes=1,
                required_memory_bytes=base_memory + offset,
                required_accelerator_bytes=0,
                expected_working_set_bytes=1,
                context_tokens=1,
            )
            result = inventory.reference_machine_preflight(profile, cpu_envelope)
            self.assertEqual(result["dimensions"]["memory"]["status"], expected)
        for offset, expected in ((-1, "CANDIDATE"), (0, "CANDIDATE"), (1, "BLOCKED-HARDWARE")):
            profile = _make_profile(
                required_disk_bytes=1,
                required_memory_bytes=1,
                required_accelerator_bytes=0,
                expected_working_set_bytes=1,
                context_tokens=base_context + offset,
            )
            result = inventory.reference_machine_preflight(profile, cpu_envelope)
            self.assertEqual(result["dimensions"]["context"]["status"], expected)

    def test_reference_machine_preflight_matrix_is_non_acquiring_and_complete(self) -> None:
        snapshot = _synthetic_snapshot()
        result = inventory.preflight_matrix(snapshot)
        self.assertEqual(result["acquisition_authorized"], False)
        self.assertEqual(
            len(result["entries"]), len(inventory.EXACT_ARTIFACT_PROFILES)
        )
        self.assertEqual(
            len(result["envelopes"]), len(inventory.REFERENCE_MACHINE_ENVELOPES)
        )
        expected_result_count = len(inventory.EXACT_ARTIFACT_PROFILES) * len(
            inventory.REFERENCE_MACHINE_ENVELOPES
        )
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
                self.assertIn(
                    envelope_result["status"], {"CANDIDATE", "BLOCKED-HARDWARE"}
                )
                self.assertEqual(
                    set(envelope_result["dimensions"].keys()),
                    set(inventory.PREFLIGHT_DIMENSIONS),
                )
        # matrix has real dispositions rather than unconditional BLOCKED placeholders
        statuses = {
            result_["status"]
            for entry in result["entries"]
            for result_ in entry["results"]
        }
        self.assertIn("CANDIDATE", statuses)
        self.assertIn("BLOCKED-HARDWARE", statuses)
        self.assertNotIn("BLOCKED", statuses)

    def test_preflight_matrix_validation_and_deep_compare(self) -> None:
        snapshot = _synthetic_snapshot()
        matrix = inventory.preflight_matrix(snapshot)
        self.assertEqual(inventory.validate_preflight_matrix(snapshot, matrix), [])

        mutations = (
            lambda m: m["entries"][0].update({"repository": "google/imposter"}),
            lambda m: m["entries"][0].update({"revision": "0" * 40}),
            lambda m: m["entries"][0]["results"][0].update(
                {"repository": "google/imposter"}
            ),
            lambda m: m["entries"][0]["results"][0].update({"status": "CANDIDATE"}),
            lambda m: m["entries"][0]["results"][0]["dimensions"]["disk"].update(
                {"status": "CANDIDATE"}
            ),
            lambda m: m["entries"][0]["results"][0]["dimensions"]["disk"].update(
                {"envelope_available_bytes": 1}
            ),
            lambda m: m["entries"][0]["results"][0]["dimensions"]["disk"].update(
                {"profile_required_bytes": 1}
            ),
            lambda m: m["envelopes"].pop(),
            lambda m: m.update({"envelope_count": 999}),
            lambda m: m["counts"]["by_status"].update({"CANDIDATE": 9999}),
            lambda m: m.update({"acquisition_authorized": True}),
            lambda m: m.update({"source_snapshot_sha256": "0" * 64}),
            lambda m: m["entries"].pop(),
            lambda m: m["entries"].append(copy.deepcopy(m["entries"][0])),
        )
        for mutate in mutations:
            mutated = copy.deepcopy(matrix)
            mutate(mutated)
            self.assertTrue(
                inventory.validate_preflight_matrix(snapshot, mutated),
                msg=f"mutation should be detected: {mutate}",
            )

    def test_preflight_matrix_result_dimensions_and_envelope_pairing(self) -> None:
        snapshot = _synthetic_snapshot()
        matrix = inventory.preflight_matrix(snapshot)
        expected_pairs = len(inventory.EXACT_ARTIFACT_PROFILES) * len(
            inventory.REFERENCE_MACHINE_ENVELOPES
        )
        actual_pairs = sum(len(entry["results"]) for entry in matrix["entries"])
        self.assertEqual(actual_pairs, expected_pairs)
        seen = set()
        for entry in matrix["entries"]:
            for result in entry["results"]:
                key = (entry["profile_id"], result["envelope_id"])
                self.assertNotIn(key, seen)
                seen.add(key)
        self.assertEqual(len(seen), expected_pairs)

    def test_stored_preflight_matrix_is_retained_and_deterministic(self) -> None:
        self.assertTrue(inventory.PREFLIGHT_MATRIX.exists())
        snapshot = inventory.read_json(inventory.SOURCE_SNAPSHOT)
        stored = inventory.read_json(inventory.PREFLIGHT_MATRIX)
        self.assertEqual(inventory.validate_preflight_matrix(snapshot, stored), [])
        expected_pairs = len(inventory.EXACT_ARTIFACT_PROFILES) * len(
            inventory.REFERENCE_MACHINE_ENVELOPES
        )
        self.assertEqual(stored["counts"]["results"], expected_pairs)


if __name__ == "__main__":
    unittest.main()
