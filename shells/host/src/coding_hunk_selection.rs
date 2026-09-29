//! A write derived from a person's hunk selection (AMR-04.2.2, Decision 0114).
//!
//! When a person accepts only some hunks of a proposed structured patch, the
//! original call is refused and never runs. The trusted boundary derives one
//! separate call to a shell-only tool whose arguments carry the original patch
//! arguments and the exact selection. The derived call is self-contained: it
//! re-plans the original patch over the exact current bytes, recomputes the
//! hunks and the selection, and writes only the selected postimage through the
//! existing structured-write path, with its own preview, approval and grant. A
//! changed file, an unknown hunk or any digest that no longer matches is refused.

use std::collections::BTreeSet;

use agentmage_capability_repository_map::{
    StructuredEdit, StructuredFileChangePlan, StructuredLanguage,
};
use agentmage_kernel_contracts::{
    ContractPayload, RuntimeHunkSelection, ToolCall, ToolDefinition, ToolId, ValidationIssue,
};
use agentmage_kernel_engine::{
    runtime_loop::RuntimeDerivedToolCall,
    tooling::{Tool, ToolRegistry},
    write_approval::selective::{HunkedTextChange, MAX_SELECTIVE_HUNKS, SelectedTextChange},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::coding_changes::{
    CONTROLLED_CHANGE_TOOL_VERSION, CodingChangeError, CodingWriteScope, STRUCTURED_PATCH_TOOL_ID,
    StructuredPatchProposal, bind_structured_patch_proposal, controlled_change_definition,
    proposal_issue, validate_patch,
};

/// Stable native identity of the shell-only selected write.
pub const HUNK_SELECTION_TOOL_ID: &str = "agentmage.code.apply-hunk-selection";

/// Closed input-schema identity of one selected write.
pub const HUNK_SELECTION_INPUT_SCHEMA_ID: &str = "agentmage.code.apply-hunk-selection.input";

/// Canonical closed schema of one selected write. It is never offered to a model,
/// so it names no external dialect URI; the nested original patch is validated by
/// the same closed validator as a model's patch proposal.
pub const HUNK_SELECTION_INPUT_SCHEMA_JSON: &str = r##"{"$id":"agentmage.code.apply-hunk-selection.input","type":"object","additionalProperties":false,"required":["schema_version","original_tool_call_id","original_arguments_sha256","original","preimage_sha256","proposal_sha256","accepted_hunk_ids","rejected_hunk_ids","selection_sha256","postimage_sha256"],"properties":{"schema_version":{"const":1},"original_tool_call_id":{"type":"string","pattern":"^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$"},"original_arguments_sha256":{"$ref":"#/$defs/sha"},"original":{"type":"object"},"preimage_sha256":{"$ref":"#/$defs/sha"},"proposal_sha256":{"$ref":"#/$defs/sha"},"accepted_hunk_ids":{"$ref":"#/$defs/ids"},"rejected_hunk_ids":{"$ref":"#/$defs/ids"},"selection_sha256":{"$ref":"#/$defs/sha"},"postimage_sha256":{"$ref":"#/$defs/sha"}},"$defs":{"sha":{"type":"string","pattern":"^[0-9a-f]{64}$"},"ids":{"type":"array","minItems":1,"maxItems":512,"uniqueItems":true,"items":{"$ref":"#/$defs/sha"}}}}"##;

/// Arguments of one selected write: the original patch and the exact selection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HunkSelectionWriteProposal {
    /// Closed request schema version.
    pub schema_version: u16,
    /// The refused model call whose change was narrowed.
    pub original_tool_call_id: String,
    /// Digest of the refused call's exact arguments.
    pub original_arguments_sha256: String,
    /// The refused call's exact patch arguments.
    pub original: StructuredPatchProposal,
    /// Digest of the preimage the hunks were computed over.
    pub preimage_sha256: String,
    /// Digest of the original patch's complete postimage.
    pub proposal_sha256: String,
    /// Accepted hunk identities, sorted.
    pub accepted_hunk_ids: Vec<String>,
    /// Rejected hunk identities, sorted; their preimage lines are kept.
    pub rejected_hunk_ids: Vec<String>,
    /// Digest sealing the selection (Decision 0107).
    pub selection_sha256: String,
    /// Digest of the selected postimage that will be written.
    pub postimage_sha256: String,
}

