//! Exact, bounded reconciliation methods that never coerce source values through floating point.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::tabular::{TabularComparison, TabularMatchReason};

const MAX_DECIMAL_DIGITS: usize = 4_096;
const MAX_ALLOCATION_RECIPIENTS: usize = 100_000;

/// Stable failure for an exact reconciliation method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdvancedReconciliationError {
    /// A decimal, policy, identity, or source binding is malformed.
    InvalidInput,
    /// An admitted digit, scale, recipient, or weight limit was exceeded.
    ResourceLimit,
    /// A source is not current for the declared reconciliation instant.
    StaleSource,
}

impl AdvancedReconciliationError {
    /// Stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "reconciliation.advanced.invalid",
            Self::ResourceLimit => "reconciliation.advanced.resource_limit",
            Self::StaleSource => "reconciliation.advanced.stale_source",
        }
    }
}

impl std::fmt::Display for AdvancedReconciliationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for AdvancedReconciliationError {}

/// Exact base-ten decimal retained as canonical digits and scale.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExactDecimal {
    /// Whether the non-zero value is negative.
    pub negative: bool,
    /// Canonical unsigned coefficient with no leading zeroes.
    pub digits: String,
    /// Number of digits to the right of the decimal point.
    pub scale: u32,
}

impl ExactDecimal {
    /// Parses a plain base-ten value without exponent or floating-point conversion.
    pub fn parse(value: &str) -> Result<Self, AdvancedReconciliationError> {
        if value.is_empty() || value.trim() != value {
            return Err(AdvancedReconciliationError::InvalidInput);
        }
        let (negative, unsigned) = value
            .strip_prefix('-')
            .map_or((false, value), |remaining| (true, remaining));
        let (integer, fraction) = unsigned
            .split_once('.')
            .map_or((unsigned, ""), |parts| parts);
        if integer.is_empty()
            || !integer.bytes().all(|byte| byte.is_ascii_digit())
            || !fraction.bytes().all(|byte| byte.is_ascii_digit())
            || unsigned.matches('.').count() > 1
        {
            return Err(AdvancedReconciliationError::InvalidInput);
        }
        let digit_count = integer.len().saturating_add(fraction.len());
        if digit_count == 0 || digit_count > MAX_DECIMAL_DIGITS {
            return Err(AdvancedReconciliationError::ResourceLimit);
        }
        let scale = u32::try_from(fraction.len())
            .map_err(|_| AdvancedReconciliationError::ResourceLimit)?;
        Self::from_parts(negative, format!("{integer}{fraction}"), scale)
    }

    fn from_parts(
        negative: bool,
        mut digits: String,
        scale: u32,
    ) -> Result<Self, AdvancedReconciliationError> {
        if digits.is_empty()
            || digits.len() > MAX_DECIMAL_DIGITS
            || !digits.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(AdvancedReconciliationError::ResourceLimit);
        }
        let first_nonzero = digits
            .bytes()
            .position(|byte| byte != b'0')
            .unwrap_or(digits.len().saturating_sub(1));
        digits.drain(..first_nonzero);
        if digits.is_empty() {
            digits.push('0');
        }
        Ok(Self {
            negative: negative && digits != "0",
            digits,
            scale,
        })
    }

    fn aligned_digits(&self, scale: u32) -> Result<String, AdvancedReconciliationError> {
        let padding = usize::try_from(scale.saturating_sub(self.scale))
            .map_err(|_| AdvancedReconciliationError::ResourceLimit)?;
        if self.digits.len().saturating_add(padding) > MAX_DECIMAL_DIGITS {
            return Err(AdvancedReconciliationError::ResourceLimit);
        }
        Ok(format!("{}{}", self.digits, "0".repeat(padding)))
    }

    /// Canonical plain base-ten rendering.
    #[must_use]
    pub fn canonical(&self) -> String {
        let mut digits = self.digits.clone();
        let scale = usize::try_from(self.scale).unwrap_or(usize::MAX);
        let body = if scale == 0 {
            digits
        } else if digits.len() > scale {
            digits.insert(digits.len() - scale, '.');
            digits
        } else {
            format!("0.{}{}", "0".repeat(scale - digits.len()), digits)
        };
        if self.negative {
            format!("-{body}")
        } else {
            body
        }
    }
}

