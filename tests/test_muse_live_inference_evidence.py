from __future__ import annotations

import copy
import unittest

from scripts import muse_live_inference_evidence as evidence


def valid_report() -> dict[str, object]:
    return {
        "schema_version": 1,
        "record_type": "muse_sandboxed_live_inference_evidence",
        "source_revision": "a" * 40,
        "source_sha256": {path: "b" * 64 for path in evidence.SOURCE_PATHS},
        "profile_id": evidence.PROFILE_ID,
        "tuple": {},
        "execution": {
            "terminal_state": "advisory_text",
            "response_sha256": "c" * 64,
            "input_tokens": 653,
            "output_tokens": 64,
            "pre_request_cancellation_terminal": "cancelled",
            "raw_output_retained": False,
        },
        "sandbox": {"socket_residue_names": []},
        "checks": [
            {"id": f"check-{index}", "result": "PASS", "code": "pass"}
            for index in range(10)
        ],
        "disposition": {
            "status": "SANDBOXED-LIVE-INFERENCE-PASS",
            "exact_tuple_live_inference_proven": True,
            "external_egress_enforced_by_namespace": True,
            "packet_capture_executed": False,
            "quality_evaluated": False,
            "repeatability_evaluated": False,
            "product_profile_enabled": False,
            "release_approval": False,
            "automatic_fallback": False,
        },
        "limitations": [],
    }


class MuseLiveInferenceEvidenceTests(unittest.TestCase):
    def test_closed_live_report_validates(self) -> None:
        self.assertEqual(evidence.validate_report(valid_report()), [])

    def test_failed_check_requires_blocked_non_live_disposition(self) -> None:
        report = valid_report()
        report["checks"][0]["result"] = "FAIL"
        self.assertTrue(evidence.validate_report(report))
        report["disposition"]["status"] = "BLOCKED"
        report["disposition"]["exact_tuple_live_inference_proven"] = False
        report["disposition"]["external_egress_enforced_by_namespace"] = False
        self.assertEqual(evidence.validate_report(report), [])

    def test_quality_repeatability_capture_product_and_release_overclaims_fail(self) -> None:
        for field in (
            "packet_capture_executed", "quality_evaluated", "repeatability_evaluated",
            "product_profile_enabled", "release_approval", "automatic_fallback",
        ):
            report = copy.deepcopy(valid_report())
            report["disposition"][field] = True
            self.assertTrue(evidence.validate_report(report), field)

    def test_source_output_token_cancellation_and_cleanup_mutations_fail(self) -> None:
        mutations = (
            lambda report: report["source_sha256"].pop(evidence.SOURCE_PATHS[0]),
            lambda report: report["execution"].update({"response_sha256": "invalid"}),
            lambda report: report["execution"].update({"output_tokens": 65}),
            lambda report: report["execution"].update({"pre_request_cancellation_terminal": "completed"}),
            lambda report: report["execution"].update({"raw_output_retained": True}),
            lambda report: report["sandbox"].update({"socket_residue_names": ["llama-server.sock"]}),
            lambda report: report.update({"unknown": True}),
        )
        for mutate in mutations:
            report = copy.deepcopy(valid_report())
            mutate(report)
            self.assertTrue(evidence.validate_report(report))


if __name__ == "__main__":
    unittest.main()