/// Content-free reason a selection cannot be honored. The original call is then
/// refused and nothing is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HunkSelectionError {
    /// The refused call is not a structured patch, or its arguments do not decode.
    NotNarrowable,
    /// The selection names another preimage or proposal than the one held.
    Stale,
    /// A hunk identity is not one of this change's hunks, or the file changed.
    Mismatch,
    /// The selection accepts every hunk or none; that is an allow or a deny.
    NotASubset,
    /// The derived arguments or the selected write failed closed validation.
    Invalid,
}

impl HunkSelectionError {
    /// Stable denial reason recorded for the refused original call.
    #[must_use]
    pub const fn reason_code(self) -> &'static str {
        match self {
            Self::NotNarrowable => "runtime.coding.selection-not-narrowable",
            Self::Stale => "runtime.coding.selection-stale",
            Self::Mismatch => "runtime.coding.selection-mismatch",
            Self::NotASubset => "runtime.coding.selection-not-a-subset",
            Self::Invalid => "runtime.coding.selection-invalid",
        }
    }
}

/// Returns the shell-only selected-write definition.
#[must_use]
pub fn hunk_selection_tool_definition() -> ToolDefinition {
    controlled_change_definition(
        HUNK_SELECTION_TOOL_ID,
        "Apply selected hunks",
        "Writes only the hunks a person accepted from one refused structured patch, over the same exact preimage. Proposed by the runtime from the person's selection, never by a model.",
        HUNK_SELECTION_INPUT_SCHEMA_ID,
        HUNK_SELECTION_INPUT_SCHEMA_JSON,
    )
}

struct RegisteredHunkSelectionTool {
    definition: ToolDefinition,
    scope: CodingWriteScope,
}

impl Tool for RegisteredHunkSelectionTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    fn validate_arguments(&self, arguments: &[u8]) -> Vec<ValidationIssue> {
        proposal_issue(
            decode_hunk_selection_proposal(&self.scope, arguments)
                .map(|_| ())
                .map_err(|_| CodingChangeError::InvalidRequest),
        )
    }
}

/// Registers the selected write as a tool only a shell may propose.
pub fn register_hunk_selection_runtime_tool(
    registry: &mut ToolRegistry,
    scope: CodingWriteScope,
) -> Result<(), CodingChangeError> {
    registry
        .register_shell_tool(Box::new(RegisteredHunkSelectionTool {
            definition: hunk_selection_tool_definition(),
            scope,
        }))
        .map_err(|_| CodingChangeError::RegistrationDenied)
}

/// Decodes and structurally validates one selected write's arguments.
pub fn decode_hunk_selection_proposal(
    scope: &CodingWriteScope,
    arguments: &[u8],
) -> Result<HunkSelectionWriteProposal, HunkSelectionError> {
    let proposal: HunkSelectionWriteProposal =
        serde_json::from_slice(arguments).map_err(|_| HunkSelectionError::Invalid)?;
    let ids_valid = |ids: &[String]| {
        !ids.is_empty()
            && ids.len() <= MAX_SELECTIVE_HUNKS
            && ids.iter().all(|id| is_sha256(id))
            && ids.windows(2).all(|pair| pair[0] < pair[1])
    };
    if proposal.schema_version != 1
        || !valid_identifier(&proposal.original_tool_call_id)
        || ![
            &proposal.original_arguments_sha256,
            &proposal.preimage_sha256,
            &proposal.proposal_sha256,
            &proposal.selection_sha256,
            &proposal.postimage_sha256,
        ]
        .into_iter()
        .all(|digest| is_sha256(digest))
        || !ids_valid(&proposal.accepted_hunk_ids)
        || !ids_valid(&proposal.rejected_hunk_ids)
        || proposal
            .accepted_hunk_ids
            .iter()
            .any(|id| proposal.rejected_hunk_ids.binary_search(id).is_ok())
        || proposal.original.expected_preimage_sha256 != proposal.preimage_sha256
        || validate_patch(scope, &proposal.original).is_err()
    {
        return Err(HunkSelectionError::Invalid);
    }
    Ok(proposal)
}

