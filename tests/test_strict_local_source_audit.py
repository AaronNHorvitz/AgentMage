"""Mutation tests for the strict-local product-source closure."""

from __future__ import annotations

import copy
import json
import unittest
from unittest import mock

from scripts import strict_local_source_audit as audit


class StrictLocalSourceAuditTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.policy = audit.load_policy()
        cls.sources = audit.source_map(cls.policy)

    def test_current_source_closure_passes(self) -> None:
        self.assertEqual(audit.audit(self.policy, self.sources), [])

    def test_each_hidden_remote_feature_fails(self) -> None:
        mutations = {
            "telemetry": "fetch(telemetryDestination);",
            "crash-upload": "node:https",
            "remote-font": "@import url('https://fonts.example.test/style.css');",
            "marketplace": "new WebSocket(marketplaceEndpoint);",
            "update": "Command::new(\"curl\")",
            "proxy": "const proxy = HTTP_PROXY;",
            "vscode-external": "vscode.env.openExternal(candidate);",
            "child-download": "spawn('wget', arguments);",
        }
        for identifier, injected in mutations.items():
            with self.subTest(identifier=identifier):
                sources = dict(self.sources)
                sources["shells/vscode/src/injected.ts"] = injected
                self.assertTrue(audit.scan_sources(self.policy, sources))

    def test_linux_inference_is_inside_the_closed_source_boundary(self) -> None:
        self.assertIn(
            "platforms/linux-inference/src/docker_guard_service.rs", self.sources
        )
        self.assertEqual(audit.scan_sources(self.policy, self.sources), [])

        sources = dict(self.sources)
        sources["platforms/linux-inference/src/native_runtime.rs"] = (
            "fn hidden_remote() { let _ = std::net::TcpStream::connect(\"example.invalid:443\"); }\n"
            + sources["platforms/linux-inference/src/native_runtime.rs"]
        )
        failures = audit.scan_sources(self.policy, sources)
        self.assertIn(
            "rust-standard-internet-address-api found outside its closed allowlist: platforms/linux-inference/src/native_runtime.rs",
            failures,
        )
        self.assertIn(
            "rust-network-client-api found outside its closed allowlist: platforms/linux-inference/src/native_runtime.rs",
            failures,
        )

    def test_compiled_vscode_artifact_is_inside_the_closed_source_boundary(self) -> None:
        self.assertIn("shells/vscode/dist/src/host_bridge.js", self.sources)
        sources = dict(self.sources)
        sources["shells/vscode/dist/src/host_bridge.js"] += (
            '\nfetch("https://telemetry.example.invalid");\n'
        )
        failures = audit.scan_sources(self.policy, sources)
        self.assertIn(
            "undeclared external URI in product source: shells/vscode/dist/src/host_bridge.js",
            failures,
        )
        self.assertIn(
            "javascript-network-api found outside its closed allowlist: shells/vscode/dist/src/host_bridge.js",
            failures,
        )

    def test_vscode_manifest_network_surfaces_are_closed(self) -> None:
        manifest = json.loads(
            (audit.ROOT / "shells/vscode/package.json").read_text(encoding="utf-8")
        )
        self.assertEqual(audit.audit_vscode_manifest(self.policy, manifest, set()), [])

        extension_dependency = copy.deepcopy(manifest)
        extension_dependency["extensionDependencies"] = ["publisher.remote-helper"]
        self.assertIn(
            "VS Code manifest surface changed",
            audit.audit_vscode_manifest(self.policy, extension_dependency, set()),
        )
        activation = copy.deepcopy(manifest)
        activation["activationEvents"] = ["onUri"]
        self.assertIn(
            "VS Code activation events changed",
            audit.audit_vscode_manifest(self.policy, activation, set()),
        )
        install_script = copy.deepcopy(manifest)
        install_script["scripts"]["postinstall"] = "node download-runtime.mjs"
        self.assertIn(
            "VS Code package scripts changed",
            audit.audit_vscode_manifest(self.policy, install_script, set()),
        )
        contribution = copy.deepcopy(manifest)
        contribution["contributes"]["commands"] = [{"command": "agentmage.update"}]
        self.assertIn(
            "VS Code contribution surface changed",
            audit.audit_vscode_manifest(self.policy, contribution, set()),
        )
        self.assertIn(
            "VS Code runtime package closure changed",
            audit.audit_vscode_manifest(self.policy, manifest, {"hidden-runtime"}),
        )

    def test_cargo_package_closure_requires_explicit_review(self) -> None:
        changed = set(self.policy["approved_cargo_packages"])
        changed.add("hidden-network-package@1.0.0")
        with mock.patch.object(audit, "cargo_runtime_packages", return_value=changed):
            failures = audit.audit(self.policy, self.sources)
        self.assertIn("reviewed Cargo package closure changed", failures)

    def test_cargo_manifest_features_and_build_scripts_require_review(self) -> None:
        changed = {
            "platforms/linux/Cargo.toml": (
                audit.ROOT / "platforms/linux/Cargo.toml"
            ).read_bytes()
            + b'\nnetwork-client = "1"\n'
        }
        self.assertIn(
            "reviewed Cargo manifest changed: platforms/linux/Cargo.toml",
            audit.audit_cargo_manifests(
                self.policy,
                content_overrides=changed,
                observed_build_scripts=[],
            ),
        )
        self.assertIn(
            "first-party Cargo build-script surface changed",
            audit.audit_cargo_manifests(
                self.policy,
                observed_build_scripts=["platforms/linux/build.rs"],
            ),
        )

    def test_internet_api_allowlist_cannot_move_or_expand(self) -> None:
        moved = dict(self.sources)
        moved["kernel/engine/src/injected.rs"] = "use std::net::IpAddr;"
        self.assertIn(
            "rust-standard-internet-address-api found outside its closed allowlist: kernel/engine/src/injected.rs",
            audit.scan_sources(self.policy, moved),
        )
        expanded = dict(self.sources)
        expanded["shells/vscode/src/host_bridge.ts"] += '\nimport "node:https";\n'
        self.assertIn(
            "javascript-network-api found outside its closed allowlist: shells/vscode/src/host_bridge.ts",
            audit.scan_sources(self.policy, expanded),
        )

        added = dict(self.sources)
        added["shells/vscode/src/host_bridge.ts"] += (
            '\ncreateConnection({ host: "127.0.0.1", port: 12434 });\n'
        )
        self.assertIn(
            "exact source fragment count changed: vscode-host-bridge-one-connection-call",
            audit.scan_sources(self.policy, added),
        )

        substituted = dict(self.sources)
        substituted["shells/vscode/src/host_bridge.ts"] = substituted[
            "shells/vscode/src/host_bridge.ts"
        ].replace(
            "createConnection({ path: this.endpoint })",
            'createConnection({ host: "127.0.0.1", port: 12434 })',
        )
        self.assertIn(
            "exact source fragment count changed: vscode-host-bridge-unix-path-only",
            audit.scan_sources(self.policy, substituted),
        )

        changed = dict(self.sources)
        changed["shells/vscode/src/host_bridge.ts"] = changed[
            "shells/vscode/src/host_bridge.ts"
        ].replace(
            "this.endpoint = credentials.endpoint;",
            "this.endpoint = process.env.AGENTMAGE_ENDPOINT ?? credentials.endpoint;",
        )
        self.assertIn(
            "exact source fragment count changed: vscode-host-bridge-package-endpoint-only",
            audit.scan_sources(self.policy, changed),
        )

    def test_terminal_rust_unit_tests_are_not_product_source(self) -> None:
        test_only = dict(self.sources)
        test_only["shells/host/src/test_fixture.rs"] = (
            "pub fn product() {}\n"
            "#[cfg(test)]\nmod tests {\n"
            "    use std::os::unix::net::UnixStream;\n"
            '    const FIXTURE: &str = "https://test-only.example.invalid";\n'
            "}\n"
        )
        self.assertEqual(audit.scan_sources(self.policy, test_only), [])

        production = dict(test_only)
        production["shells/host/src/test_fixture.rs"] = (
            "use std::os::unix::net::UnixStream;\n"
            + production["shells/host/src/test_fixture.rs"]
        )
        self.assertIn(
            "unix-domain-ipc-api found outside its closed allowlist: shells/host/src/test_fixture.rs",
            audit.scan_sources(self.policy, production),
        )

        production_uri = dict(test_only)
        production_uri["shells/host/src/test_fixture.rs"] = (
            'const REMOTE: &str = "https://product.example.invalid";\n'
            + production_uri["shells/host/src/test_fixture.rs"]
        )
        self.assertIn(
            "undeclared external URI in product source: shells/host/src/test_fixture.rs",
            audit.scan_sources(self.policy, production_uri),
        )

    def test_uri_match_excludes_an_escaped_string_delimiter(self) -> None:
        matches = audit.URI.findall(r'\"https://example.invalid/schema\"')
        self.assertEqual(matches, ["https://example.invalid/schema"])

    def test_uri_allowance_is_exact_and_staleness_is_a_failure(self) -> None:
        sources = dict(self.sources)
        path = "capabilities/read-only/src/catalog.rs"
        sources[path] = sources[path].replace(
            "#[cfg(test)]",
            '"https://second.example.test/schema";\n#[cfg(test)]',
            1,
        )
        self.assertIn(
            f"undeclared external URI in product source: {path}",
            audit.scan_sources(self.policy, sources),
        )
        removed = dict(self.sources)
        removed[path] = removed[path].replace(
            "https://json-schema.org/draft/2020-12/schema", "schema-without-uri"
        )
        self.assertIn(
            f"URI allowance is stale or incomplete: {path}",
            audit.scan_sources(self.policy, removed),
        )


if __name__ == "__main__":
    unittest.main()
