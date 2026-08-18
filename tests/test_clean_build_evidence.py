from __future__ import annotations

import copy
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

from scripts.clean_build_evidence import (
    EXPECTED_COMMANDS,
    EXPECTED_CONTROLS,
    EXPECTED_TOOLCHAINS,
    ROOT,
    build_report,
    check_report,
    container_run_argv,
    expected_toolchains_for_revision,
    git_source_identity,
    normalized_sha256_id,
    read_json,
    report_applicability,
    validate_policy,
    validate_report,
    working_tree_failures,
)
from scripts.clean_standard_build import make_tree_user_writable


def run_git(root: Path, *args: str) -> None:
    result = subprocess.run(
        ["git", *args],
        cwd=root,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise AssertionError(result.stderr)


def initialize_repository(root: Path, files: dict[str, str]) -> None:
    run_git(root, "init", "--quiet")
    run_git(root, "config", "user.name", "AgentMage Test")
    run_git(root, "config", "user.email", "agentmage-test@example.invalid")
    for relative, content in files.items():
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")
    run_git(root, "add", ".")
    run_git(root, "commit", "--quiet", "-m", "fixture")


class CleanBuildEvidenceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.policy = read_json(ROOT / "architecture/clean-build-policy.json")
        self.source = git_source_identity(ROOT, "HEAD")
        self.runs = {}
        for platform_id, platform in self.policy["linux_platforms"].items():
            self.runs[platform_id] = {
                "status": "pass",
                "platform": {
                    "architecture": "x86_64",
                    "id": platform_id,
                    "os_id": platform["os_id"],
                    "pretty_name": "synthetic fixture",
                    "version_id": platform["version_id"],
                },
                "execution": {
                    "cargo_incremental": "disabled",
                    "effective_gid": 10001,
                    "effective_uid": 10001,
                    "privileged": False,
                    "source_archive": "read-only",
                    "user_class": "standard-unprivileged",
                    "writable_storage": "fresh-temporary-filesystem",
                },
                "source": self.source,
                "toolchains": [
                    {
                        "executable_sha256": "c" * 64,
                        "id": toolchain,
                        "version": f"{toolchain} synthetic-version",
                    }
                    for toolchain in expected_toolchains_for_revision(ROOT, "HEAD")
                ],
                "commands": [
                    {"id": command, "status": "pass"}
                    for command in EXPECTED_COMMANDS
                ],
                "checks": {
                    "all_commands_passed": True,
                    "ambient_dependency_detected": False,
                    "clean_home": True,
                    "clean_npm_cache": True,
                    "clean_source_archive": True,
                    "clean_target": True,
                    "post_bootstrap_network_denied": True,
                    "source_content_verified_during_bootstrap": True,
                    "supply_chain_outputs_validated": True,
                },
                "macos_support_claim": "none",
                "base_image": platform["base_image"],
                "container_image_id": "sha256:" + "b" * 64,
                "container_controls": EXPECTED_CONTROLS,
            }
        self.report = build_report(self.runs, self.source)

    def test_canonical_policy_is_valid(self) -> None:
        self.assertEqual(validate_policy(self.policy), [])

    def test_only_locked_dependency_bootstraps_may_use_network(self) -> None:
        self.assertEqual(
            [(item["id"], item["argv"]) for item in self.policy["bootstrap_commands"]],
            [
                (
                    "npm-clean-install",
                    ["npm", "ci", "--ignore-scripts", "--no-audit", "--no-fund"],
                ),
                ("cargo-fetch", ["cargo", "fetch", "--locked"]),
            ],
        )
        self.assertTrue(
            all(
                item["network"] == "container-disabled"
                for item in self.policy["verification_commands"]
            )
        )

    def test_checked_in_current_report_remains_source_valid(self) -> None:
        self.assertEqual(check_report(), [])
        report = read_json(
            ROOT
            / "artifacts/sprints/sprint-1/story-1.1/clean-build-report.json"
        )
        self.assertEqual(report["schema_version"], 2)
        self.assertEqual(report["status"], "pass-linux")
        self.assertIn(
            report_applicability(report, report["source"]["revision"]),
            {"current-reviewed-source", "reviewed-source-dirty"},
        )

    def test_complete_linux_fixture_does_not_promote_macos(self) -> None:
        self.assertEqual(validate_report(self.report), [])
        self.assertEqual(self.report["status"], "pass-linux")
        self.assertTrue(self.report["summary"]["linux_scope_complete"])
        self.assertFalse(self.report["summary"]["cross_platform_task_complete"])

    def test_root_execution_missing_commands_and_toolchains_are_rejected(self) -> None:
        root = copy.deepcopy(self.report)
        root["platform_runs"]["fedora-x86_64"]["execution"]["effective_uid"] = 0
        command = copy.deepcopy(self.report)
        command["platform_runs"]["ubuntu-x86_64"]["commands"].pop()
        tool = copy.deepcopy(self.report)
        tool["platform_runs"]["fedora-x86_64"]["toolchains"].pop()
        self.assertTrue(validate_report(root))
        self.assertTrue(validate_report(command))
        self.assertTrue(validate_report(tool))

    def test_tree_archive_and_revision_mismatch_are_rejected(self) -> None:
        for key, value in (
            ("tree", "0" * 40),
            ("archive_sha256", "0" * 64),
            ("content_sha256", "0" * 64),
            ("revision", "0" * 40),
        ):
            mutated = copy.deepcopy(self.report)
            mutated["source"][key] = value
            for run in mutated["platform_runs"].values():
                run["source"] = mutated["source"]
            self.assertTrue(validate_report(mutated), key)

    def test_post_bootstrap_runtime_has_no_network(self) -> None:
        argv = container_run_argv("fixture", "fedora-x86_64", self.source)
        self.assertIn("--network=none", argv)
        self.assertIn("--memory=12884901888", argv)
        self.assertIn(
            "--tmpfs=/tmp:rw,exec,nosuid,nodev,size=8589934592",
            argv,
        )
        mutated = copy.deepcopy(self.report)
        mutated["platform_runs"]["fedora-x86_64"]["checks"][
            "post_bootstrap_network_denied"
        ] = False
        self.assertTrue(validate_report(mutated))

    def test_container_bootstrap_and_runtime_permission_order_is_fixed(self) -> None:
        recipe = (ROOT / "release/clean-build/Containerfile.linux").read_text()
        self.assertIn("dnf install -y bubblewrap ca-certificates curl gcc git ", recipe)
        self.assertIn("bubblewrap build-essential ca-certificates curl git ", recipe)
        self.assertIn("python3 shadow-utils systemd xz", recipe)
        self.assertIn("passwd python3 systemd xz-utils", recipe)
        ownership = recipe.index("chown -R 10001:10001 /opt/cargo")
        unprivileged_bootstrap = recipe.index("USER 10001:10001", ownership)
        source_verification = recipe.index("--verify-source-content", unprivileged_bootstrap)
        npm_bootstrap = recipe.index("npm ci --ignore-scripts", source_verification)
        lock_down = recipe.index("chmod -R a-w /opt/cargo /opt/rustup /source")
        self.assertLess(ownership, unprivileged_bootstrap)
        self.assertLess(unprivileged_bootstrap, source_verification)
        self.assertLess(source_verification, npm_bootstrap)
        self.assertLess(npm_bootstrap, lock_down)

    def test_git_runtime_dependency_is_declared_and_recorded(self) -> None:
        self.assertEqual(self.policy["toolchains"]["git"], "platform-packaged")
        self.assertIn("git", EXPECTED_TOOLCHAINS)
        mutated = copy.deepcopy(self.policy)
        mutated["toolchains"].pop("git")
        self.assertIn(
            "clean-build Git runtime dependency is not declared",
            validate_policy(mutated),
        )

    def test_linux_security_runtime_dependencies_are_closed(self) -> None:
        self.assertEqual(
            self.policy["runtime_dependencies"],
            {"platform_packaged": ["bubblewrap", "git", "systemd-run"]},
        )
        mutated = copy.deepcopy(self.policy)
        mutated["runtime_dependencies"]["platform_packaged"].remove("bubblewrap")
        self.assertIn(
            "clean-build platform runtime dependencies drifted",
            validate_policy(mutated),
        )

    def test_isolated_work_tree_can_be_made_writable(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            nested = root / "shells/vscode"
            nested.mkdir(parents=True)
            source = nested / "source.ts"
            source.write_text("export {};\n", encoding="utf-8")
            for path in (source, nested, nested.parent, root):
                path.chmod(path.stat().st_mode & ~0o222)
            make_tree_user_writable(root)
            output = nested / "dist"
            output.mkdir()
            (output / "index.js").write_text("export {};\n", encoding="utf-8")
            self.assertTrue((output / "index.js").is_file())

    def test_dirty_untracked_and_unapproved_ignored_sources_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            initialize_repository(root, {"tracked.rs": "fn main() {}\n"})
            (root / "tracked.rs").write_text("fn changed() {}\n", encoding="utf-8")
            (root / "untracked.rs").write_text("hidden\n", encoding="utf-8")
            failures = working_tree_failures(root, self.policy)
            self.assertIn("clean-build source worktree is dirty", failures)
            self.assertIn("clean-build source has an untracked path", failures)

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            initialize_repository(root, {"report.json": "{}\n", "source.rs": "ok\n"})
            (root / "report.json").write_text('{"status":"new"}\n', encoding="utf-8")
            self.assertEqual(
                working_tree_failures(
                    root,
                    self.policy,
                    allowed_generated_outputs=("report.json",),
                ),
                [],
            )
            (root / "source.rs").write_text("changed\n", encoding="utf-8")
            self.assertIn(
                "clean-build source worktree is dirty",
                working_tree_failures(
                    root,
                    self.policy,
                    allowed_generated_outputs=("report.json",),
                ),
            )

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            initialize_repository(root, {".gitignore": "hidden.rs\n", "main.rs": "ok\n"})
            (root / "hidden.rs").write_text("ignored product source\n", encoding="utf-8")
            self.assertIn(
                "clean-build source has an unapproved ignored path",
                working_tree_failures(root, self.policy),
            )

    def test_approved_transient_cache_is_excluded_but_lockfile_changes_bind(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            initialize_repository(
                root,
                {".gitignore": "target/\n", "Cargo.lock": "version = 1\n"},
            )
            before = git_source_identity(root, "HEAD")
            (root / "target").mkdir()
            (root / "target/cache").write_text("transient\n", encoding="utf-8")
            self.assertEqual(working_tree_failures(root, self.policy), [])
            (root / "Cargo.lock").write_text("version = 2\n", encoding="utf-8")
            run_git(root, "add", "Cargo.lock")
            run_git(root, "commit", "--quiet", "-m", "change lock")
            after = git_source_identity(root, "HEAD")
            self.assertNotEqual(before["tree"], after["tree"])
            self.assertNotEqual(before["archive_sha256"], after["archive_sha256"])
            self.assertNotEqual(before["content_sha256"], after["content_sha256"])

    def test_tracked_symlink_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            initialize_repository(root, {"target.txt": "value\n"})
            os.symlink("target.txt", root / "link.txt")
            run_git(root, "add", "link.txt")
            run_git(root, "commit", "--quiet", "-m", "add link")
            self.assertIn(
                "clean-build tracked tree contains a symbolic link",
                working_tree_failures(root, self.policy),
            )

    def test_network_policy_and_macos_promotion_are_rejected(self) -> None:
        policy = copy.deepcopy(self.policy)
        policy["verification_commands"][0]["network"] = "disabled-by-client"
        self.assertTrue(validate_policy(policy))
        policy = copy.deepcopy(self.policy)
        policy["bootstrap_commands"][1]["argv"].remove("--locked")
        self.assertTrue(validate_policy(policy))
        report = copy.deepcopy(self.report)
        report["macos"]["status"] = "pass"
        report["summary"]["cross_platform_task_complete"] = True
        self.assertTrue(validate_report(report))
        report = copy.deepcopy(self.report)
        report["platform_runs"]["ubuntu-x86_64"]["checks"][
            "supply_chain_outputs_validated"
        ] = False
        self.assertTrue(validate_report(report))

    def test_bare_podman_image_id_is_normalized(self) -> None:
        value = "c" * 64
        self.assertEqual(normalized_sha256_id(value), f"sha256:{value}")

    def test_non_sha256_image_id_is_rejected(self) -> None:
        with self.assertRaises(OSError):
            normalized_sha256_id("latest")


if __name__ == "__main__":
    unittest.main()