fn compare_unsigned(left: &str, right: &str) -> Ordering {
    left.len().cmp(&right.len()).then_with(|| left.cmp(right))
}

fn subtract_unsigned(larger: &str, smaller: &str) -> String {
    let mut right = smaller.bytes().rev();
    let mut borrow = 0_i16;
    let mut result = Vec::with_capacity(larger.len());
    for left in larger.bytes().rev() {
        let mut value = i16::from(left - b'0') - borrow;
        let other = right.next().map_or(0, |byte| i16::from(byte - b'0'));
        if value < other {
            value += 10;
            borrow = 1;
        } else {
            borrow = 0;
        }
        result.push(u8::try_from(value - other).unwrap_or(0) + b'0');
    }
    while result.len() > 1 && result.last() == Some(&b'0') {
        result.pop();
    }
    result.reverse();
    String::from_utf8(result).unwrap_or_else(|_| "0".to_owned())
}

fn add_unsigned(left: &str, right: &str) -> Result<String, AdvancedReconciliationError> {
    let mut left = left.bytes().rev();
    let mut right = right.bytes().rev();
    let mut carry = 0_u8;
    let mut output = Vec::new();
    loop {
        let first = left.next();
        let second = right.next();
        if first.is_none() && second.is_none() && carry == 0 {
            break;
        }
        let sum =
            first.map_or(0, |value| value - b'0') + second.map_or(0, |value| value - b'0') + carry;
        output.push((sum % 10) + b'0');
        carry = sum / 10;
        if output.len() > MAX_DECIMAL_DIGITS {
            return Err(AdvancedReconciliationError::ResourceLimit);
        }
    }
    output.reverse();
    String::from_utf8(output).map_err(|_| AdvancedReconciliationError::InvalidInput)
}

fn absolute_difference(
    left: &ExactDecimal,
    right: &ExactDecimal,
) -> Result<ExactDecimal, AdvancedReconciliationError> {
    let scale = left.scale.max(right.scale);
    let left_digits = left.aligned_digits(scale)?;
    let right_digits = right.aligned_digits(scale)?;
    if left.negative != right.negative {
        ExactDecimal::from_parts(false, add_unsigned(&left_digits, &right_digits)?, scale)
    } else {
        let difference = match compare_unsigned(&left_digits, &right_digits) {
            Ordering::Less => subtract_unsigned(&right_digits, &left_digits),
            _ => subtract_unsigned(&left_digits, &right_digits),
        };
        ExactDecimal::from_parts(false, difference, scale)
    }
}

/// Closed financial rounding rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinancialRoundingMode {
    /// Ties round to an even retained digit.
    HalfEven,
    /// Ties round away from zero.
    HalfAwayFromZero,
}

