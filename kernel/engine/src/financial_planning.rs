//! Deterministic, explainable, and non-executing financial planning.
#![allow(missing_docs)]

use crate::financial_domain::{FinancialError, Money, UserDisposition};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BudgetRuleKind {
    Category,
    Envelope,
    Rollover,
    ScheduledIncome,
    ScheduledExpense,
    Allocation,
    Overspending,
    GoalContribution,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BudgetRule {
    pub rule_id: String,
    pub kind: BudgetRuleKind,
    pub priority: u32,
    pub effective_start: String,
    pub effective_end: String,
    pub source_record_ids: BTreeSet<String>,
    pub version: u32,
    pub conflict_rule_ids: BTreeSet<String>,
    pub override_rule_id: Option<String>,
    pub user_disposition: UserDisposition,
    pub approved_locally: bool,
    pub amount: Money,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScenarioKind {
    Budget,
    CashFlow,
    SavingsGoal,
    DebtAmortization,
    NetWorth,
    Comparison,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfidenceClass {
    Exact,
    Estimated,
    Incomplete,
    Conflicted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultClass {
    Actual,
    Scenario,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinancialScenario {
    pub scenario_id: String,
    pub kind: ScenarioKind,
    pub calculation_version: u32,
    pub horizon_start: String,
    pub horizon_end: String,
    pub source_record_ids: BTreeSet<String>,
    pub rule_ids: BTreeSet<String>,
    pub assumptions: BTreeSet<String>,
    pub missing_data: BTreeSet<String>,
    pub sensitivity_basis_points: i32,
    pub confidence: ConfidenceClass,
    pub limitations: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinancialPlanningResult {
    pub scenario_id: String,
    pub calculation_version: u32,
    pub result: Money,
    pub source_record_ids: BTreeSet<String>,
    pub assumptions: BTreeSet<String>,
    pub missing_data: BTreeSet<String>,
    pub sensitivity_basis_points: i32,
    pub confidence: ConfidenceClass,
    pub limitations: BTreeSet<String>,
    pub result_class: ResultClass,
    pub execution_authority: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanningError {
    Invalid,
    UnapprovedRule,
    Conflict,
    CurrencyOrScale,
    Overflow,
    UnsupportedEffect,
}

fn identifier(value: &str) -> bool {
    !value.is_empty() && value.len() <= 1024
}

pub fn validate_rule(rule: &BudgetRule) -> Result<(), PlanningError> {
    if !identifier(&rule.rule_id)
        || rule.priority == 0
        || rule.version == 0
        || !identifier(&rule.effective_start)
        || !identifier(&rule.effective_end)
        || rule.source_record_ids.is_empty()
        || !rule.approved_locally
        || rule.user_disposition != UserDisposition::Accepted
    {
        return Err(PlanningError::UnapprovedRule);
    }
    if rule.conflict_rule_ids.contains(&rule.rule_id)
        || rule.override_rule_id.as_deref() == Some(rule.rule_id.as_str())
    {
        return Err(PlanningError::Conflict);
    }
    Ok(())
}

pub fn exact_total(values: &[Money]) -> Result<Money, PlanningError> {
    let mut ordered = values.to_vec();
    ordered.sort_by_key(Money::canonical);
    let mut values = ordered.into_iter();
    let mut total = values.next().ok_or(PlanningError::Invalid)?;
    for value in values {
        total = total.checked_add(&value).map_err(|error| match error {
            FinancialError::Overflow => PlanningError::Overflow,
            _ => PlanningError::CurrencyOrScale,
        })?;
    }
    Ok(total)
}

pub fn compute_scenario(
    scenario: &FinancialScenario,
    values: &[Money],
) -> Result<FinancialPlanningResult, PlanningError> {
    if !identifier(&scenario.scenario_id)
        || scenario.calculation_version == 0
        || !identifier(&scenario.horizon_start)
        || !identifier(&scenario.horizon_end)
        || scenario.source_record_ids.is_empty()
    {
        return Err(PlanningError::Invalid);
    }
    if (!scenario.missing_data.is_empty()
        || matches!(
            scenario.confidence,
            ConfidenceClass::Incomplete | ConfidenceClass::Conflicted
        ))
        && scenario.limitations.is_empty()
    {
        return Err(PlanningError::Invalid);
    }
    Ok(FinancialPlanningResult {
        scenario_id: scenario.scenario_id.clone(),
        calculation_version: scenario.calculation_version,
        result: exact_total(values)?,
        source_record_ids: scenario.source_record_ids.clone(),
        assumptions: scenario.assumptions.clone(),
        missing_data: scenario.missing_data.clone(),
        sensitivity_basis_points: scenario.sensitivity_basis_points,
        confidence: scenario.confidence,
        limitations: scenario.limitations.clone(),
        result_class: ResultClass::Scenario,
        execution_authority: false,
    })
}

pub fn reject_external_effect(_description: &str) -> Result<(), PlanningError> {
    Err(PlanningError::UnsupportedEffect)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::financial_domain::Currency;
    fn usd(value: i128) -> Money {
        Money::new(value, Currency::new("USD").unwrap(), 2).unwrap()
    }
    fn scenario() -> FinancialScenario {
        FinancialScenario {
            scenario_id: "scenario".into(),
            kind: ScenarioKind::CashFlow,
            calculation_version: 1,
            horizon_start: "2026-01-01".into(),
            horizon_end: "2026-12-31".into(),
            source_record_ids: BTreeSet::from(["source".into()]),
            rule_ids: BTreeSet::from(["rule".into()]),
            assumptions: BTreeSet::from(["income stable".into()]),
            missing_data: BTreeSet::new(),
            sensitivity_basis_points: 500,
            confidence: ConfidenceClass::Exact,
            limitations: BTreeSet::new(),
        }
    }
    #[test]
    fn reordered_computation_is_exact() {
        let first = compute_scenario(&scenario(), &[usd(25), usd(-5), usd(10)]).unwrap();
        let second = compute_scenario(&scenario(), &[usd(10), usd(25), usd(-5)]).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.result.minor_units(), 30);
        assert!(!first.execution_authority);
    }
    #[test]
    fn missing_data_requires_visible_limitations() {
        let mut value = scenario();
        value.missing_data.insert("February".into());
        value.confidence = ConfidenceClass::Incomplete;
        assert_eq!(
            compute_scenario(&value, &[usd(1)]),
            Err(PlanningError::Invalid)
        );
        value.limitations.insert("February absent".into());
        assert!(compute_scenario(&value, &[usd(1)]).is_ok());
    }
    #[test]
    fn currency_mismatch_fails_closed() {
        let eur = Money::new(1, Currency::new("EUR").unwrap(), 2).unwrap();
        assert_eq!(
            compute_scenario(&scenario(), &[usd(1), eur]),
            Err(PlanningError::CurrencyOrScale)
        );
    }
    #[test]
    fn plans_never_create_external_authority() {
        for request in ["send funds", "place order", "file return", "open account"] {
            assert_eq!(
                reject_external_effect(request),
                Err(PlanningError::UnsupportedEffect)
            );
        }
    }
}
