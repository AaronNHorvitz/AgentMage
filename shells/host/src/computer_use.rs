//! Confirmed local computer-use planning and before/after evidence contracts.

use serde::{Deserialize, Serialize};

/// Why structured tools did not satisfy the requested operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuredToolDisposition {
    /// No structured tool exists for the exact operation.
    Unavailable,
    /// A structured tool exists but is currently unsupported.
    Unsupported,
    /// A structured tool can perform the operation; computer use is forbidden.
    Available,
}

/// Closed visible local action family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputerUseAction {
    /// Reversible pointer activation.
    Click,
    /// Form submission with external effect.
    Submit,
    /// Upload exact approved bytes.
    Upload,
    /// Send exact approved content.
    Send,
    /// Another explicitly classified external state change.
    ExternalStateChange,
}

impl ComputerUseAction {
    const fn irreversible(self) -> bool {
        matches!(
            self,
            Self::Submit | Self::Upload | Self::Send | Self::ExternalStateChange
        )
    }
}

/// Exact visible action proposed to a separately owned native adapter.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComputerUseRequest {
    /// Stable attempt identity.
    pub action_id: String,
    /// Exact local application identity.
    pub application_id: String,
    /// Digest of the exact foreground window identity and bounds.
    pub foreground_window_sha256: String,
    /// Digest of the immediately preceding screenshot.
    pub before_screenshot_sha256: String,
    /// Exact x coordinate inside the bound window.
    pub x: u32,
    /// Exact y coordinate inside the bound window.
    pub y: u32,
    /// Digest of the visible control identity.
    pub control_sha256: String,
    /// Digest of the exact payload, or absent for a payload-free click.
    pub payload_sha256: Option<String>,
    /// Digest of the exact current precondition.
    pub precondition_sha256: String,
    /// Digest of the exact single-use action grant.
    pub grant_sha256: String,
    /// Structured-tool disposition checked before fallback.
    pub structured_tool_disposition: StructuredToolDisposition,
    /// Closed action class.
    pub action: ComputerUseAction,
    /// Digest of a separate current user confirmation for irreversible actions.
    pub confirmation_sha256: Option<String>,
}

/// Immutable visible preview; possession grants no action authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComputerUsePreview {
    /// Stable attempt identity.
    pub action_id: String,
    /// Exact application identity.
    pub application_id: String,
    /// Bound foreground window.
    pub foreground_window_sha256: String,
    /// Bound before screenshot.
    pub before_screenshot_sha256: String,
    /// Exact coordinates.
    pub coordinates: (u32, u32),
    /// Bound visible control.
    pub control_sha256: String,
    /// Bound payload.
    pub payload_sha256: Option<String>,
    /// Bound precondition.
    pub precondition_sha256: String,
    /// Exact action grant.
    pub grant_sha256: String,
    /// Separate confirmation when required.
    pub confirmation_sha256: Option<String>,
    /// Closed action class.
    pub action: ComputerUseAction,
}

/// Terminal adapter observation for one attempted visible action.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComputerUseReceipt {
    /// Exact action identity.
    pub action_id: String,
    /// Digest of the verified post-action screenshot.
    pub after_screenshot_sha256: String,
    /// Digest of exact observed application state after the action.
    pub after_state_sha256: String,
    /// Whether the requested effect is known to have occurred exactly once.
    pub effect_confirmed: bool,
    /// Whether the effect outcome is uncertain and requires reconciliation.
    pub uncertain: bool,
    /// Whether owned native work terminated.
    pub terminated: bool,
}

/// Stable fail-closed request or receipt error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComputerUseError {
    /// Request identity, fallback, foreground state, grant, or confirmation is invalid.
    RequestDenied,
    /// Terminal evidence is missing, contradictory, uncertain, or not terminated.
    ReceiptDenied,
}