/// Rounds an exact decimal under an explicit base-ten policy.
pub fn round_financial(
    value: &ExactDecimal,
    target_scale: u32,
    mode: FinancialRoundingMode,
) -> Result<ExactDecimal, AdvancedReconciliationError> {
    if target_scale >= value.scale {
        return Ok(value.clone());
    }
    let removed = usize::try_from(value.scale - target_scale)
        .map_err(|_| AdvancedReconciliationError::ResourceLimit)?;
    let split = value.digits.len().saturating_sub(removed);
    let retained = if split == 0 {
        "0"
    } else {
        &value.digits[..split]
    };
    let padded;
    let discarded = if split == 0 {
        padded = format!(
            "{}{}",
            "0".repeat(removed - value.digits.len()),
            value.digits
        );
        padded.as_str()
    } else {
        &value.digits[split..]
    };
    let above_half = discarded.as_bytes()[0] > b'5'
        || (discarded.as_bytes()[0] == b'5'
            && discarded.as_bytes()[1..].iter().any(|digit| *digit != b'0'));
    let exactly_half = discarded.as_bytes()[0] == b'5'
        && discarded.as_bytes()[1..].iter().all(|digit| *digit == b'0');
    let odd = retained
        .as_bytes()
        .last()
        .is_some_and(|digit| (digit - b'0') % 2 == 1);
    let increment =
        above_half || (exactly_half && (mode == FinancialRoundingMode::HalfAwayFromZero || odd));
    let rounded = if increment {
        add_unsigned(retained, "1")?
    } else {
        retained.to_owned()
    };
    ExactDecimal::from_parts(value.negative, rounded, target_scale)
}

/// True when the absolute exact difference is within the inclusive tolerance.
pub fn within_tolerance(
    left: &ExactDecimal,
    right: &ExactDecimal,
    tolerance: &ExactDecimal,
) -> Result<bool, AdvancedReconciliationError> {
    if tolerance.negative {
        return Err(AdvancedReconciliationError::InvalidInput);
    }
    let difference = absolute_difference(left, right)?;
    let scale = difference.scale.max(tolerance.scale);
    Ok(compare_unsigned(
        &difference.aligned_digits(scale)?,
        &tolerance.aligned_digits(scale)?,
    ) != Ordering::Greater)
}

/// Source identity and freshness observation used by reconciliation admission.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReconciliationSourceFreshness {
    /// Exact captured source digest.
    pub captured_sha256: String,
    /// Digest observed immediately before reconciliation.
    pub current_sha256: String,
    /// Capture instant as Unix seconds.
    pub captured_at_unix_seconds: u64,
}

/// Fails when a source changed, came from the future, or exceeded the declared maximum age.
pub fn require_current_source(
    source: &ReconciliationSourceFreshness,
    as_of_unix_seconds: u64,
    maximum_age_seconds: u64,
) -> Result<(), AdvancedReconciliationError> {
    if source.captured_sha256.len() != 64
        || source.current_sha256.len() != 64
        || !source
            .captured_sha256
            .bytes()
            .chain(source.current_sha256.bytes())
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        || maximum_age_seconds == 0
    {
        return Err(AdvancedReconciliationError::InvalidInput);
    }
    if source.captured_sha256 != source.current_sha256
        || source.captured_at_unix_seconds > as_of_unix_seconds
        || as_of_unix_seconds - source.captured_at_unix_seconds > maximum_age_seconds
    {
        return Err(AdvancedReconciliationError::StaleSource);
    }
    Ok(())
}

/// One stable weighted recipient.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllocationRecipient {
    /// Stable non-secret recipient identity.
    pub recipient_id: String,
    /// Positive integer weight.
    pub weight: u64,
}

/// One exact deterministic allocation result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllocationResult {
    /// Stable recipient identity.
    pub recipient_id: String,
    /// Exact allocated amount.
    pub amount: ExactDecimal,
}

fn divide_weighted(digits: &str, weight: u64, divisor: u64) -> (String, u64) {
    let mut quotient = String::with_capacity(digits.len());
    let mut remainder = 0_u128;
    for digit in digits.bytes() {
        let numerator = remainder * 10 + u128::from(digit - b'0') * u128::from(weight);
        let next = numerator / u128::from(divisor);
        remainder = numerator % u128::from(divisor);
        quotient.push(char::from(u8::try_from(next).unwrap_or(0) + b'0'));
    }
    let trimmed = quotient.trim_start_matches('0');
    (
        if trimmed.is_empty() { "0" } else { trimmed }.to_owned(),
        u64::try_from(remainder).unwrap_or(0),
    )
}

