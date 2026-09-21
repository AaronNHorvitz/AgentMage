import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from scripts import coding_model_lab as lab


class CodingModelLabTests(unittest.TestCase):
    def setUp(self):
        self.cfg = json.loads(lab.CONFIG.read_text())

    def test_profile_is_not_product_activation(self):
        self.assertFalse(self.cfg["product_enabled"])
        self.assertEqual(set(self.cfg["models"]), {"muse", "gpt-oss"})
        self.assertEqual(self.cfg["context_tokens"], 32768)
        for model in self.cfg["models"].values():
            self.assertEqual(model["coding_qualification"], "not-qualified")

    def test_profile_rejects_activation_multiple_slots_and_unbounded_context(self):
        for field, value in (("product_enabled", True), ("parallel_slots", 2), ("threads", 8), ("context_tokens", 131072), ("output_tokens", 192)):
            cfg = copy.deepcopy(self.cfg)
            cfg[field] = value
            with self.subTest(field=field), patch.object(Path, "read_text", return_value=json.dumps(cfg)):
                with self.assertRaises(RuntimeError):
                    lab.load_config()

    def test_complete_artifact_requires_size_format_and_digest(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "model.gguf"
            data = b"GGUFsynthetic-model"
            path.write_bytes(data)
            expected = hashlib.sha256(data).hexdigest()
            lab.verify_file(path, len(data), expected)
            with self.assertRaises(RuntimeError):
                lab.verify_file(path, len(data) + 1, expected)
            with self.assertRaises(RuntimeError):
                lab.verify_file(path, len(data), "0" * 64)
            path.write_bytes(b"<html>error</html>")
            with self.assertRaises(RuntimeError):
                lab.verify_file(path, path.stat().st_size, lab.digest(path))

    def test_artifact_symlink_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "target"
            target.write_bytes(b"GGUFtest")
            link = Path(directory) / "link"
            link.symlink_to(target)
            with self.assertRaises(OSError):
                lab.verify_file(link, 8, lab.digest(target))

    def test_private_state_rejects_public_directory_and_symlink(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            public = root / "public"
            public.mkdir(mode=0o755)
            public.chmod(0o755)
            with self.assertRaises(RuntimeError):
                lab.private_directory(public)
            link = root / "link"
            link.symlink_to(root, target_is_directory=True)
            with self.assertRaises(RuntimeError):
                lab.private_directory(link)

    def test_promotion_is_verified_and_does_not_overwrite(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "quarantine").mkdir()
            partial = root / "quarantine/gpt-oss-20b-MXFP4.gguf.part"
            data = b"GGUFsynthetic-model"
            partial.write_bytes(data)
            destination = root / "models/model.gguf"
            cfg = {"data_root": str(root), "models": {"gpt-oss": {
                "path": str(destination), "size_bytes": len(data),
                "sha256": hashlib.sha256(data).hexdigest(),
            }}}
            lab.promote_download(cfg)
            self.assertFalse(partial.exists())
            self.assertEqual(destination.read_bytes(), data)
            self.assertEqual(destination.stat().st_mode & 0o777, 0o600)
            destination.write_bytes(b"GGUFother-artifact")
            with self.assertRaises(RuntimeError):
                lab.promote_download(cfg)
            self.assertEqual(destination.read_bytes(), b"GGUFother-artifact")

    def test_incomplete_download_is_not_promoted(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "quarantine").mkdir()
            partial = root / "quarantine/gpt-oss-20b-MXFP4.gguf.part"
            partial.write_bytes(b"GGUF")
            destination = root / "models/model.gguf"
            cfg = {"data_root": str(root), "models": {"gpt-oss": {
                "path": str(destination), "size_bytes": 100, "sha256": "0" * 64,
            }}}
            with self.assertRaises(RuntimeError):
                lab.promote_download(cfg)
            self.assertTrue(partial.exists())
            self.assertFalse(destination.exists())

    def test_sandbox_has_no_home_workspace_network_or_tools(self):
        for model_id in self.cfg["models"]:
            command = lab.sandbox_command(self.cfg, model_id, Path("/verified-runtime"), Path("/private-state"))
            self.assertIn("--unshare-all", command)
            self.assertIn("--clearenv", command)
            self.assertIn("--offline", command)
            self.assertIn("--no-agent", command)
            self.assertIn("--no-webui", command)
            self.assertNotIn("--tools", command)
            self.assertNotIn("--mmproj", command)
            self.assertNotIn("--api-key", command)
            self.assertNotIn("--model-url", command)
            for index, value in enumerate(command):
                if value in ("--bind", "--ro-bind"):
                    self.assertNotIn(command[index + 2], ("/", "/home", str(lab.ROOT)))
            self.assertEqual(command[command.index("--host") + 1], "/run/agentmage/model.sock")
            self.assertEqual(command[command.index("--parallel") + 1], "1")
            self.assertEqual(command[command.index("--ctx-size") + 1], "32768")
            self.assertEqual(command[command.index("--fit") + 1], "off")

    def test_resource_light_does_not_mean_reasoning_disabled(self):
        self.assertEqual(self.cfg["models"]["muse"]["chat_template_kwargs"], {"reasoning_strength": "medium"})
        self.assertEqual(self.cfg["models"]["gpt-oss"]["chat_template_kwargs"], {"reasoning_effort": "medium"})

    def test_length_terminated_output_cannot_pass(self):
        server = lab.ModelServer(self.cfg, "muse", Path("/unused"))
        for reason in ("length", None, "error"):
            with self.subTest(reason=reason), patch.object(server, "call", return_value={"choices": [{"finish_reason": reason}]}):
                with self.assertRaises(RuntimeError):
                    server.complete([])

    def test_evidence_write_does_not_overwrite_prior_results(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "result.json"
            lab.save_json(path, {"status": "failed"})
            with self.assertRaises(FileExistsError):
                lab.save_json(path, {"status": "passed"})
            self.assertEqual(json.loads(path.read_text()), {"status": "failed"})

    def test_scope_marker_does_not_bypass_resource_limits(self):
        limits = {"memory.max": str(6 * 1024**3), "memory.high": str(5 * 1024**3),
                  "memory.swap.max": str(512 * 1024**2), "memory.peak": "1", "cpu.max": "200000 100000"}
        with patch.dict(lab.os.environ, {"AGENTMAGE_MODEL_LAB_SCOPE": "1"}), patch.object(lab, "scope_resources", return_value=limits):
            lab.ensure_scope()
        for key, value in (("memory.max", "max"), ("memory.high", str(6 * 1024**3)),
                           ("memory.swap.max", "max"), ("cpu.max", "300000 100000")):
            bad = dict(limits, **{key: value})
            with self.subTest(key=key), patch.dict(lab.os.environ, {"AGENTMAGE_MODEL_LAB_SCOPE": "1"}), patch.object(lab, "scope_resources", return_value=bad):
                with self.assertRaises(RuntimeError):
                    lab.ensure_scope()

    def test_multiple_gpus_require_a_different_explicit_profile(self):
        with patch.object(lab.subprocess, "run") as run:
            run.return_value.stdout = "1000, 23000\n1000, 23000\n"
            with self.assertRaises(RuntimeError):
                lab.gpu_memory()

    def test_runtime_inventory_drift_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary = root / "llama-server"
            binary.write_bytes(b"original")
            manifest = {"runtime_root": str(root), "runtime_files_sha256": {"llama-server": lab.digest(binary)}, "runtime_symlinks": {}}
            with patch.object(Path, "read_text", return_value=json.dumps(manifest)):
                self.assertEqual(lab.verify_runtime(self.cfg), root)
                binary.write_bytes(b"modified")
                with self.assertRaises(RuntimeError):
                    lab.verify_runtime(self.cfg)
                (root / "extra-library").write_bytes(b"extra")
                with self.assertRaises(RuntimeError):
                    lab.verify_runtime(self.cfg)


if __name__ == "__main__":
    unittest.main()
