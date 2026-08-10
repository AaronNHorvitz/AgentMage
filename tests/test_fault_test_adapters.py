from __future__ import annotations

import ast
import copy
import json
import unittest

from fixtures.fault_test_adapters import (
    RESOURCE_DIMENSIONS,
    AdversarialDisposition,
    AdversarialTestAdapter,
    CancellationToken,
    CrashMode,
    CrashTestAdapter,
    FaultAdapter,
    FaultStatus,
    NetworkMode,
    NetworkTestAdapter,
    ResourceTestAdapter,
    canonical_json,
    trace_sha256,
)
from scripts.fault_test_adapter_evidence import (
    PROFILE_PATH,
    ROOT,
    build_report,
    check_report,
    read_json,
    run_scenarios,
    validate_profile,
    validate_report,
)


class FaultTestAdapterTests(unittest.TestCase):
    def setUp(self) -> None:
        self.profile = read_json(PROFILE_PATH)
        self.bounds = {
            "max_events": self.profile["bounds"]["max_events_per_adapter"],
            "max_detail_bytes": self.profile["bounds"]["max_detail_bytes"],
        }

    def test_profile_canonical_run_and_report_are_current(self) -> None:
        self.assertEqual(validate_profile(self.profile), [])
        self.assertEqual(check_report(), [])
        report = build_report()
        self.assertEqual(validate_report(report), [])
        self.assertEqual(report["event_count"], 29)
        self.assertEqual(report["side_effect_count"], 0)

    def test_canonical_run_is_deterministic_and_closes_every_adapter(self) -> None:
        first = run_scenarios(self.profile)
        second = run_scenarios(self.profile)
        self.assertEqual(first["records"], second["records"])
        self.assertEqual(first["trace_sha256"], second["trace_sha256"])
        self.assertTrue(all(adapter.closed for adapter in first["adapters"]))
        self.assertTrue(first["resource_counters_zeroed"])

    def test_crash_modes_preserve_commit_state_distinctions(self) -> None:
        adapter = CrashTestAdapter(**self.bounds)
        expected = {
            CrashMode.HEALTHY: (FaultStatus.SUCCEEDED, False),
            CrashMode.BEFORE_OPERATION: (FaultStatus.CRASHED, False),
            CrashMode.BEFORE_COMMIT: (FaultStatus.CRASHED, False),
            CrashMode.AFTER_COMMIT: (FaultStatus.CRASHED, True),
            CrashMode.UNCERTAIN_COMMIT: (FaultStatus.UNCERTAIN, None),
        }
        for mode, (status, committed) in expected.items():
            with self.subTest(mode=mode.value):
                outcome = adapter.inject("checkpoint", mode)
                self.assertIs(outcome.status, status)
                self.assertEqual(outcome.event.committed, committed)
                self.assertEqual(outcome.event.side_effects, ())

    def test_network_modes_are_simulated_without_network_calls(self) -> None:
        allowlist = self.profile["network_allowlist"]
        adapter = NetworkTestAdapter(allowlist, **self.bounds)
        expected = {
            NetworkMode.ALLOWLISTED_SUCCESS: FaultStatus.SUCCEEDED,
            NetworkMode.POLICY_DENIAL: FaultStatus.DENIED,
            NetworkMode.DNS_FAILURE: FaultStatus.FAILED,
            NetworkMode.CONNECT_TIMEOUT: FaultStatus.TIMED_OUT,
            NetworkMode.READ_TIMEOUT: FaultStatus.TIMED_OUT,
            NetworkMode.CONNECTION_RESET: FaultStatus.FAILED,
            NetworkMode.PARTIAL_RESPONSE: FaultStatus.PARTIAL,
            NetworkMode.OFFLINE: FaultStatus.UNAVAILABLE,
        }
        for mode, status in expected.items():
            outcome = adapter.request(allowlist[0], mode)
            self.assertIs(outcome.status, status)
        denied = adapter.request(
            "https://not-allowed.fixture.invalid/v1/chat",
            NetworkMode.ALLOWLISTED_SUCCESS,
        )
        self.assertIs(denied.status, FaultStatus.DENIED)
        self.assertEqual(denied.event.mode, "policy-denial")

    def test_network_adapter_rejects_non_synthetic_endpoints(self) -> None:
        adapter = NetworkTestAdapter(self.profile["network_allowlist"], **self.bounds)
        for endpoint in (
            "https://example.com/v1/chat",
            "http://model.fixture.invalid/v1/chat",
            "https://person@model.fixture.invalid/v1/chat",
            "https://model.fixture.invalid/v1/chat?value=1",
        ):
            with self.subTest(endpoint=endpoint), self.assertRaises(ValueError):
                adapter.request(endpoint, NetworkMode.OFFLINE)

    def test_adapter_module_imports_no_network_or_process_client(self) -> None:
        source = (ROOT / "fixtures/fault_test_adapters.py").read_text(encoding="utf-8")
        tree = ast.parse(source)
        imports = set()
        for item in ast.walk(tree):
            if isinstance(item, ast.Import):
                imports.update(alias.name for alias in item.names)
            elif isinstance(item, ast.ImportFrom) and item.module is not None:
                imports.add(item.module)
        self.assertTrue(
            imports.isdisjoint(
                {"socket", "requests", "http.client", "urllib.request", "subprocess"}
            )
        )

    def test_resource_exhaustion_uses_integer_accounting_only(self) -> None:
        adapter = ResourceTestAdapter(self.profile["resource_limits"], **self.bounds)
        zero = {key: 0 for key in RESOURCE_DIMENSIONS}
        accepted = dict(zero)
        accepted["cpu_steps"] = 1
        self.assertIs(
            adapter.consume("accepted", accepted).status,
            FaultStatus.SUCCEEDED,
        )
        before = dict(adapter.used)
        exhausted = dict(zero)
        exhausted["memory_bytes"] = self.profile["resource_limits"]["memory_bytes"] + 1
        outcome = adapter.consume("exhausted", exhausted)
        self.assertIs(outcome.status, FaultStatus.RESOURCE_EXHAUSTED)
        self.assertEqual(outcome.metadata["exhausted_dimension"], "memory_bytes")
        self.assertEqual(adapter.used, before)

    def test_every_resource_dimension_has_an_exhaustion_case(self) -> None:
        exhausted = [
            item["exhausted_dimension"]
            for item in self.profile["resource_cases"]
            if item["exhausted_dimension"] is not None
        ]
        self.assertEqual(exhausted, list(RESOURCE_DIMENSIONS))

    def test_adversarial_adapter_retains_metadata_not_payloads(self) -> None:
        adapter = AdversarialTestAdapter(**self.bounds)
        outcome = adapter.inspect(
            case_id="prompt-injection",
            archive_path="adversarial/prompt-injection/requests.json",
            classification="prompt-injection",
            disposition=AdversarialDisposition.REJECT,
            payload_sha256="a" * 64,
            payload_bytes=128,
        )
        serialized = json.dumps(outcome.event.as_record(), sort_keys=True)
        self.assertIs(outcome.status, FaultStatus.REJECTED)
        self.assertFalse(outcome.metadata["raw_payload_retained"])
        self.assertNotIn("payload", serialized.lower())
        self.assertEqual(outcome.event.side_effects, ())

    def test_pre_cancelled_operations_are_visible_for_every_adapter(self) -> None:
        token = CancellationToken()
        token.cancel()
        adapters = [
            CrashTestAdapter(**self.bounds),
            NetworkTestAdapter(self.profile["network_allowlist"], **self.bounds),
            ResourceTestAdapter(self.profile["resource_limits"], **self.bounds),
            AdversarialTestAdapter(**self.bounds),
        ]
        outcomes = [
            adapters[0].inject("cancelled", CrashMode.HEALTHY, token),
            adapters[1].request(
                self.profile["network_allowlist"][0],
                NetworkMode.ALLOWLISTED_SUCCESS,
                token,
            ),
            adapters[2].consume(
                "cancelled", {key: 0 for key in RESOURCE_DIMENSIONS}, token
            ),
            adapters[3].inspect(
                case_id="cancelled",
                archive_path="adversarial/prompt-injection/requests.json",
                classification="prompt-injection",
                disposition=AdversarialDisposition.REJECT,
                payload_sha256="a" * 64,
                payload_bytes=1,
                token=token,
            ),
        ]
        self.assertTrue(all(item.status is FaultStatus.CANCELLED for item in outcomes))

    def test_close_is_idempotent_zeroes_resources_and_rejects_operations(self) -> None:
        adapter = ResourceTestAdapter(self.profile["resource_limits"], **self.bounds)
        demand = {key: 0 for key in RESOURCE_DIMENSIONS}
        demand["token_count"] = 1
        adapter.consume("accepted", demand)
        adapter.close()
        adapter.close()
        self.assertTrue(adapter.closed)
        self.assertTrue(all(value == 0 for value in adapter.used.values()))
        with self.assertRaises(RuntimeError):
            adapter.consume("after-close", demand)

    def test_event_and_detail_bounds_fail_closed(self) -> None:
        adapter = FaultAdapter("bounded", max_events=1, max_detail_bytes=8)
        first = adapter._record(
            operation="record",
            mode="fixture",
            status=FaultStatus.FAILED,
            request={},
            logical_duration_ms=0,
            committed=False,
            detail="0123456789",
        )
        self.assertEqual(len(first.event.detail.encode("utf-8")), 8)
        with self.assertRaises(RuntimeError):
            adapter._record(
                operation="record",
                mode="fixture",
                status=FaultStatus.FAILED,
                request={},
                logical_duration_ms=0,
                committed=False,
                detail=None,
            )

    def test_invalid_resource_and_adversarial_metadata_fail_closed(self) -> None:
        resource = ResourceTestAdapter(self.profile["resource_limits"], **self.bounds)
        invalid_demand = {key: 0 for key in RESOURCE_DIMENSIONS}
        invalid_demand["memory_bytes"] = -1
        with self.assertRaises(ValueError):
            resource.consume("invalid", invalid_demand)

        adversarial = AdversarialTestAdapter(**self.bounds)
        with self.assertRaises(ValueError):
            adversarial.inspect(
                case_id="invalid",
                archive_path="../outside",
                classification="prompt-injection",
                disposition=AdversarialDisposition.REJECT,
                payload_sha256="not-a-hash",
                payload_bytes=-1,
            )

    def test_trace_hash_detects_event_mutation(self) -> None:
        first = CrashTestAdapter(**self.bounds)
        second = CrashTestAdapter(**self.bounds)
        first.inject("first", CrashMode.HEALTHY)
        second.inject("first", CrashMode.HEALTHY)
        self.assertEqual(trace_sha256([first]), trace_sha256([second]))
        second.inject("second", CrashMode.BEFORE_COMMIT)
        self.assertNotEqual(trace_sha256([first]), trace_sha256([second]))

    def test_profile_rejects_missing_modes_side_effects_and_macos_claims(self) -> None:
        missing = copy.deepcopy(self.profile)
        missing["network_cases"].pop()
        networked = copy.deepcopy(self.profile)
        networked["execution_contract"]["uses_network"] = True
        unblocked = copy.deepcopy(self.profile)
        unblocked["macos_execution_status"] = "pass"
        self.assertTrue(validate_profile(missing))
        self.assertTrue(validate_profile(networked))
        self.assertTrue(validate_profile(unblocked))

    def test_report_contains_hashes_and_counts_but_no_raw_events(self) -> None:
        report = build_report()
        serialized = json.dumps(report, sort_keys=True)
        self.assertNotIn("records", report)
        self.assertNotIn("request_sha256", serialized)
        self.assertFalse(report["raw_payloads_retained"])
        self.assertEqual(report["network_calls_performed"], 0)
        self.assertEqual(report["product_support_claim"], "none")
        self.assertEqual(report["macos_execution_status"], "blocked-macos")
        mutated = copy.deepcopy(report)
        mutated["network_calls_performed"] = 1
        self.assertTrue(validate_report(mutated))


if __name__ == "__main__":
    unittest.main()
