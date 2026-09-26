//! Host compatibility exports for the shared closed research-plan preparation.

pub use agentmage_kernel_engine::research_plan::{
    PreparedResearchPlan, ResearchPlanDraft, ResearchPlanError,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_plan_decoder_uses_the_canonical_owners_exact_type() {
        let decode: fn(&[u8]) -> Result<PreparedResearchPlan, ResearchPlanError> =
            agentmage_kernel_engine::research_plan::PreparedResearchPlan::decode;
        assert_eq!(
            decode(b"{}").err(),
            PreparedResearchPlan::decode(b"{}").err()
        );
        assert_eq!(decode(b"{}").err(), Some(ResearchPlanError::Invalid));
    }
}