/// Derives the selected write from a person's selection over one held patch.
///
/// `preimage` and `proposal` are the exact bytes of the pending patch's plan. The
/// selection must name them, accept a strict subset of their hunks, and still
/// match `current`, the file's bytes now.
pub fn derive_hunk_selection_call(
    original: &ToolCall,
    preimage: &[u8],
    proposal: &[u8],
    current: &[u8],
    selection: &RuntimeHunkSelection,
) -> Result<RuntimeDerivedToolCall, HunkSelectionError> {
    if original.tool_id.as_str() != STRUCTURED_PATCH_TOOL_ID
        || original.tool_version != CONTROLLED_CHANGE_TOOL_VERSION
    {
        return Err(HunkSelectionError::NotNarrowable);
    }
    let patch: StructuredPatchProposal = serde_json::from_slice(&original.arguments.bytes)
        .map_err(|_| HunkSelectionError::NotNarrowable)?;
    let change =
        HunkedTextChange::new(preimage, proposal).map_err(|_| HunkSelectionError::NotNarrowable)?;
    if selection.preimage_sha256 != change.preimage_sha256()
        || selection.proposal_sha256 != change.proposal_sha256()
        || patch.expected_preimage_sha256 != change.preimage_sha256()
    {
        return Err(HunkSelectionError::Stale);
    }
    let selected = select(&change, current, &selection.accepted_hunk_ids)?;
    let arguments = HunkSelectionWriteProposal {
        schema_version: 1,
        original_tool_call_id: original.tool_call_id.as_str().to_owned(),
        original_arguments_sha256: original.arguments.sha256.clone(),
        original: patch,
        preimage_sha256: selected.preimage_sha256().to_owned(),
        proposal_sha256: selected.proposal_sha256().to_owned(),
        accepted_hunk_ids: selected.accepted_hunk_ids().to_vec(),
        rejected_hunk_ids: selected.rejected_hunk_ids().to_vec(),
        selection_sha256: selected.selection_sha256().to_owned(),
        postimage_sha256: selected.postimage_sha256().to_owned(),
    };
    let bytes = serde_json::to_vec(&arguments).map_err(|_| HunkSelectionError::Invalid)?;
    let definition = hunk_selection_tool_definition();
    Ok(RuntimeDerivedToolCall {
        tool_id: ToolId::from_raw(HUNK_SELECTION_TOOL_ID),
        tool_version: CONTROLLED_CHANGE_TOOL_VERSION.to_owned(),
        arguments: ContractPayload {
            schema: definition.input_schema,
            media_type: "application/json".to_owned(),
            sha256: sha256_hex(&bytes),
            bytes,
        },
    })
}

/// Recomputes one selected write over the exact current bytes.
///
/// Returns the change the write would make and the complete selected postimage.
/// Every digest in the arguments must match what the recomputation produces.
pub fn recompute_hunk_selection(
    scope: &CodingWriteScope,
    proposal: &HunkSelectionWriteProposal,
    current: &[u8],
) -> Result<(HunkedTextChange, SelectedTextChange), HunkSelectionError> {
    let original =
        bind_structured_patch_proposal(scope, proposal.original.clone(), current.to_vec())
            .map_err(|_| HunkSelectionError::Mismatch)?;
    let change = HunkedTextChange::new(current, original.postimage())
        .map_err(|_| HunkSelectionError::Mismatch)?;
    if change.preimage_sha256() != proposal.preimage_sha256
        || change.proposal_sha256() != proposal.proposal_sha256
    {
        return Err(HunkSelectionError::Stale);
    }
    let selected = select(&change, current, &proposal.accepted_hunk_ids)?;
    if selected.rejected_hunk_ids() != proposal.rejected_hunk_ids.as_slice()
        || selected.selection_sha256() != proposal.selection_sha256
        || selected.postimage_sha256() != proposal.postimage_sha256
    {
        return Err(HunkSelectionError::Mismatch);
    }
    Ok((change, selected))
}