/// Allocates an exact total by positive weights and assigns indivisible remainder units stably.
pub fn allocate_many_to_many(
    total: &ExactDecimal,
    recipients: &[AllocationRecipient],
) -> Result<Vec<AllocationResult>, AdvancedReconciliationError> {
    if recipients.is_empty() || recipients.len() > MAX_ALLOCATION_RECIPIENTS {
        return Err(AdvancedReconciliationError::ResourceLimit);
    }
    let mut ordered = recipients.to_vec();
    ordered.sort_by(|left, right| left.recipient_id.cmp(&right.recipient_id));
    if ordered.iter().any(|item| {
        item.weight == 0
            || item.recipient_id.is_empty()
            || item.recipient_id.len() > 128
            || !item.recipient_id.is_ascii()
    }) || ordered
        .windows(2)
        .any(|items| items[0].recipient_id == items[1].recipient_id)
    {
        return Err(AdvancedReconciliationError::InvalidInput);
    }
    let weight_sum = ordered.iter().try_fold(0_u64, |sum, item| {
        sum.checked_add(item.weight)
            .ok_or(AdvancedReconciliationError::ResourceLimit)
    })?;
    let mut shares = ordered
        .iter()
        .map(|item| {
            let (digits, remainder) = divide_weighted(&total.digits, item.weight, weight_sum);
            (item.recipient_id.clone(), digits, remainder)
        })
        .collect::<Vec<_>>();
    let base_sum = shares
        .iter()
        .try_fold("0".to_owned(), |sum, item| add_unsigned(&sum, &item.1))?;
    let remaining = subtract_unsigned(&total.digits, &base_sum)
        .parse::<usize>()
        .map_err(|_| AdvancedReconciliationError::ResourceLimit)?;
    let mut priority = (0..shares.len()).collect::<Vec<_>>();
    priority.sort_by(|left, right| {
        shares[*right]
            .2
            .cmp(&shares[*left].2)
            .then_with(|| shares[*left].0.cmp(&shares[*right].0))
    });
    for index in priority.into_iter().take(remaining) {
        shares[index].1 = add_unsigned(&shares[index].1, "1")?;
    }
    shares.sort_by(|left, right| left.0.cmp(&right.0));
    shares
        .into_iter()
        .map(|(recipient_id, digits, _)| {
            Ok(AllocationResult {
                recipient_id,
                amount: ExactDecimal::from_parts(total.negative, digits, total.scale)?,
            })
        })
        .collect()
}

/// Hash-only method record proving the exact comparison and policy choices used.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdvancedReconciliationRecord {
    /// Exact left source digest.
    pub left_source_sha256: String,
    /// Exact right source digest.
    pub right_source_sha256: String,
    /// Stable method identity.
    pub method_id: String,
    /// Stable method version.
    pub method_version: String,
    /// Closed schema decision.
    pub schema_decision: String,
    /// Closed type decision.
    pub type_decision: String,
    /// Closed formula policy.
    pub formula_policy_id: String,
    /// Exact join columns.
    pub join_keys: Vec<String>,
    /// Exact inclusive tolerance.
    pub tolerance: ExactDecimal,
    /// Exact financial rounding policy.
    pub rounding_mode: FinancialRoundingMode,
    /// Keys missing from one side.
    pub unmatched_key_count: usize,
    /// Keys with conflicting values or multiplicity.
    pub conflicting_key_count: usize,
    /// Exact left total supplied by the caller.
    pub left_total: ExactDecimal,
    /// Exact right total supplied by the caller.
    pub right_total: ExactDecimal,
    /// Whether the supplied totals reconcile within tolerance.
    pub totals_within_tolerance: bool,
}

