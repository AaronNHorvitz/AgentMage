"""Request observations from owned non-model children; no CLI/model qualification."""
from contextlib import ExitStack
import io
import json
import os
from pathlib import Path
import select
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

from scripts import coding_harness as harness


class HarnessModelDiagnosticsTests(unittest.TestCase):
    def test_only_one_closed_model_argument_is_reported_without_serving_claims(self):
        for selection in harness.MODEL_CHOICES:
            with self.subTest(selection=selection):
                result = harness._model_request_diagnosis(
                    [b"synthetic-owned-cli", b"--model", selection.encode(), b""]
                )
                self.assertEqual(result, {
                    "selection": selection, "observation": "owned-process-arguments",
                    "serving": "not-observed", "qualification": "not-assessed",
                })

    def test_missing_conflicting_unknown_and_private_arguments_are_not_inferred_or_echoed(self):
        for arguments in (
            None, [], [b"fixture"], [b"--model"], [b"--model", b""],
            [b"--model", b"MUSE"], [b"--model", b"\xff"],
            [b"--model", b"private-model-argument"],
            [b"--model=muse"],
            [b"--model", b"muse", b"--model", b"gpt-oss"],
            [b"--model", b"muse", b"--model", b"muse"],
            [b"--model", b"muse", b"--model=scripted"],
        ):
            with self.subTest(arguments=arguments):
                result = harness._model_request_diagnosis(arguments)
                self.assertEqual(result, {
                    "selection": None, "observation": "unavailable",
                    "serving": "not-observed", "qualification": "not-assessed",
                })
                self.assertNotIn("private-model-argument", json.dumps(result))

    def test_owned_non_model_process_reports_request_and_preserves_legacy_record_lifecycle(self):
        executable = Path(sys.executable).resolve()
        with tempfile.TemporaryDirectory(prefix="am-md-") as temporary, ExitStack() as stack:
            base = Path(temporary) / "fixture"
            stack.enter_context(mock.patch.object(harness, "binary", return_value=executable))
            stack.enter_context(mock.patch.object(harness, "confinement_diagnosis", return_value={"ready": False}))
            stack.enter_context(mock.patch.object(harness.CandidateResourceGuard, "acquire",
                                                  side_effect=AssertionError("diagnosis must not acquire GPU resources")))
            stack.enter_context(mock.patch("sys.stdout", new=io.StringIO()))
            harness.setup(base)
            state, _, _ = harness.paths(base)
            key = state / "operational-store-development-v1.key"
            for model_arguments, expected in (
                (["--model", "scripted"], "scripted"),
                (["--model", "muse"], "muse"),
                (["--model", "gpt-oss"], "gpt-oss"),
                ([], None),
                (["--model", "private-model-argument"], None),
                (["--model", "muse", "--model", "scripted"], None),
            ):
                with self.subTest(model_arguments=model_arguments):
                    owner = harness.RunRecordReservation(state, base, executable)
                    child = None
                    try:
                        before = (state / harness.RUN_RECORD).read_bytes()
                        self.assertEqual(owner.record["schema_version"], 2)
                        self.assertNotIn("model", owner.record)
                        self.assertNotIn("requested_model", owner.record)
                        reserved = harness.diagnose(base)
                        self.assertEqual(reserved["lifecycle"], "reserved")
                        self.assertIsNone(reserved["model_request"]["selection"])
                        self.assertEqual((state / harness.RUN_RECORD).read_bytes(), before)

                        command = [str(executable), "-c",
                                   "import time; print('ready', flush=True); time.sleep(30)",
                                   "--development", *model_arguments]
                        for flag, path in zip(("--state-root", "--disposable-root", "--workspace-root"),
                                              harness.paths(base)):
                            command.extend((flag, str(path)))
                        child = subprocess.Popen(command, stdin=subprocess.DEVNULL,
                                                 stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
                        self.assertTrue(select.select([child.stdout], [], [], 5)[0], "owned child not ready")
                        self.assertEqual(child.stdout.readline(), "ready\n")
                        owner.identify(child)
                        retained = (state / harness.RUN_RECORD).read_bytes()
                        self.assertTrue(harness.exact_running_process(owner.record, base))
                        starting = harness.diagnose(base)
                        self.assertEqual(starting["schema_version"], 2)
                        self.assertEqual(starting["scope"], "development-diagnostics-only")
                        self.assertEqual(starting["fixture_profile_id"], harness.PROFILE)
                        self.assertNotIn("profile_id", starting)
                        self.assertNotIn("qualification", starting)
                        self.assertEqual(starting["lifecycle"], "starting")
                        self.assertEqual(starting["model_request"]["selection"], expected)
                        self.assertEqual(starting["model_request"]["serving"], "not-observed")
                        self.assertEqual(starting["model_request"]["qualification"], "not-assessed")
                        self.assertNotIn("private-model-argument", json.dumps(starting))
                        self.assertEqual((state / harness.RUN_RECORD).read_bytes(), retained)

                        harness.private_file(key, "synthetic state-key presence only")
                        running = harness.diagnose(base)
                        self.assertEqual(running["lifecycle"], "running")
                        self.assertEqual(running["model_request"], starting["model_request"])
                        changed_identity = {**owner.record["process_identity"],
                                            "start_ticks": owner.record["process_identity"]["start_ticks"] + 1}
                        with mock.patch.object(harness, "linux_process_identity", return_value=changed_identity):
                            stale = harness.diagnose(base)
                        self.assertEqual(stale["lifecycle"], "stale-record")
                        self.assertIsNone(stale["model_request"]["selection"])
                        self.assertEqual((state / harness.RUN_RECORD).read_bytes(), retained)

                        child.terminate()
                        child.wait(timeout=5)
                        stale = harness.diagnose(base)
                        self.assertEqual(stale["lifecycle"], "stale-record")
                        self.assertIsNone(stale["model_request"]["selection"])
                        self.assertEqual((state / harness.RUN_RECORD).read_bytes(), retained)
                    finally:
                        if child is not None:
                            if child.poll() is None:
                                child.kill()
                            child.communicate(timeout=5)
                            child.stdout.close()
                            child.stderr.close()
                        owner.close(child_reaped=child is None or child.poll() is not None)
                        if key.exists():
                            key.unlink()
                    idle = harness.diagnose(base)
                    self.assertEqual(idle["lifecycle"], "ready")
                    self.assertIsNone(idle["model_request"]["selection"])


if __name__ == "__main__":
    unittest.main()
