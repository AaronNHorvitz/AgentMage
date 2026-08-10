from __future__ import annotations

import copy
import json
import platform
import tempfile
import unittest
from pathlib import Path

from scripts.platform_result_recorder import (
    EXPECTED_ENVIRONMENT_FIELDS,
    EXPECTED_FORBIDDEN_FIELDS,
    EXPECTED_RESULT_IDENTITIES,
    PROFILE_PATH,
    build_report,
    canonical_json,
    check_report,
    collect_current_linux_environment,
    nested_keys,
    parse_os_release,
    read_json,
    record_platform_result,
    record_set_sha256,
    sha256_bytes,
    validate_environment,
    validate_profile,
    validate_record,
    validate_report,
    validate_result_context,
)


class PlatformResultRecorderTests(unittest.TestCase):
    def setUp(self) -> None:
        self.profile = read_json(PROFILE_PATH)
        self.records = [
            record_platform_result(
                run["environment"],
                run["result"],
                run["captured_at"],
                self.profile["allowlist_version"],
            )
            for run in self.profile["synthetic_runs"]
        ]

    def test_checked_in_profile_report_and_records_are_current(self) -> None:
        self.assertEqual(validate_profile(self.profile), [])
        self.assertEqual(check_report(), [])
        self.assertTrue(all(validate_record(record) == [] for record in self.records))

    def test_environment_output_is_an_exact_allowlist(self) -> None:
        for record in self.records:
            self.assertEqual(set(record["environment"]), set(EXPECTED_ENVIRONMENT_FIELDS))
            self.assertFalse(nested_keys(record) & set(EXPECTED_FORBIDDEN_FIELDS))
            self.assertFalse(record["redaction"]["ambient_environment_values_recorded"])
            self.assertFalse(record["redaction"]["private_paths_recorded"])
            self.assertFalse(record["redaction"]["secret_store_values_recorded"])

    def test_build_fixture_model_runtime_and_policy_identities_are_required(self) -> None:
        for record in self.records:
            result = record["result"]
            self.assertTrue(all(field in result for field in EXPECTED_RESULT_IDENTITIES))
            for field in EXPECTED_RESULT_IDENTITIES:
                self.assertEqual(set(result[field]), {"id", "sha256"})
                self.assertEqual(len(result[field]["sha256"]), 64)

    def test_pass_and_failure_statuses_are_preserved(self) -> None:
        self.assertEqual([record["result"]["status"] for record in self.records], ["pass", "fail"])
        self.assertEqual(
            [record["result"]["result_id"] for record in self.records],
            ["synthetic-fedora-result", "synthetic-ubuntu-result"],
        )

    def test_record_hash_detects_any_record_mutation(self) -> None:
        record = copy.deepcopy(self.records[0])
        unhashed = dict(record)
        recorded_hash = unhashed.pop("record_sha256")
        self.assertEqual(sha256_bytes(canonical_json(unhashed)), recorded_hash)
        record["result"]["status"] = "fail"
        self.assertTrue(validate_record(record))

    def test_record_set_hash_is_order_and_content_sensitive(self) -> None:
        original = record_set_sha256(self.records)
        self.assertNotEqual(original, record_set_sha256(list(reversed(self.records))))
        mutated = copy.deepcopy(self.records)
        mutated[0]["record_sha256"] = "f" * 64
        self.assertNotEqual(original, record_set_sha256(mutated))

    def test_ambient_secret_values_are_never_copied(self) -> None:
        if platform.system().lower() != "linux":
            self.skipTest("Linux collector is intentionally the only implemented collector")
        synthetic_values = {
            "HOME": "/synthetic/private/home",
            "PATH": "/synthetic/secret/bin",
            "AGENTMAGE_TOKEN": "AM_SYNTHETIC_SECRET_DO_NOT_RECORD",
            "CI": "true",
            "container": "synthetic-container-marker",
        }
        observed = collect_current_linux_environment(environment=synthetic_values)
        serialized = json.dumps(observed, sort_keys=True)
        for key in ("HOME", "PATH", "AGENTMAGE_TOKEN", "container"):
            value = synthetic_values[key]
            self.assertNotIn(value, serialized)
        for key in synthetic_values:
            self.assertNotIn(f'"{key}"', serialized)
        self.assertTrue(observed["continuous_integration"])
        self.assertTrue(observed["containerized"])

    def test_os_release_parser_ignores_non_allowlisted_values(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "os-release"
            path.write_text(
                'ID="fedora"\nVERSION_ID="44"\nPRETTY_NAME="SYNTHETIC SECRET"\nHOME_URL="value"\n',
                encoding="utf-8",
            )
            self.assertEqual(parse_os_release(path), {"ID": "fedora", "VERSION_ID": "44"})

    def test_current_linux_collection_satisfies_the_allowlist_without_persistence(self) -> None:
        if platform.system().lower() != "linux":
            self.skipTest("Linux collector is intentionally the only implemented collector")
        observed = collect_current_linux_environment(environment={})
        self.assertEqual(validate_environment(observed), [])
        self.assertEqual(set(observed), set(EXPECTED_ENVIRONMENT_FIELDS))
        self.assertNotIn("hostname", observed)
        self.assertNotIn("uid", observed)
        self.assertNotIn("working_directory", observed)
        self.assertFalse(build_report()["current_host_record_persisted"])

    def test_extra_environment_fields_and_sensitive_lookalikes_are_rejected(self) -> None:
        extra = copy.deepcopy(self.profile["synthetic_runs"][0]["environment"])
        extra["hostname"] = "fixture-host"
        sensitive = copy.deepcopy(self.profile["synthetic_runs"][0]["environment"])
        sensitive["kernel_release"] = "synthetic-secret-token"
        self.assertTrue(validate_environment(extra))
        self.assertTrue(validate_environment(sensitive))

    def test_missing_identity_invalid_hash_and_unknown_status_are_rejected(self) -> None:
        base = self.profile["synthetic_runs"][0]["result"]
        missing = copy.deepcopy(base)
        missing.pop("policy")
        invalid_hash = copy.deepcopy(base)
        invalid_hash["build"]["sha256"] = "short"
        unknown_status = copy.deepcopy(base)
        unknown_status["status"] = "flaky"
        self.assertTrue(validate_result_context(missing))
        self.assertTrue(validate_result_context(invalid_hash))
        self.assertTrue(validate_result_context(unknown_status))

    def test_invalid_allowlist_version_or_timestamp_is_rejected(self) -> None:
        run = self.profile["synthetic_runs"][0]
        with self.assertRaises(ValueError):
            record_platform_result(run["environment"], run["result"], "not-a-time")
        with self.assertRaises(ValueError):
            record_platform_result(
                run["environment"],
                run["result"],
                run["captured_at"],
                "expanded-environment-v2",
            )

    def test_profile_cannot_remove_forbidden_fields_or_unblock_macos(self) -> None:
        weakened = copy.deepcopy(self.profile)
        weakened["forbidden_environment_fields"].remove("hostname")
        unblocked = copy.deepcopy(self.profile)
        unblocked["macos_execution_status"] = "pass"
        self.assertTrue(validate_profile(weakened))
        self.assertTrue(validate_profile(unblocked))

    def test_report_is_deterministic_synthetic_and_contains_no_current_host_record(self) -> None:
        first = build_report()
        second = build_report()
        self.assertEqual(first, second)
        self.assertEqual(first["synthetic_record_set"]["record_count"], 2)
        self.assertFalse(first["ambient_environment_values_recorded"])
        self.assertFalse(first["current_host_record_persisted"])
        mutated = copy.deepcopy(first)
        mutated["current_host_record_persisted"] = True
        self.assertTrue(validate_report(mutated))


if __name__ == "__main__":
    unittest.main()
