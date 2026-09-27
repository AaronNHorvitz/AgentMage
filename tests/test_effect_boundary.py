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

    def test_sandbox_supervisor_has_only_the_closed_internal_interface(self) -> None:
        path = Path("platforms/linux/src/sandbox_supervision.rs")
        source = self.source(str(path))
        for entry, original_visibility in (("run", "pub(super)"), ("admission_snapshot", "pub(crate)"), ("run_owned", "pub(crate)")):
            for visibility in ("pub", "pub(in crate)"):
                changed = source.replace(f"{original_visibility} fn {entry}(", f"{visibility} fn {entry}(", 1)
                self.assertNotEqual(source, changed)
                self.assertIn("Linux sandbox supervisor exceeds its closed internal boundary", validate_effect_boundary(overrides={path: changed}))
        invented = "pub(super) fn unrelated_process_entry() {}\n" + source
        self.assertIn("Linux sandbox supervisor exceeds its closed internal boundary", validate_effect_boundary(overrides={path: invented}))
        parent = Path("platforms/linux/src/sandbox.rs")
        changed = self.source(str(parent)).replace("pub(crate) mod supervision;", "pub mod supervision;", 1)
        self.assertIn("Linux sandbox supervisor module must remain crate-internal", validate_effect_boundary(overrides={parent: changed}))

    def test_supervisor_owners_consumers_and_accounting_are_closed(self) -> None:
        path = Path("platforms/linux/src/sandbox_supervision.rs")
        original = self.source(str(path))
        for changed in (original.replace("pub owner: LaunchOwner,", "pub owner: String,", 1),
                        "pub use crate::unrelated;\n" + original):
            self.assertIn("Linux sandbox supervisor exceeds its closed internal boundary", validate_effect_boundary(overrides={path: changed}))
        changed = original.replace("Read(Arc<LinuxSandboxManifest>),", "Read(Arc<LinuxSandboxManifest>),\n    Arbitrary(String),", 1)
        self.assertIn("Linux sandbox supervisor owner set is not closed", validate_effect_boundary(overrides={path: changed}))
        changed = original.replace("static OWNED_ATTEMPT:", "static SECOND_OWNER:", 1)
        self.assertIn("Linux sandbox supervisor must retain its single attempt owner", validate_effect_boundary(overrides={path: changed}))
        caller = Path("platforms/linux/src/lib.rs")
        changed = "pub use sandbox::supervision::run_owned;\n" + self.source(str(caller))
        self.assertIn(f"unregistered native supervisor consumer: {caller}", validate_effect_boundary(overrides={caller: changed}))
        helper = Path("platforms/linux/src/sandbox_supervision/resource_usage.rs")
        source = self.source(str(helper))
        for changed in (source.replace("pub(super) fn sample", "pub(crate) fn sample", 1),
                        "pub(super) fn launch() {}\n" + source):
            self.assertIn("Linux sandbox accounting exceeds its closed internal boundary", validate_effect_boundary(overrides={helper: changed}))
        changed = "fn launch() { Command::new(\"unused\"); }\n" + source
        self.assertIn("Linux sandbox accounting cannot own processes or sockets", validate_effect_boundary(overrides={helper: changed}))

    def test_sandbox_supervision_cannot_regain_unbounded_waits(self) -> None:
        for path in (Path("platforms/linux/src/sandbox.rs"), Path("platforms/linux/src/sandbox_supervision.rs")):
            original = self.source(str(path))
            for call in ("child.wait()", "child.wait_with_output()", "command.status()", "command.output()", "reader.join()", "reader.join(\n)", "reader.join(/* owner */)", "reader.join(// owner\n)"):
                changed = f"fn unbounded() {{ {call}; }}\n" + original
                self.assertIn(f"unbounded blocking worker/control wait reintroduced: {path}", validate_effect_boundary(overrides={path: changed}))

    def test_path_and_string_joins_are_not_blocking_thread_joins(self) -> None:
        for path in (Path("platforms/linux/src/sandbox.rs"), Path("platforms/linux/src/sandbox_supervision.rs")):
            original = self.source(str(path))
            for call in ('Path::new("/app").join("worker")', 'names.join(",")', 'root.join(\n "worker"\n)'):
                changed = f"fn bounded_data() {{ {call}; }}\n" + original
                self.assertNotIn(f"unbounded blocking worker/control wait reintroduced: {path}", validate_effect_boundary(overrides={path: changed}))

    def test_public_configuration_effect_is_rejected(self) -> None:
        relative = Path("platforms/linux/src/configuration_store.rs")
        source = self.source(str(relative)).replace(
            "    fn apply(\n", "    pub fn apply(\n", 1
        )
        failures = validate_effect_boundary(overrides={relative: source})
        self.assertIn("raw Linux configuration effect is public: apply", failures)

    def test_kernel_configuration_filesystem_dependency_is_rejected(self) -> None:
        relative = Path("kernel/engine/src/configuration.rs")
        source = self.source(str(relative)) + "\nuse std::fs;\n"
        failures = validate_effect_boundary(overrides={relative: source})
        self.assertIn(
            "kernel configuration contains a native filesystem dependency", failures
        )

    def test_in_memory_coordinator_cannot_expose_effect_launch(self) -> None:
        relative = Path("kernel/engine/src/authority_transaction.rs")
        source = self.source(str(relative)).replace(
            "    pub(crate) fn execute_effect<D: EffectDriver>(",
            "    pub fn execute_effect<D: EffectDriver>(",
            1,
        )
        failures = validate_effect_boundary(overrides={relative: source})
        self.assertIn(
            "in-memory authority coordinator exposes an effect entry point", failures
        )

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
        source = self.source(str(relative)).replace(
            "#[cfg(test)]",
            "use std::process::Command;\n\n#[cfg(test)]",
            1,
        )
        failures = validate_effect_boundary(overrides={relative: source})
        self.assertIn(f"{relative} contains direct process launch", failures)

    def test_shell_process_exit_code_is_not_misclassified_as_process_launch(self) -> None:
        relative = Path("shells/host/src/bin/agent.rs")
        source = self.source(str(relative))
        self.assertIn("std::process::ExitCode", source)
        failures = validate_effect_boundary()
        self.assertNotIn(f"{relative} contains direct process launch", failures)

    def test_exact_feature_gated_lifecycle_fixture_is_not_product_code(self) -> None:
        relative = Path(
            "capabilities/read-only/src/bin/agentmage-read-only-lifecycle-fixture.rs"
        )
        failures = validate_effect_boundary()

        self.assertNotIn(f"{relative} contains direct filesystem mutation", failures)

    def test_lifecycle_fixture_without_required_feature_is_product_code(self) -> None:
        manifest = Path("capabilities/read-only/Cargo.toml")
        source = self.source(str(manifest)).replace(
            'required-features = ["lifecycle-fixture"]\n', "", 1
        )
        failures = validate_effect_boundary(overrides={manifest: source})

        self.assertIn(
            "capabilities/read-only/src/bin/agentmage-read-only-lifecycle-fixture.rs "
            "contains direct filesystem mutation",
            failures,
        )

    def test_neighboring_capability_binary_does_not_inherit_fixture_exemption(self) -> None:
        relative = Path("capabilities/read-only/src/bin/agentmage-read-only-worker.rs")
        source = self.source(str(relative)) + '\nfn mutate() { std::fs::write("x", b"x").unwrap(); }\n'
        failures = validate_effect_boundary(overrides={relative: source})

        self.assertIn(f"{relative} contains direct filesystem mutation", failures)

    def test_early_test_attribute_cannot_hide_later_product_process_authority(self) -> None:
        relative = Path("shells/host/src/main.rs")
        source = self.source(str(relative)).replace(
            "fn main()",
            "#[cfg(test)]\nuse std::fmt;\n\nuse std::process::Command;\n\nfn main()",
            1,
        )
        failures = validate_effect_boundary(overrides={relative: source})
        self.assertIn(f"{relative} contains direct process launch", failures)

    def test_unregistered_permit_consumer_is_rejected(self) -> None:
        relative = Path("capabilities/read-only/src/lib.rs")
        source = self.source(str(relative)) + "\n// EffectAuthorization\n"
        failures = validate_effect_boundary(overrides={relative: source})
        self.assertIn(
            f"unregistered effect-authorization consumer: {relative}", failures
        )

    def test_command_runner_is_a_registered_permit_consumer(self) -> None:
        failures = validate_effect_boundary()

        self.assertNotIn(
            "unregistered effect-authorization consumer: "
            "kernel/engine/src/command_runner.rs",
            failures,
        )

    def test_research_binding_does_not_admit_neighboring_packet_consumers(self) -> None:
        binding = "kernel/engine/src/research_effect_binding.rs"
        self.assertIn("EffectAuthorization", self.source(binding))
        self.assertNotIn(
            f"unregistered effect-authorization consumer: {binding}",
            validate_effect_boundary(),
        )
        packet = Path("kernel/engine/src/research_fetch.rs")
        failures = validate_effect_boundary(
            overrides={packet: self.source(str(packet)) + "\n// EffectAuthorization\n"}
        )
        self.assertIn(f"unregistered effect-authorization consumer: {packet}", failures)

    def test_research_dispatch_is_exactly_registered_not_a_module_exemption(self) -> None:
        dispatch = Path("kernel/engine/src/research_dispatch.rs")
        self.assertIn("EffectAuthorization", self.source(str(dispatch)))
        self.assertNotIn(f"unregistered effect-authorization consumer: {dispatch}", validate_effect_boundary())
        journal = Path("kernel/engine/src/research_journal.rs")
        failures = validate_effect_boundary(overrides={journal: self.source(str(journal)) + "\n// EffectAuthorization\n"})
        self.assertIn(f"unregistered effect-authorization consumer: {journal}", failures)

    def test_research_proof_field_clone_and_public_constructor_are_rejected(self) -> None:
        path = Path("kernel/engine/src/research_dispatch.rs")
        original = self.source(str(path))
        cases = (
            (original.replace("    material: &'a FreshResearchDispatch,", "    pub material: &'a FreshResearchDispatch,", 1),
             "research dispatch proof fields must remain private"),
            (original.replace("pub struct ResearchDispatch<'a>", "#[derive(Clone)]\npub struct ResearchDispatch<'a>", 1),
             "research dispatch proof must not be duplicable or serializable"),
            (original + "\npub fn forged() -> ResearchDispatch<'static> { todo!() }\n",
             "research dispatch proof exposes a public constructor"),
            (original.replace("pub(crate) struct FreshResearchDispatch", "pub struct FreshResearchDispatch", 1),
             "research dispatch owner material exceeds crate visibility"),
        )
        for changed, diagnostic in cases:
            self.assertNotEqual(changed, original)
            self.assertIn(diagnostic, validate_effect_boundary(overrides={path: changed}))

    def test_research_preflight_cannot_become_a_public_bypass_callback(self) -> None:
        path = Path("kernel/engine/src/operational_store.rs")
        original = self.source(str(path))
        for visibility in ("pub", "pub(crate)", "pub(super)"):
            changed = original.replace(
                "    fn begin_effect_with_preflight<D, F>(",
                f"    {visibility} fn begin_effect_with_preflight<D, F>(",
                1,
            )
            self.assertNotEqual(changed, original)
            self.assertIn(
                "research preflight callback must remain owner-private",
                validate_effect_boundary(overrides={path: changed}),
            )

    def test_research_native_driver_stays_feature_gated_private_and_dual_proof(self) -> None:
        path = Path("platforms/linux/src/research_sandbox.rs")
        original = self.source(str(path))
        for changed, diagnostic in (
            (original.replace("    fn run(", "    pub(crate) fn run(", 1),
             "raw Linux research execution must remain module-private"),
            (original + "\nimpl EffectDriver for LinuxPublicResearchEffectDriver<'_> {}\n",
             "Linux research driver cannot bypass the dual-proof interface"),
            (original.replace("dispatch: ResearchDispatch<'_>", "dispatch: String", 1),
             "Linux research driver must consume both existing proofs"),
        ):
            self.assertNotEqual(original, changed)
            self.assertIn(diagnostic, validate_effect_boundary(overrides={path: changed}))
        parent = Path("platforms/linux/src/sandbox.rs")
        source = self.source(str(parent))
        guarded = '#[cfg(feature = "public-research-worker")]\n#[path = "research_sandbox.rs"]'
        changed = source.replace(guarded, '#[path = "research_sandbox.rs"]', 1)
        self.assertNotEqual(source, changed)
        self.assertIn("Linux research adapter must remain feature-gated and crate-internal", validate_effect_boundary(overrides={parent: changed}))
        for sibling in (Path("platforms/linux/src/research_resolver.rs"), Path("platforms/linux/src/research_seccomp.rs")):
            changed = "// EffectAuthorization\nfn bypass() { supervision::run_owned(); }\n" + self.source(str(sibling))
            failures = validate_effect_boundary(overrides={sibling: changed})
            self.assertIn(f"unregistered effect-authorization consumer: {sibling}", failures)
            self.assertIn(f"unregistered native supervisor consumer: {sibling}", failures)

    def test_repository_safety_is_a_registered_permit_consumer(self) -> None:
        failures = validate_effect_boundary()

        self.assertNotIn(
            "unregistered effect-authorization consumer: "
            "kernel/engine/src/repository_safety.rs",
            failures,
        )

    def test_repository_inspection_is_a_registered_permit_consumer(self) -> None:
        failures = validate_effect_boundary()

        self.assertNotIn(
            "unregistered effect-authorization consumer: "
            "kernel/engine/src/repository_inspection.rs",
            failures,
        )

    def test_local_commit_is_a_registered_permit_consumer(self) -> None:
        failures = validate_effect_boundary()

        self.assertNotIn(
            "unregistered effect-authorization consumer: "
            "kernel/engine/src/local_commit.rs",
            failures,
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
            'agentmage-kernel-engine = { workspace = true, features = '
            '["runtime-projections", "source-preparation", '
            '"verified-workflow-supervisor"] }\n',
            "",
            1,
        )
        failures = validate_effect_boundary(overrides={relative: source})
        self.assertTrue(
            any("platform-linux internal imports" in failure for failure in failures)
        )

    def test_socket_listener_export_is_rejected(self) -> None:
        relative = Path("platforms/linux/src/lib.rs")
        source = self.source(str(relative)) + "\npub use ipc::PrivateUnixListener;\n"
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
