from __future__ import annotations

import copy
import unittest

from scripts.parser_ocr_placement import PLATFORM_IDS, load_record, validate_record


class ParserOcrPlacementTests(unittest.TestCase):
    def setUp(self) -> None:
        self.record = load_record()

    def _platform(self, record: dict, platform_id: str) -> dict:
        return next(item for item in record["platforms"] if item["id"] == platform_id)

    def test_canonical_record_passes(self) -> None:
        self.assertEqual(validate_record(self.record), [])

    def test_fields_are_closed(self) -> None:
        root = copy.deepcopy(self.record)
        root["auto_install_ocr"] = True
        self.assertIn("record field closure changed", validate_record(root))
        platform = copy.deepcopy(self.record)
        platform["platforms"][0]["fallback_host"] = "typescript"
        self.assertIn("platform[0] field closure changed", validate_record(platform))

    def test_authority_digest_is_current(self) -> None:
        changed = copy.deepcopy(self.record)
        changed["authorities"][0]["sha256"] = "0" * 64
        self.assertIn("authority[0] digest differs from current authority", validate_record(changed))

    def test_parser_ownership_and_launch_cannot_move(self) -> None:
        owner = copy.deepcopy(self.record)
        owner["global_placement"]["parser_owner"] = "shells-vscode-typescript"
        self.assertIn("parser ownership changed", validate_record(owner))
        launcher = copy.deepcopy(self.record)
        launcher["global_placement"]["process_launcher_owner"] = "capability-knowledge"
        self.assertIn("worker launch escaped the platform adapter", validate_record(launcher))

    def test_untrusted_parser_authority_cannot_widen(self) -> None:
        for key in (
            "typescript_parser_allowed",
            "in_process_complex_parser_allowed",
            "ocr_in_host_allowed",
            "worker_network_allowed",
            "ambient_workspace_allowed",
            "credential_access_allowed",
            "second_store_allowed",
        ):
            with self.subTest(key=key):
                changed = copy.deepcopy(self.record)
                changed["global_placement"][key] = True
                self.assertIn(f"global_placement.{key} must remain false", validate_record(changed))

    def test_no_feature_is_enabled_by_default(self) -> None:
        changed = copy.deepcopy(self.record)
        changed["package_features"]["default_enabled"] = ["parser-pdf"]
        self.assertIn("parser or OCR feature became enabled by default", validate_record(changed))

    def test_ocr_and_dynamic_plugins_cannot_be_promoted(self) -> None:
        ocr = copy.deepcopy(self.record)
        ocr["package_features"]["ocr_status"] = "enabled"
        self.assertIn("OCR disposition was promoted", validate_record(ocr))
        plugin = copy.deepcopy(self.record)
        plugin["package_features"]["dynamic_plugin_discovery_allowed"] = True
        self.assertIn(
            "package_features.dynamic_plugin_discovery_allowed must remain false",
            validate_record(plugin),
        )

    def test_byte_crossing_never_uses_paths_plaintext_temp_or_unvalidated_output(self) -> None:
        for key in (
            "source_in_arguments_or_environment_allowed",
            "plaintext_shared_temp_allowed",
            "path_as_authority_allowed",
            "worker_output_trusted_before_validation",
            "raw_source_return_default",
        ):
            with self.subTest(key=key):
                changed = copy.deepcopy(self.record)
                changed["byte_crossing"][key] = True
                self.assertIn(f"byte_crossing.{key} must remain false", validate_record(changed))

    def test_byte_crossing_receipt_is_complete(self) -> None:
        changed = copy.deepcopy(self.record)
        changed["byte_crossing"]["crossing_receipt_fields"].remove("source-sha256")
        self.assertIn("byte-crossing receipt fields changed", validate_record(changed))

    def test_cancellation_reaps_worker_and_publishes_no_partial_derivative(self) -> None:
        terminal = copy.deepcopy(self.record)
        terminal["cancellation"]["terminal_before_worker_reaped_allowed"] = True
        self.assertIn(
            "cancellation.terminal_before_worker_reaped_allowed must remain false",
            validate_record(terminal),
        )
        partial = copy.deepcopy(self.record)
        partial["cancellation"]["partial_derivative_publication_allowed"] = True
        self.assertIn(
            "cancellation.partial_derivative_publication_allowed must remain false",
            validate_record(partial),
        )

    def test_fallback_never_moves_to_cloud_typescript_or_host(self) -> None:
        for key in (
            "cloud_ocr_allowed",
            "typescript_fallback_allowed",
            "in_host_complex_parser_fallback_allowed",
            "silent_format_downgrade_allowed",
            "local_path_reinterpretation_allowed",
        ):
            with self.subTest(key=key):
                changed = copy.deepcopy(self.record)
                changed["fallback"][key] = True
                self.assertIn(f"fallback.{key} must remain false", validate_record(changed))

    def test_platform_inventory_is_complete_and_ordered(self) -> None:
        self.assertEqual([item["id"] for item in self.record["platforms"]], PLATFORM_IDS)
        changed = copy.deepcopy(self.record)
        changed["platforms"].pop()
        self.assertIn("platform inventory must remain complete and ordered", validate_record(changed))

    def test_macos_evidence_remains_blocked(self) -> None:
        parser = copy.deepcopy(self.record)
        self._platform(parser, "macos-apple-silicon")["parser_status"] = "supported"
        self.assertIn("macOS parser and OCR must remain blocked-post-ga", validate_record(parser))
        evidence = copy.deepcopy(self.record)
        self._platform(evidence, "macos-apple-silicon")["native_evidence"] = "pass"
        self.assertIn("macOS evidence status was overclaimed", validate_record(evidence))

    def test_non_macos_platform_evidence_remains_pending(self) -> None:
        for platform_id in PLATFORM_IDS:
            if platform_id == "macos-apple-silicon":
                continue
            with self.subTest(platform=platform_id):
                changed = copy.deepcopy(self.record)
                self._platform(changed, platform_id)["native_evidence"] = "pass"
                self.assertIn(f"{platform_id} native evidence was overclaimed", validate_record(changed))

    def test_remote_hosts_remain_workspace_side_and_paths_are_not_forwarded(self) -> None:
        for platform_id in ("wsl", "remote-ssh", "dev-containers"):
            with self.subTest(platform=platform_id):
                host = copy.deepcopy(self.record)
                self._platform(host, platform_id)["host_locus"] = "local-ui-machine"
                self.assertIn(f"{platform_id} host must remain workspace-side", validate_record(host))
                source = copy.deepcopy(self.record)
                self._platform(source, platform_id)["source_rule"] = "forward-local-path"
                self.assertIn(f"{platform_id} source crossing rule was widened", validate_record(source))

    def test_product_truth_cannot_claim_implementation_support_or_release(self) -> None:
        for key in (
            "features_enabled",
            "parser_worker_implemented",
            "ocr_worker_implemented",
            "remote_placement_implemented",
            "native_evidence_complete",
            "platform_support_claimed",
            "release_readiness",
        ):
            with self.subTest(key=key):
                changed = copy.deepcopy(self.record)
                changed["product_truth"][key] = True
                self.assertIn("product truth was widened", validate_record(changed))


if __name__ == "__main__":
    unittest.main()
