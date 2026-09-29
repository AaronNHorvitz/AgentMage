//! Hunk review of one proposed coding write for the protected approval display
//! (AMR-04.2.1, Decision 0112).
//!
//! The review is a pure function of the challenge's exact arguments and of
//! preimage bytes whose digest equals the preimage those arguments bind. It
//! therefore shows exactly the change the approved call can make, whichever
//! file the bytes were read from. It grants, selects and narrows nothing: the
//! approval still binds the complete arguments, and the write still refuses a
//! preimage that changed before it runs.

use std::fmt::Write as _;

use agentmage_capability_repository_map::{
    StructuredFileChangeRequest, build_structured_file_change,
};
use agentmage_kernel_contracts::{RuntimeApprovalChallenge, WorkspaceId, WorkspacePath};
use agentmage_kernel_engine::write_approval::selective::{HunkedTextChange, SelectiveChangeError};
use sha2::{Digest, Sha256};

use crate::coding_changes::{
    CONTROLLED_CREATE_TOOL_ID, ControlledFileCreationProposal, STRUCTURED_PATCH_TOOL_ID,
    StructuredPatchProposal,
};

/// Unchanged lines shown around each hunk.
pub const CHANGE_REVIEW_CONTEXT_LINES: usize = 3;

/// Content-free reason a hunk review cannot be shown; whole-call review still applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeReviewUnavailable {
    /// The arguments do not decode as this tool's closed proposal.
    InvalidArguments,
    /// The current file bytes differ from the preimage the arguments bind.
    PreimageChanged,
    /// The target cannot be read, or a file exists where a creation expects none.
    TargetUnavailable,
    /// The existing planner refused the edits for this preimage.
    PlannerRefused,
    /// The change is not bounded UTF-8 text for hunk review.
    NotReviewable,
}

impl ChangeReviewUnavailable {
    /// Stable diagnostic without paths or content.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidArguments => "coding.change-review.arguments-invalid",
            Self::PreimageChanged => "coding.change-review.preimage-changed",
            Self::TargetUnavailable => "coding.change-review.target-unavailable",
            Self::PlannerRefused => "coding.change-review.planner-refused",
            Self::NotReviewable => "coding.change-review.not-reviewable",
        }
    }

    const fn explanation(self) -> &'static str {
        match self {
            Self::InvalidArguments => "the arguments do not decode as this tool's proposal",
            Self::PreimageChanged => {
                "the file changed since the proposal was made; this exact write would be refused"
            }
            Self::TargetUnavailable => "the target file cannot be read in its expected state",
            Self::PlannerRefused => "the edits do not apply to the current file",
            Self::NotReviewable => "the change is not bounded text; review the whole call",
        }
    }
}

/// Hunks of one exact proposed write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeReview {
    /// Canonical workspace-relative path components.
    pub path: Vec<String>,
    /// Digest of the reviewed preimage; empty text for a creation.
    pub preimage_sha256: String,
    /// Digest of the complete proposed postimage.
    pub postimage_sha256: String,
    /// Number of hunks.
    pub hunk_count: usize,
    /// Bounded escaped unified-style display.
    pub rendered: String,
}

/// Reviews the change a coding write challenge proposes. Returns `None` for any
/// other tool. `read_current` returns the current bytes at workspace-relative
/// components, or `None` when the path holds no readable regular file.
pub fn review_coding_write(
    challenge: &RuntimeApprovalChallenge,
    workspace_id: &WorkspaceId,
    read_current: &dyn Fn(&[String]) -> Option<Vec<u8>>,
) -> Option<Result<ChangeReview, ChangeReviewUnavailable>> {
    let arguments = &challenge.presentation.arguments.bytes;
    match challenge.presentation.tool_id.as_str() {
        STRUCTURED_PATCH_TOOL_ID => Some(review_patch(arguments, workspace_id, read_current)),
        CONTROLLED_CREATE_TOOL_ID => Some(review_create(arguments, read_current)),
        _ => None,
    }
}