/// Binds one selected write to the exact current bytes through the existing
/// structured planner, as a whole-file replacement of the exact preimage.
pub fn bind_hunk_selection_proposal(
    scope: &CodingWriteScope,
    proposal: &HunkSelectionWriteProposal,
    current: Vec<u8>,
) -> Result<StructuredFileChangePlan, HunkSelectionError> {
    let (_, selected) = recompute_hunk_selection(scope, proposal, &current)?;
    let current_text =
        String::from_utf8(current.clone()).map_err(|_| HunkSelectionError::Mismatch)?;
    let replacement = String::from_utf8(selected.postimage().to_vec())
        .map_err(|_| HunkSelectionError::Mismatch)?;
    let edit_id = format!("selection:{}", &proposal.selection_sha256[..32]);
    let original = &proposal.original;
    let edit = if syntax_aware(original.language) {
        StructuredEdit::ReplaceSyntaxNode {
            edit_id,
            start_byte: 0,
            end_byte: current.len() as u64,
            expected_node_sha256: proposal.preimage_sha256.clone(),
            replacement,
        }
    } else {
        StructuredEdit::ReplaceExactText {
            edit_id,
            expected: current_text,
            replacement,
        }
    };
    let plan = bind_structured_patch_proposal(
        scope,
        StructuredPatchProposal {
            schema_version: 1,
            change_id: format!("selection-{}", &proposal.selection_sha256[..32]),
            path: original.path.clone(),
            expected_preimage_sha256: proposal.preimage_sha256.clone(),
            intent_sha256: original.intent_sha256.clone(),
            change_plan_sha256: original.change_plan_sha256.clone(),
            language: original.language,
            artifact_class: original.artifact_class,
            edits: vec![edit],
            additional_review_hooks: original.additional_review_hooks.clone(),
            generated: original.generated,
            allow_generated: original.allow_generated,
        },
        current,
    )
    .map_err(|_| HunkSelectionError::Invalid)?;
    if plan.postimage() != selected.postimage() {
        return Err(HunkSelectionError::Invalid);
    }
    Ok(plan)
}

fn select(
    change: &HunkedTextChange,
    current: &[u8],
    accepted: &[String],
) -> Result<SelectedTextChange, HunkSelectionError> {
    let accepted: BTreeSet<String> = accepted.iter().cloned().collect();
    let selected = change
        .select(current, &accepted)
        .map_err(|_| HunkSelectionError::Mismatch)?;
    if selected.accepted_hunk_ids().is_empty() || selected.rejected_hunk_ids().is_empty() {
        return Err(HunkSelectionError::NotASubset);
    }
    Ok(selected)
}

