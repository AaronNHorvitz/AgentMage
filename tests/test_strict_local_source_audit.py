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

    def test_native_operation_control_cannot_be_removed_from_the_pinned_helper(self) -> None:
        original = audit.production_source(
            audit.MODEL_SOCKET_PATH, self.sources[audit.MODEL_SOCKET_PATH]
        )
        span = audit.model_socket_span(original)
        self.assertIsNotNone(span)
        start, end = span
        helper = original[start:end]
        mutations = (
            ("self.control = control;", "self.control = None;"),
            ("control.remaining_ms()", "Ok::<_, ModelOperationStop>(std::num::NonZeroU64::new(1).unwrap())"),
        )
        for before, after in mutations:
            with self.subTest(before=before):
                self.assertIn(before, helper)
                changed = original[:start] + helper.replace(before, after, 1) + original[end:]
                self.assertIsNone(audit.model_socket_span(changed))
                sources = dict(self.sources)
                sources[audit.MODEL_SOCKET_PATH] = changed
                self.assertIn(
                    f"pinned native Unix exchange changed or expanded: {audit.MODEL_SOCKET_PATH}",
                    audit.scan_sources(self.policy, sources),
                )

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

    def test_research_address_classification_does_not_admit_socket_or_dns_effects(self) -> None:
        path = "kernel/engine/src/research_budget.rs"
        self.assertIn(path, self.sources)
        self.assertIn("use std::net::IpAddr;", self.sources[path])
        for injected in (
            "use std::net::TcpStream;",
            "use std::net::TcpListener;",
            "use std::net::UdpSocket;",
            "use std::net::ToSocketAddrs;",
            "use ureq::Agent;",
        ):
            with self.subTest(injected=injected):
                sources = dict(self.sources)
                # Place the mutation before the test-only section so a production
                # network API cannot inherit the address-value exception.
                sources[path] = injected + "\n" + sources[path]
                self.assertIn(
                    f"rust-network-client-api found outside its closed allowlist: {path}",
                    audit.scan_sources(self.policy, sources),
                )

    def test_research_resolver_and_filter_only_admit_the_exact_value_import(self) -> None:
        for path, (declaration, package, module) in audit.RESEARCH_VALUE_IMPORTS.items():
            original = self.sources[path]
            self.assertIn(declaration, original)
            mutations = [
                original.replace(declaration, "", 1),
                declaration + "\n" + original,
                f"use {package}::{module} as hidden;\n" + original,
                f"use {package}::{{{module} as hidden}};\n" + original,
                f"use {package} as hidden;\n" + original,
                f"extern crate {package} as hidden;\n" + original,
                f"fn extra() {{ {package}::{module}::socket(); }}\n" + original,
            ]
            for changed in mutations:
                with self.subTest(path=path, mutation=changed[:100]):
                    self.assertNotEqual(changed, original)
                    sources = dict(self.sources)
                    sources[path] = changed
                    self.assertIn(f"research value-only network import expanded: {path}", audit.scan_sources(self.policy, sources))
            sources = dict(self.sources)
            sources[path] = "use std::net::TcpStream;\n" + original
            self.assertIn(f"rust-network-client-api found outside its closed allowlist: {path}", audit.scan_sources(self.policy, sources))
        sources = dict(self.sources)
        path = "platforms/linux/src/research_sandbox.rs"
        sources[path] = "use rustix::net::socket;\n" + sources[path]
        self.assertIn(f"rustix-socket-api found outside its closed allowlist: {path}", audit.scan_sources(self.policy, sources))

    def test_production_network_after_test_module_is_still_audited(self) -> None:
        path = "platforms/linux/src/research_sandbox.rs"
        sources = dict(self.sources)
        sources[path] += "\nfn bypass() { rustix::net::socket(); }\n"
        self.assertIn(
            f"rustix-socket-api found outside its closed allowlist: {path}",
            audit.scan_sources(self.policy, sources),
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
        activation = copy.deepcopy(manifest)
        activation["activationEvents"].append("onUri")
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
        contribution = copy.deepcopy(manifest)
        contribution["contributes"]["commands"].append(
            {"command": "agentmage.remote", "title": "Remote Agent"}
        )
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

    def test_research_worker_does_not_globally_admit_another_http_package(self) -> None:
        changed = set(self.policy["approved_cargo_packages"]) | {"reqwest@0.12.0"}
        with mock.patch.object(audit, "cargo_runtime_packages", return_value=changed):
            failures = audit.audit(self.policy, self.sources)
        self.assertIn("denied network-capable Rust dependency is present", failures)
        self.assertIn("reviewed Cargo package closure changed", failures)

    def test_research_transport_module_cannot_enter_default_library(self) -> None:
        changed = dict(self.sources)
        path = "platforms/linux/src/lib.rs"
        changed[path] += "\nmod public_research_transport;\n"
        self.assertIn(
            f"research worker module referenced outside its binary: {path}",
            audit.audit(self.policy, changed),
        )

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

    def test_worker_environment_canaries_are_test_only_not_a_production_exception(self) -> None:
        path = "platforms/linux/src/bin/agentmage-public-research-worker.rs"
        source = self.sources[path]
        self.assertIn("#[cfg(test)]\nmod tests {", source)
        self.assertIn("HTTPS_PROXY", source)
        self.assertNotIn("HTTPS_PROXY", audit.production_source(path, source))
        for prefix, diagnostic in (
            ('const PROXY: &str = "HTTPS_PROXY";\n', "ambient-proxy-or-dns-override found outside its closed allowlist"),
            ('const URI: &str = "https://proxy.example.com";\n', "undeclared external URI in product source"),
        ):
            changed = dict(self.sources)
            changed[path] = prefix + source
            self.assertIn(f"{diagnostic}: {path}", audit.scan_sources(self.policy, changed))

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


class NativeModelSocketAuditTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.policy = audit.load_policy()
        cls.original = (audit.ROOT / audit.MODEL_SOCKET_PATH).read_text(encoding="utf-8")
        cls.product = audit.production_source(audit.MODEL_SOCKET_PATH, cls.original)
        start = cls.product.index("mod socket_exchange {")
        end = cls.product.index("\n}\n\nuse owned_files", start) + 2
        cls.helper = cls.product[start:end]

    def assert_rejected(self, changed: str) -> None:
        self.assertNotEqual(changed, self.product)
        product = audit.production_source(audit.MODEL_SOCKET_PATH, changed)
        self.assertIsNone(audit.model_socket_span(product))
        self.assertIn(
            f"pinned native Unix exchange changed or expanded: {audit.MODEL_SOCKET_PATH}",
            audit.scan_sources(self.policy, {audit.MODEL_SOCKET_PATH: changed}),
        )

    def test_exact_private_helper_has_no_file_wide_socket_allowance(self) -> None:
        span = audit.model_socket_span(self.product)
        self.assertIsNotNone(span)
        self.assertEqual(self.product[slice(*span)], self.helper)
        rule = next(rule for rule in self.policy["symbol_rules"] if rule["id"] == "rustix-socket-api")
        self.assertNotIn(audit.MODEL_SOCKET_PATH, rule["allowed_paths"])
        self.assertIsNotNone(audit.model_socket_span("// { ignored }\n" + self.product))

    def test_helper_pin_rejects_socket_authority_and_control_changes(self) -> None:
        for before, after in (
            ("AddressFamily::UNIX", "AddressFamily::INET"),
            ("AddressFamily::UNIX", "AddressFamily::INET6"),
            ("SocketType::STREAM", "SocketType::DGRAM"),
            ("SocketType::STREAM", "SocketType::RAW"),
            ("SocketFlags::NONBLOCK | SocketFlags::CLOEXEC", "SocketFlags::CLOEXEC"),
            ("SocketFlags::NONBLOCK | SocketFlags::CLOEXEC", "SocketFlags::NONBLOCK"),
            ("SocketAddrUnix::new(path)", "SocketAddrUnix::new(other_path)"),
            ("self.check_stop()?;", "// removed stop check"),
            ("use rustix::event", "use rustix::net::socket;\n    use rustix::event"),
            ("connect(&socket, &address)", "connect(&socket, &other_address)"),
            ("pub(super) struct SocketExchange", "pub struct SocketExchange"),
        ):
            with self.subTest(after=after):
                changed = self.helper.replace(before, after, 1)
                self.assertNotEqual(changed, self.helper)
                self.assert_rejected(self.product.replace(self.helper, changed, 1))

    def test_namespace_cannot_expand_through_aliases_or_extra_calls(self) -> None:
        for injected in (
            "use rustix::net::socket;",
            "use ::rustix :: net :: socket;",
            "use rustix::{net as hidden};",
            "use ::rustix::{self as hidden};",
            "use rustix as hidden;",
            "extern crate rustix as hidden;",
            "use crate::llama_server_driver::socket_exchange as hidden;",
            "use socket_exchange::*;",
            "use socket_exchange::{SocketExchange as Hidden};",
            "fn extra() { rustix::net::socket(); }",
            "fn extra() { rustix /* comment */ :: net :: socket(); }",
            "use r#rustix::net::socket;",
            "use rustix::io::Errno;",
        ):
            with self.subTest(injected=injected):
                self.assert_rejected(injected + "\n" + self.product)
                # The existing test exclusion must retain production appended
                # after the actual test module, too.
                self.assert_rejected(self.original + "\n" + injected + "\n")

    def test_module_and_import_must_be_unique_private_top_level_code(self) -> None:
        for changed in (
            self.product.replace(self.helper, "", 1),
            self.helper + "\n" + self.product,
            self.product.replace(self.helper, "mod nested {\n" + self.helper + "\n}", 1),
            self.product.replace(self.helper, "macro_call!(\n" + self.helper + "\n);", 1),
            self.product.replace(self.helper, "/*\n" + self.helper + "\n*/", 1),
            self.product.replace(self.helper, 'const TEXT: &str = r###"\n' + self.helper + '\n"###;', 1),
            self.product.replace("mod socket_exchange {", "pub mod socket_exchange {", 1),
            self.product.replace("mod socket_exchange {", "pub\nmod socket_exchange {", 1),
            self.product.replace("mod socket_exchange {", "pub(crate)\nmod socket_exchange {", 1),
            self.product.replace(audit.MODEL_SOCKET_IMPORT, "", 1),
            self.product.replace(audit.MODEL_SOCKET_IMPORT, "// " + audit.MODEL_SOCKET_IMPORT, 1),
            self.product.replace(audit.MODEL_SOCKET_IMPORT, "pub\n" + audit.MODEL_SOCKET_IMPORT, 1),
            self.product.replace(audit.MODEL_SOCKET_IMPORT, "mod nested {\n" + audit.MODEL_SOCKET_IMPORT + "\n}", 1),
            audit.MODEL_SOCKET_IMPORT + "\n" + self.product,
            self.product + "\n/* unterminated",
            self.product + "\n}",
            self.product + "\nfn incomplete() {",
        ):
            with self.subTest(changed=changed[:80]):
                self.assert_rejected(changed)

    def test_pin_cannot_move_to_another_file_or_hide_other_network_detectors(self) -> None:
        foreign = "platforms/linux-inference/src/injected.rs"
        failures = audit.scan_sources(self.policy, {foreign: self.helper})
        self.assertIn(f"rustix-socket-api found outside its closed allowlist: {foreign}", failures)
        self.assertIn(f"pinned native Unix exchange is absent: {audit.MODEL_SOCKET_PATH}", failures)
        for injected, diagnostic in (
            ("use std::net::TcpStream;", "rust-network-client-api found outside its closed allowlist"),
            ('const URI: &str = "https://example.invalid";', "undeclared external URI in product source"),
        ):
            with self.subTest(injected=injected):
                changed = injected + "\n" + self.product
                self.assertIsNotNone(audit.model_socket_span(changed))
                self.assertIn(
                    f"{diagnostic}: {audit.MODEL_SOCKET_PATH}",
                    audit.scan_sources(self.policy, {audit.MODEL_SOCKET_PATH: changed}),
                )


if __name__ == "__main__":
    unittest.main()