fn review_patch(
    arguments: &[u8],
    workspace_id: &WorkspaceId,
    read_current: &dyn Fn(&[String]) -> Option<Vec<u8>>,
) -> Result<ChangeReview, ChangeReviewUnavailable> {
    let proposal: StructuredPatchProposal =
        serde_json::from_slice(arguments).map_err(|_| ChangeReviewUnavailable::InvalidArguments)?;
    let path = WorkspacePath::new(workspace_id.clone(), proposal.path.iter().cloned())
        .map_err(|_| ChangeReviewUnavailable::InvalidArguments)?;
    let current = read_current(&proposal.path).ok_or(ChangeReviewUnavailable::TargetUnavailable)?;
    if hex_sha256(&current) != proposal.expected_preimage_sha256 {
        return Err(ChangeReviewUnavailable::PreimageChanged);
    }
    let plan = build_structured_file_change(StructuredFileChangeRequest {
        change_id: proposal.change_id,
        path,
        intent_sha256: proposal.intent_sha256,
        change_plan_sha256: proposal.change_plan_sha256,
        language: proposal.language,
        artifact_class: proposal.artifact_class,
        preimage: current,
        expected_preimage_sha256: proposal.expected_preimage_sha256,
        edits: proposal.edits,
        additional_review_hooks: proposal.additional_review_hooks,
        generated: proposal.generated,
        allow_generated: proposal.allow_generated,
    })
    .map_err(|_| ChangeReviewUnavailable::PlannerRefused)?;
    hunks(proposal.path, plan.preimage(), plan.postimage())
}

fn review_create(
    arguments: &[u8],
    read_current: &dyn Fn(&[String]) -> Option<Vec<u8>>,
) -> Result<ChangeReview, ChangeReviewUnavailable> {
    let proposal: ControlledFileCreationProposal =
        serde_json::from_slice(arguments).map_err(|_| ChangeReviewUnavailable::InvalidArguments)?;
    if proposal.path.is_empty()
        || proposal.path.iter().any(|part| {
            part.is_empty() || matches!(part.as_str(), "." | "..") || part.contains('/')
        })
    {
        return Err(ChangeReviewUnavailable::InvalidArguments);
    }
    if read_current(&proposal.path).is_some() {
        return Err(ChangeReviewUnavailable::TargetUnavailable);
    }
    hunks(proposal.path, b"", proposal.content.as_bytes())
}

fn hunks(
    path: Vec<String>,
    preimage: &[u8],
    postimage: &[u8],
) -> Result<ChangeReview, ChangeReviewUnavailable> {
    let change = HunkedTextChange::new(preimage, postimage).map_err(|error| match error {
        SelectiveChangeError::NotText
        | SelectiveChangeError::TooLarge
        | SelectiveChangeError::TooComplex => ChangeReviewUnavailable::NotReviewable,
        SelectiveChangeError::UnknownHunk | SelectiveChangeError::PreimageDrift => {
            ChangeReviewUnavailable::PlannerRefused
        }
    })?;
    Ok(ChangeReview {
        path,
        preimage_sha256: change.preimage_sha256().to_owned(),
        postimage_sha256: change.proposal_sha256().to_owned(),
        hunk_count: change.hunks().len(),
        rendered: change.render(CHANGE_REVIEW_CONTEXT_LINES),
    })
}

/// Bounded text shown after the exact challenge. It describes what the call
/// would change; the approval itself still covers the complete arguments.
#[must_use]
pub fn render_change_review(review: &Result<ChangeReview, ChangeReviewUnavailable>) -> String {
    let mut output = String::new();
    match review {
        Ok(review) => {
            let _ = writeln!(
                output,
                "change review for {:?}: {} hunk(s), preimage {} -> postimage {}",
                review.path.join("/"),
                review.hunk_count,
                &review.preimage_sha256[..12],
                &review.postimage_sha256[..12]
            );
            output.push_str(&review.rendered);
            output.push_str(
                "this review is derived from the exact arguments; approving allows the whole call\n",
            );
        }
        Err(reason) => {
            let _ = writeln!(
                output,
                "change review unavailable ({}): {}",
                reason.code(),
                reason.explanation()
            );
        }
    }
    output
}

