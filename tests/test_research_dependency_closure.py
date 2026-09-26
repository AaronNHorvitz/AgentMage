"""Adversarial fixtures for the optional worker's separate production closures."""

from __future__ import annotations

import copy
import hashlib
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from scripts import research_dependency_closure as closure


def manifest() -> dict:
    return {
        "features": {"test-support": [], closure.FEATURE: ["dep:ureq", "dep:url"]},
        "dependencies": copy.deepcopy(closure.DEPENDENCIES),
        "bin": [{"name": closure.WORKER_NAME, "path": closure.WORKER_PATH,
                 "required-features": [closure.FEATURE]}],
    }


def policy() -> dict:
    return {
        "schema_version": 1, "target": closure.TARGET, "feature": closure.FEATURE,
        "lock_sha256": hashlib.sha256(b"synthetic lock").hexdigest(),
        "default": [{"package": "agentmage-platform-linux@0.0.0", "features": []}],
        "connected": [
            {"package": "agentmage-platform-linux@0.0.0", "features": [closure.FEATURE]},
            {"package": "ureq@3.4.2", "features": ["rustls"]},
            {"package": "url@2.5.8", "features": ["std"]},
        ],
    }


class ResearchDependencyClosureTests(unittest.TestCase):
    def test_exact_optional_topology(self) -> None:
        self.assertEqual(closure.validate_topology(manifest()), [])

    def test_worker_source_is_not_importable_from_the_ordinary_library(self) -> None:
        sources = {closure.WORKER_SOURCE: '\n'.join(
            f'#[path = "../{module}.rs"]' for module in closure.WORKER_MODULES)}
        self.assertEqual(closure.validate_worker_sources(sources), [])
        for path in ("platforms/linux/src/lib.rs", "shells/host/src/main.rs"):
            changed = dict(sources)
            changed[path] = "mod public_research_transport;"
            self.assertTrue(closure.validate_worker_sources(changed))
        self.assertTrue(closure.validate_worker_sources({}))

    def test_current_locked_inventory_matches_both_actual_production_graphs(self) -> None:
        self.assertEqual(closure.check(), [])

    def test_default_alias_or_weak_dependency_activation_refused(self) -> None:
        for activation in (closure.FEATURE, "dep:ureq", "ureq/rustls", "ureq?/cookies",
                           "url", "url/std"):
            changed = manifest()
            changed["features"]["default"] = [activation]
            with self.subTest(activation=activation):
                self.assertTrue(closure.validate_topology(changed))

    def test_dependency_versions_defaults_features_and_optionality_are_exact(self) -> None:
        for key, value in (("version", "3"), ("default-features", True),
                           ("features", ["rustls", "cookies"]), ("optional", False)):
            changed = manifest()
            changed["dependencies"]["ureq"][key] = value
            self.assertTrue(closure.validate_topology(changed))
        changed = manifest()
        changed["dependencies"]["other-http"] = changed["dependencies"].pop("ureq")
        self.assertTrue(closure.validate_topology(changed))

    def test_duplicate_dependency_paths_refused(self) -> None:
        for table in ("dev-dependencies", "build-dependencies", "dependencies"):
            changed = manifest()
            changed["target"] = {"cfg(unix)": {table: {"ureq": "3.4.2"}}}
            self.assertTrue(closure.validate_topology(changed))
        changed = manifest()
        changed["dev-dependencies"] = {"ureq": "3.4.2"}
        self.assertTrue(closure.validate_topology(changed))

    def test_binary_gate_cannot_be_removed_renamed_or_duplicated(self) -> None:
        for key, value in (("required-features", []), ("name", "alias-worker"),
                           ("path", "src/main.rs")):
            changed = manifest()
            changed["bin"][0][key] = value
            self.assertTrue(closure.validate_topology(changed))
        changed = manifest()
        changed["bin"].append(copy.deepcopy(changed["bin"][0]))
        self.assertTrue(closure.validate_topology(changed))

    def test_closed_policy_and_feature_contexts(self) -> None:
        closure.validate_policy(policy())
        for version in (True, False, "1", 1.0, None):
            changed = policy()
            changed["schema_version"] = version
            with self.subTest(version=version), self.assertRaises(closure.ResearchClosureError):
                closure.validate_policy(changed)
        for mutation in range(8):
            changed = policy()
            if mutation == 0:
                changed["target"] = "another-target"
            elif mutation == 1:
                changed["unchecked"] = True
            elif mutation == 2:
                changed["default"] = []
            elif mutation == 3:
                changed["connected"].append(changed["connected"][0])
            elif mutation == 4:
                changed["default"][0]["features"] = ["a", "a"]
            elif mutation == 5:
                changed["default"][0]["package"] = "package-without-version"
            elif mutation == 6:
                changed["lock_sha256"] = "unbound"
            else:
                changed["connected"].reverse()
            with self.subTest(mutation=mutation), self.assertRaises(closure.ResearchClosureError):
                closure.validate_policy(changed)

    def test_actual_graph_is_required_even_when_inventory_is_self_consistent(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "Cargo.lock").write_bytes(b"synthetic lock")
            path = root / closure.MANIFEST_PATH
            path.parent.mkdir(parents=True)
            path.write_text("# topology supplied independently by fixture")
            expected = policy()
            observations = {name: expected[name] for name in ("default", "connected")}
            with mock.patch.object(closure, "validate_topology", return_value=[]):
                self.assertEqual(closure.check(root, policy=expected, observations=observations), [])
                changed = copy.deepcopy(observations)
                changed["default"] += [{"package": "ureq@3.4.2", "features": ["cookies"]}]
                self.assertIn("research default production closure changed",
                              closure.check(root, policy=expected, observations=changed))
                widened = copy.deepcopy(expected)
                widened["default"] = widened["connected"]
                observed = {name: widened[name] for name in ("default", "connected")}
                self.assertIn("default production graph contains the research worker",
                              closure.check(root, policy=widened, observations=observed))
                (root / "Cargo.lock").write_bytes(b"changed checksum, same package names")
                self.assertIn("research dependency complete lockfile binding changed",
                              closure.check(root, policy=expected, observations=observations))

    def test_graph_parser_preserves_distinct_build_and_target_feature_contexts(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "Cargo.toml").write_text('[workspace]\nmembers = ["local"]\n')
            (root / "local").mkdir()
            (root / "local/Cargo.toml").write_text('[package]\nname="local-package"\n')
            valid = (f"local-package v0.0.0 ({root / 'local'})||\n"
                     "syn v2.0.1||full\nsyn v2.0.1||derive,full\n"
                     "syn v2.0.1||full\nmacro v1.0.0 (proc-macro)||\n")
            parsed = closure.normalize_tree(valid, root)
            self.assertEqual(len(parsed), 4)
            self.assertEqual([r["features"] for r in parsed if r["package"] == "syn@2.0.1"],
                             [["derive", "full"], ["full"]])
            self.assertNotIn(str(root), str(parsed))
            for invalid in ("syn v2.0.1||full (*)", "syn v2.0.1 (https://example.test)||",
                            "syn v2.0.1||full,derive", "", "unexpected output",
                            "syn v2.0.1||full,full", "syn v2.0.1||http://example.test"):
                with self.subTest(invalid=invalid), self.assertRaises(closure.ResearchClosureError):
                    closure.normalize_tree(invalid, root)


if __name__ == "__main__":
    unittest.main()
