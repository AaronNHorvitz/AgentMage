//! Inert quick/deep query plans for the existing runtime and artifact owner.
//!
//! Planning does not contact a provider, mint grants, run a second coordinator,
//! or turn model-generated queries into user-approved disclosures.

use std::collections::BTreeSet;
use std::fmt;

use crate::research_budget::{
    ResearchBudgetError, ResearchDepth, ResearchLimits, ResearchNetworkMode, ResearchScope,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::public_research::{PublicSearchRequest, validate_public_search};

const MAX_PLAN_BYTES: usize = 64 * 1024;

/// Closed draft sent to the existing user approval and canonical artifact paths.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchPlanDraft {
    /// Only schema 1 is admitted.
    pub schema_version: u16,
    /// The existing runtime task; never a new scheduler identity.
    pub task_id: String,
    /// Quick or bounded deep research.
    pub depth: ResearchDepth,
    /// Internet search permission is distinct from model inference routing.
    pub network_mode: ResearchNetworkMode,
    /// Downward-only frozen task ceilings.
    pub limits: ResearchLimits,
    /// Exact provider and source hosts whose independent grants may be requested.
    pub destination_domains: BTreeSet<String>,
    /// All proposed disclosures must be inspected before any outbound execution.
    pub queries: Vec<PublicSearchRequest>,
}

impl fmt::Debug for ResearchPlanDraft {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResearchPlanDraft")
            .field("depth", &self.depth)
            .field("network_mode", &self.network_mode)
            .field("query_count", &self.queries.len())
            .field("destination_count", &self.destination_domains.len())
            .finish_non_exhaustive()
    }
}

/// Validated descriptive plan, not proof of approval or an execution handle.
#[derive(Clone)]
pub struct PreparedResearchPlan {
    draft: ResearchPlanDraft,
    scope: ResearchScope,
    plan_sha256: String,
}

impl fmt::Debug for PreparedResearchPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedResearchPlan")
            .field("plan_sha256", &self.plan_sha256)
            .field("draft", &self.draft)
            .finish_non_exhaustive()
    }
}

/// Content-free draft refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResearchPlanError {
    /// Invalid version, identity, duplicate query or inconsistent ceilings.
    Invalid,
    /// Detected secret material cannot enter a disclosure plan.
    Secret,
    /// A query asks for domains or bytes outside the overall task restriction.
    Scope,
}

impl PreparedResearchPlan {
    /// Parses only a bounded closed draft, never a persisted approval or budget.
    pub fn decode(bytes: &[u8]) -> Result<Self, ResearchPlanError> {
        if bytes.is_empty() || bytes.len() > MAX_PLAN_BYTES {
            return Err(ResearchPlanError::Invalid);
        }
        Self::prepare(serde_json::from_slice(bytes).map_err(|_| ResearchPlanError::Invalid)?)
    }

    /// Validates a complete draft before presenting it for independent approval.
    pub fn prepare(draft: ResearchPlanDraft) -> Result<Self, ResearchPlanError> {
        let ceiling = ResearchLimits::ceiling(draft.depth);
        if draft.schema_version != 1
            || draft.task_id.len() > 128
            || draft.queries.is_empty()
            || draft.queries.len() > usize::from(draft.limits.queries)
            || draft.queries.len() > usize::from(ceiling.queries)
            || draft.destination_domains.len() > usize::from(ceiling.domains)
            || draft
                .destination_domains
                .iter()
                .any(|domain| domain.len() > 253)
        {
            return Err(ResearchPlanError::Invalid);
        }
        let mut ids = BTreeSet::new();
        let mut bytes = 0u64;
        for query in &draft.queries {
            validate_public_search(query).map_err(|error| {
                if error == crate::public_research::PublicResearchError::SecretDenied {
                    ResearchPlanError::Secret
                } else {
                    ResearchPlanError::Invalid
                }
            })?;
            if !ids.insert(&query.request_id) {
                return Err(ResearchPlanError::Invalid);
            }
            if query.domains.is_empty()
                || query
                    .domains
                    .iter()
                    .any(|domain| !draft.destination_domains.contains(domain))
            {
                return Err(ResearchPlanError::Scope);
            }
            bytes = bytes
                .checked_add(query.max_total_bytes)
                .ok_or(ResearchPlanError::Scope)?;
        }
        if bytes >= draft.limits.downloaded_bytes {
            // Reserve space for at least one source visit; a search-only metadata
            // budget cannot claim a source-backed report.
            return Err(ResearchPlanError::Scope);
        }
        let scope = ResearchScope::new(
            draft.task_id.clone(),
            draft.depth,
            draft.network_mode,
            draft.limits.clone(),
            draft.destination_domains.clone(),
            &draft
                .queries
                .iter()
                .map(|request| request.query.clone())
                .collect::<Vec<_>>(),
        )
        .map_err(|error| {
            if error == ResearchBudgetError::Secret {
                ResearchPlanError::Secret
            } else {
                ResearchPlanError::Invalid
            }
        })?;
        let encoded = serde_json::to_vec(&draft).map_err(|_| ResearchPlanError::Invalid)?;
        if encoded.len() > MAX_PLAN_BYTES {
            return Err(ResearchPlanError::Invalid);
        }
        let plan_sha256 = Sha256::digest(&encoded)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        Ok(Self {
            draft,
            scope,
            plan_sha256,
        })
    }

    /// Exact inspectable draft for the existing consent/artifact presentation path.
    #[must_use]
    pub const fn draft(&self) -> &ResearchPlanDraft {
        &self.draft
    }

    /// Extra restrictions for the existing effect owner, not a grant.
    #[must_use]
    pub const fn scope(&self) -> &ResearchScope {
        &self.scope
    }

