//! Reproducible, explainable, non-authoritative financial anomaly indicators.
#![allow(missing_docs)]

use crate::financial_domain::UserDisposition;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndicatorKind {
    Amount,
    Frequency,
    Merchant,
    Category,
    Time,
    Location,
    Duplicate,
    Sequence,
    RecurrenceBreak,
    AccountPattern,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndicatorMethod {
    DeterministicRule,
    RobustStatistic,
    ModelAssistedExplanation,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndicatorBaseline {
    pub baseline_id: String,
    pub population_record_ids: BTreeSet<String>,
    pub horizon_days: u32,
    pub seasonality: String,
    pub minimum_sample_count: u32,
    pub observed_sample_count: u32,
    pub method_version: u32,
    pub source_record_ids: BTreeSet<String>,
    pub drift_basis_points: u16,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinancialAnomalyIndicator {
    pub indicator_id: String,
    pub kind: IndicatorKind,
    pub feature_sha256: String,
    pub baseline_id: String,
    pub method: IndicatorMethod,
    pub method_version: u32,
    pub threshold_millionths: u32,
    pub score_millionths: u32,
    pub confidence_basis_points: u16,
    pub limitation_codes: BTreeSet<String>,
    pub source_record_ids: BTreeSet<String>,
    pub disposition: UserDisposition,
    pub potential_signal: bool,
    pub definitive_claim: bool,
    pub authority: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndicatorReplay {
    pub indicator_id: String,
    pub corpus_id: String,
    pub corpus_version: u32,
    pub ordered_input_sha256: String,
    pub calculation_sha256: String,
    pub output_sha256: String,
    pub deterministic: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndicatorEvaluation {
    pub corpus_id: String,
    pub corpus_version: u32,
    pub labeled_case_count: u32,
    pub precision_basis_points: u16,
    pub recall_basis_points: u16,
    pub false_positive_basis_points: u16,
    pub false_negative_basis_points: u16,
    pub calibration_error_basis_points: u16,
    pub stability_basis_points: u16,
    pub explanation_fidelity_basis_points: u16,
    pub subgroup_limitation_codes: BTreeSet<String>,
    pub drift_observed: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndicatorFeedback {
    pub indicator_id: String,
    pub disposition: UserDisposition,
    pub reason_code: String,
    pub effective_future_evaluation: u64,
    pub rewrites_prior_evidence: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinancialAnomalyError {
    Invalid,
    InsufficientData,
    Uncertain,
    UnsupportedClaim,
    UnsupportedEffect,
}

fn id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 1024
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
pub fn validate_baseline(value: &IndicatorBaseline) -> Result<(), FinancialAnomalyError> {
    if !id(&value.baseline_id)
        || value.population_record_ids.is_empty()
        || value.horizon_days == 0
        || !id(&value.seasonality)
        || value.minimum_sample_count == 0
        || value.method_version == 0
        || value.source_record_ids.is_empty()
        || value.drift_basis_points > 10_000
    {
        return Err(FinancialAnomalyError::Invalid);
    }
    if value.observed_sample_count < value.minimum_sample_count {
        return Err(FinancialAnomalyError::InsufficientData);
    }
    Ok(())
}
pub fn validate_indicator(
    value: &FinancialAnomalyIndicator,
    baseline: &IndicatorBaseline,
) -> Result<(), FinancialAnomalyError> {
    if !id(&value.indicator_id)
        || !digest(&value.feature_sha256)
        || value.baseline_id != baseline.baseline_id
        || value.method_version == 0
        || value.threshold_millionths > 1_000_000
        || value.score_millionths > 1_000_000
        || value.confidence_basis_points > 10_000
        || value.source_record_ids.is_empty()
        || !value.potential_signal
        || value.definitive_claim
        || value.authority
    {
        return Err(FinancialAnomalyError::Invalid);
    }
    if (value.confidence_basis_points < 10_000 || baseline.drift_basis_points > 0)
        && value.limitation_codes.is_empty()
    {
        return Err(FinancialAnomalyError::Uncertain);
    }
    if value.method == IndicatorMethod::ModelAssistedExplanation
        && value.disposition == UserDisposition::Accepted
    {
        return Err(FinancialAnomalyError::Uncertain);
    }
    Ok(())
}
pub fn validate_replay(value: &IndicatorReplay) -> Result<(), FinancialAnomalyError> {
    if !id(&value.indicator_id)
        || !id(&value.corpus_id)
        || value.corpus_version == 0
        || !digest(&value.ordered_input_sha256)
        || !digest(&value.calculation_sha256)
        || !digest(&value.output_sha256)
        || !value.deterministic
    {
        return Err(FinancialAnomalyError::Invalid);
    }
    Ok(())
}
pub fn validate_evaluation(value: &IndicatorEvaluation) -> Result<(), FinancialAnomalyError> {
    let rates = [
        value.precision_basis_points,
        value.recall_basis_points,
        value.false_positive_basis_points,
        value.false_negative_basis_points,
        value.calibration_error_basis_points,
        value.stability_basis_points,
        value.explanation_fidelity_basis_points,
    ];
    if !id(&value.corpus_id)
        || value.corpus_version == 0
        || value.labeled_case_count == 0
        || rates.into_iter().any(|rate| rate > 10_000)
        || (value.drift_observed && value.subgroup_limitation_codes.is_empty())
    {
        return Err(FinancialAnomalyError::Invalid);
    }
    Ok(())
}
pub fn record_feedback(value: &IndicatorFeedback) -> Result<(), FinancialAnomalyError> {
    if !id(&value.indicator_id)
        || !id(&value.reason_code)
        || value.effective_future_evaluation == 0
        || value.rewrites_prior_evidence
    {
        return Err(FinancialAnomalyError::Invalid);
    }
    Ok(())
}
pub fn reject_definitive_claim(_claim: &str) -> Result<(), FinancialAnomalyError> {
    Err(FinancialAnomalyError::UnsupportedClaim)
}
pub fn reject_indicator_action(_action: &str) -> Result<(), FinancialAnomalyError> {
    Err(FinancialAnomalyError::UnsupportedEffect)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn baseline() -> IndicatorBaseline {
        IndicatorBaseline {
            baseline_id: "baseline".into(),
            population_record_ids: BTreeSet::from(["population".into()]),
            horizon_days: 365,
            seasonality: "monthly".into(),
            minimum_sample_count: 12,
            observed_sample_count: 24,
            method_version: 1,
            source_record_ids: BTreeSet::from(["transaction".into()]),
            drift_basis_points: 100,
        }
    }
    fn indicator(method: IndicatorMethod) -> FinancialAnomalyIndicator {
        FinancialAnomalyIndicator {
            indicator_id: "indicator".into(),
            kind: IndicatorKind::Amount,
            feature_sha256: "a".repeat(64),
            baseline_id: "baseline".into(),
            method,
            method_version: 1,
            threshold_millionths: 800_000,
            score_millionths: 900_000,
            confidence_basis_points: 8_000,
            limitation_codes: BTreeSet::from(["drift-observed".into()]),
            source_record_ids: BTreeSet::from(["transaction".into()]),
            disposition: UserDisposition::Unresolved,
            potential_signal: true,
            definitive_claim: false,
            authority: false,
        }
    }
    #[test]
    fn methods_are_separate_and_uncertainty_visible() {
        for method in [
            IndicatorMethod::DeterministicRule,
            IndicatorMethod::RobustStatistic,
            IndicatorMethod::ModelAssistedExplanation,
        ] {
            assert_eq!(validate_indicator(&indicator(method), &baseline()), Ok(()));
        }
        let mut hidden = indicator(IndicatorMethod::RobustStatistic);
        hidden.limitation_codes.clear();
        assert_eq!(
            validate_indicator(&hidden, &baseline()),
            Err(FinancialAnomalyError::Uncertain)
        );
    }
    #[test]
    fn sparse_baselines_fail_closed() {
        let mut sparse = baseline();
        sparse.observed_sample_count = 11;
        assert_eq!(
            validate_baseline(&sparse),
            Err(FinancialAnomalyError::InsufficientData)
        );
    }
    #[test]
    fn replay_and_feedback_are_immutable() {
        let replay = IndicatorReplay {
            indicator_id: "indicator".into(),
            corpus_id: "synthetic".into(),
            corpus_version: 1,
            ordered_input_sha256: "a".repeat(64),
            calculation_sha256: "b".repeat(64),
            output_sha256: "c".repeat(64),
            deterministic: true,
        };
        assert_eq!(validate_replay(&replay), Ok(()));
        let feedback = IndicatorFeedback {
            indicator_id: "indicator".into(),
            disposition: UserDisposition::Rejected,
            reason_code: "benign-shift".into(),
            effective_future_evaluation: 2,
            rewrites_prior_evidence: false,
        };
        assert_eq!(record_feedback(&feedback), Ok(()));
    }
    #[test]
    fn claims_and_actions_are_never_admitted() {
        assert_eq!(
            reject_definitive_claim("fraud"),
            Err(FinancialAnomalyError::UnsupportedClaim)
        );
        assert_eq!(
            reject_indicator_action("freeze account"),
            Err(FinancialAnomalyError::UnsupportedEffect)
        );
    }
}
