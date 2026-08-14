from __future__ import annotations

import copy
import unittest

from scripts import muse_llama_runtime as runtime


class MuseLlamaRuntimeTests(unittest.TestCase):
    def test_exact_profile_is_closed_and_inactive(self) -> None:
        profile, digest = runtime.load_profile()
        self.assertEqual(len(digest), 64)
        self.assertEqual(profile["release"], "b10423")
        self.assertEqual(profile["decision"]["enabled_models"], 0)
        self.assertFalse(profile["decision"]["inference_implemented"])
        self.assertFalse(profile["decision"]["release_approval"])
        destinations = {record["destination"] for record in profile["source_files"]}
        self.assertEqual(destinations & {"ggml-rpc-server", "llama-cli", "llama-quantize"}, set())

    def test_authority_listener_identity_and_decision_widening_fail(self) -> None:
        profile, _ = runtime.load_profile()
        mutations = (
            lambda value: value["authority"].update({"workspace": True}),
            lambda value: value["authority"].update({"egress": True}),
            lambda value: value["listener"].update({"non_loopback_bind": True}),
            lambda value: value["decision"].update({"enabled_models": 1}),
            lambda value: value.update({"source_commit": "0" * 40}),
        )
        for mutate in mutations:
            changed = copy.deepcopy(profile)
            mutate(changed)
            with self.assertRaises(runtime.MuseRuntimeError):
                runtime.validate_profile(changed)

    def test_prohibited_entrypoints_unknown_fields_and_link_escape_fail(self) -> None:
        profile, _ = runtime.load_profile()

        changed = copy.deepcopy(profile)
        changed["source_files"][1]["destination"] = "bin/ggml-rpc-server"
        with self.assertRaises(runtime.MuseRuntimeError):
            runtime.validate_profile(changed)

        changed = copy.deepcopy(profile)
        changed["unknown"] = True
        with self.assertRaises(runtime.MuseRuntimeError):
            runtime.validate_profile(changed)

        changed = copy.deepcopy(profile)
        link = next(record for record in changed["source_files"] if record["type"] == "symlink")
        link["target"] = "../escape"
        with self.assertRaises(runtime.MuseRuntimeError):
            runtime.validate_profile(changed)

    def test_safe_path_rejects_absolute_parent_empty_and_backslash(self) -> None:
        for value in ("/absolute", "../parent", "a/../parent", "", "a\\b"):
            with self.assertRaises(runtime.MuseRuntimeError):
                runtime.safe_path(value)


if __name__ == "__main__":
    unittest.main()
