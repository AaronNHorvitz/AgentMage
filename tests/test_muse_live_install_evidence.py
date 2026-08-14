from __future__ import annotations

import copy
import unittest

from scripts import muse_live_install_evidence as evidence


def valid_report() -> dict[str, object]:
    return {
        "schema_version": 1,
        "record_type": "muse_isolated_live_install_evidence",
        "source_revision": "a" * 40,
        "profile_id": evidence.PROFILE_ID,
        "tuple": {},
        "execution": {},
        "observations": {"socket_residue_names": []},
        "checks": [
            {"id": f"check-{index}", "result": "PASS", "code": "pass"}
            for index in range(5)
        ],
        "disposition": {
            "status": "ISOLATED-LIFECYCLE-PASS",
            "adapter_live_proven_for_exact_tuple": True,
            "evidence_store_generation": 1,
            "product_profile_enabled": False,
            "release_approval": False,
            "automatic_fallback": False,
            "zero_egress_observed": False,
            "quality_evaluated": False,
            "repeatability_evaluated": False,
        },
        "limitations": [],
    }


class MuseLiveInstallEvidenceTests(unittest.TestCase):
    def test_closed_live_report_validates(self) -> None:
        self.assertEqual(evidence.validate_report(valid_report()), [])

    def test_failed_check_requires_blocked_non_live_disposition(self) -> None:
        report = valid_report()
        report["checks"][0]["result"] = "FAIL"
        self.assertTrue(evidence.validate_report(report))
        report["disposition"]["status"] = "BLOCKED"
        report["disposition"]["adapter_live_proven_for_exact_tuple"] = False
        self.assertEqual(evidence.validate_report(report), [])

    def test_product_zero_egress_quality_and_release_overclaims_fail(self) -> None:
        for field in (
            "product_profile_enabled", "release_approval", "automatic_fallback",
            "zero_egress_observed", "quality_evaluated", "repeatability_evaluated",
        ):
            report = copy.deepcopy(valid_report())
            report["disposition"][field] = True
            self.assertTrue(evidence.validate_report(report), field)

    def test_unknown_fields_revision_drift_and_path_residue_fail(self) -> None:
        mutations = (
            lambda report: report.update({"unknown": True}),
            lambda report: report.update({"source_revision": "working-tree"}),
            lambda report: report["observations"].update({"socket_residue_names": ["private/path"]}),
        )
        for mutate in mutations:
            report = copy.deepcopy(valid_report())
            mutate(report)
            self.assertTrue(evidence.validate_report(report))


if __name__ == "__main__":
    unittest.main()
