// SPDX-License-Identifier: Apache-2.0
//! Descriptive model identity and token provenance, never execution authority.

use crate::{ExactModelProfile, ModelTokenUsage};

/// The identity a runtime model port can actually supply.
///
/// An exact profile still needs its existing admission and dispatch checks. A
/// person-selected endpoint cannot attest to weights, tokenizer, serving process
/// or hardware. Neither variant grants access to an endpoint or a tool.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
#[serde(tag = "kind", content = "identity", rename_all = "snake_case")]
pub enum RuntimeModelBinding<'a> {
    /// A complete profile tuple, compared without dropping any native field.
    ExactProfile(&'a ExactModelProfile),
    /// Configuration selected by the person, without native model attestation.
    SelectedEndpoint {
        /// Canonical address from the credential-owning transport.
        endpoint: &'a str,
        /// Requested model name, not verified weight identity.
        model: &'a str,
    },
}

impl RuntimeModelBinding<'_> {
    /// Compares only exact profiles. Endpoint names, hashes and provider reports
    /// can never satisfy the native coordinator's complete profile requirement.
    #[must_use]
    pub fn matches_exact_profile(&self, expected: &ExactModelProfile) -> bool {
        matches!(self, Self::ExactProfile(actual) if *actual == expected)
    }
}

/// Optional counts reported by an endpoint, without local tokenizer evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ProviderTokenUsage {
    /// Reported prompt tokens, including any cached tokens.
    pub prompt_tokens: u32,
    /// Reported generated tokens, including any reasoning tokens.
    pub completion_tokens: u32,
    /// Reported sum; consumers must retain their existing structural checks.
    pub total_tokens: u32,
}

/// Token accounting whose provenance remains explicit at the runtime boundary.
/// This serializable inspection value has no deserializer or grant constructor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", content = "observation", rename_all = "snake_case")]
pub enum RuntimeModelTokenAccounting<'a> {
    /// Exact native dispatch accounting; existing native validation still applies.
    ExactNative(&'a ModelTokenUsage),
    /// A completed endpoint response may report counts, or leave them unknown.
    SelectedEndpoint(EndpointTokenAccounting),
}

/// Endpoint token observations cannot become verified native token counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub struct EndpointTokenAccounting {
    provider_reported: Option<ProviderTokenUsage>,
    verified_input_tokens: Option<u64>,
    verified_output_tokens: Option<u64>,
}

impl EndpointTokenAccounting {
    /// Describes optional counts from a completed response. Callers must not
    /// supply counts from partial, cancelled or malformed responses. This
    /// constructor performs no admission and establishes no verified counts.
    #[must_use]
    pub const fn from_completed_provider_report(usage: Option<ProviderTokenUsage>) -> Self {
        Self {
            provider_reported: usage,
            verified_input_tokens: None,
            verified_output_tokens: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{EndpointTokenAccounting, ProviderTokenUsage, RuntimeModelTokenAccounting};

    #[test]
    fn endpoint_accounting_keeps_missing_zero_and_nonzero_provider_counts_distinct() {
        for usage in [
            None,
            Some(ProviderTokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            }),
            Some(ProviderTokenUsage {
                prompt_tokens: 100,
                completion_tokens: 20,
                total_tokens: 120,
            }),
        ] {
            let accounting = RuntimeModelTokenAccounting::SelectedEndpoint(
                EndpointTokenAccounting::from_completed_provider_report(usage),
            );
            let value = serde_json::to_value(accounting).unwrap();
            assert_eq!(value["kind"], "selected_endpoint");
            assert_eq!(
                value["observation"]["provider_reported"],
                serde_json::to_value(usage).unwrap()
            );
            assert_eq!(
                value["observation"]["verified_input_tokens"],
                serde_json::Value::Null
            );
            assert_eq!(
                value["observation"]["verified_output_tokens"],
                serde_json::Value::Null
            );
            assert!(value["observation"].get("rendered_prompt_tokens").is_none());
            assert!(
                value["observation"]
                    .get("remaining_capacity_tokens")
                    .is_none()
            );
        }
    }
}
