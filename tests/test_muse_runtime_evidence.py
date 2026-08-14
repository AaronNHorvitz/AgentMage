from __future__ import annotations

import copy
import unittest

from scripts import muse_runtime_evidence as evidence


def valid_report() -> dict[str, object]:
    return {
        "schema_version": 1,
        "record_type": "muse_native_runtime_package_evidence",
        "source_revision": "a" * 40,
        "profile_path": "profile.json",
        "profile_sha256": "b" * 64,
        "source_archive": {},
        "package": {},
        "member_count": 21,
        "file_modes": {},
        "prohibited_entrypoints": [],
        "self_check": {"model_loaded": False, "listener_started": False},
        "checks": [
            {"id": f"check-{index}", "result": "PASS", "code": "pass"}
            for index in range(6)
        ],
        "disposition": {
            "status": "PACKAGE-VERIFIED-NOT-ADMITTED",
            "enabled_models": 0,
            "inference_implemented": False,
            "adapter_implemented": False,
            "release_approval": False,
            "automatic_fallback": False,
        },
        "limitations": [],
    }


class MuseRuntimeEvidenceTests(unittest.TestCase):
    def test_closed_inactive_report_validates(self) -> None:
        self.assertEqual(evidence.validate_report(valid_report()), [])

    def test_failed_check_requires_blocked_disposition(self) -> None:
        report = valid_report()
        report["checks"][0]["result"] = "FAIL"
        self.assertTrue(evidence.validate_report(report))
        report["disposition"]["status"] = "BLOCKED"
        self.assertEqual(evidence.validate_report(report), [])

    def test_activation_execution_and_unknown_fields_fail(self) -> None:
        for mutate in (
            lambda report: report["disposition"].update({"enabled_models": 1}),
            lambda report: report["disposition"].update({"inference_implemented": True}),
            lambda report: report["self_check"].update({"listener_started": True}),
            lambda report: report.update({"unknown": True}),
        ):
            report = copy.deepcopy(valid_report())
            mutate(report)
            self.assertTrue(evidence.validate_report(report))


if __name__ == "__main__":
    unittest.main()
