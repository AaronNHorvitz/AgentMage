from __future__ import annotations

import copy
import json
import unittest

from scripts.story_22_2_artifact_pressure_evidence import (
    METRIC_PREFIX,
    REPORT_PATH,
    ArtifactPressureEvidenceError,
    parse_metrics,
    read_report,
    validate_metrics,
    validate_report,
)


def valid_metrics() -> dict[str, object]:
    return {
        "maximum_payload_bytes": 67_108_864,
        "page_bytes": 4_096,
        "mixed_object_count": 64,
        "mixed_total_payload_bytes": 2_129_920,
        "mixed_final_active_object_count": 0,
        "checkpoint_reference_count": 1_024,
        "checkpoint_payload_rows": 66,
        "checkpoint_artifact_rows": 1_089,
        "checkpoint_lifecycle_event_rows": 1_219,
        "checkpoint_resume_binding_rows": 1,
        "checkpoint_resume_artifact_rows": 1_024,
        "overflow_reference_count_rejected": 1_025,
        "deduplicated_reference_count": 1_023,
        "active_object_count_at_checkpoint": 1,
        "final_active_object_count": 0,
        "final_payload_rows": 66,
        "final_artifact_rows": 1_089,
        "final_lifecycle_event_rows": 3_267,
        "final_resume_binding_rows": 2,
        "final_resume_artifact_rows": 1_024,
        "maximum_elapsed_ms": 90_000,
        "mixed_publish_elapsed_ms": 2_000,
        "mixed_collection_elapsed_ms": 1_000,
        "reference_elapsed_ms": 2_000,
        "reopen_elapsed_ms": 1_000,
        "collection_elapsed_ms": 1_000,
        "total_elapsed_ms": 100_000,
        "resident_start_kib": 20_000,
        "resident_peak_kib": 90_000,
        "resident_delta_kib": 70_000,
        "maximum_disk_bytes": 68_000_000,
        "mixed_disk_bytes": 8_000_000,
        "reference_disk_bytes": 5_000_000,
        "final_disk_bytes": 5_000_000,
        "latency_ceiling_ms": 300_000,
        "resident_delta_ceiling_kib": 524_288,
        "retained_disk_ceiling_bytes": 134_217_728,
        "checkpoint_release_blocked": True,
        "final_reopen_verified": True,
        "external_network_used": False,
        "manual_fuzzing_executed": False,
    }


class Story222ArtifactPressureEvidenceTests(unittest.TestCase):
    def test_exact_ceiling_metrics_pass(self) -> None:
        self.assertEqual(validate_metrics(valid_metrics()), [])
        output = f"{METRIC_PREFIX}{json.dumps(valid_metrics(), sort_keys=True)}\n"
        self.assertEqual(parse_metrics(output), valid_metrics())

    def test_every_fixed_claim_and_resource_relationship_fails_closed(self) -> None:
        mutations = {
            "payload": ("maximum_payload_bytes", 67_108_863),
            "reference_count": ("checkpoint_reference_count", 1_023),
            "overflow": ("overflow_reference_count_rejected", 1_024),
            "dedup": ("deduplicated_reference_count", 1_022),
            "mixed_count": ("mixed_object_count", 63),
            "mixed_bytes": ("mixed_total_payload_bytes", 2_129_919),
            "mixed_cleanup": ("mixed_final_active_object_count", 1),
            "release": ("checkpoint_release_blocked", False),
            "checkpoint_rows": ("checkpoint_artifact_rows", 1_088),
            "final_rows": ("final_lifecycle_event_rows", 3_266),
            "reopen": ("final_reopen_verified", False),
            "network": ("external_network_used", True),
            "fuzz": ("manual_fuzzing_executed", True),
            "latency": ("total_elapsed_ms", 300_001),
            "disk": ("maximum_disk_bytes", 134_217_729),
            "memory": ("resident_delta_kib", 69_999),
        }
        for name, (field, value) in mutations.items():
            with self.subTest(name=name):
                changed = copy.deepcopy(valid_metrics())
                changed[field] = value
                self.assertTrue(validate_metrics(changed))

    def test_missing_duplicate_and_malformed_metrics_fail_closed(self) -> None:
        with self.assertRaises(ArtifactPressureEvidenceError):
            parse_metrics("no metric")
        record = f"{METRIC_PREFIX}{json.dumps(valid_metrics())}\n"
        with self.assertRaises(ArtifactPressureEvidenceError):
            parse_metrics(record + record)
        with self.assertRaises(ArtifactPressureEvidenceError):
            parse_metrics(f"{METRIC_PREFIX}{{not-json}}\n")

    @unittest.skipUnless(REPORT_PATH.is_file(), "retained report is generated after source commit")
    def test_current_report_and_raw_trace_are_hash_bound(self) -> None:
        self.assertEqual(validate_report(read_report()), [])


if __name__ == "__main__":
    unittest.main()
