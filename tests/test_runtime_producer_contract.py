"""Checks of the runtime producer contract's fixtures and document (Decision 0135)."""

from __future__ import annotations

import hashlib
import json
import unittest
from pathlib import Path

from scripts.runtime_producer_fixtures import printed_fixtures

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "fixtures/runtime-producer/v1"
DOCUMENT = ROOT / "docs/architecture/runtime-producer-contract-v1.md"
RECORDS = {
    "run-request.json": ("run_request", 2),
    "run-recipe.json": ("run_recipe", 1),
    "run-declarations.json": ("run_declarations", 4),
    "run-declarations-absent.json": ("run_declarations", 4),
    "job-control-request.json": ("job_control_request", 1),
    "job-control.json": ("job_control", 1),
    "job-status.json": ("job_status", 1),
    "ended-run-histories.json": ("ended_run_histories", 1),
}


def strings(value: object) -> list[str]:
    if isinstance(value, str):
        return [value]
    if isinstance(value, dict):
        return [text for key, item in value.items() for text in [key, *strings(item)]]
    if isinstance(value, list):
        return [text for item in value for text in strings(item)]
    return []


class RuntimeProducerContractTests(unittest.TestCase):
    def manifest(self) -> dict:
        return json.loads((FIXTURES / "manifest.json").read_text(encoding="utf-8"))

    def test_the_manifest_names_every_fixture_with_its_digest(self) -> None:
        manifest = self.manifest()
        self.assertEqual(manifest["contract"], "agentmage-runtime-producer")
        self.assertEqual(manifest["contract_version"], 1)
        self.assertEqual(manifest["wire_version"], 15)
        listed = {entry["file"]: entry for entry in manifest["records"]}
        self.assertEqual(len(listed), len(manifest["records"]))
        self.assertEqual(set(listed), set(RECORDS))
        present = {path.name for path in FIXTURES.iterdir()}
        self.assertEqual(present, set(RECORDS) | {"manifest.json"})
        for name, (record, schema_version) in RECORDS.items():
            entry = listed[name]
            self.assertEqual((entry["record"], entry["schema_version"]), (record, schema_version))
            data = (FIXTURES / name).read_bytes()
            self.assertEqual(entry["sha256"], hashlib.sha256(data).hexdigest(), name)
            self.assertTrue(data.endswith(b"}\n"), name)
            json.loads(data)

    def test_the_declarations_fixtures_hold_every_part_and_none(self) -> None:
        parts = ("recoverability", "context_inspections", "effect_history",
                 "job_control_history", "route_receipt", "route_history", "recipe_plan")
        present = json.loads((FIXTURES / "run-declarations.json").read_text(encoding="utf-8"))
        absent = json.loads((FIXTURES / "run-declarations-absent.json").read_text(encoding="utf-8"))
        request = json.loads((FIXTURES / "run-request.json").read_text(encoding="utf-8"))
        for declarations in (present, absent):
            self.assertEqual(declarations["schema_version"], 4)
            self.assertEqual(declarations["run_id"], request["run_id"])
            self.assertEqual(declarations["request_sha256"], request["request_sha256"])
            self.assertEqual(set(declarations), {"schema_version", "run_id", "request_sha256", *parts})
        for part in parts:
            self.assertIsNotNone(present[part], part)
            self.assertIsNone(absent[part], part)
        bound = [constraint for constraint in request["task"]["constraints"]
                 if constraint.startswith("Recipe plan digest: ")]
        self.assertEqual(bound, [f"Recipe plan digest: {present['recipe_plan']['plan_sha256']}"])

    def test_the_document_names_every_fixture_and_version(self) -> None:
        text = DOCUMENT.read_text(encoding="utf-8")
        for name in [*RECORDS, "manifest.json"]:
            self.assertIn(f"`{name}`", text)
        self.assertIn("| This contract | 1 |", text)
        self.assertIn("| Transport wire | 15 |", text)
        self.assertIn("| Run declarations | schema 4 |", text)
        self.assertIn("unexecuted producer specification", text)

    def test_fixtures_hold_no_host_paths_or_private_names(self) -> None:
        for name in [*RECORDS, "manifest.json"]:
            data = json.loads((FIXTURES / name).read_text(encoding="utf-8"))
            for text in strings(data):
                self.assertFalse(text.startswith("/"), (name, text))
                self.assertNotIn("/home/", text, name)
                self.assertNotIn("\\", text, name)
        for path in (DOCUMENT, *FIXTURES.iterdir()):
            text = path.read_text(encoding="utf-8")
            self.assertNotIn("/home/", text, path.name)
            self.assertNotIn("/var/", text, path.name)

    def test_the_regeneration_script_reads_only_its_marked_line(self) -> None:
        files = {"a.json": "{}\n"}
        output = "\n".join(["running 1 test", "BEGIN-RUNTIME-PRODUCER-FIXTURES",
                            json.dumps(files), "END-RUNTIME-PRODUCER-FIXTURES", "ok"])
        self.assertEqual(printed_fixtures(output), files)
        for broken in (
            output.replace("BEGIN-RUNTIME-PRODUCER-FIXTURES", ""),
            output + "\nEND-RUNTIME-PRODUCER-FIXTURES",
            output.replace(json.dumps(files), json.dumps(files) + "\nextra"),
            output.replace(json.dumps(files), json.dumps({"../a.json": "{}"})),
            output.replace(json.dumps(files), json.dumps({"a.txt": "{}"})),
            output.replace(json.dumps(files), json.dumps({})),
        ):
            with self.assertRaises(ValueError):
                printed_fixtures(broken)


if __name__ == "__main__":
    unittest.main()
