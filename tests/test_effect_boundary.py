from __future__ import annotations

import unittest
from pathlib import Path

from scripts.effect_boundary import ROOT, validate_effect_boundary


class EffectBoundaryTests(unittest.TestCase):
    def source(self, relative: str) -> str:
        return (ROOT / relative).read_text(encoding="utf-8")

    def test_current_boundary_passes(self) -> None:
        self.assertEqual(validate_effect_boundary(), [])

    def test_public_sandbox_effect_is_rejected(self) -> None:
        relative = Path("platforms/linux/src/sandbox.rs")
        source = self.source(str(relative)).replace(
            "    fn run(\n", "    pub fn run(\n", 1
        )
        failures = validate_effect_boundary(overrides={relative: source})
        self.assertIn("raw Linux sandbox execution is public", failures)

    def test_clonable_permit_is_rejected(self) -> None:
        relative = Path("kernel/engine/src/authority_transaction.rs")
        source = self.source(str(relative)).replace(
            "pub struct EffectAuthorization<'transaction>",
            "#[derive(Clone)]\npub struct EffectAuthorization<'transaction>",
            1,
        )
        failures = validate_effect_boundary(overrides={relative: source})
        self.assertIn(
            "effect authorization must not be duplicable or serializable", failures
        )

    def test_shell_process_launch_is_rejected(self) -> None:
        relative = Path("shells/host/src/main.rs")
        source = self.source(str(relative)) + "\nuse std::process::Command;\n"
        failures = validate_effect_boundary(overrides={relative: source})
        self.assertIn(f"{relative} contains direct process launch", failures)

    def test_unregistered_permit_consumer_is_rejected(self) -> None:
        relative = Path("capabilities/read-only/src/lib.rs")
        source = self.source(str(relative)) + "\n// EffectAuthorization\n"
        failures = validate_effect_boundary(overrides={relative: source})
        self.assertIn(
            f"unregistered effect-authorization consumer: {relative}", failures
        )

    def test_observation_module_cannot_convert_to_effect_authority(self) -> None:
        relative = Path("platforms/linux/src/inventory.rs")
        source = self.source(str(relative)) + "\n// EffectAuthorization\n"
        failures = validate_effect_boundary(overrides={relative: source})
        self.assertIn(
            f"unregistered effect-authorization consumer: {relative}", failures
        )

    def test_platform_cannot_drop_kernel_mediation_dependency(self) -> None:
        relative = Path("platforms/linux/Cargo.toml")
        source = self.source(str(relative)).replace(
            "agentmage-kernel-engine.workspace = true\n", "", 1
        )
        failures = validate_effect_boundary(overrides={relative: source})
        self.assertTrue(
            any("platform-linux internal imports" in failure for failure in failures)
        )

    def test_socket_listener_export_is_rejected(self) -> None:
        relative = Path("platforms/linux/src/lib.rs")
        source = self.source(str(relative)).replace(
            "    LinuxPeerIdentity,\n", "    LinuxPeerIdentity, PrivateUnixListener,\n", 1
        )
        failures = validate_effect_boundary(overrides={relative: source})
        self.assertIn(
            "private Unix listener is exported from the Linux adapter", failures
        )

    def test_socket_listener_crate_visibility_is_rejected(self) -> None:
        relative = Path("platforms/linux/src/ipc.rs")
        source = self.source(str(relative)).replace(
            "struct PrivateUnixListener", "pub(crate) struct PrivateUnixListener", 1
        )
        failures = validate_effect_boundary(overrides={relative: source})
        self.assertIn("private Unix listener exceeds module visibility", failures)


if __name__ == "__main__":
    unittest.main()
