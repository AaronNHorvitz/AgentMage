import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from scripts.model_feasibility import (
    Completion,
    FeasibilityError,
    OpenAIAdapter,
    case_passed,
    compare_thresholds,
    container_network_snapshot,
    context_limit_trial,
    format_citation_case,
    format_repository_case,
    malformed_trial,
    namespace_interfaces,
    parse_json_object,
    score_chat,
    score_citation,
    score_repository,
    score_tool,
    split_http_url,
    verify_dmr_inputs,
    verify_native_inputs,
    validate_result_directory,
    write_results,
)


def completion(content="", *, tool_calls=None):
    message = {"role": "assistant", "content": content}
    if tool_calls is not None:
        message["tool_calls"] = tool_calls
    return Completion(
        response={
            "choices": [{"message": message}],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5},
            "timings": {"predicted_per_second": 20.0},
        },
        elapsed_seconds=0.5,
    )


class ModelFeasibilityTests(unittest.TestCase):
    def test_parse_json_object_accepts_plain_and_fenced_objects(self):
        self.assertEqual(parse_json_object('{"answer": 42}'), {"answer": 42})
        self.assertEqual(parse_json_object('```json\n{"answer": 42}\n```'), {"answer": 42})
        self.assertIsNone(parse_json_object("[42]"))
        self.assertIsNone(parse_json_object("{broken"))

    def test_chat_scoring_requires_exact_schema_and_values(self):
        case = {
            "expected": {
                "answer": 42,
                "evidence_state": "Derived",
                "uncertainty": "none",
                "no_extra_keys": True,
            }
        }
        accepted = score_chat(
            case,
            completion('{"answer":42,"evidence_state":"Derived","uncertainty":"none"}'),
        )
        rejected = score_chat(
            case,
            completion('{"answer":42,"evidence_state":"Derived","uncertainty":"none","extra":1}'),
        )
        self.assertTrue(accepted["exact"])
        self.assertFalse(rejected["schema_valid"])

    def test_repository_formatter_numbers_synthetic_files(self):
        case = {"input": {"files": {"src/lib.rs": "one\ntwo"}, "prompt": "Report."}}
        formatted = format_repository_case(case)
        self.assertIn("FILE src/lib.rs\n1: one\n2: two", formatted)
        self.assertTrue(formatted.endswith("Report."))

    def test_repository_scoring_requires_facts_citations_and_no_claimed_actions(self):
        case = {"expected": {"prohibited_claims": ["files were changed"]}}
        accepted = score_repository(
            case,
            completion("parse_record src/parser.rs:1; render_record src/lib.rs:1"),
        )
        rejected = score_repository(
            case,
            completion("parse_record src/parser.rs:1; render_record src/lib.rs:1; files were changed"),
        )
        self.assertEqual(accepted["fact_accuracy"], 1.0)
        self.assertEqual(accepted["citation_precision"], 1.0)
        self.assertFalse(accepted["unsupported_action"])
        self.assertTrue(rejected["unsupported_action"])

    def test_citation_formatter_and_scoring_are_evidence_bounded(self):
        case = {
            "input": {"evidence": [{"id": "E1", "text": "A"}], "prompt": "Compare."},
            "expected": {"required_citations": ["E1", "E2"]},
        }
        self.assertIn("[E1] A", format_citation_case(case))
        score = score_citation(case, completion("Adapter A has lower latency [E1] [E2]."))
        self.assertEqual(score["citation_precision"], 1.0)
        self.assertEqual(score["citation_recall"], 1.0)
        self.assertTrue(score["factual_accuracy"])

    def test_tool_scoring_rejects_changed_constant_argument(self):
        case = {
            "expected": {
                "tool_name": "search_workspace",
                "arguments": {"query": "parse_record", "root_id": "fixture-root"},
            }
        }
        valid_call = [{"function": {"name": "search_workspace", "arguments": json.dumps(case["expected"]["arguments"])}}]
        invalid_call = [{"function": {"name": "search_workspace", "arguments": '{"query":"parse_record","root_id":"fixture"}'}}]
        self.assertTrue(score_tool(case, completion(tool_calls=valid_call))["tool_call_valid"])
        self.assertFalse(score_tool(case, completion(tool_calls=invalid_call))["tool_call_valid"])

    def test_unsupported_tool_case_requires_visible_refusal(self):
        case = {"expected": {"tool_calls": [], "must_state_unavailable": True}}
        score = score_tool(case, completion("That operation is unavailable.", tool_calls=[]))
        self.assertTrue(score["tool_call_valid"])
        self.assertTrue(score["states_unavailable"])
        self.assertFalse(score["unsupported_action"])

    def test_malformed_output_and_context_overflow_do_not_reach_inference(self):
        malformed = malformed_trial(
            {"input": {"injected_model_output": "{broken", "recovery_attempts": 1}}
        )
        overflow = context_limit_trial({"input": {"generated_input_tokens": 8193}})
        self.assertFalse(malformed["malformed_output_accepted"])
        self.assertEqual(malformed["inference_request_count"], 0)
        self.assertEqual(overflow["inference_request_count"], 0)
        self.assertEqual(overflow["terminal_state"], "blocked_context_limit")

    def test_threshold_comparison_respects_upper_and_lower_bounds(self):
        observed = {
            "schema_valid_rate": 1.0,
            "unsupported_action_rate": 0.0,
            "cancellation_max_seconds": 1.0,
        }
        expected = {
            "schema_valid_rate": 1.0,
            "unsupported_action_rate": 0.0,
            "cancellation_max_seconds": 2.0,
        }
        result = compare_thresholds(observed, expected)
        self.assertTrue(all(item["passed"] for item in result.values()))
        observed["unsupported_action_rate"] = 0.1
        self.assertFalse(compare_thresholds(observed, expected)["unsupported_action_rate"]["passed"])

    def test_case_passed_is_fail_closed(self):
        case = {
            "case_id": "CHAT-001",
            "trials_expected": 1,
            "trials_completed": 1,
            "trials": [{"schema_valid": True, "exact": True}],
        }
        self.assertTrue(case_passed(case))
        case["trials"][0]["exact"] = False
        self.assertFalse(case_passed(case))

    def test_resource_and_network_cases_require_measured_outcomes(self):
        memory = {
            "case_id": "MEM-001",
            "trials_expected": 1,
            "trials_completed": 1,
            "trials": [{"swap_growth_bytes": 0, "unload_returns_to_bounded_baseline": True}],
        }
        network = {
            "case_id": "NET-001",
            "trials_expected": 1,
            "trials_completed": 1,
            "trials": [{
                "dns_queries": 0,
                "outbound_connection_attempts": 0,
                "egress_bytes": 0,
                "undeclared_listeners": 0,
            }],
        }
        self.assertTrue(case_passed(memory))
        self.assertTrue(case_passed(network))
        memory["trials"][0]["swap_growth_bytes"] = 1
        network["trials"][0]["dns_queries"] = 1
        self.assertFalse(case_passed(memory))
        self.assertFalse(case_passed(network))

    def test_native_input_verification_requires_exact_hash_and_size(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            paths = {}
            expected = {}
            for label in ("model", "projector", "runtime"):
                path = root / label
                path.write_bytes(label.encode())
                paths[label] = path
                expected[label] = __import__("hashlib").sha256(label.encode()).hexdigest()
            admission = root / "admission.json"
            admission.write_text(
                json.dumps(
                    {
                        "gguf_identity": {
                            "size": paths["model"].stat().st_size,
                            "sha256": expected["model"],
                            "multimodal_projector": {
                                "size": paths["projector"].stat().st_size,
                                "sha256": expected["projector"],
                            },
                        },
                        "native_runtime": {"llama_server_sha256": expected["runtime"]},
                    }
                ),
                encoding="utf-8",
            )
            identities = verify_native_inputs(
                admission, paths["runtime"], paths["model"], paths["projector"]
            )
            self.assertEqual(set(identities), {"model", "projector", "runtime"})
            paths["model"].write_bytes(b"changed")
            with self.assertRaises(FeasibilityError):
                verify_native_inputs(admission, paths["runtime"], paths["model"], paths["projector"])

    def test_result_manifest_hashes_every_emitted_file(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary)
            (output / "server.log").write_text("public synthetic log\n", encoding="utf-8")
            result = {"adapter_id": "linux-native-vulkan", "corpus_id": "corpus", "status": "FAIL"}
            write_results(output, result)
            manifest = json.loads((output / "manifest.json").read_text(encoding="utf-8"))
            self.assertEqual([item["path"] for item in manifest["files"]], ["results.json", "server.log"])
            self.assertTrue(all(len(item["sha256"]) == 64 for item in manifest["files"]))

    def test_result_directory_rejects_a_manifest_hash_substitution(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary)
            (output / "results.json").write_text("{}\n", encoding="utf-8")
            (output / "manifest.json").write_text(
                json.dumps({"files": [{"path": "results.json", "sha256": "0" * 64, "size_bytes": 3}]}),
                encoding="utf-8",
            )
            failures = validate_result_directory(output)
            self.assertTrue(any("hash mismatch" in failure for failure in failures))

    def test_http_endpoint_parser_is_loopback_http_only(self):
        self.assertEqual(
            split_http_url("http://127.0.0.1:18081/v1/chat/completions"),
            ("127.0.0.1", 18081, "/v1/chat/completions"),
        )
        with self.assertRaises(FeasibilityError):
            split_http_url("https://example.test/v1/chat/completions")

    def test_interface_inventory_uses_network_namespace_link_table(self):
        self.assertIn("lo", namespace_interfaces())

    @patch("scripts.model_feasibility.unix_http_request")
    def test_usage_probe_token_counter_is_cached_and_uses_fixed_decoder(self, request_mock):
        adapter = OpenAIAdapter(
            None,
            "ai/gemma4:e4b",
            socket_path=Path("/tmp/model-runner.sock"),
            path_prefix="/engines/llama.cpp",
            usage_token_counter=True,
        )
        request_mock.return_value = (
            200,
            json.dumps({"usage": {"prompt_tokens": 37}}).encode(),
            0.1,
        )
        messages = [{"role": "user", "content": "count me"}]
        self.assertEqual(adapter.tokenize_messages(messages), 37)
        self.assertEqual(adapter.tokenize_messages(messages), 37)
        request_mock.assert_called_once()
        payload = request_mock.call_args.args[3]
        self.assertEqual(payload["max_tokens"], 1)
        self.assertEqual(payload["top_k"], 1)
        self.assertEqual(payload["seed"], 4242)
        self.assertIs(payload["cache_prompt"], False)

    @patch("scripts.model_feasibility.unix_http_request")
    def test_usage_probe_accepts_exact_count_from_context_error(self, request_mock):
        request_mock.return_value = (
            400,
            json.dumps(
                {
                    "error": {
                        "type": "exceed_context_size_error",
                        "n_prompt_tokens": 24930,
                    }
                }
            ).encode(),
            0.1,
        )
        adapter = OpenAIAdapter(
            None,
            "ai/gemma4:e4b",
            socket_path=Path("/tmp/model-runner.sock"),
            path_prefix="/engines/llama.cpp",
            usage_token_counter=True,
        )
        self.assertEqual(
            adapter.tokenize_messages([{"role": "user", "content": "too long"}]),
            24930,
        )

    @patch("scripts.model_feasibility.command_text")
    def test_container_network_snapshot_requires_only_loopback(self, command_text_mock):
        command_text_mock.side_effect = [
            "Inter-| Receive | Transmit\n face |bytes\n lo: 10 1 0 0 0 0 0 0 12 1 0 0 0 0 0 0\n",
            "Iface Destination Gateway Flags\n",
            "sl local_address rem_address st\n",
            "sl local_address rem_address st\n",
        ]
        snapshot = container_network_snapshot("podman", "fixture", "loaded")
        self.assertTrue(snapshot["isolated"])
        self.assertEqual(snapshot["interfaces"]["lo"]["rx_bytes"], 10)
        self.assertTrue(all(value == 0 for value in snapshot["counters"].values()))

    @patch("scripts.model_feasibility.command_text")
    @patch("scripts.model_feasibility.command_json")
    def test_dmr_input_verification_binds_artifacts_and_container_policy(
        self, command_json_mock, command_text_mock
    ):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            artifacts = {}
            hashes = {}
            for name in ("model", "projector", "manifest", "config"):
                path = root / name
                path.write_bytes(name.encode())
                artifacts[name] = path
                hashes[name] = __import__("hashlib").sha256(name.encode()).hexdigest()
            admission = root / "admission.json"
            admission.write_text(
                json.dumps(
                    {
                        "gguf_identity": {
                            "size": artifacts["model"].stat().st_size,
                            "sha256": hashes["model"],
                            "multimodal_projector": {
                                "size": artifacts["projector"].stat().st_size,
                                "sha256": hashes["projector"],
                            },
                        },
                        "docker_engine": {"digest": "sha256:runtime"},
                        "docker_model": {
                            "digest": "sha256:" + hashes["manifest"],
                            "config_digest": "sha256:" + hashes["config"],
                        },
                    }
                ),
                encoding="utf-8",
            )
            container = {
                "ImageDigest": "sha256:runtime",
                "ImageName": "fixture@sha256:runtime",
                "State": {"Running": True, "Pid": 123},
                "Config": {"User": "modelrunner"},
                "HostConfig": {
                    "NetworkMode": "none",
                    "Privileged": False,
                    "SecurityOpt": ["no-new-privileges"],
                    "CapAdd": [],
                    "PortBindings": {},
                },
            }
            image = {
                "Digest": "sha256:runtime",
                "Labels": {
                    "org.opencontainers.image.revision": "revision",
                    "org.opencontainers.image.version": "version",
                },
            }
            command_json_mock.side_effect = [[container], [image]]
            command_text_mock.return_value = "CapEff:\t0000000000000000\n"
            identities, environment = verify_dmr_inputs(
                admission,
                "podman",
                "fixture",
                artifacts["model"],
                artifacts["projector"],
                artifacts["manifest"],
                artifacts["config"],
            )
            self.assertEqual(identities["runtime_image"], "sha256:runtime")
            self.assertEqual(environment["network_mode"], "none")
            container["HostConfig"]["NetworkMode"] = "bridge"
            command_json_mock.side_effect = [[container], [image]]
            with self.assertRaisesRegex(FeasibilityError, "network mode is not none"):
                verify_dmr_inputs(
                    admission,
                    "podman",
                    "fixture",
                    artifacts["model"],
                    artifacts["projector"],
                    artifacts["manifest"],
                    artifacts["config"],
                )


if __name__ == "__main__":
    unittest.main()
