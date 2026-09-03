from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import knowledge_sprint_boundary_review as review


class KnowledgeSprintBoundaryReviewTests(unittest.TestCase):
    def fixture(self, sprint: int) -> dict[str, object]:
        source = (
            b"deterministic citation parity missing instruction-like prose are untrusted data "
            b"ObsidianVaultSelection ObsidianEntryKind::SymbolicLink symlink_metadata "
            b"apply_watch_batch derived_only source_files_mutated external_process_started "
            b"network_accessed SemanticAdmissionReceipt LocalSemanticIndex max_results "
            b"lexical_fallback_available approved_local_semantic_workflow_evidence "
            b"KnowledgeRetrievalMode::ApprovedLocalSemantic retrieval_result_sha256 "
            b"remote_enabled: false source_mutated: false decision_sha256 "
            b"automatic_decision: false bundle_digest compose_memory_backup_plan "
            b"RestoreLastGood PreserveConflict Conflicts compose_portable_memory_export "
            b"expected_sha256 read_only: true preview_conversation_branch branch_from_turn_id "
            b"revalidate_resume ResumeDriftDimension ConversationDeletionApproval "
            b"approved_preview_sha256 source_hash_set_sha256 receipt_ids "
            b"ConversationClientCommand GrantOperation::DatabaseRead separately keyed "
            b"OperationalStoreKeyProvider EvidenceBundlePreview detect_secret_classes "
            b"external_delivery_attempted: false execute_conversation_command "
            b"trait ConversationKernel impl ConversationKernel for OperationalStore "
            b"ConversationCommandContext::Resume ConversationCommandContext::Branch "
            b"evidence-bound transition preview no apply authority "
            b"SkillAuthorityCeiling::denied() executed: false SkillInfluenceReceipt conflicts "
            b"proposed_write_count: 0 PlainWorkspaceSteward ObsidianVaultSteward "
            b"ClientSurface::NativeChat ClientSurface::InteractiveCli"
        )
        with patch.object(review, "git_bytes", return_value=source):
            return review.expected(sprint, "a" * 40)

    def test_all_review_shapes_pass(self) -> None:
        for sprint in review.SOURCES:
            value = self.fixture(sprint)
            with patch.object(review, "expected", return_value=value):
                self.assertEqual(review.validate(value), [])

    def test_suppression_human_and_source_mutations_fail(self) -> None:
        original = self.fixture(28)
        for mutate in (
            lambda value: value["checks"].update({"watch_batch_is_atomic": False}),
            lambda value: value.update({"independent_human_review_performed": True}),
            lambda value: value["source_sha256"].pop(next(iter(value["source_sha256"]))),
        ):
            value = copy.deepcopy(original)
            mutate(value)
            with patch.object(review, "expected", return_value=original):
                self.assertTrue(review.validate(value))


if __name__ == "__main__":
    unittest.main()
