// SPDX-License-Identifier: Apache-2.0
//! Static endpoint run admission. No native profile, dispatch, grant or loop.

use super::*;

/// Seals the complete endpoint run, including its selection and byte ceilings.
/// This does not approve a destination or compose a running coordinator.
pub fn seal_runtime_endpoint_run_request(
    mut request: RuntimeEndpointRunRequest,
) -> Result<RuntimeEndpointRunRequest, RuntimeCoordinatorError> {
    request.schema_version = CONTRACT_SCHEMA_VERSION;
    request.request_sha256 = ZERO_SHA256.to_owned();
    validate_endpoint_shape(&request)?;
    request.request_sha256 = canonical_sha256(&request)?;
    Ok(request)
}

/// Checks the same task, workspace, policy, catalog and budget relationships as
/// native run framing, with endpoint-specific identity and byte-only context.
pub fn verify_runtime_endpoint_run_request(
    request: &RuntimeEndpointRunRequest,
) -> Result<(), RuntimeCoordinatorError> {
    validate_endpoint_shape(request)?;
    let mut preimage = request.clone();
    preimage.request_sha256 = ZERO_SHA256.to_owned();
    if canonical_sha256(&preimage)? != request.request_sha256 {
        return Err(RuntimeCoordinatorError::DigestMismatch);
    }
    Ok(())
}

fn validate_endpoint_shape(
    request: &RuntimeEndpointRunRequest,
) -> Result<(), RuntimeCoordinatorError> {
    validate_common_run_shape(&RuntimeRunShape::from(request), || {
        let selection = &request.model_selection;
        // A content-free selection guard, not URL resolution or disclosure
        // consent. The credential-owning transport still validates destinations.
        if !valid_text(&selection.endpoint)
            || !(selection.endpoint.starts_with("http://")
                || selection.endpoint.starts_with("https://"))
            || selection.endpoint.bytes().any(|byte| {
                byte.is_ascii_whitespace() || byte.is_ascii_control() || b"@?#\\".contains(&byte)
            })
            || !valid_text(&selection.model)
            || selection.model.trim() != selection.model
            || selection.model.chars().any(char::is_control)
        {
            return Err(RuntimeCoordinatorError::InvalidValue);
        }
        let budget = &request.context_budget;
        if budget.max_messages == 0
            || budget.max_messages > 256
            || budget.max_content_bytes == 0
            || budget.max_content_bytes > budget.max_serialized_messages_bytes
            || budget.max_serialized_messages_bytes > budget.max_request_body_bytes
            || budget.max_request_body_bytes > 256 * 1024
            || !(1..=32768).contains(&request.max_output_tokens)
        {
            return Err(RuntimeCoordinatorError::InvalidLimits);
        }
        validate_runtime_run_limits(&request.limits)?;
        RuntimeHardeningLimits::from_endpoint_request(request)
            .map_err(|_| RuntimeCoordinatorError::InvalidLimits)?;
        Ok(())
    })
}

#[cfg(test)]
mod tests;
