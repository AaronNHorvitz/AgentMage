from __future__ import annotations

import ast
import copy
import json
import unittest
from pathlib import Path

from fixtures.fake_adapters import (
    CrashInjector,
    FakeClock,
    FakeConnector,
    FakeInferenceRuntime,
    FakeMode,
    FakeModel,
    FakeSecretStore,
    FakeStatus,
    FakeTool,
    ScenarioPlan,
    SyntheticSecret,
    baseline_trace,
)
from scripts.fake_adapter_evidence import (
    CONTRACT_PATH,
    build_report,
    check_report,
    read_json,
    validate_contract,
    validate_report,
)


class FakeAdapterTests(unittest.TestCase):
    def setUp(self) -> None:
        self.contract = read_json(CONTRACT_PATH)

    def test_checked_in_contract_and_report_are_current(self) -> None:
        self.assertEqual(validate_contract(self.contract), [])
        self.assertEqual(check_report(), [])

    def test_baseline_trace_is_deterministic(self) -> None:
        self.assertEqual(baseline_trace(), baseline_trace())
        self.assertEqual(baseline_trace()["event_count"], 13)

    def test_each_failure_mode_has_a_typed_outcome(self) -> None:
        for mode in FakeMode:
            with self.subTest(mode=mode.value):
                plan = ScenarioPlan({"fake-model.generate": [mode]})
                outcome = FakeModel(plan).generate("fixture")
                expected = {
                    FakeMode.SUCCESS: FakeStatus.SUCCEEDED,
                    FakeMode.DENIAL: FakeStatus.DENIED,
                    FakeMode.MALFORMED: FakeStatus.MALFORMED,
                    FakeMode.CANCELLATION: FakeStatus.CANCELLED,
                    FakeMode.TIMEOUT: FakeStatus.TIMED_OUT,
                    FakeMode.CRASH_BEFORE: FakeStatus.CRASHED,
                    FakeMode.CRASH_AFTER: FakeStatus.CRASHED,
                    FakeMode.UNCERTAIN: FakeStatus.UNCERTAIN,
                }[mode]
                self.assertEqual(outcome.status, expected)

    def test_scenario_plan_consumes_modes_in_order(self) -> None:
        plan = ScenarioPlan(
            {"fake-tool.invoke": [FakeMode.DENIAL, FakeMode.TIMEOUT]}
        )
        tool = FakeTool(plan)
        self.assertEqual(tool.invoke("fixture", {}).status, FakeStatus.DENIED)
        self.assertEqual(tool.invoke("fixture", {}).status, FakeStatus.TIMED_OUT)
        self.assertEqual(tool.invoke("fixture", {}).status, FakeStatus.SUCCEEDED)
        self.assertEqual(plan.pending(), 0)

    def test_fake_runtime_enforces_lifecycle(self) -> None:
        runtime = FakeInferenceRuntime()
        self.assertEqual(runtime.infer("before start").status, FakeStatus.DENIED)
        self.assertEqual(runtime.start("fixture-profile").status, FakeStatus.SUCCEEDED)
        self.assertTrue(runtime.running)
        self.assertEqual(runtime.infer("running").status, FakeStatus.SUCCEEDED)
        self.assertEqual(runtime.stop().status, FakeStatus.SUCCEEDED)
        self.assertFalse(runtime.running)

    def test_fake_connector_has_only_synthetic_in_memory_records(self) -> None:
        connector = FakeConnector()
        listed = connector.list()
        self.assertEqual(listed.payload["ids"], ["record-alpha", "record-beta"])
        self.assertEqual(connector.read("missing").status, FakeStatus.DENIED)

    def test_fake_clock_advances_without_wall_clock_access(self) -> None:
        clock = FakeClock()
        self.assertEqual(clock.now(), 1704067200)
        self.assertEqual(clock.advance(90), 1704067290)
        with self.assertRaises(ValueError):
            clock.advance(-1)

    def test_secret_value_is_redacted_from_repr_and_trace(self) -> None:
        secret = SyntheticSecret("AM_SYNTHETIC_SECRET_TEST_ONLY")
        store = FakeSecretStore()
        store.store("handle", secret)
        loaded = store.load("handle")
        self.assertEqual(loaded.payload.reveal_for_test(), "AM_SYNTHETIC_SECRET_TEST_ONLY")
        self.assertEqual(repr(loaded.payload), "<SyntheticSecret redacted>")
        serialized_events = json.dumps([event.as_record() for event in store.events])
        self.assertNotIn("AM_SYNTHETIC_SECRET_TEST_ONLY", serialized_events)

    def test_secret_store_rejects_non_synthetic_value(self) -> None:
        with self.assertRaises(ValueError):
            SyntheticSecret("not-a-synthetic-secret")

    def test_crash_injector_distinguishes_commit_boundary(self) -> None:
        plan = ScenarioPlan(
            {
                "crash-injector.checkpoint": [
                    FakeMode.CRASH_BEFORE,
                    FakeMode.CRASH_AFTER,
                    FakeMode.UNCERTAIN,
                ]
            }
        )
        injector = CrashInjector(plan)
        before = injector.checkpoint("before")
        after = injector.checkpoint("after")
        uncertain = injector.checkpoint("uncertain")
        self.assertFalse(before.event.committed)
        self.assertTrue(after.event.committed)
        self.assertIsNone(uncertain.event.committed)

    def test_fake_module_imports_no_network_or_process_library(self) -> None:
        source = Path("fixtures/fake_adapters.py").read_text(encoding="utf-8")
        tree = ast.parse(source)
        imports = {
            alias.name.split(".", 1)[0]
            for node in ast.walk(tree)
            if isinstance(node, ast.Import)
            for alias in node.names
        }
        imports.update(
            node.module.split(".", 1)[0]
            for node in ast.walk(tree)
            if isinstance(node, ast.ImportFrom) and node.module
        )
        self.assertTrue(imports.isdisjoint({"socket", "subprocess", "urllib", "requests"}))

    def test_product_adapter_claim_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.contract)
        mutated["product_adapter_claim"] = "implemented"
        self.assertTrue(validate_contract(mutated))

    def test_report_cannot_expose_secret_or_claim_platform_support(self) -> None:
        mutated = build_report()
        mutated["platform_support_claim"] = "macos"
        mutated["raw_secret"] = "AM_SYNTHETIC_SECRET_FORBIDDEN"
        self.assertTrue(validate_report(mutated))


if __name__ == "__main__":
    unittest.main()
