//! Exact fixed-point financial values and immutable record lineage.
#![allow(missing_docs)]

use std::collections::BTreeSet;

pub const MAX_SCALE: u8 = 18;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoundingMode {
    TowardZero,
    AwayFromZero,
    Floor,
    Ceiling,
    HalfEven,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Currency(String);

impl Currency {
    pub fn new(code: &str) -> Result<Self, FinancialError> {
        if code.len() != 3 || !code.bytes().all(|b| b.is_ascii_uppercase()) {
            return Err(FinancialError::AmbiguousCurrency);
        }
        Ok(Self(code.into()))
    }
    pub fn code(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Money {
    minor_units: i128,
    currency: Currency,
    scale: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinancialError {
    AmbiguousCurrency,
    UnsupportedScale,
    CurrencyMismatch,
    ScaleMismatch,
    Overflow,
    InvalidAllocation,
    InvalidRecord,
    LineageConflict,
}

fn factor(power: u8) -> Result<i128, FinancialError> {
    10_i128
        .checked_pow(u32::from(power))
        .ok_or(FinancialError::Overflow)
}

impl Money {
    pub fn new(minor_units: i128, currency: Currency, scale: u8) -> Result<Self, FinancialError> {
        if scale > MAX_SCALE {
            return Err(FinancialError::UnsupportedScale);
        }
        Ok(Self {
            minor_units,
            currency,
            scale,
        })
    }
    pub fn minor_units(&self) -> i128 {
        self.minor_units
    }
    pub fn currency(&self) -> &Currency {
        &self.currency
    }
    pub fn scale(&self) -> u8 {
        self.scale
    }
    pub fn canonical(&self) -> String {
        format!(
            "{}:{}:{}",
            self.currency.code(),
            self.scale,
            self.minor_units
        )
    }
    pub fn checked_add(&self, other: &Self) -> Result<Self, FinancialError> {
        if self.currency != other.currency {
            return Err(FinancialError::CurrencyMismatch);
        }
        if self.scale != other.scale {
            return Err(FinancialError::ScaleMismatch);
        }
        Self::new(
            self.minor_units
                .checked_add(other.minor_units)
                .ok_or(FinancialError::Overflow)?,
            self.currency.clone(),
            self.scale,
        )
    }
    pub fn rescale(&self, target: u8, mode: RoundingMode) -> Result<Self, FinancialError> {
        if target > MAX_SCALE {
            return Err(FinancialError::UnsupportedScale);
        }
        if target >= self.scale {
            let multiplier = factor(target - self.scale)?;
            return Self::new(
                self.minor_units
                    .checked_mul(multiplier)
                    .ok_or(FinancialError::Overflow)?,
                self.currency.clone(),
                target,
            );
        }
        let divisor = factor(self.scale - target)?;
        let quotient = self.minor_units / divisor;
        let remainder = self.minor_units % divisor;
        let sign = self.minor_units.signum();
        let increment = match mode {
            RoundingMode::TowardZero => 0,
            RoundingMode::AwayFromZero if remainder != 0 => sign,
            RoundingMode::Floor if remainder < 0 => -1,
            RoundingMode::Ceiling if remainder > 0 => 1,
            RoundingMode::HalfEven => {
                let twice = remainder
                    .abs()
                    .checked_mul(2)
                    .ok_or(FinancialError::Overflow)?;
                if twice > divisor || (twice == divisor && quotient % 2 != 0) {
                    sign
                } else {
                    0
                }
            }
            _ => 0,
        };
        Self::new(
            quotient
                .checked_add(increment)
                .ok_or(FinancialError::Overflow)?,
            self.currency.clone(),
            target,
        )
    }
    pub fn allocate(&self, weights: &[u64]) -> Result<Vec<Self>, FinancialError> {
        if weights.is_empty() || weights.iter().all(|weight| *weight == 0) {
            return Err(FinancialError::InvalidAllocation);
        }
        let total_weight: i128 = weights.iter().try_fold(0_i128, |sum, weight| {
            sum.checked_add(i128::from(*weight))
                .ok_or(FinancialError::Overflow)
        })?;
        let mut parts = Vec::with_capacity(weights.len());
        let mut assigned = 0_i128;
        for weight in weights {
            let numerator = self
                .minor_units
                .checked_mul(i128::from(*weight))
                .ok_or(FinancialError::Overflow)?;
            let units = numerator / total_weight;
            assigned = assigned
                .checked_add(units)
                .ok_or(FinancialError::Overflow)?;
            parts.push(Self::new(units, self.currency.clone(), self.scale)?);
        }
        let mut remainder = self
            .minor_units
            .checked_sub(assigned)
            .ok_or(FinancialError::Overflow)?;
        let step = remainder.signum();
        let mut index = 0_usize;
        while remainder != 0 {
            if weights[index] != 0 {
                parts[index].minor_units = parts[index]
                    .minor_units
                    .checked_add(step)
                    .ok_or(FinancialError::Overflow)?;
                remainder -= step;
            }
            index = (index + 1) % weights.len();
        }
        Ok(parts)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FinancialObjectKind {
    Institution,
    Account,
    Statement,
    Transaction,
    PendingTransaction,
    PostedTransaction,
    Split,
    Transfer,
    Category,
    Payee,
    RecurringStream,
    Budget,
    Goal,
    Debt,
    Asset,
    Liability,
    Receipt,
    Invoice,
    Reimbursement,
    TaxLabel,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinancialSourceRecord {
    pub record_id: String,
    pub kind: FinancialObjectKind,
    pub account_id: String,
    pub amount: Money,
    pub effective_date: String,
    pub source_id: String,
    pub source_sha256: String,
    pub conversion_source: Option<String>,
    pub parent_record_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdjustmentRecord {
    pub adjustment_id: String,
    pub original_record_id: String,
    pub supersedes_record_id: Option<String>,
    pub replacement: FinancialSourceRecord,
    pub reason: String,
    pub author_id: String,
    pub recorded_at: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UserDisposition {
    Unresolved,
    Accepted,
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReconciliationRecord {
    pub reconciliation_id: String,
    pub statement_id: String,
    pub account_id: String,
    pub candidate_record_ids: BTreeSet<String>,
    pub duplicate_candidate_ids: BTreeSet<String>,
    pub match_confidence_basis_points: u16,
    pub disposition: UserDisposition,
    pub conflict_reason: Option<String>,
}

fn identifier(value: &str) -> bool {
    !value.is_empty() && value.len() <= 1024
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

impl FinancialSourceRecord {
    pub fn validate(&self) -> Result<(), FinancialError> {
        if !identifier(&self.record_id)
            || !identifier(&self.account_id)
            || !identifier(&self.source_id)
            || !identifier(&self.effective_date)
            || !digest(&self.source_sha256)
            || self.parent_record_id.as_deref() == Some(self.record_id.as_str())
        {
            return Err(FinancialError::InvalidRecord);
        }
        Ok(())
    }
}

impl AdjustmentRecord {
    pub fn validate(&self) -> Result<(), FinancialError> {
        self.replacement.validate()?;
        if !identifier(&self.adjustment_id)
            || !identifier(&self.original_record_id)
            || !identifier(&self.reason)
            || !identifier(&self.author_id)
            || !identifier(&self.recorded_at)
            || self.replacement.record_id == self.original_record_id
            || self.replacement.parent_record_id.as_deref()
                != Some(self.original_record_id.as_str())
        {
            return Err(FinancialError::LineageConflict);
        }
        Ok(())
    }
}

impl ReconciliationRecord {
    pub fn validate(&self) -> Result<(), FinancialError> {
        if !identifier(&self.reconciliation_id)
            || !identifier(&self.statement_id)
            || !identifier(&self.account_id)
            || self.match_confidence_basis_points > 10_000
            || !self
                .duplicate_candidate_ids
                .is_subset(&self.candidate_record_ids)
            || (self.disposition == UserDisposition::Unresolved && self.conflict_reason.is_none())
        {
            return Err(FinancialError::InvalidRecord);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usd(units: i128, scale: u8) -> Money {
        Money::new(units, Currency::new("USD").unwrap(), scale).unwrap()
    }

    #[test]
    fn checked_arithmetic_rejects_currency_scale_and_overflow() {
        assert_eq!(
            usd(100, 2).checked_add(&usd(50, 2)).unwrap().minor_units(),
            150
        );
        assert_eq!(
            usd(1, 2).checked_add(&usd(1, 3)),
            Err(FinancialError::ScaleMismatch)
        );
        let eur = Money::new(1, Currency::new("EUR").unwrap(), 2).unwrap();
        assert_eq!(
            usd(1, 2).checked_add(&eur),
            Err(FinancialError::CurrencyMismatch)
        );
        assert_eq!(
            usd(i128::MAX, 2).checked_add(&usd(1, 2)),
            Err(FinancialError::Overflow)
        );
    }

    #[test]
    fn rounding_is_explicit_and_half_even_is_exact() {
        assert_eq!(
            usd(125, 2)
                .rescale(1, RoundingMode::HalfEven)
                .unwrap()
                .minor_units(),
            12
        );
        assert_eq!(
            usd(135, 2)
                .rescale(1, RoundingMode::HalfEven)
                .unwrap()
                .minor_units(),
            14
        );
        assert_eq!(
            usd(-121, 2)
                .rescale(1, RoundingMode::Floor)
                .unwrap()
                .minor_units(),
            -13
        );
        assert_eq!(
            usd(121, 2)
                .rescale(1, RoundingMode::Ceiling)
                .unwrap()
                .minor_units(),
            13
        );
    }

    #[test]
    fn allocation_is_stable_and_conserves_exact_total() {
        let parts = usd(10, 2).allocate(&[1, 1, 1]).unwrap();
        assert_eq!(
            parts.iter().map(Money::minor_units).collect::<Vec<_>>(),
            vec![4, 3, 3]
        );
        assert_eq!(parts.iter().map(Money::minor_units).sum::<i128>(), 10);
        assert_eq!(
            usd(-10, 2)
                .allocate(&[1, 1, 1])
                .unwrap()
                .iter()
                .map(Money::minor_units)
                .sum::<i128>(),
            -10
        );
    }

    #[test]
    fn corrections_preserve_original_identity_and_complete_lineage() {
        let replacement = FinancialSourceRecord {
            record_id: "txn-corrected".into(),
            kind: FinancialObjectKind::Transaction,
            account_id: "account".into(),
            amount: usd(100, 2),
            effective_date: "2026-09-04".into(),
            source_id: "import-1".into(),
            source_sha256: "a".repeat(64),
            conversion_source: None,
            parent_record_id: Some("txn-original".into()),
        };
        let adjustment = AdjustmentRecord {
            adjustment_id: "adjustment".into(),
            original_record_id: "txn-original".into(),
            supersedes_record_id: None,
            replacement,
            reason: "source correction".into(),
            author_id: "user".into(),
            recorded_at: "2026-09-04T00:00:00Z".into(),
        };
        assert_eq!(adjustment.validate(), Ok(()));
    }

    #[test]
    fn reconciliation_requires_visible_conflicts_and_bounded_confidence() {
        let record = ReconciliationRecord {
            reconciliation_id: "r".into(),
            statement_id: "s".into(),
            account_id: "a".into(),
            candidate_record_ids: BTreeSet::from(["t".into()]),
            duplicate_candidate_ids: BTreeSet::new(),
            match_confidence_basis_points: 10_000,
            disposition: UserDisposition::Unresolved,
            conflict_reason: Some("currency mismatch".into()),
        };
        assert_eq!(record.validate(), Ok(()));
    }
}
