"""Mutation tests for the strict-local product-source closure."""

from __future__ import annotations

import unittest

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

    def test_uri_allowance_is_exact_and_staleness_is_a_failure(self) -> None:
        sources = dict(self.sources)
        sources["kernel/contracts/src/display_link.rs"] += '\n"https://second.example.test/x";\n'
        self.assertIn(
            "undeclared external URI in product source: kernel/contracts/src/display_link.rs",
            audit.scan_sources(self.policy, sources),
        )
        removed = dict(self.sources)
        removed["kernel/contracts/src/display_link.rs"] = removed[
            "kernel/contracts/src/display_link.rs"
        ].replace("https://example.test/x", "fixture-without-uri")
        self.assertIn(
            "URI allowance is stale or incomplete: kernel/contracts/src/display_link.rs",
            audit.scan_sources(self.policy, removed),
        )


if __name__ == "__main__":
    unittest.main()
