from __future__ import annotations

import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from scripts.macos_model_feasibility import (
    ADAPTER_ID,
    DEFAULT_MODEL_ADMISSION,
    DEFAULT_RUNTIME_ADMISSION,
    FeasibilityError,
    hardware_manifest,
    network_evidence,
    parse_interface_bytes,
    parse_interface_state,
    parse_vm_stat,
    process_tree_pids,
    validate_result,
    verify_file,
    verify_server_listener,
    write_results,
)
from scripts.macos_runtime_admission import load_record as load_runtime_record
from scripts.model_corpus import load_corpus


ROOT = Path(__file__).resolve().parents[1]


class MacosModelFeasibilityTests(unittest.TestCase):
    def test_file_identity_requires_exact_size_and_hash(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            artifact = Path(temporary) / "artifact"
            artifact.write_bytes(b"exact")
            identity = {
                "size_bytes": 5,
                "sha256": hashlib.sha256(b"exact").hexdigest(),
            }
            self.assertEqual(verify_file(artifact, identity, "fixture"), identity["sha256"])
            artifact.write_bytes(b"changed")
            with self.assertRaisesRegex(FeasibilityError, "size"):
                verify_file(artifact, identity, "fixture")

    def test_vm_stat_parser_counts_available_page_classes(self) -> None:
        raw = """Mach Virtual Memory Statistics: (page size of 16384 bytes)
Pages free:                               10.
Pages inactive:                           20.
Pages speculative:                         3.
Pages purgeable:                           2.
"""
        self.assertEqual(parse_vm_stat(raw), (16384, 35))
        with self.assertRaisesRegex(FeasibilityError, "page size"):
            parse_vm_stat("Pages free: 1.\n")

    def test_process_tree_uses_only_declared_descendants(self) -> None:
        table = {10: (1, 100), 11: (10, 200), 12: (11, 300), 99: (1, 400)}
        self.assertEqual(process_tree_pids(10, table), [10, 11, 12])

    def test_interface_parsers_expose_active_paths_and_byte_counters(self) -> None:
        state = parse_interface_state(
            """lo0: flags=8049<UP,LOOPBACK,RUNNING,MULTICAST> mtu 16384
\tinet 127.0.0.1 netmask 0xff000000
en0: flags=8863<UP,BROADCAST,SMART,RUNNING> mtu 1500
\tinet 192.0.2.10 netmask 0xffffff00
\tstatus: active
en1: flags=8822<BROADCAST,SMART> mtu 1500
\tstatus: inactive
"""
        )
        self.assertEqual(state["en0"]["status"], "active")
        self.assertEqual(state["en0"]["addresses"], ["192.0.2.10"])
        counters = parse_interface_bytes(
            """Name Mtu Network Address Ipkts Ierrs Ibytes Opkts Oerrs Obytes Coll
en0 1500 <Link#4> aa:bb 10 0 1000 12 0 2000 0
en0 1500 192.0.2 192.0.2.10 8 0 900 9 0 1800 0
"""
        )
        self.assertEqual(counters["en0"], {"rx_bytes": 1000, "tx_bytes": 2000})

    def test_network_evidence_fails_closed_on_active_path_or_bytes(self) -> None:
        base = {
            "phase": "startup",
            "isolated": True,
            "interface_bytes": {"lo0": {"rx_bytes": 10, "tx_bytes": 10}},
        }
        isolated = network_evidence([base, {**base, "phase": "loaded"}, {**base, "phase": "shutdown"}])
        self.assertTrue(isolated["isolated"])
        self.assertEqual(isolated["counters"]["agentmage_egress_v4_bytes"], 0)

        active = copy.deepcopy(base)
        active["phase"] = "shutdown"
        active["isolated"] = False
        active["interface_bytes"]["en0"] = {"rx_bytes": 20, "tx_bytes": 30}
        failed = network_evidence([base, {**base, "phase": "loaded"}, active])
        self.assertFalse(failed["isolated"])
        self.assertEqual(failed["counters"]["agentmage_egress_v4_bytes"], 30)

    def test_listener_must_be_exact_loopback_endpoint(self) -> None:
        verify_server_listener({"server_listeners": ["127.0.0.1:18082"]}, 18082)
        with self.assertRaisesRegex(FeasibilityError, "exclusively on loopback"):
            verify_server_listener({"server_listeners": ["*:18082"]}, 18082)

    def test_non_macos_host_is_rejected_before_evaluation(self) -> None:
        with patch("scripts.macos_model_feasibility.platform.system", return_value="Linux"):
            with self.assertRaisesRegex(FeasibilityError, "Darwin on arm64"):
                hardware_manifest()

    @patch("scripts.macos_model_feasibility.committed_file")
    def test_mac_result_validator_recomputes_identity_cases_metrics_and_status(
        self, committed_file_mock
    ) -> None:
        result = json.loads(
            (ROOT / "artifacts/sprints/sprint-0/story-0.3/native-linux-result.json").read_text(
                encoding="utf-8"
            )
        )
        runner = (ROOT / "scripts/macos_model_feasibility.py").read_bytes()
        committed_file_mock.return_value = runner
        corpus = load_corpus()
        model_admission = json.loads(DEFAULT_MODEL_ADMISSION.read_text(encoding="utf-8"))
        runtime_admission = load_runtime_record(DEFAULT_RUNTIME_ADMISSION)
        critical = runtime_admission["critical_files"]
        result.update(
            {
                "adapter_id": ADAPTER_ID,
                "runner_transform_version": "1.0.0",
                "runner_sha256": hashlib.sha256(runner).hexdigest(),
                "source_revision": "0" * 40,
                "identities": {
                    "model": model_admission["gguf_identity"]["sha256"],
                    "projector": model_admission["gguf_identity"]["multimodal_projector"]["sha256"],
                    "runtime_archive": runtime_admission["artifact"]["sha256"],
                    "runtime": critical["llama_server"]["sha256"],
                    "metal_library": critical["libggml_metal"]["sha256"],
                },
                "runtime_settings": {
                    "context_tokens": corpus["decoder"]["operational_context_tokens"],
                    "gpu_layers": "all",
                    "device": "Metal",
                    "parallel_slots": 1,
                    "batch_size": 2048,
                    "micro_batch_size": 512,
                    "offline": True,
                    "host": "127.0.0.1",
                    "port": 18082,
                    "decoder": corpus["decoder"],
                    "hardware": {
                        "platform": "macOS",
                        "architecture": "arm64",
                        "machine_name": "MacBook Pro",
                        "chip": "Apple M5",
                    },
                },
            }
        )
        self.assertEqual(
            validate_result(result, corpus, model_admission, runtime_admission),
            [],
        )
        result["runtime_settings"]["hardware"]["chip"] = "Apple M4"
        self.assertTrue(
            any(
                "required MacBook Pro M5" in failure
                for failure in validate_result(result, corpus, model_admission, runtime_admission)
            )
        )

    def test_result_writer_refuses_preexisting_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary)
            (output / "foreign.txt").write_text("do not overwrite", encoding="utf-8")
            with self.assertRaisesRegex(FeasibilityError, "unexpected files"):
                write_results(output, {"corpus_id": "fixture"})


if __name__ == "__main__":
    unittest.main()