/// Builds one deterministic method record from a hash-only tabular comparison.
pub fn record_advanced_reconciliation(
    comparison: &TabularComparison,
    tolerance: ExactDecimal,
    rounding_mode: FinancialRoundingMode,
    left_total: ExactDecimal,
    right_total: ExactDecimal,
) -> Result<AdvancedReconciliationRecord, AdvancedReconciliationError> {
    let unmatched_key_count = comparison
        .matches
        .iter()
        .filter(|item| {
            matches!(
                item.reason,
                TabularMatchReason::MissingLeft | TabularMatchReason::MissingRight
            )
        })
        .count();
    let conflicting_key_count = comparison
        .matches
        .iter()
        .filter(|item| {
            matches!(
                item.reason,
                TabularMatchReason::DifferingFields
                    | TabularMatchReason::DuplicateLeft
                    | TabularMatchReason::DuplicateRight
            )
        })
        .count();
    let totals_within_tolerance = within_tolerance(&left_total, &right_total, &tolerance)?;
    Ok(AdvancedReconciliationRecord {
        left_source_sha256: comparison.left_source_sha256.clone(),
        right_source_sha256: comparison.right_source_sha256.clone(),
        method_id: "exact-decimal-reconciliation".to_owned(),
        method_version: "1".to_owned(),
        schema_decision: "closed-tabular-comparison-v1".to_owned(),
        type_decision: "string-backed-base10-decimal".to_owned(),
        formula_policy_id: "closed-summary-formulas-v1".to_owned(),
        join_keys: comparison.key_columns.clone(),
        tolerance,
        rounding_mode,
        unmatched_key_count,
        conflicting_key_count,
        left_total,
        right_total,
        totals_within_tolerance,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_thousands_of_digits_without_float_coercion() {
        let text = format!("{}.{:0>1000}", "9".repeat(3_000), "1");
        let value = ExactDecimal::parse(&text).expect("exact decimal");
        assert_eq!(value.canonical(), text);
        assert_eq!(
            ExactDecimal::parse("-000.1200").unwrap().canonical(),
            "-0.1200"
        );
        assert!(ExactDecimal::parse("1e3").is_err());
        assert_eq!(
            ExactDecimal::parse(&"1".repeat(MAX_DECIMAL_DIGITS + 1)),
            Err(AdvancedReconciliationError::ResourceLimit)
        );
    }

    #[test]
    fn tolerance_is_inclusive_and_sign_correct() {
        let tolerance = ExactDecimal::parse("0.005").unwrap();
        assert!(
            within_tolerance(
                &ExactDecimal::parse("10.000").unwrap(),
                &ExactDecimal::parse("10.005").unwrap(),
                &tolerance
            )
            .unwrap()
        );
        assert!(
            !within_tolerance(
                &ExactDecimal::parse("-10").unwrap(),
                &ExactDecimal::parse("10").unwrap(),
                &tolerance
            )
            .unwrap()
        );
        assert_eq!(
            within_tolerance(
                &ExactDecimal::parse("1").unwrap(),
                &ExactDecimal::parse("1").unwrap(),
                &ExactDecimal::parse("-0.1").unwrap()
            ),
            Err(AdvancedReconciliationError::InvalidInput)
        );
    }

    #[test]
    fn financial_rounding_ties_are_explicit() {
        let even = FinancialRoundingMode::HalfEven;
        assert_eq!(
            round_financial(&ExactDecimal::parse("2.345").unwrap(), 2, even)
                .unwrap()
                .canonical(),
            "2.34"
        );
        assert_eq!(
            round_financial(&ExactDecimal::parse("2.355").unwrap(), 2, even)
                .unwrap()
                .canonical(),
            "2.36"
        );
        assert_eq!(
            round_financial(
                &ExactDecimal::parse("-2.345").unwrap(),
                2,
                FinancialRoundingMode::HalfAwayFromZero
            )
            .unwrap()
            .canonical(),
            "-2.35"
        );
    }

    #[test]
    fn weighted_allocation_conserves_exact_minor_units() {
        let result = allocate_many_to_many(
            &ExactDecimal::parse("10.00").unwrap(),
            &[
                AllocationRecipient {
                    recipient_id: "b".to_owned(),
                    weight: 1,
                },
                AllocationRecipient {
                    recipient_id: "a".to_owned(),
                    weight: 1,
                },
                AllocationRecipient {
                    recipient_id: "c".to_owned(),
                    weight: 1,
                },
            ],
        )
        .unwrap();
        assert_eq!(
            result
                .iter()
                .map(|item| (item.recipient_id.as_str(), item.amount.canonical()))
                .collect::<Vec<_>>(),
            [
                ("a", "3.34".to_owned()),
                ("b", "3.33".to_owned()),
                ("c", "3.33".to_owned())
            ]
        );
    }

    #[test]
    fn allocation_rejects_duplicate_zero_and_excessive_recipients() {
        let total = ExactDecimal::parse("1.00").unwrap();
        let duplicate = AllocationRecipient {
            recipient_id: "same".to_owned(),
            weight: 1,
        };
        assert_eq!(
            allocate_many_to_many(&total, &[duplicate.clone(), duplicate]),
            Err(AdvancedReconciliationError::InvalidInput)
        );
        assert_eq!(
            allocate_many_to_many(
                &total,
                &[AllocationRecipient {
                    recipient_id: "zero".to_owned(),
                    weight: 0,
                }]
            ),
            Err(AdvancedReconciliationError::InvalidInput)
        );
        assert_eq!(
            allocate_many_to_many(&total, &[]),
            Err(AdvancedReconciliationError::ResourceLimit)
        );
    }

    #[test]
    fn stale_or_changed_sources_fail_closed() {
        let source = ReconciliationSourceFreshness {
            captured_sha256: "a".repeat(64),
            current_sha256: "a".repeat(64),
            captured_at_unix_seconds: 100,
        };
        assert_eq!(require_current_source(&source, 110, 10), Ok(()));
        assert_eq!(
            require_current_source(&source, 111, 10),
            Err(AdvancedReconciliationError::StaleSource)
        );
        let mut changed = source;
        changed.current_sha256 = "b".repeat(64);
        assert_eq!(
            require_current_source(&changed, 110, 10),
            Err(AdvancedReconciliationError::StaleSource)
        );
    }

    #[test]
    fn method_record_binds_sources_policies_totals_and_discrepancies() {
        let comparison = TabularComparison {
            schema_version: 2,
            left_source_sha256: "a".repeat(64),
            right_source_sha256: "b".repeat(64),
            key_columns: vec!["account".to_owned()],
            matches: vec![
                crate::tabular::TabularMatch {
                    key_sha256: "c".repeat(64),
                    reason: TabularMatchReason::MissingRight,
                    left_rows: vec![1],
                    right_rows: vec![],
                    differing_fields: vec![],
                },
                crate::tabular::TabularMatch {
                    key_sha256: "d".repeat(64),
                    reason: TabularMatchReason::DifferingFields,
                    left_rows: vec![2],
                    right_rows: vec![1],
                    differing_fields: vec!["amount".to_owned()],
                },
            ],
            overlap_key_count: 1,
            filesystem_effect_performed: false,
            network_access_performed: false,
            execution_performed: false,
        };
        let record = record_advanced_reconciliation(
            &comparison,
            ExactDecimal::parse("0.01").unwrap(),
            FinancialRoundingMode::HalfEven,
            ExactDecimal::parse("100.00").unwrap(),
            ExactDecimal::parse("100.01").unwrap(),
        )
        .unwrap();
        assert_eq!(record.left_source_sha256, "a".repeat(64));
        assert_eq!(record.join_keys, ["account"]);
        assert_eq!(record.unmatched_key_count, 1);
        assert_eq!(record.conflicting_key_count, 1);
        assert!(record.totals_within_tolerance);
        assert_eq!(record.type_decision, "string-backed-base10-decimal");
    }
}
