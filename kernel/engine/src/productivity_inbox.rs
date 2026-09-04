//! Local cited activity inbox that never mutates provider state.
use std::cmp::Ordering;
/// Closed local inbox dispositions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[allow(missing_docs)]
pub enum InboxState {
    Action,
    Response,
    Decision,
    Waiting,
    Blocked,
    Due,
    Reference,
    Duplicate,
    Stale,
    Incomplete,
    Uncertain,
    Dismissed,
    Completed,
}
/// Priority origin stays visible and non-authoritative.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PriorityOrigin {
    /// Deterministic local rule.
    Deterministic,
    /// Explicit user-authored rule.
    UserRule,
    /// Labeled model suggestion.
    ModelSuggestion,
}
/// One native record projection; source truth and local/model state are separate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InboxItem {
    /// Provider namespace.
    pub provider_id: String,
    /// Account namespace.
    pub account_id: String,
    /// Immutable native identity.
    pub native_id: String,
    /// Native item class.
    pub item_class: String,
    /// Source receipt citations.
    pub citations: Vec<String>,
    /// Observation epoch.
    pub observed_at: u64,
    /// Effective source epoch.
    pub effective_at: u64,
    /// Freshness deadline.
    pub fresh_until: u64,
    /// Classification.
    pub classification: String,
    /// Provider synchronization truth.
    pub provider_state: String,
    /// Local-only disposition.
    pub local_state: InboxState,
    /// Optional labeled model suggestion.
    pub model_suggestion: Option<String>,
    /// Priority provenance.
    pub priority_origin: PriorityOrigin,
    /// Permission/history gap reason.
    pub coverage_gap: Option<String>,
    /// Grouping key that never merges native records.
    pub group_key: String,
    /// Deterministic numeric priority.
    pub priority: u16,
}
/// Closed filters over exact stored fields.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InboxFilter {
    /// Provider filter.
    pub provider_id: Option<String>,
    /// Account filter.
    pub account_id: Option<String>,
    /// Class filter.
    pub item_class: Option<String>,
    /// Local-state filter.
    pub state: Option<InboxState>,
    /// Classification filter.
    pub classification: Option<String>,
    /// Require a visible coverage gap.
    pub has_coverage_gap: Option<bool>,
}
/// Stable fail-closed inbox result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InboxError {
    /// Missing native/source identity or citation.
    InvalidSource,
    /// A model suggestion was not labeled.
    UnlabeledSuggestion,
}
/// Validates one item without granting provider authority.
pub fn validate_item(item: &InboxItem) -> Result<(), InboxError> {
    if item.provider_id.is_empty()
        || item.account_id.is_empty()
        || item.native_id.is_empty()
        || item.citations.is_empty()
        || item.citations.iter().any(|v| v.len() != 64)
    {
        return Err(InboxError::InvalidSource);
    }
    if item.model_suggestion.is_some() && item.priority_origin != PriorityOrigin::ModelSuggestion {
        return Err(InboxError::UnlabeledSuggestion);
    }
    Ok(())
}
/// Returns a stable filtered order while preserving every native item.
pub fn query<'a>(
    items: &'a [InboxItem],
    filter: &InboxFilter,
) -> Result<Vec<&'a InboxItem>, InboxError> {
    for item in items {
        validate_item(item)?
    }
    let mut out: Vec<_> = items
        .iter()
        .filter(|i| {
            filter
                .provider_id
                .as_ref()
                .is_none_or(|v| v == &i.provider_id)
                && filter
                    .account_id
                    .as_ref()
                    .is_none_or(|v| v == &i.account_id)
                && filter
                    .item_class
                    .as_ref()
                    .is_none_or(|v| v == &i.item_class)
                && filter.state.is_none_or(|v| v == i.local_state)
                && filter
                    .classification
                    .as_ref()
                    .is_none_or(|v| v == &i.classification)
                && filter
                    .has_coverage_gap
                    .is_none_or(|v| v == i.coverage_gap.is_some())
        })
        .collect();
    out.sort_by(|a, b| match b.priority.cmp(&a.priority) {
        Ordering::Equal => (a.effective_at, a.provider_id.as_str(), a.native_id.as_str()).cmp(&(
            b.effective_at,
            b.provider_id.as_str(),
            b.native_id.as_str(),
        )),
        v => v,
    });
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn item(id: &str) -> InboxItem {
        InboxItem {
            provider_id: "p".into(),
            account_id: "a".into(),
            native_id: id.into(),
            item_class: "message".into(),
            citations: vec!["a".repeat(64)],
            observed_at: 1,
            effective_at: 2,
            fresh_until: 3,
            classification: "internal".into(),
            provider_state: "current".into(),
            local_state: InboxState::Action,
            model_suggestion: None,
            priority_origin: PriorityOrigin::Deterministic,
            coverage_gap: None,
            group_key: "subject".into(),
            priority: 10,
        }
    }
    #[test]
    fn duplicates_remain_native_records() {
        let items = [item("one"), item("two")];
        let out = query(&items, &InboxFilter::default()).unwrap();
        assert_eq!(out.len(), 2);
        assert_ne!(out[0].native_id, out[1].native_id)
    }
    #[test]
    fn incomplete_coverage_is_filterable() {
        let mut i = item("one");
        i.coverage_gap = Some("permission_gap".into());
        i.local_state = InboxState::Incomplete;
        let items = [i];
        assert_eq!(
            query(
                &items,
                &InboxFilter {
                    has_coverage_gap: Some(true),
                    ..Default::default()
                }
            )
            .unwrap()
            .len(),
            1
        )
    }
    #[test]
    fn model_suggestion_must_stay_labeled() {
        let mut i = item("one");
        i.model_suggestion = Some("priority".into());
        assert_eq!(validate_item(&i), Err(InboxError::UnlabeledSuggestion));
        i.priority_origin = PriorityOrigin::ModelSuggestion;
        assert_eq!(validate_item(&i), Ok(()))
    }
}
