from __future__ import annotations

import copy
import unittest

from scripts import muse_preflight as preflight


def passing_host() -> dict[str, object]:
    return {
        "system": "Linux",
        "architecture": "x86_64",
        "system_memory_bytes": preflight.MIN_SYSTEM_BYTES,
        "disk_available_bytes": preflight.MIN_DISK_BYTES,
        "accelerator": {
            "available": True,
            "name": "fixture",
            "total_bytes": preflight.MIN_ACCELERATOR_BYTES,
            "free_bytes": preflight.MIN_ACCELERATOR_BYTES,
            "driver": "610.43.03",
        },
    }


class MusePreflightTests(unittest.TestCase):
    def test_missing_runtime_model_and_evidence_stay_blocked(self) -> None:
        report = preflight.evaluate(
            host=passing_host(),
            model={"exact": False},
            runtime_archive={"exact": False},
            source_revision="a" * 40,
        )
        self.assertEqual(preflight.validate_report(report), [])
        self.assertEqual(report["disposition"]["status"], "BLOCKED")
        self.assertFalse(report["disposition"]["activation"])
        self.assertIn("model-artifact-missing-or-drifted", report["disposition"]["blockers"])
        self.assertIn("runtime-package-not-admitted", report["disposition"]["blockers"])

    def test_each_host_boundary_blocks_independently(self) -> None:
        mutations = (
            lambda host: host.update({"system": "Darwin"}),
            lambda host: host.update({"architecture": "aarch64"}),
            lambda host: host.update({"system_memory_bytes": preflight.MIN_SYSTEM_BYTES - 1}),
            lambda host: host.update({"disk_available_bytes": preflight.MIN_DISK_BYTES - 1}),
            lambda host: host["accelerator"].update({"total_bytes": preflight.MIN_ACCELERATOR_BYTES - 1}),
            lambda host: host["accelerator"].update({"driver": "changed"}),
        )
        for mutate in mutations:
            host = passing_host()
            mutate(host)
            report = preflight.evaluate(
                host=host,
                model={"exact": True},
                runtime_archive={"exact": True},
                source_revision="b" * 40,
            )
            self.assertEqual(report["disposition"]["status"], "BLOCKED")

    def test_report_cannot_claim_pass_with_remaining_blockers(self) -> None:
        report = preflight.evaluate(
            host=passing_host(),
            model={"exact": True},
            runtime_archive={"exact": True},
            source_revision="c" * 40,
        )
        changed = copy.deepcopy(report)
        changed["disposition"]["status"] = "PASS-EVALUATION"
        changed["disposition"]["activation"] = True
        self.assertTrue(preflight.validate_report(changed))


if __name__ == "__main__":
    unittest.main()
