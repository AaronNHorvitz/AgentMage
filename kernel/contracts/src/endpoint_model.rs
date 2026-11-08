// SPDX-License-Identifier: Apache-2.0
//! Selected-endpoint exchanges, without native model attestation or authority.

use std::time::Duration;

use crate::{EndpointTokenAccounting, ProviderTokenUsage, RuntimeModelBinding};

/// The person's selected address and requested name, never verified weights.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EndpointModelSelection {
    /// Canonical address supplied by the credential-owning transport.
    pub endpoint: String,
    /// Requested model name, without an artifact or serving-process claim.
    pub model: String,
}

/// Observations of the exact JSON message array, before server-side rendering.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct EndpointModelContext {
    /// Number of ordered messages in the request body.
    pub message_count: usize,
    /// Length of the serialized JSON message array, including roles and escaping.
    pub serialized_messages_bytes: usize,
    /// SHA-256 of that array, excluding credentials and other request fields.
    pub serialized_messages_sha256: String,
}

/// Fingerprint of the frozen body, excluding headers and credentials.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct EndpointModelBody {
    /// SHA-256 of exactly the JSON body prepared for transmission.
    pub request_body_sha256: String,
    /// Exact body length, whether or not transmission eventually succeeds.
    pub request_body_bytes: usize,
}

/// Enforced transport bounds and a requested, unverified server token ceiling.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct EndpointModelLimits {
    /// Requested output ceiling, not a local tokenizer observation.
    pub requested_max_output_tokens: u32,
    /// Whether the prepared body requests optional provider usage.
    pub provider_usage_requested: bool,
    /// Total transport deadline, without rounding fractional milliseconds.
    pub total_timeout: Duration,
    /// Inactivity deadline, never extending the total deadline.
    pub idle_timeout: Duration,
    /// Maximum response wire bytes, including framing.
    pub response_wire_limit_bytes: usize,
    /// Maximum decoded response body bytes.
    pub response_body_limit_bytes: usize,
    /// Maximum decoded untrusted text bytes.
    pub response_text_limit_bytes: usize,
}

/// Inspectable endpoint request facts. These cannot reconstruct a request,
/// credential, approval or exact native profile. The transport retains the body.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct EndpointModelRequest {
    version: u16,
    identity_kind: &'static str,
    #[serde(flatten)]
    selection: EndpointModelSelection,
    /// Exact message-array observations, never rendered prompt tokens.
    pub context: EndpointModelContext,
    #[serde(flatten)]
    body: EndpointModelBody,
    #[serde(flatten)]
    limits: EndpointModelLimits,
    verified_input_tokens: Option<u64>,
}

impl EndpointModelRequest {
    /// Records facts already checked by a transport. This performs no admission,
    /// validates no destination and grants no inference or tool authority.
    #[must_use]
    pub const fn from_prepared_body(
        selection: EndpointModelSelection,
        context: EndpointModelContext,
        body: EndpointModelBody,
        limits: EndpointModelLimits,
    ) -> Self {
        Self {
            version: 1,
            identity_kind: "selected_endpoint_unverified_model",
            selection,
            context,
            body,
            limits,
            verified_input_tokens: None,
        }
    }

    /// Selected address and name, never a native profile.
    #[must_use]
    pub fn runtime_model_binding(&self) -> RuntimeModelBinding<'_> {
        RuntimeModelBinding::SelectedEndpoint {
            endpoint: &self.selection.endpoint,
            model: &self.selection.model,
        }
    }

    /// Requested model name.
    #[must_use]
    pub fn model(&self) -> &str {
        &self.selection.model
    }

    /// Immutable prepared-body fingerprint.
    #[must_use]
    pub const fn body(&self) -> &EndpointModelBody {
        &self.body
    }

    /// Transport bounds and requested server token ceiling.
    #[must_use]
    pub const fn limits(&self) -> &EndpointModelLimits {
        &self.limits
    }
}

/// Truthful complete codec finish, never a task-success assertion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EndpointModelFinish {
    /// The provider reported a normal stop.
    Stop,
    /// The provider reported its output limit; consumers must refuse strict edits.
    OutputTokenLimit,
}

/// Locally observed dispatch duration with unavailable server memory left unknown.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct EndpointModelResources {
    /// Local monotonic time from dispatch entry through complete HTTP/codec EOF.
    /// Includes synchronous consumer callbacks, excludes preparation and approval.
    pub dispatch_elapsed: Duration,
    resident_memory_bytes: Option<u64>,
    accelerator_memory_bytes: Option<u64>,
}

/// Completed endpoint inference, still inert and untrusted. Partial, cancelled
/// or malformed streams cannot supply this observation through the transport.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct EndpointModelResult {
    version: u16,
    /// Complete selection, context, body and bound tuple used for this dispatch.
    pub request: EndpointModelRequest,
    /// Codec finish, independently of strict-edit or task acceptance.
    pub finish: EndpointModelFinish,
    /// SHA-256 of concatenated decoded text fragments, excluding stream framing.
    pub response_text_sha256: String,
    /// UTF-8 bytes in the same concatenated decoded text.
    pub response_text_bytes: usize,
    /// Local time and explicitly unknown server memory.
    pub resources: EndpointModelResources,
    /// Optional provider counts; verified input and output always remain unknown.
    pub token_accounting: EndpointTokenAccounting,
}

impl EndpointModelResult {
    /// Records a completed response checked by a transport. Descriptive only;
    /// it neither verifies provider counts nor authorizes any subsequent effect.
    #[must_use]
    pub const fn from_completed_response(
        request: EndpointModelRequest,
        finish: EndpointModelFinish,
        response_text_sha256: String,
        response_text_bytes: usize,
        dispatch_elapsed: Duration,
        usage: Option<ProviderTokenUsage>,
    ) -> Self {
        Self {
            version: 1,
            request,
            finish,
            response_text_sha256,
            response_text_bytes,
            resources: EndpointModelResources {
                dispatch_elapsed,
                resident_memory_bytes: None,
                accelerator_memory_bytes: None,
            },
            token_accounting: EndpointTokenAccounting::from_completed_provider_report(usage),
        }
    }
}
