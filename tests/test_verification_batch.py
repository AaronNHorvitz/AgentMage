from __future__ import annotations

import hashlib
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from scripts import verification_batch as batch


def digest(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


class VerificationBatchTests(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.repo = Path(self.directory.name) / "repo"
        self.repo.mkdir()
        self.git("init", "-q")
        self.git("config", "user.email", "fixture@example.invalid")
        self.git("config", "user.name", "Fixture")
        root = mock.patch.object(batch, "ROOT", self.repo)
        root.start()
        self.addCleanup(root.stop)
        self.addCleanup(self.directory.cleanup)

    def git(self, *arguments: str) -> str:
        return subprocess.run(
            ("git", *arguments), cwd=self.repo, check=True, capture_output=True, text=True
        ).stdout.strip()

    def write(self, path: str, text: str) -> None:
        target = self.repo / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8")

    def commit(self, message: str) -> str:
        self.git("add", "-A")
        self.git("commit", "-q", "-m", message)
        return self.git("rev-parse", "HEAD")

    def two_revisions(self) -> tuple[str, str]:
        self.write("src/a.txt", "old\n")
        self.write("evidence/bound.json", json.dumps({"inputs": {"src/a.txt": digest("old\n")}}))
        self.write("evidence/older.json", json.dumps(
            {"sources": [{"path": "src/a.txt", "sha256": digest("older\n")}]}))
        base = self.commit("base")
        self.write("src/a.txt", "new\n")
        self.write("evidence/renewed.json", json.dumps({"src/a.txt": digest("new\n")}))
        return base, self.commit("change")

    def test_inventory_reads_both_revisions_from_git_and_reproduces_later(self) -> None:
        base, head = self.two_revisions()
        value = batch.inventory(base, head)
        self.assertEqual(value["changed_paths"], ["evidence/renewed.json", "src/a.txt"])
        self.assertEqual(
            value["counts"],
            {"current": 1, "newly-stale": 1, "previously-stale-or-historical": 1},
        )
        self.assertEqual(value["newly_stale_artifacts"], ["evidence/bound.json"])
        # A later commit and a dirty worktree do not change the result.
        self.write("src/a.txt", "later\n")
        self.commit("later")
        self.write("src/a.txt", "dirty\n")
        self.assertEqual(batch.inventory(base, head), value)
        output = self.repo.parent / "inventory.json"
        self.assertEqual(
            batch.main(["inventory", "--base", base, "--head", head, "--output", str(output)]), 0
        )
        self.assertEqual(
            batch.main(["inventory", "--base", base, "--head", head, "--check", str(output)]), 0
        )
        output.write_text(output.read_text().replace('"current"', '"forged"'))
        self.assertEqual(
            batch.main(["inventory", "--base", base, "--head", head, "--check", str(output)]), 1
        )
        with self.assertRaises(batch.RecordError):
            batch.inventory(head, base)

    def stage_files(self, revision: str, log_text: str) -> tuple[Path, Path]:
        private = self.repo.parent / "private"
        (private / "logs").mkdir(parents=True, exist_ok=True)
        log = private / "logs" / "01.log"
        log.write_text(log_text, encoding="utf-8")
        plan = private / "stage-plan.json"
        results = private / "stage-results.json"
        plan.write_text(json.dumps({
            "name": "fixture-core", "source_revision": revision,
            "scope": f"Stage run from {self.repo}", "commands": [["python3", "-c", "pass"]],
            "sources": {"src/a.txt": digest("new\n")},
        }))
        results.write_text(json.dumps([{
            "index": 1, "command": ["python3", "-c", "pass"], "exit_code": 0, "seconds": 0.5,
            "log": str(log), "sha256": hashlib.sha256(log.read_bytes()).hexdigest(),
        }]))
        return plan, results

    def test_retained_logs_are_redacted_bound_and_refused_when_private_names_remain(self) -> None:
        _, head = self.two_revisions()
        state = self.repo.parent / "state-root"
        text = f"Compiling crate ({self.repo}/src)\nwrote {state}/scratch/x\ntest result: ok. 3 passed; 0 failed; 1 ignored;\n"
        plan, results = self.stage_files(head, text)
        destination = self.repo / "docs/logs"
        with mock.patch.object(batch, "private_names", return_value=["fixtureuser"]):
            stage = batch.retain(plan, results, destination, [(str(state), "<state>")])
        retained = (destination / "fixture-core" / "01.log").read_text()
        self.assertEqual(
            retained,
            "Compiling crate (<repo>/src)\nwrote <state>/scratch/x\ntest result: ok. 3 passed; 0 failed; 1 ignored;\n",
        )
        self.assertEqual(stage["scope"], "Stage run from <repo>")
        self.assertEqual(stage["results"][0]["log_sha256"], digest(retained))
        self.assertEqual(batch.load_stage(destination / "fixture-core"), stage)
        with self.assertRaises(batch.RecordError):
            batch.retain(plan, results, destination, [])
        plan, results = self.stage_files(head, "user fixtureuser at host\n")
        with mock.patch.object(batch, "private_names", return_value=["fixtureuser"]):
            with self.assertRaises(batch.RecordError):
                batch.retain(plan, results, self.repo / "docs/other", [])
        self.assertFalse((self.repo / "docs/other").exists())
        plan, results = self.stage_files(head, "original\n")
        Path(json.loads(results.read_text())[0]["log"]).write_text("changed afterwards\n")
        with self.assertRaises(batch.RecordError):
            batch.retain(plan, results, self.repo / "docs/changed", [])
        (destination / "fixture-core" / "01.log").write_text("tampered\n")
        with self.assertRaises(batch.RecordError):
            batch.load_stage(destination / "fixture-core")

    def test_records_are_built_only_from_committed_material_and_checked(self) -> None:
        base, source = self.two_revisions()
        plan, results = self.stage_files(source, "test result: ok. 5 passed; 1 failed; 2 ignored;\n")
        with mock.patch.object(batch, "private_names", return_value=[]):
            batch.retain(plan, results, self.repo / "docs/logs/batch", [])
        (self.repo / "docs/logs/batch/checks").mkdir()
        (self.repo / "docs/logs/batch/checks/engine.log").write_text(
            "test result: ok. 5 passed; 0 failed; 1 ignored;\ntest result: FAILED. 2 passed; 1 failed; 0 ignored;\n"
        )
        head = self.commit("retain stage")
        inventory = batch.inventory(base, head)
        self.write("docs/inventory.json", json.dumps(inventory, indent=2) + "\n")
        spec = {
            "record_type": "fixture-verification", "date": "2026-09-29",
            "base_commit": base, "head_commit": head, "source_commit": source,
            "retained_logs": "docs/logs/batch", "stages": ["fixture-core"],
            "inventory": "docs/inventory.json",
            "check_logs": {"engine": "checks/engine.log"},
            "fields": {"claims": {"task_rows_closed": []}},
        }
        self.write("docs/spec.json", json.dumps(spec))
        record = batch.build_record(self.repo / "docs/spec.json")
        self.assertEqual([row["subject"] for row in record["commits"]], ["change", "retain stage"])
        self.assertEqual(record["check_logs"]["engine"]["passed"], 7)
        self.assertEqual(record["check_logs"]["engine"]["failed"], 1)
        self.assertEqual(record["check_logs"]["engine"]["result_lines"], 2)
        self.assertEqual(record["evidence_stages"][0]["ran"], 1)
        self.assertEqual(record["claims"], {"task_rows_closed": []})
        output = self.repo / "docs/record.json"
        self.assertEqual(batch.main(["record", "--spec", str(self.repo / "docs/spec.json"),
                                     "--output", str(output)]), 0)
        self.assertEqual(batch.main(["record", "--spec", str(self.repo / "docs/spec.json"),
                                     "--check", str(output)]), 0)
        for change in (
            {"source_commit": base},
            {"head_commit": base},
            {"fields": {"commits": []}},
        ):
            with self.subTest(change=change):
                self.write("docs/spec.json", json.dumps({**spec, **change}))
                with self.assertRaises(batch.RecordError):
                    batch.build_record(self.repo / "docs/spec.json")
        self.write("docs/spec.json", json.dumps(spec))
        forged = json.loads((self.repo / "docs/inventory.json").read_text())
        forged["counts"] = {}
        self.write("docs/inventory.json", json.dumps(forged))
        with self.assertRaises(batch.RecordError):
            batch.build_record(self.repo / "docs/spec.json")


if __name__ == "__main__":
    unittest.main()
