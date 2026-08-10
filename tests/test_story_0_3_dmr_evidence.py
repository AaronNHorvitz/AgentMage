import unittest

from scripts.story_0_3_dmr_evidence import (
    ARTIFACT_NAMES,
    Story03DMREvidenceError,
    build_manifest,
    sanitize_server_log,
)


class Story03DMREvidenceTests(unittest.TestCase):
    def test_server_log_sanitizer_removes_local_evaluation_root(self):
        source = "loading /home/example/.local/share/agentmage/evaluation/models/model.gguf\n"
        self.assertEqual(
            sanitize_server_log(source),
            "loading <EVALUATION_DATA_ROOT>/models/model.gguf\n",
        )

    def test_server_log_sanitizer_rejects_sensitive_identifiers(self):
        with self.assertRaises(Story03DMREvidenceError):
            sanitize_server_log("contact person@example.test\n")

    def test_manifest_hashes_every_declared_artifact_in_order(self):
        bundle = {name: name.encode() for name in ARTIFACT_NAMES}
        disposition = {
            "result_source_revision": "a" * 40,
            "diagnostic_source_revision": "b" * 40,
            "verification_revision": "c" * 40,
        }
        manifest = build_manifest(bundle, disposition)
        self.assertEqual(
            [entry["path"] for entry in manifest["files"]],
            list(ARTIFACT_NAMES),
        )
        self.assertTrue(all(len(entry["sha256"]) == 64 for entry in manifest["files"]))


if __name__ == "__main__":
    unittest.main()
