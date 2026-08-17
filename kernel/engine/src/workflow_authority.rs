//! Narrowing-only authority composition for future workflow callers.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::{GrantOperation, ToolId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_ENTRIES: usize = 256;

/// One required source of authority in stable evaluation order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowAuthorityLayerKind {
    /// Authority explicitly held by the user.
    User,
    /// Authority granted to the enclosing workflow.
    Workflow,
    /// Authority granted to the current workflow node.
    Node,
    /// Authority retained by the immediate caller.
    Parent,
    /// Authority allowed for the exact task.
    Task,
    /// Authority allowed by current policy.
    Policy,
    /// Separately issued exact grant authority.
    ExplicitGrant,
}

impl WorkflowAuthorityLayerKind {
    /// Every required authority layer in canonical order.
    pub const ALL: [Self; 7] = [
        Self::User,
        Self::Workflow,
        Self::Node,
        Self::Parent,
        Self::Task,
        Self::Policy,
        Self::ExplicitGrant,
    ];
}

/// One independently sourced, non-wildcard authority layer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowAuthorityLayer {
    /// Required layer identity.
    pub kind: WorkflowAuthorityLayerKind,
    /// Sorted unique operations allowed by this layer.
    pub operations: Vec<GrantOperation>,
    /// Sorted unique tool identities allowed by this layer.
    pub tool_ids: Vec<ToolId>,
    /// Sorted unique exact target-scope digests allowed by this layer.
    pub target_scope_sha256s: Vec<String>,
    /// Digest of the exact source record from which this layer was derived.
    pub source_sha256: String,
}

/// Sealed least-authority result shared by all workflow caller surfaces.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowAuthorityIntersection {
    /// Operations present in every required layer.
    pub operations: Vec<GrantOperation>,
    /// Tool identities present in every required layer.
    pub tool_ids: Vec<ToolId>,
    /// Exact target scopes present in every required layer.
    pub target_scope_sha256s: Vec<String>,
    /// Source digests in canonical layer order.
    pub source_sha256s: Vec<String>,
    /// Always false: workflow composition cannot mint unattended approval.
    pub unattended_approval: bool,
    /// Digest of this record with this field zeroed.
    pub intersection_sha256: String,
}

/// Stable rejection from workflow authority composition or verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkflowAuthorityError {
    /// A required layer is missing, duplicated, malformed, or not canonical.
    InvalidLayerSet,
    /// The retained result is malformed, broadened, or not canonically sealed.
    InvalidIntersection,
}

impl std::fmt::Display for WorkflowAuthorityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidLayerSet => "workflow.authority.invalid_layer_set",
            Self::InvalidIntersection => "workflow.authority.invalid_intersection",
        })
    }
}

impl std::error::Error for WorkflowAuthorityError {}

/// Intersects all seven required layers without aggregation or implied authority.
pub fn intersect_workflow_authority(
    mut layers: Vec<WorkflowAuthorityLayer>,
) -> Result<WorkflowAuthorityIntersection, WorkflowAuthorityError> {
    layers.sort_by_key(|layer| layer.kind);
    if layers.len() != WorkflowAuthorityLayerKind::ALL.len()
        || layers
            .iter()
            .map(|layer| layer.kind)
            .ne(WorkflowAuthorityLayerKind::ALL)
        || layers.iter().any(|layer| !valid_layer(layer))
    {
        return Err(WorkflowAuthorityError::InvalidLayerSet);
    }

    let operations = intersect_sets(layers.iter().map(|layer| &layer.operations));
    let tool_ids = intersect_sets(layers.iter().map(|layer| &layer.tool_ids));
    let target_scope_sha256s =
        intersect_sets(layers.iter().map(|layer| &layer.target_scope_sha256s));
    let source_sha256s = layers
        .iter()
        .map(|layer| layer.source_sha256.clone())
        .collect();

    seal(WorkflowAuthorityIntersection {
        operations,
        tool_ids,
        target_scope_sha256s,
        source_sha256s,
        unattended_approval: false,
        intersection_sha256: ZERO_SHA256.to_owned(),
    })
}

/// Verifies a retained intersection without trusting its claimed contents.
pub fn verify_workflow_authority_intersection(
    intersection: &WorkflowAuthorityIntersection,
) -> Result<(), WorkflowAuthorityError> {
    if seal(intersection.clone())? != *intersection {
        return Err(WorkflowAuthorityError::InvalidIntersection);
    }
    Ok(())
}

fn valid_layer(layer: &WorkflowAuthorityLayer) -> bool {
    !layer.operations.is_empty()
        && bounded_sorted_unique(&layer.operations)
        && bounded_sorted_unique(&layer.tool_ids)
        && bounded_sorted_unique(&layer.target_scope_sha256s)
        && valid_sha256(&layer.source_sha256)
        && layer
            .target_scope_sha256s
            .iter()
            .all(|value| valid_sha256(value))
}