fn hex_sha256(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

#[cfg(test)]
pub(crate) mod tests_support {
    pub(crate) use super::tests::write_challenge;

    pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
        super::hex_sha256(bytes)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::HashMap;

    use agentmage_capability_repository_map::{
        StructuredArtifactClass, StructuredEdit, StructuredLanguage,
    };
    use agentmage_kernel_contracts::{
        ApprovalId, CONTRACT_SCHEMA_VERSION, ContractPayload, GrantId, GrantOperation,
        OperationBinding, RuntimeApprovalPresentation, RuntimeOperationId, RuntimeRunId,
        RuntimeTurnId, SchemaId, SchemaReference, TaskId, ToolCallId, ToolId, ToolRiskLevel,
    };

    use super::*;
    use crate::coding_changes::ControlledFileClassification;

    pub(crate) fn write_challenge(tool_id: &str, arguments: Vec<u8>) -> RuntimeApprovalChallenge {
        RuntimeApprovalChallenge {
            schema_version: CONTRACT_SCHEMA_VERSION,
            run_id: RuntimeRunId::from_raw("run-review"),
            task_id: TaskId::from_raw("task-review"),
            turn_id: RuntimeTurnId::from_raw("turn-review"),
            operation_id: RuntimeOperationId::from_raw("operation-review"),
            tool_call_id: ToolCallId::from_raw("call-review"),
            approval_id: ApprovalId::from_raw("approval-review"),
            proposed_grant_id: GrantId::from_raw("grant-review"),
            operation: GrantOperation::WorkspaceWrite,
            presentation: RuntimeApprovalPresentation {
                tool_id: ToolId::from_raw(tool_id),
                tool_version: "1.0.0".to_owned(),
                display_name: "Fixture write".to_owned(),
                risk_level: ToolRiskLevel::Moderate,
                target_scope: "one exact file".to_owned(),
                arguments: ContractPayload {
                    schema: SchemaReference {
                        schema_id: SchemaId::from_raw("fixture.input"),
                        schema_version: 1,
                        schema_sha256: "a".repeat(64),
                    },
                    media_type: "application/json".to_owned(),
                    sha256: hex_sha256(&arguments),
                    bytes: arguments,
                },
                declared_effects: vec![OperationBinding::new(GrantOperation::WorkspaceWrite)],
                single_use: true,
                timeout_ms: 1_000,
            },
            preview_sha256: "b".repeat(64),
            expires_at_epoch_ms: 2,
            challenge_sha256: "c".repeat(64),
        }
    }

    fn patch(expected: &str, edits: Vec<StructuredEdit>, language: StructuredLanguage) -> Vec<u8> {
        serde_json::to_vec(&StructuredPatchProposal {
            schema_version: 1,
            change_id: "change-review".to_owned(),
            path: vec!["src".to_owned(), "calc.py".to_owned()],
            expected_preimage_sha256: hex_sha256(expected.as_bytes()),
            intent_sha256: "a".repeat(64),
            change_plan_sha256: "b".repeat(64),
            language,
            artifact_class: StructuredArtifactClass::Code,
            edits,
            additional_review_hooks: Vec::new(),
            generated: false,
            allow_generated: false,
        })
        .unwrap()
    }

    fn workspace() -> WorkspaceId {
        WorkspaceId::from_raw("workspace-review")
    }

    fn reader(files: &[(&str, &str)]) -> impl Fn(&[String]) -> Option<Vec<u8>> {
        let files: HashMap<String, Vec<u8>> = files
            .iter()
            .map(|(path, text)| ((*path).to_owned(), text.as_bytes().to_vec()))
            .collect();
        move |path: &[String]| files.get(&path.join("/")).cloned()
    }

    const BROKEN: &str =
        "def add(left, right):\n    return left - right\n\n\ndef keep():\n    return 1\n";

    #[test]
    fn a_patch_review_shows_exactly_the_planned_hunks_for_the_bound_preimage() {
        let start = BROKEN.find("left - right").unwrap() as u64;
        let end = start + "left - right".len() as u64;
        let edits = vec![StructuredEdit::ReplaceSyntaxNode {
            edit_id: "edit-1".to_owned(),
            start_byte: start,
            end_byte: end,
            expected_node_sha256: hex_sha256(b"left - right"),
            replacement: "left + right".to_owned(),
        }];
        let challenge = write_challenge(
            STRUCTURED_PATCH_TOOL_ID,
            patch(BROKEN, edits, StructuredLanguage::Python),
        );
        let review = review_coding_write(
            &challenge,
            &workspace(),
            &reader(&[("src/calc.py", BROKEN)]),
        )
        .unwrap()
        .unwrap();
        assert_eq!(review.hunk_count, 1);
        assert_eq!(review.preimage_sha256, hex_sha256(BROKEN.as_bytes()));
        assert_eq!(
            review.postimage_sha256,
            hex_sha256(BROKEN.replace("left - right", "left + right").as_bytes())
        );
        assert!(
            review
                .rendered
                .contains("-    return left - right\n+    return left + right\n")
        );
        let text = render_change_review(&Ok(review));
        assert!(
            text.contains("change review for \"src/calc.py\": 1 hunk(s)"),
            "{text}"
        );
        assert!(text.contains("approving allows the whole call"));
    }

    #[test]
    fn a_changed_missing_or_refused_target_never_shows_a_misleading_diff() {
        let edits = vec![StructuredEdit::ReplaceExactText {
            edit_id: "edit-1".to_owned(),
            expected: "old text".to_owned(),
            replacement: "new text".to_owned(),
        }];
        let arguments = patch("old text\n", edits, StructuredLanguage::PlainText);
        let challenge = write_challenge(STRUCTURED_PATCH_TOOL_ID, arguments);
        for (files, expected) in [
            (
                vec![("src/calc.py", "old text\nhuman edit\n")],
                ChangeReviewUnavailable::PreimageChanged,
            ),
            (vec![], ChangeReviewUnavailable::TargetUnavailable),
        ] {
            let review = review_coding_write(&challenge, &workspace(), &reader(&files)).unwrap();
            assert_eq!(review, Err(expected));
            let text = render_change_review(&review);
            assert!(text.contains(expected.code()), "{text}");
            assert!(!text.contains("new text"));
        }
        // An edit that does not apply to the bound preimage is the planner's refusal.
        let edits = vec![StructuredEdit::ReplaceExactText {
            edit_id: "edit-1".to_owned(),
            expected: "absent text".to_owned(),
            replacement: "new text".to_owned(),
        }];
        let challenge_refused = write_challenge(
            STRUCTURED_PATCH_TOOL_ID,
            patch("old text\n", edits, StructuredLanguage::PlainText),
        );
        assert_eq!(
            review_coding_write(
                &challenge_refused,
                &workspace(),
                &reader(&[("src/calc.py", "old text\n")])
            )
            .unwrap(),
            Err(ChangeReviewUnavailable::PlannerRefused)
        );
        let malformed =
            write_challenge(STRUCTURED_PATCH_TOOL_ID, b"{\"schema_version\":1}".to_vec());
        assert_eq!(
            review_coding_write(&malformed, &workspace(), &reader(&[])).unwrap(),
            Err(ChangeReviewUnavailable::InvalidArguments)
        );
        let other = write_challenge("agentmage.workspace.read-file", b"{}".to_vec());
        assert!(review_coding_write(&other, &workspace(), &reader(&[])).is_none());
    }

    #[test]
    fn a_creation_review_shows_every_added_line_escaped_and_refuses_an_existing_file() {
        let content = "def test_new():\n    assert \"\u{202e}\" != \"\"\n";
        let arguments = serde_json::to_vec(&ControlledFileCreationProposal {
            schema_version: 1,
            creation_id: "creation-review".to_owned(),
            path: vec!["tests".to_owned(), "test_new.py".to_owned()],
            content: content.to_owned(),
            mode: 0o644,
            classification: ControlledFileClassification::SourceCode,
            intent_sha256: "a".repeat(64),
            change_plan_sha256: "b".repeat(64),
            expected_parent_sha256: "c".repeat(64),
        })
        .unwrap();
        let challenge = write_challenge(CONTROLLED_CREATE_TOOL_ID, arguments);
        let review = review_coding_write(&challenge, &workspace(), &reader(&[]))
            .unwrap()
            .unwrap();
        assert_eq!(review.preimage_sha256, hex_sha256(b""));
        assert!(review.rendered.contains("+def test_new():\n"));
        assert!(!review.rendered.contains('\u{202e}'));
        assert!(review.rendered.contains("\\u{202e}"));
        assert_eq!(
            review_coding_write(
                &challenge,
                &workspace(),
                &reader(&[("tests/test_new.py", "existing\n")])
            )
            .unwrap(),
            Err(ChangeReviewUnavailable::TargetUnavailable)
        );
    }
}