/// Builds one exact preview only after structured tools are unavailable or unsupported.
pub fn prepare_computer_use(
    request: &ComputerUseRequest,
) -> Result<ComputerUsePreview, ComputerUseError> {
    if !valid_id(&request.action_id)
        || !valid_id(&request.application_id)
        || !valid_sha256(&request.foreground_window_sha256)
        || !valid_sha256(&request.before_screenshot_sha256)
        || !valid_sha256(&request.control_sha256)
        || !valid_sha256(&request.precondition_sha256)
        || !valid_sha256(&request.grant_sha256)
        || request
            .payload_sha256
            .as_deref()
            .is_some_and(|value| !valid_sha256(value))
        || request.structured_tool_disposition == StructuredToolDisposition::Available
        || (request.action.irreversible()
            != request
                .confirmation_sha256
                .as_deref()
                .is_some_and(valid_sha256))
        || (request.action != ComputerUseAction::Click && request.payload_sha256.is_none())
    {
        return Err(ComputerUseError::RequestDenied);
    }
    Ok(ComputerUsePreview {
        action_id: request.action_id.clone(),
        application_id: request.application_id.clone(),
        foreground_window_sha256: request.foreground_window_sha256.clone(),
        before_screenshot_sha256: request.before_screenshot_sha256.clone(),
        coordinates: (request.x, request.y),
        control_sha256: request.control_sha256.clone(),
        payload_sha256: request.payload_sha256.clone(),
        precondition_sha256: request.precondition_sha256.clone(),
        grant_sha256: request.grant_sha256.clone(),
        confirmation_sha256: request.confirmation_sha256.clone(),
        action: request.action,
    })
}

/// Verifies exact before/after evidence and refuses uncertain completion or blind repeat.
pub fn verify_computer_use(
    request: &ComputerUseRequest,
    preview: &ComputerUsePreview,
    receipt: &ComputerUseReceipt,
) -> Result<(), ComputerUseError> {
    if prepare_computer_use(request)? != *preview
        || receipt.action_id != request.action_id
        || !valid_sha256(&receipt.after_screenshot_sha256)
        || !valid_sha256(&receipt.after_state_sha256)
        || receipt.after_screenshot_sha256 == request.before_screenshot_sha256
        || receipt.uncertain
        || !receipt.effect_confirmed
        || !receipt.terminated
    {
        return Err(ComputerUseError::ReceiptDenied);
    }
    Ok(())
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn hash(c: char) -> String {
        c.to_string().repeat(64)
    }
    fn request(action: ComputerUseAction) -> ComputerUseRequest {
        ComputerUseRequest {
            action_id: "action-1".into(),
            application_id: "app-1".into(),
            foreground_window_sha256: hash('a'),
            before_screenshot_sha256: hash('b'),
            x: 10,
            y: 20,
            control_sha256: hash('c'),
            payload_sha256: (action != ComputerUseAction::Click).then(|| hash('d')),
            precondition_sha256: hash('e'),
            grant_sha256: hash('f'),
            structured_tool_disposition: StructuredToolDisposition::Unavailable,
            action,
            confirmation_sha256: action.irreversible().then(|| hash('1')),
        }
    }
    #[test]
    fn sprint_84_structured_tool_availability_forbids_fallback() {
        let mut value = request(ComputerUseAction::Click);
        value.structured_tool_disposition = StructuredToolDisposition::Available;
        assert_eq!(
            prepare_computer_use(&value),
            Err(ComputerUseError::RequestDenied)
        );
    }
    #[test]
    fn sprint_84_irreversible_actions_require_separate_confirmation() {
        for action in [
            ComputerUseAction::Submit,
            ComputerUseAction::Upload,
            ComputerUseAction::Send,
            ComputerUseAction::ExternalStateChange,
        ] {
            let mut value = request(action);
            value.confirmation_sha256 = None;
            assert_eq!(
                prepare_computer_use(&value),
                Err(ComputerUseError::RequestDenied)
            );
        }
    }
    #[test]
    fn sprint_84_preview_binds_every_visible_action_field() {
        let value = request(ComputerUseAction::Submit);
        let preview = prepare_computer_use(&value).expect("preview");
        assert_eq!(preview.coordinates, (10, 20));
        assert_eq!(preview.before_screenshot_sha256, hash('b'));
        assert_eq!(preview.confirmation_sha256, Some(hash('1')));
    }
    #[test]
    fn sprint_84_receipt_requires_changed_screenshot_exact_state_and_certain_termination() {
        let value = request(ComputerUseAction::Submit);
        let preview = prepare_computer_use(&value).expect("preview");
        let mut receipt = ComputerUseReceipt {
            action_id: value.action_id.clone(),
            after_screenshot_sha256: hash('2'),
            after_state_sha256: hash('3'),
            effect_confirmed: true,
            uncertain: false,
            terminated: true,
        };
        assert!(verify_computer_use(&value, &preview, &receipt).is_ok());
        receipt.uncertain = true;
        assert_eq!(
            verify_computer_use(&value, &preview, &receipt),
            Err(ComputerUseError::ReceiptDenied)
        );
    }
}
