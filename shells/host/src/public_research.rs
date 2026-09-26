//! Host compatibility exports for the shared inert research contract.
//!
//! The canonical owner and host use the same schemas and validators. These exports
//! do not register a provider, perform I/O or establish retrieval provenance.

pub use agentmage_kernel_engine::public_research::{
    PublicCitation, PublicResearchError, PublicSearchCandidate, PublicSearchRequest,
    PublicSourceType, prepare_public_search, verify_public_results,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_request_type_and_validator_are_the_shared_contract() {
        let validate: fn(PublicSearchRequest) -> Result<PublicSearchRequest, PublicResearchError> =
            agentmage_kernel_engine::public_research::prepare_public_search;
        let request = PublicSearchRequest {
            request_id: "query-1".into(),
            query: String::new(),
            domains: vec![],
            recency_days: 0,
            source_types: vec![PublicSourceType::PrimaryDocumentation],
            max_results: 1,
            max_total_bytes: 1024,
        };
        assert_eq!(validate(request.clone()), prepare_public_search(request));
    }
}
