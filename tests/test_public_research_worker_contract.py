"""Checks of the public research worker contract's fixtures and document (Decision 0141)."""

from __future__ import annotations

import hashlib
import json
import unittest
from pathlib import Path

from scripts.public_research_worker_fixtures import printed_fixtures

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "fixtures/public-research-worker/v1"
DOCUMENT = ROOT / "docs/architecture/public-research-worker-contract-v1.md"
RECORDS = {
    "request-visit.json": ("request", 1),
    "request-search.json": ("request", 1),
    "response-visit-v1.frame": ("frame", 1),
    "response-visit-v2.frame": ("frame", 2),
    "response-search-v1.frame": ("frame", 1),
    "failures.json": ("failure_reports", 1),
    "binding-visit.json": ("permit_binding", 1),
}
FRAMES = {
    "response-visit-v1.frame": "request-visit.json",
    "response-visit-v2.frame": "request-visit.json",
    "response-search-v1.frame": "request-search.json",
}
CODES = [
    "research.worker.environment-denied",
    "research.worker.input-denied",
    "research.worker.destination-denied",
    "research.worker.transport-failed",
    "research.worker.response-denied",
    "research.worker.limit",
    "research.worker.deadline",
    "research.worker.output-failed",
]


def frame_parts(data: bytes) -> tuple[bytes, bytes]:
    """Splits a frame into its metadata and body bytes."""
    length = int.from_bytes(data[:4], "big")
    return data[4:4 + length], data[4 + length:]


def compact(value: object) -> bytes:
    return json.dumps(value, separators=(",", ":"), ensure_ascii=False).encode()


class PublicResearchWorkerContractTests(unittest.TestCase):
    def manifest(self) -> dict:
        return json.loads((FIXTURES / "manifest.json").read_text(encoding="utf-8"))

    def test_the_manifest_names_every_fixture_with_its_digest(self) -> None:
        manifest = self.manifest()
        self.assertEqual(manifest["contract"], "agentmage-public-research-worker")
        self.assertEqual(manifest["contract_version"], 1)
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

    def test_each_packet_is_one_compact_json_object_in_declared_order(self) -> None:
        for name in ("request-visit.json", "request-search.json"):
            data = (FIXTURES / name).read_bytes()
            packet = json.loads(data)
            # Python keeps member order, so re-encoding compactly is the same bytes.
            self.assertEqual(compact(packet), data, name)
            self.assertEqual(list(packet), ["schema_version", "task_id", "policy_sha256",
                                            "prepared_at_epoch_ms", "deadline_epoch_ms", "request"])
            request = packet["request"]
            self.assertEqual(packet["deadline_epoch_ms"],
                             packet["prepared_at_epoch_ms"] + request["timeout_ms"])
            self.assertEqual(request["target"]["domain"].count("."), 2)

    def test_each_frame_is_compact_metadata_then_the_exact_body(self) -> None:
        for name, request in FRAMES.items():
            data = (FIXTURES / name).read_bytes()
            metadata, body = frame_parts(data)
            value = json.loads(metadata)
            self.assertEqual(compact(value), metadata, name)
            observation = value["observation"] if "worker_reported_urls" in value else value
            if name.endswith("-v2.frame"):
                self.assertEqual(value["schema_version"], 2)
                self.assertEqual(len(value["worker_reported_urls"]), len(observation["hops"]))
            packet = (FIXTURES / request).read_bytes()
            self.assertEqual(observation["request_sha256"], hashlib.sha256(packet).hexdigest())
            self.assertEqual(observation["body_sha256"], hashlib.sha256(body).hexdigest())
            self.assertEqual(observation["hops"][-1]["status"], 200)
            self.assertEqual(observation["hops"][-1]["body_bytes"], len(body))
            body.decode("utf-8")

    def test_the_failure_reports_are_the_closed_codes(self) -> None:
        failures = json.loads((FIXTURES / "failures.json").read_text(encoding="utf-8"))
        self.assertEqual(failures["exit_status"], 5)
        self.assertEqual([report["code"] for report in failures["reports"]], CODES)
        for report in failures["reports"]:
            self.assertEqual(report["stderr"], report["code"] + "\n")

    def test_the_binding_names_the_packet_as_the_only_effect(self) -> None:
        binding = json.loads((FIXTURES / "binding-visit.json").read_text(encoding="utf-8"))
        packet_bytes = (FIXTURES / binding["packet"]).read_bytes()
        packet = json.loads(packet_bytes)
        digest = hashlib.sha256(packet_bytes).hexdigest()
        self.assertEqual(binding["packet_sha256"], digest)
        self.assertEqual(binding["grant"]["preview_sha256"], digest)
        self.assertEqual(binding["grant"]["expected_effect"]["details_sha256"], digest)
        self.assertEqual(binding["grant"]["expected_effect"]["target_indexes"], [0])
        self.assertTrue(binding["grant"]["single_use"])
        arguments = binding["arguments"].encode()
        self.assertEqual(binding["arguments_sha256"], hashlib.sha256(arguments).hexdigest())
        self.assertEqual(json.loads(arguments), packet["request"])
        self.assertEqual(binding["tool_call_id"], packet["request"]["operation_id"])
        self.assertEqual(binding["network_scope"],
                         f"https:{packet['request']['target']['domain']}:443")

    def test_the_document_names_every_fixture_and_code(self) -> None:
        text = DOCUMENT.read_text(encoding="utf-8")
        for name in [*RECORDS, "manifest.json"]:
            self.assertIn(f"`{name}`", text)
        for code in CODES:
            self.assertIn(f"`{code}`", text)
        self.assertIn("| This contract | 1 |", text)
        self.assertIn("unexecuted", text)

    def test_fixtures_hold_no_host_paths_or_private_names(self) -> None:
        for path in (DOCUMENT, *FIXTURES.iterdir()):
            text = path.read_bytes().decode("utf-8", errors="replace")
            self.assertNotIn("/home/", text, path.name)
            self.assertNotIn("/var/", text, path.name)
            self.assertNotIn("/lane/", text, path.name)

    def test_the_regeneration_script_reads_only_its_marked_line(self) -> None:
        files = {"a.json": "7b7d", "b.frame": "00"}
        output = "\n".join(["running 1 test", "BEGIN-PUBLIC-RESEARCH-WORKER-FIXTURES",
                            json.dumps(files), "END-PUBLIC-RESEARCH-WORKER-FIXTURES", "ok"])
        self.assertEqual(printed_fixtures(output), {"a.json": b"{}", "b.frame": b"\0"})
        for broken in (
            output.replace("BEGIN-PUBLIC-RESEARCH-WORKER-FIXTURES", ""),
            output + "\nEND-PUBLIC-RESEARCH-WORKER-FIXTURES",
            output.replace(json.dumps(files), json.dumps(files) + "\nextra"),
            output.replace(json.dumps(files), json.dumps({"../a.json": "00"})),
            output.replace(json.dumps(files), json.dumps({"a.txt": "00"})),
            output.replace(json.dumps(files), json.dumps({"a.json": "zz"})),
            output.replace(json.dumps(files), json.dumps({})),
        ):
            with self.assertRaises(ValueError):
                printed_fixtures(broken)


if __name__ == "__main__":
    unittest.main()
