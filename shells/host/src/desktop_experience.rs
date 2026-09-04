//! Effect-free desktop conversation and workspace projections over kernel-owned identities.
#![allow(missing_docs)]

use agentmage_kernel_contracts::{CONTRACT_SCHEMA_VERSION, ConversationStatus};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::Write;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopArchitectureSelection {
    pub architecture_id: String,
    pub thin_local_client: bool,
    pub kernel_protocol_only: bool,
    pub external_web_service: bool,
    pub client_authority: bool,
    pub client_canonical_state: bool,
    pub local_assets_only: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationNavigationFilter {
    pub from_local_date: Option<String>,
    pub to_local_date: Option<String>,
    pub workspace_id: Option<String>,
    pub project_id: Option<String>,
    pub model_profile_id: Option<String>,
    pub tag: Option<String>,
    pub status: Option<ConversationStatus>,
    pub pinned: Option<bool>,
    pub archived: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopConversationRow {
    pub conversation_id: String,
    pub title: String,
    pub local_date: String,
    pub updated_at_epoch_ms: u64,
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub model_profile_id: String,
    pub tags: Vec<String>,
    pub status: ConversationStatus,
    pub pinned: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesktopTranscriptItemKind {
    Markdown,
    Link,
    Task,
    Backlink,
    Preview,
    Citation,
    Tool,
    Approval,
    Error,
    Checkpoint,
    Branch,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopTranscriptItem {
    pub item_id: String,
    pub kind: DesktopTranscriptItemKind,
    pub label: String,
    pub content_sha256: String,
    pub source_reference: Option<String>,
    pub executable: bool,
    pub authority_granting: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesktopConversationAction {
    Open,
    Continue,
    Branch,
    Rename,
    Archive,
    Export,
    Delete,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopActionPreview {
    pub schema_version: u16,
    pub preview_id: String,
    pub conversation_id: String,
    pub action: DesktopConversationAction,
    pub target_turn_id: Option<String>,
    pub proposed_title: Option<String>,
    pub kernel_request_sha256: String,
    pub approval_required: bool,
    pub approval_present: bool,
    pub effect_applied: bool,
    pub preview_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopCheckpointSnapshot {
    pub checkpoint_id: String,
    pub files_sha256: String,
    pub instructions_sha256: String,
    pub repository_sha256: String,
    pub model_sha256: String,
    pub permissions_sha256: String,
    pub next_action_sha256: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointDimension {
    Files,
    Instructions,
    Repository,
    Model,
    Permissions,
    NextAction,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopCheckpointComparison {
    pub before_checkpoint_id: String,
    pub after_checkpoint_id: String,
    pub changed_dimensions: Vec<CheckpointDimension>,
    pub comparison_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopWorkspaceSelection {
    pub selection_id: String,
    pub workspace_id: String,
    pub display_name: String,
    pub relative_root_components: Vec<String>,
    pub user_selected: bool,
    pub local_only: bool,
    pub obsidian_dependency: bool,
    pub client_path_authority: bool,
    pub selection_sha256: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesktopApprovalKind {
    ExactDiff,
    Command,
    Write,
    Delete,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopApprovalScreen {
    pub screen_id: String,
    pub kind: DesktopApprovalKind,
    pub operation_id: String,
    pub target_label: String,
    pub preview_sha256: String,
    pub policy_sha256: String,
    pub grant_id: Option<String>,
    pub approved: bool,
    pub effect_applied: bool,
    pub screen_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopApprovalInput<'a> {
    pub screen_id: &'a str,
    pub kind: DesktopApprovalKind,
    pub operation_id: &'a str,
    pub target_label: &'a str,
    pub preview_sha256: &'a str,
    pub policy_sha256: &'a str,
    pub grant_id: Option<String>,
    pub approved: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesktopExperienceError {
    InvalidInput,
    AuthorityBoundary,
    ApprovalRequired,
}

impl DesktopExperienceError {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "desktop.input.invalid",
            Self::AuthorityBoundary => "desktop.authority.boundary",
            Self::ApprovalRequired => "desktop.approval.required",
        }
    }
}

impl std::fmt::Display for DesktopExperienceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}
impl std::error::Error for DesktopExperienceError {}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn local_date(value: &str) -> bool {
    value.len() == 10
        && value.as_bytes()[4] == b'-'
        && value.as_bytes()[7] == b'-'
        && value
            .bytes()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
}

fn bounded_label(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 512 && !value.chars().any(char::is_control)
}

fn digest(parts: impl IntoIterator<Item = impl AsRef<[u8]>>) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        let bytes = part.as_ref();
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
    }
    let mut output = String::with_capacity(64);
    for byte in hasher.finalize() {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

pub fn validate_desktop_architecture(
    selection: &DesktopArchitectureSelection,
) -> Result<(), DesktopExperienceError> {
    if !identifier(&selection.architecture_id)
        || !selection.thin_local_client
        || !selection.kernel_protocol_only
        || selection.external_web_service
        || selection.client_authority
        || selection.client_canonical_state
        || !selection.local_assets_only
    {
        return Err(DesktopExperienceError::AuthorityBoundary);
    }
    Ok(())
}

pub fn project_conversation_navigation(
    filter: &ConversationNavigationFilter,
    rows: &[DesktopConversationRow],
) -> Result<Vec<DesktopConversationRow>, DesktopExperienceError> {
    if filter
        .from_local_date
        .as_deref()
        .is_some_and(|value| !local_date(value))
        || filter
            .to_local_date
            .as_deref()
            .is_some_and(|value| !local_date(value))
        || filter
            .from_local_date
            .as_ref()
            .zip(filter.to_local_date.as_ref())
            .is_some_and(|(from, to)| from > to)
    {
        return Err(DesktopExperienceError::InvalidInput);
    }
    let mut projected = Vec::new();
    for row in rows {
        if !identifier(&row.conversation_id)
            || !bounded_label(&row.title)
            || !local_date(&row.local_date)
            || row.updated_at_epoch_ms == 0
            || !identifier(&row.workspace_id)
            || !identifier(&row.model_profile_id)
            || row
                .project_id
                .as_deref()
                .is_some_and(|value| !identifier(value))
            || row.tags.iter().any(|tag| !identifier(tag))
        {
            return Err(DesktopExperienceError::InvalidInput);
        }
        let archived = row.status == ConversationStatus::Archived;
        let admitted = filter
            .from_local_date
            .as_ref()
            .is_none_or(|from| &row.local_date >= from)
            && filter
                .to_local_date
                .as_ref()
                .is_none_or(|to| &row.local_date <= to)
            && filter
                .workspace_id
                .as_ref()
                .is_none_or(|value| &row.workspace_id == value)
            && filter
                .project_id
                .as_ref()
                .is_none_or(|value| row.project_id.as_ref() == Some(value))
            && filter
                .model_profile_id
                .as_ref()
                .is_none_or(|value| &row.model_profile_id == value)
            && filter
                .tag
                .as_ref()
                .is_none_or(|value| row.tags.contains(value))
            && filter.status.is_none_or(|value| row.status == value)
            && filter.pinned.is_none_or(|value| row.pinned == value)
            && filter.archived.is_none_or(|value| archived == value);
        if admitted {
            projected.push(row.clone());
        }
    }
    projected.sort_by(|left, right| {
        right
            .pinned
            .cmp(&left.pinned)
            .then_with(|| right.updated_at_epoch_ms.cmp(&left.updated_at_epoch_ms))
            .then_with(|| left.conversation_id.cmp(&right.conversation_id))
    });
    Ok(projected)
}

pub fn validate_transcript_items(
    items: &[DesktopTranscriptItem],
) -> Result<(), DesktopExperienceError> {
    if items.len() > 4_096
        || items.iter().any(|item| {
            !identifier(&item.item_id)
                || !bounded_label(&item.label)
                || !sha256(&item.content_sha256)
                || item
                    .source_reference
                    .as_deref()
                    .is_some_and(|value| !identifier(value))
                || item.executable
                || item.authority_granting
        })
    {
        return Err(DesktopExperienceError::AuthorityBoundary);
    }
    Ok(())
}

pub fn build_action_preview(
    preview_id: &str,
    conversation_id: &str,
    action: DesktopConversationAction,
    target_turn_id: Option<String>,
    proposed_title: Option<String>,
    kernel_request_sha256: &str,
    approval_present: bool,
) -> Result<DesktopActionPreview, DesktopExperienceError> {
    let approval_required = matches!(action, DesktopConversationAction::Delete);
    if !identifier(preview_id)
        || !identifier(conversation_id)
        || !sha256(kernel_request_sha256)
        || target_turn_id
            .as_deref()
            .is_some_and(|value| !identifier(value))
        || proposed_title
            .as_deref()
            .is_some_and(|value| !bounded_label(value))
        || matches!(action, DesktopConversationAction::Branch) != target_turn_id.is_some()
        || matches!(action, DesktopConversationAction::Rename) != proposed_title.is_some()
    {
        return Err(DesktopExperienceError::InvalidInput);
    }
    if approval_required && !approval_present {
        return Err(DesktopExperienceError::ApprovalRequired);
    }
    let action_name = format!("{action:?}");
    let preview_sha256 = digest([
        preview_id,
        conversation_id,
        action_name.as_str(),
        target_turn_id.as_deref().unwrap_or(""),
        proposed_title.as_deref().unwrap_or(""),
        kernel_request_sha256,
    ]);
    Ok(DesktopActionPreview {
        schema_version: CONTRACT_SCHEMA_VERSION,
        preview_id: preview_id.to_owned(),
        conversation_id: conversation_id.to_owned(),
        action,
        target_turn_id,
        proposed_title,
        kernel_request_sha256: kernel_request_sha256.to_owned(),
        approval_required,
        approval_present,
        effect_applied: false,
        preview_sha256,
    })
}

pub fn compare_checkpoints(
    before: &DesktopCheckpointSnapshot,
    after: &DesktopCheckpointSnapshot,
) -> Result<DesktopCheckpointComparison, DesktopExperienceError> {
    let valid = |snapshot: &DesktopCheckpointSnapshot| {
        identifier(&snapshot.checkpoint_id)
            && [
                &snapshot.files_sha256,
                &snapshot.instructions_sha256,
                &snapshot.repository_sha256,
                &snapshot.model_sha256,
                &snapshot.permissions_sha256,
                &snapshot.next_action_sha256,
            ]
            .into_iter()
            .all(|value| sha256(value))
    };
    if !valid(before) || !valid(after) || before.checkpoint_id == after.checkpoint_id {
        return Err(DesktopExperienceError::InvalidInput);
    }
    let mut changed_dimensions = Vec::new();
    for (dimension, changed) in [
        (
            CheckpointDimension::Files,
            before.files_sha256 != after.files_sha256,
        ),
        (
            CheckpointDimension::Instructions,
            before.instructions_sha256 != after.instructions_sha256,
        ),
        (
            CheckpointDimension::Repository,
            before.repository_sha256 != after.repository_sha256,
        ),
        (
            CheckpointDimension::Model,
            before.model_sha256 != after.model_sha256,
        ),
        (
            CheckpointDimension::Permissions,
            before.permissions_sha256 != after.permissions_sha256,
        ),
        (
            CheckpointDimension::NextAction,
            before.next_action_sha256 != after.next_action_sha256,
        ),
    ] {
        if changed {
            changed_dimensions.push(dimension);
        }
    }
    let dimensions = format!("{changed_dimensions:?}");
    let comparison_sha256 = digest([
        before.checkpoint_id.as_str(),
        after.checkpoint_id.as_str(),
        dimensions.as_str(),
    ]);
    Ok(DesktopCheckpointComparison {
        before_checkpoint_id: before.checkpoint_id.clone(),
        after_checkpoint_id: after.checkpoint_id.clone(),
        changed_dimensions,
        comparison_sha256,
    })
}

pub fn select_workspace(
    selection_id: &str,
    workspace_id: &str,
    display_name: &str,
    relative_root_components: Vec<String>,
    user_selected: bool,
) -> Result<DesktopWorkspaceSelection, DesktopExperienceError> {
    if !identifier(selection_id)
        || !identifier(workspace_id)
        || !bounded_label(display_name)
        || relative_root_components.is_empty()
        || relative_root_components.len() > 64
        || relative_root_components.iter().any(|component| {
            component.is_empty()
                || component.len() > 255
                || matches!(component.as_str(), "." | "..")
                || component.contains(['/', '\\', '\0'])
        })
        || !user_selected
    {
        return Err(DesktopExperienceError::InvalidInput);
    }
    let mut parts = vec![selection_id, workspace_id, display_name];
    parts.extend(relative_root_components.iter().map(String::as_str));
    let selection_sha256 = digest(parts);
    Ok(DesktopWorkspaceSelection {
        selection_id: selection_id.to_owned(),
        workspace_id: workspace_id.to_owned(),
        display_name: display_name.to_owned(),
        relative_root_components,
        user_selected,
        local_only: true,
        obsidian_dependency: false,
        client_path_authority: false,
        selection_sha256,
    })
}

pub fn build_approval_screen(
    input: DesktopApprovalInput<'_>,
) -> Result<DesktopApprovalScreen, DesktopExperienceError> {
    if !identifier(input.screen_id)
        || !identifier(input.operation_id)
        || !bounded_label(input.target_label)
        || !sha256(input.preview_sha256)
        || !sha256(input.policy_sha256)
        || input
            .grant_id
            .as_deref()
            .is_some_and(|value| !identifier(value))
        || input.approved != input.grant_id.is_some()
    {
        return Err(DesktopExperienceError::InvalidInput);
    }
    let kind_name = format!("{:?}", input.kind);
    let screen_sha256 = digest([
        input.screen_id,
        kind_name.as_str(),
        input.operation_id,
        input.target_label,
        input.preview_sha256,
        input.policy_sha256,
        input.grant_id.as_deref().unwrap_or(""),
    ]);
    Ok(DesktopApprovalScreen {
        screen_id: input.screen_id.to_owned(),
        kind: input.kind,
        operation_id: input.operation_id.to_owned(),
        target_label: input.target_label.to_owned(),
        preview_sha256: input.preview_sha256.to_owned(),
        policy_sha256: input.policy_sha256.to_owned(),
        grant_id: input.grant_id,
        approved: input.approved,
        effect_applied: false,
        screen_sha256,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const SHA_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn row(
        id: &str,
        date: &str,
        status: ConversationStatus,
        pinned: bool,
    ) -> DesktopConversationRow {
        DesktopConversationRow {
            conversation_id: id.to_owned(),
            title: format!("Conversation {id}"),
            local_date: date.to_owned(),
            updated_at_epoch_ms: 10,
            workspace_id: "workspace-1".to_owned(),
            project_id: Some("project-1".to_owned()),
            model_profile_id: "profile-1".to_owned(),
            tags: vec!["tag-1".to_owned()],
            status,
            pinned,
        }
    }

    #[test]
    fn architecture_has_no_second_authority_boundary() {
        let selection = DesktopArchitectureSelection {
            architecture_id: "desktop-local-thin-client-v1".to_owned(),
            thin_local_client: true,
            kernel_protocol_only: true,
            external_web_service: false,
            client_authority: false,
            client_canonical_state: false,
            local_assets_only: true,
        };
        assert_eq!(validate_desktop_architecture(&selection), Ok(()));
        let mut invalid = selection;
        invalid.client_authority = true;
        assert_eq!(
            validate_desktop_architecture(&invalid),
            Err(DesktopExperienceError::AuthorityBoundary)
        );
    }

    #[test]
    fn navigation_filters_every_declared_dimension() {
        let filter = ConversationNavigationFilter {
            from_local_date: Some("2026-09-01".to_owned()),
            to_local_date: Some("2026-09-04".to_owned()),
            workspace_id: Some("workspace-1".to_owned()),
            project_id: Some("project-1".to_owned()),
            model_profile_id: Some("profile-1".to_owned()),
            tag: Some("tag-1".to_owned()),
            status: Some(ConversationStatus::Active),
            pinned: Some(true),
            archived: Some(false),
        };
        let rows = vec![
            row(
                "conversation-1",
                "2026-09-03",
                ConversationStatus::Active,
                true,
            ),
            row(
                "conversation-2",
                "2026-09-03",
                ConversationStatus::Archived,
                true,
            ),
        ];
        assert_eq!(
            project_conversation_navigation(&filter, &rows)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn transcript_items_are_inert() {
        let item = DesktopTranscriptItem {
            item_id: "item-1".to_owned(),
            kind: DesktopTranscriptItemKind::Citation,
            label: "Source citation".to_owned(),
            content_sha256: SHA_A.to_owned(),
            source_reference: Some("source-1".to_owned()),
            executable: false,
            authority_granting: false,
        };
        assert_eq!(
            validate_transcript_items(std::slice::from_ref(&item)),
            Ok(())
        );
        let mut invalid = item;
        invalid.executable = true;
        assert_eq!(
            validate_transcript_items(&[invalid]),
            Err(DesktopExperienceError::AuthorityBoundary)
        );
    }

    #[test]
    fn every_conversation_action_remains_a_preview() {
        for action in [
            DesktopConversationAction::Open,
            DesktopConversationAction::Continue,
            DesktopConversationAction::Branch,
            DesktopConversationAction::Rename,
            DesktopConversationAction::Archive,
            DesktopConversationAction::Export,
            DesktopConversationAction::Delete,
        ] {
            let target = (action == DesktopConversationAction::Branch).then(|| "turn-1".to_owned());
            let title =
                (action == DesktopConversationAction::Rename).then(|| "New title".to_owned());
            let approval = action == DesktopConversationAction::Delete;
            let preview = build_action_preview(
                "preview-1",
                "conversation-1",
                action,
                target,
                title,
                SHA_A,
                approval,
            )
            .unwrap();
            assert!(!preview.effect_applied);
        }
    }

    #[test]
    fn delete_requires_explicit_approval() {
        assert_eq!(
            build_action_preview(
                "preview-1",
                "conversation-1",
                DesktopConversationAction::Delete,
                None,
                None,
                SHA_A,
                false,
            ),
            Err(DesktopExperienceError::ApprovalRequired)
        );
    }

    #[test]
    fn checkpoint_comparison_reports_exact_dimensions() {
        let before = DesktopCheckpointSnapshot {
            checkpoint_id: "checkpoint-1".to_owned(),
            files_sha256: SHA_A.to_owned(),
            instructions_sha256: SHA_A.to_owned(),
            repository_sha256: SHA_A.to_owned(),
            model_sha256: SHA_A.to_owned(),
            permissions_sha256: SHA_A.to_owned(),
            next_action_sha256: SHA_A.to_owned(),
        };
        let mut after = before.clone();
        after.checkpoint_id = "checkpoint-2".to_owned();
        after.repository_sha256 = SHA_B.to_owned();
        after.permissions_sha256 = SHA_B.to_owned();
        assert_eq!(
            compare_checkpoints(&before, &after)
                .unwrap()
                .changed_dimensions,
            vec![
                CheckpointDimension::Repository,
                CheckpointDimension::Permissions
            ]
        );
    }

    #[test]
    fn workspace_selection_is_local_relative_and_obsidian_independent() {
        let selection = select_workspace(
            "selection-1",
            "workspace-1",
            "Notes",
            vec!["vault".to_owned(), "notes".to_owned()],
            true,
        )
        .unwrap();
        assert!(selection.local_only);
        assert!(!selection.obsidian_dependency);
        assert!(!selection.client_path_authority);
        assert!(
            select_workspace(
                "selection-1",
                "workspace-1",
                "Notes",
                vec!["..".to_owned()],
                true,
            )
            .is_err()
        );
    }

    #[test]
    fn approval_screens_carry_no_effect() {
        for kind in [
            DesktopApprovalKind::ExactDiff,
            DesktopApprovalKind::Command,
            DesktopApprovalKind::Write,
            DesktopApprovalKind::Delete,
        ] {
            let screen = build_approval_screen(DesktopApprovalInput {
                screen_id: "screen-1",
                kind,
                operation_id: "operation-1",
                target_label: "notes/file.md",
                preview_sha256: SHA_A,
                policy_sha256: SHA_B,
                grant_id: Some("grant-1".to_owned()),
                approved: true,
            })
            .unwrap();
            assert!(!screen.effect_applied);
        }
    }

    #[test]
    fn approval_claim_requires_a_grant_identity() {
        assert_eq!(
            build_approval_screen(DesktopApprovalInput {
                screen_id: "screen-1",
                kind: DesktopApprovalKind::Write,
                operation_id: "operation-1",
                target_label: "notes/file.md",
                preview_sha256: SHA_A,
                policy_sha256: SHA_B,
                grant_id: None,
                approved: true,
            }),
            Err(DesktopExperienceError::InvalidInput)
        );
    }
}