    /// Whole-plan binding, including requests, sources, network mode and ceilings.
    #[must_use]
    pub fn plan_sha256(&self) -> &str {
        &self.plan_sha256
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public_research::PublicSourceType;

    #[test]
    fn bounded_decoder_refuses_oversize_duplicate_fields_and_raw_diagnostic_content() {
        let draft = draft(ResearchDepth::Quick);
        let bytes = serde_json::to_vec(&draft).unwrap();
        let parsed = PreparedResearchPlan::decode(&bytes).unwrap();
        let diagnostic = format!("{parsed:?}");
        for private in [
            &draft.task_id,
            &draft.queries[0].query,
            &draft.queries[0].request_id,
        ] {
            assert!(!diagnostic.contains(private));
        }
        let duplicate =
            String::from_utf8(bytes)
                .unwrap()
                .replacen('{', "{\"schema_version\":1,", 1);
        assert_eq!(
            PreparedResearchPlan::decode(duplicate.as_bytes()).err(),
            Some(ResearchPlanError::Invalid)
        );
        assert_eq!(
            PreparedResearchPlan::decode(&vec![b' '; MAX_PLAN_BYTES + 1]).err(),
            Some(ResearchPlanError::Invalid)
        );
        let mut increased = draft.clone();
        increased.limits.queries = u16::MAX;
        increased.queries = vec![draft.queries[0].clone(); 7];
        assert_eq!(
            PreparedResearchPlan::prepare(increased).err(),
            Some(ResearchPlanError::Invalid)
        );
    }
    fn draft(depth: ResearchDepth) -> ResearchPlanDraft {
        let count = if depth == ResearchDepth::Quick { 1 } else { 3 };
        ResearchPlanDraft {
            schema_version: 1,
            task_id: "research-task".into(),
            depth,
            network_mode: ResearchNetworkMode::Ask,
            limits: ResearchLimits::ceiling(depth),
            destination_domains: BTreeSet::from([
                "docs.example.com".into(),
                "search.example.com".into(),
            ]),
            queries: (0..count)
                .map(|index| PublicSearchRequest {
                    request_id: format!("query-{index}"),
                    query: format!("public package version {index}"),
                    domains: vec!["docs.example.com".into()],
                    recency_days: 30,
                    source_types: vec![PublicSourceType::PrimaryDocumentation],
                    max_results: 5,
                    max_total_bytes: 64 * 1024,
                })
                .collect(),
        }
    }
    #[test]
    fn quick_and_multiquery_deep_plans_are_bound_without_execution() {
        for depth in [ResearchDepth::Quick, ResearchDepth::Deep] {
            let plan = PreparedResearchPlan::prepare(draft(depth)).unwrap();
            assert_eq!(plan.plan_sha256().len(), 64);
            assert_eq!(
                plan.scope().network_requirement(),
                Ok(ResearchNetworkMode::Ask)
            );
            assert_eq!(plan.draft().depth, depth);
        }
    }
    #[test]
    fn schema_unknown_fields_and_scope_are_closed() {
        let mut value = serde_json::to_value(draft(ResearchDepth::Quick)).unwrap();
        value["approve_all"] = true.into();
        assert!(serde_json::from_value::<ResearchPlanDraft>(value).is_err());
        let mut invalid = draft(ResearchDepth::Quick);
        invalid.schema_version = 2;
        assert_eq!(
            PreparedResearchPlan::prepare(invalid).err(),
            Some(ResearchPlanError::Invalid)
        );
        let mut invalid = draft(ResearchDepth::Quick);
        invalid.queries[0].domains = vec!["other.example.com".into()];
        assert_eq!(
            PreparedResearchPlan::prepare(invalid).err(),
            Some(ResearchPlanError::Scope)
        );
    }
    #[test]
    fn changed_queries_and_limits_change_binding_and_secrets_refuse() {
        let initial = draft(ResearchDepth::Deep);
        let before = PreparedResearchPlan::prepare(initial.clone()).unwrap();
        let mut changed = initial.clone();
        changed.queries[0].query.push_str(" revised");
        assert_ne!(
            before.plan_sha256(),
            PreparedResearchPlan::prepare(changed)
                .unwrap()
                .plan_sha256()
        );
        let mut changed = initial.clone();
        changed.limits.visits -= 1;
        assert_ne!(
            before.plan_sha256(),
            PreparedResearchPlan::prepare(changed)
                .unwrap()
                .plan_sha256()
        );
        let mut changed = initial;
        changed.queries[0].query = format!("Bearer {}", "x".repeat(32));
        assert_eq!(
            PreparedResearchPlan::prepare(changed).err(),
            Some(ResearchPlanError::Secret)
        );
    }
    #[test]
    fn query_duplicates_search_byte_exhaustion_and_offline_remain_explicit() {
        let mut invalid = draft(ResearchDepth::Deep);
        invalid.queries[1] = invalid.queries[0].clone();
        assert_eq!(
            PreparedResearchPlan::prepare(invalid).err(),
            Some(ResearchPlanError::Invalid)
        );
        let mut invalid = draft(ResearchDepth::Quick);
        invalid.queries[0].max_total_bytes = invalid.limits.downloaded_bytes;
        assert_eq!(
            PreparedResearchPlan::prepare(invalid).err(),
            Some(ResearchPlanError::Scope)
        );
        let mut offline = draft(ResearchDepth::Quick);
        offline.network_mode = ResearchNetworkMode::Offline;
        let plan = PreparedResearchPlan::prepare(offline).unwrap();
        assert_eq!(
            plan.scope().network_requirement(),
            Err(ResearchBudgetError::Offline)
        );
    }
}