const fn syntax_aware(language: StructuredLanguage) -> bool {
    matches!(
        language,
        StructuredLanguage::Rust
            | StructuredLanguage::Python
            | StructuredLanguage::TypeScript
            | StructuredLanguage::Tsx
            | StructuredLanguage::JavaScript
            | StructuredLanguage::Swift
    )
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_alphanumeric() || (index > 0 && matches!(byte, b'.' | b'_' | b':' | b'-'))
        })
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use agentmage_capability_repository_map::{StructuredArtifactClass, StructuredEdit};
    use agentmage_kernel_contracts::{
        ActionId, CONTRACT_SCHEMA_VERSION, CorrelationId, ToolCallId, WorkspaceId,
    };

    use super::*;
    use crate::coding_changes::structured_patch_tool_definition;

    const PLAIN: &str = "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\n";
    const PYTHON: &str = "def add(left, right):\n    return left - right\n\n\ndef keep():\n    return 1\n\n\ndef sub(left, right):\n    return left + right\n";

    fn scope() -> CodingWriteScope {
        CodingWriteScope::new(
            WorkspaceId::from_raw("workspace-selection"),
            vec![vec!["src".to_owned()]],
        )
        .unwrap()
    }

    fn patch(preimage: &str, language: StructuredLanguage) -> StructuredPatchProposal {
        let edits = if language == StructuredLanguage::Python {
            ["left - right", "left + right"]
                .into_iter()
                .zip(["left + right", "left - right"])
                .enumerate()
                .map(|(index, (old, new))| {
                    let start = if index == 0 {
                        preimage.find(old).unwrap()
                    } else {
                        preimage.rfind(old).unwrap()
                    } as u64;
                    StructuredEdit::ReplaceSyntaxNode {
                        edit_id: format!("edit-{index}"),
                        start_byte: start,
                        end_byte: start + old.len() as u64,
                        expected_node_sha256: sha256_hex(old.as_bytes()),
                        replacement: new.to_owned(),
                    }
                })
                .collect()
        } else {
            vec![
                StructuredEdit::ReplaceExactText {
                    edit_id: "edit-1-two".to_owned(),
                    expected: "two".to_owned(),
                    replacement: "TWO".to_owned(),
                },
                StructuredEdit::ReplaceExactText {
                    edit_id: "edit-2-eight".to_owned(),
                    expected: "eight".to_owned(),
                    replacement: "EIGHT".to_owned(),
                },
            ]
        };
        StructuredPatchProposal {
            schema_version: 1,
            change_id: "change-selection".to_owned(),
            path: vec![
                "src".to_owned(),
                if language == StructuredLanguage::Python {
                    "target.py"
                } else {
                    "target.txt"
                }
                .to_owned(),
            ],
            expected_preimage_sha256: sha256_hex(preimage.as_bytes()),
            intent_sha256: "a".repeat(64),
            change_plan_sha256: "b".repeat(64),
            language,
            artifact_class: StructuredArtifactClass::Code,
            edits,
            additional_review_hooks: Vec::new(),
            generated: false,
            allow_generated: false,
        }
    }

    fn call(proposal: &StructuredPatchProposal) -> ToolCall {
        let bytes = serde_json::to_vec(proposal).unwrap();
        let definition = structured_patch_tool_definition();
        ToolCall {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw("call-original"),
            correlation_id: CorrelationId::from_raw("correlation-selection"),
            action_id: ActionId::from_raw("action-original"),
            tool_id: definition.tool_id,
            tool_version: definition.tool_version,
            arguments: ContractPayload {
                schema: definition.input_schema,
                media_type: "application/json".to_owned(),
                sha256: sha256_hex(&bytes),
                bytes,
            },
        }
    }

    struct Pending {
        call: ToolCall,
        preimage: Vec<u8>,
        proposal: Vec<u8>,
        change: HunkedTextChange,
    }

    fn pending(preimage: &str, language: StructuredLanguage) -> Pending {
        let proposal = patch(preimage, language);
        let plan = bind_structured_patch_proposal(
            &scope(),
            proposal.clone(),
            preimage.as_bytes().to_vec(),
        )
        .unwrap();
        let change = HunkedTextChange::new(plan.preimage(), plan.postimage()).unwrap();
        assert_eq!(change.hunks().len(), 2, "two separate hunks");
        Pending {
            call: call(&proposal),
            preimage: plan.preimage().to_vec(),
            proposal: plan.postimage().to_vec(),
            change,
        }
    }

    fn selection(pending: &Pending, accepted: &[usize]) -> RuntimeHunkSelection {
        let mut accepted_hunk_ids = accepted
            .iter()
            .map(|index| pending.change.hunks()[*index].hunk_id().to_owned())
            .collect::<Vec<_>>();
        accepted_hunk_ids.sort();
        RuntimeHunkSelection {
            preimage_sha256: pending.change.preimage_sha256().to_owned(),
            proposal_sha256: pending.change.proposal_sha256().to_owned(),
            accepted_hunk_ids,
        }
    }

    #[test]
    fn a_strict_subset_derives_one_self_contained_write_of_only_the_accepted_hunks() {
        for (preimage, language, expected) in [
            (
                PLAIN,
                StructuredLanguage::PlainText,
                PLAIN.replace("eight", "EIGHT"),
            ),
            (
                PYTHON,
                StructuredLanguage::Python,
                PYTHON.replacen("left + right", "left - right", 1),
            ),
        ] {
            let pending = pending(preimage, language);
            let derived = derive_hunk_selection_call(
                &pending.call,
                &pending.preimage,
                &pending.proposal,
                &pending.preimage,
                &selection(&pending, &[1]),
            )
            .unwrap();
            assert_eq!(derived.tool_id.as_str(), HUNK_SELECTION_TOOL_ID);
            assert_eq!(
                derived.arguments.schema,
                hunk_selection_tool_definition().input_schema
            );
            assert_eq!(
                derived.arguments.sha256,
                sha256_hex(&derived.arguments.bytes)
            );
            let proposal =
                decode_hunk_selection_proposal(&scope(), &derived.arguments.bytes).unwrap();
            assert_eq!(proposal.original_tool_call_id, "call-original");
            assert_eq!(
                proposal.original_arguments_sha256,
                pending.call.arguments.sha256
            );
            assert_eq!(proposal.accepted_hunk_ids.len(), 1);
            assert_eq!(proposal.rejected_hunk_ids.len(), 1);
            assert_eq!(proposal.postimage_sha256, sha256_hex(expected.as_bytes()));
            // The derived write is bound to the same exact preimage and writes
            // only the accepted hunk; the rejected hunk keeps its preimage line.
            let plan = bind_hunk_selection_proposal(&scope(), &proposal, pending.preimage.clone())
                .unwrap();
            assert_eq!(plan.preimage(), pending.preimage.as_slice());
            assert_eq!(plan.postimage(), expected.as_bytes());
        }
    }

    #[test]
    fn stale_mismatched_and_non_subset_selections_are_refused() {
        let pending = pending(PLAIN, StructuredLanguage::PlainText);
        let derive = |current: &[u8], selection: &RuntimeHunkSelection| {
            derive_hunk_selection_call(
                &pending.call,
                &pending.preimage,
                &pending.proposal,
                current,
                selection,
            )
            .err()
        };
        let mut stale = selection(&pending, &[0]);
        stale.proposal_sha256 = "c".repeat(64);
        assert_eq!(
            derive(&pending.preimage, &stale),
            Some(HunkSelectionError::Stale)
        );
        let mut unknown = selection(&pending, &[0]);
        unknown.accepted_hunk_ids = vec!["d".repeat(64)];
        assert_eq!(
            derive(&pending.preimage, &unknown),
            Some(HunkSelectionError::Mismatch)
        );
        // A person edited the file after the proposal: the selection is refused.
        let edited = PLAIN.replace("five", "five (edited by a person)");
        assert_eq!(
            derive(edited.as_bytes(), &selection(&pending, &[0])),
            Some(HunkSelectionError::Mismatch)
        );
        for accepted in [&[][..], &[0, 1][..]] {
            assert_eq!(
                derive(&pending.preimage, &selection(&pending, accepted)),
                Some(HunkSelectionError::NotASubset)
            );
        }
        let mut create = pending.call.clone();
        create.tool_id = ToolId::from_raw(crate::coding_changes::CONTROLLED_CREATE_TOOL_ID);
        assert_eq!(
            derive_hunk_selection_call(
                &create,
                &pending.preimage,
                &pending.proposal,
                &pending.preimage,
                &selection(&pending, &[0]),
            )
            .err(),
            Some(HunkSelectionError::NotNarrowable)
        );
    }

    #[test]
    fn the_selected_write_refuses_drift_and_tampered_arguments() {
        let pending = pending(PLAIN, StructuredLanguage::PlainText);
        let derived = derive_hunk_selection_call(
            &pending.call,
            &pending.preimage,
            &pending.proposal,
            &pending.preimage,
            &selection(&pending, &[0]),
        )
        .unwrap();
        let proposal = decode_hunk_selection_proposal(&scope(), &derived.arguments.bytes).unwrap();
        // A concurrent human edit after the approval is refused at the write.
        let edited = PLAIN.replace("five", "five (edited by a person)");
        assert!(bind_hunk_selection_proposal(&scope(), &proposal, edited.into_bytes()).is_err());
        // Swapping the accepted and rejected hunks changes what would be written.
        let mut swapped = proposal.clone();
        std::mem::swap(
            &mut swapped.accepted_hunk_ids,
            &mut swapped.rejected_hunk_ids,
        );
        assert_eq!(
            recompute_hunk_selection(&scope(), &swapped, &pending.preimage).err(),
            Some(HunkSelectionError::Mismatch)
        );
        let mut postimage = proposal.clone();
        postimage.postimage_sha256 = "e".repeat(64);
        assert_eq!(
            recompute_hunk_selection(&scope(), &postimage, &pending.preimage).err(),
            Some(HunkSelectionError::Mismatch)
        );
        // Closed structural validation of the arguments.
        let encode = |proposal: &HunkSelectionWriteProposal| serde_json::to_vec(proposal).unwrap();
        let mut overlapping = proposal.clone();
        overlapping.rejected_hunk_ids = overlapping.accepted_hunk_ids.clone();
        let mut unsorted = proposal.clone();
        unsorted.accepted_hunk_ids = vec!["f".repeat(64), "0".repeat(64)];
        let mut foreign = proposal.clone();
        foreign.preimage_sha256 = "1".repeat(64);
        let mut outside = proposal.clone();
        outside.original.path = vec!["docs".to_owned(), "target.txt".to_owned()];
        for invalid in [overlapping, unsorted, foreign, outside] {
            assert_eq!(
                decode_hunk_selection_proposal(&scope(), &encode(&invalid)).err(),
                Some(HunkSelectionError::Invalid)
            );
        }
        let mut extra: serde_json::Value = serde_json::from_slice(&encode(&proposal)).unwrap();
        extra["all_hunks"] = serde_json::Value::Bool(true);
        assert_eq!(
            decode_hunk_selection_proposal(&scope(), &serde_json::to_vec(&extra).unwrap()).err(),
            Some(HunkSelectionError::Invalid)
        );
    }

    #[test]
    fn the_selected_write_is_registered_for_shells_only() {
        let mut registry = ToolRegistry::new();
        register_hunk_selection_runtime_tool(&mut registry, scope()).unwrap();
        let definition = hunk_selection_tool_definition();
        assert!(!registry.is_model_proposable(&definition.tool_id, &definition.tool_version));
        assert!(registry.list_model_tools().is_empty());
        assert_eq!(registry.list_tools(), vec![&definition]);
        assert_eq!(
            definition.input_schema.schema_sha256,
            sha256_hex(HUNK_SELECTION_INPUT_SCHEMA_JSON.as_bytes())
        );
        let schema: serde_json::Value =
            serde_json::from_str(HUNK_SELECTION_INPUT_SCHEMA_JSON).unwrap();
        assert_eq!(schema["$id"], HUNK_SELECTION_INPUT_SCHEMA_ID);
        assert_eq!(schema["additionalProperties"], false);
    }
}