fn seal(
    mut intersection: WorkflowAuthorityIntersection,
) -> Result<WorkflowAuthorityIntersection, WorkflowAuthorityError> {
    let valid = !intersection.operations.is_empty()
        && bounded_sorted_unique(&intersection.operations)
        && bounded_sorted_unique(&intersection.tool_ids)
        && bounded_sorted_unique(&intersection.target_scope_sha256s)
        && intersection.source_sha256s.len() == WorkflowAuthorityLayerKind::ALL.len()
        && intersection
            .source_sha256s
            .iter()
            .all(|value| valid_sha256(value))
        && intersection
            .target_scope_sha256s
            .iter()
            .all(|value| valid_sha256(value))
        && !intersection.unattended_approval;
    if !valid {
        return Err(WorkflowAuthorityError::InvalidIntersection);
    }
    intersection.intersection_sha256 = ZERO_SHA256.to_owned();
    intersection.intersection_sha256 = canonical_sha256(&intersection)?;
    Ok(intersection)
}

fn intersect_sets<'a, T: Clone + Ord + 'a>(mut sets: impl Iterator<Item = &'a Vec<T>>) -> Vec<T> {
    let Some(first) = sets.next() else {
        return Vec::new();
    };
    let mut result = first.iter().cloned().collect::<BTreeSet<_>>();
    for values in sets {
        let current = values.iter().cloned().collect::<BTreeSet<_>>();
        result.retain(|value| current.contains(value));
    }
    result.into_iter().collect()
}

fn bounded_sorted_unique<T: Ord>(values: &[T]) -> bool {
    values.len() <= MAX_ENTRIES && values.windows(2).all(|pair| pair[0] < pair[1])
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn canonical_sha256(value: &impl Serialize) -> Result<String, WorkflowAuthorityError> {
    let bytes =
        serde_json::to_vec(value).map_err(|_| WorkflowAuthorityError::InvalidIntersection)?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layers() -> Vec<WorkflowAuthorityLayer> {
        WorkflowAuthorityLayerKind::ALL
            .into_iter()
            .enumerate()
            .map(|(index, kind)| WorkflowAuthorityLayer {
                kind,
                operations: if index == 6 {
                    vec![
                        GrantOperation::WorkspaceRead,
                        GrantOperation::ModelInference,
                    ]
                } else {
                    vec![
                        GrantOperation::WorkspaceRead,
                        GrantOperation::WorkspaceWrite,
                        GrantOperation::ModelInference,
                    ]
                },
                tool_ids: vec![ToolId::from_raw("fixture.read")],
                target_scope_sha256s: vec!["a".repeat(64)],
                source_sha256: format!("{}", index + 1).repeat(64),
            })
            .collect()
    }

    #[test]
    fn seven_layers_can_only_narrow_authority() {
        let result = intersect_workflow_authority(layers()).expect("intersection must seal");
        assert_eq!(
            result.operations,
            vec![
                GrantOperation::WorkspaceRead,
                GrantOperation::ModelInference
            ]
        );
        assert_eq!(result.tool_ids, vec![ToolId::from_raw("fixture.read")]);
        assert!(!result.unattended_approval);
        verify_workflow_authority_intersection(&result).expect("result must verify");
    }

    #[test]
    fn missing_duplicate_and_noncanonical_layers_fail_closed() {
        let mut missing = layers();
        missing.pop();
        assert_eq!(
            intersect_workflow_authority(missing),
            Err(WorkflowAuthorityError::InvalidLayerSet)
        );

        let mut duplicate = layers();
        duplicate[6].kind = WorkflowAuthorityLayerKind::Policy;
        assert_eq!(
            intersect_workflow_authority(duplicate),
            Err(WorkflowAuthorityError::InvalidLayerSet)
        );

        let mut unsorted = layers();
        unsorted[0].operations.swap(0, 1);
        assert_eq!(
            intersect_workflow_authority(unsorted),
            Err(WorkflowAuthorityError::InvalidLayerSet)
        );
    }

    #[test]
    fn retained_intersection_cannot_broaden_or_enable_unattended_approval() {
        let mut result = intersect_workflow_authority(layers()).expect("intersection must seal");
        result.operations.push(GrantOperation::GitPush);
        assert_eq!(
            verify_workflow_authority_intersection(&result),
            Err(WorkflowAuthorityError::InvalidIntersection)
        );

        let mut result = intersect_workflow_authority(layers()).expect("intersection must seal");
        result.unattended_approval = true;
        assert_eq!(
            verify_workflow_authority_intersection(&result),
            Err(WorkflowAuthorityError::InvalidIntersection)
        );
    }
}
