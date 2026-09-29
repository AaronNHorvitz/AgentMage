//! Content-free inspection of one composed context packet, and a check that a
//! recomposition keeps every retained constraint and source link (CAP-28).
//!
//! Inspection shows what the model receives and why other sources were left
//! out: pinned constraints, selected sources, token and byte use, and each
//! omission reason. It carries identities and counts, never excerpts, and it
//! grants nothing. The survival check compares two compositions of the same
//! session state, such as a smaller-budget reflow or a model switch with
//! another token counter. It does not judge a later turn whose request or
//! sources legitimately changed.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use agentmage_kernel_contracts::{
    ComposedContextPacket, ContextItemKind, ContextOmissionReason, ContextSensitivity,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::context_management::verify_composed_context;

/// Largest rendered inspection; longer text ends with an explicit truncation line.
pub const MAX_CONTEXT_INSPECTION_RENDER_BYTES: usize = 64 * 1024;

/// Whether a context item constrains the task and so must survive recomposition.
#[must_use]
pub const fn is_retained_constraint(kind: ContextItemKind, essential: bool) -> bool {
    essential
        || matches!(
            kind,
            ContextItemKind::Instruction
                | ContextItemKind::NewestRequest
                | ContextItemKind::Correction
                | ContextItemKind::Approval
                | ContextItemKind::ActiveObjective
                | ContextItemKind::Blocker
                | ContextItemKind::ExpectedOutput
        )
}

/// Content-free refusal to inspect or compare packets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextInspectionError {
    /// A packet failed the composed-context verification.
    InvalidPacket,
    /// Canonical serialization failed.
    Serialization,
}

/// One accounted source, without its excerpt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InspectedContextItem {
    /// Stable candidate identity.
    pub item_id: String,
    /// Semantic class.
    pub kind: ContextItemKind,
    /// Sensitivity label.
    pub sensitivity: ContextSensitivity,
    /// Stable source identity.
    pub source_id: String,
    /// Exact source revision.
    pub source_revision: String,
    /// Source content digest.
    pub content_sha256: String,
    /// Accounted tokens.
    pub token_count: u32,
    /// Accounted bytes.
    pub byte_count: u64,
    /// Why the item was left out, when it was.
    pub omission: Option<ContextOmissionReason>,
}

/// Token and byte use of one semantic class among included items.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ContextKindTotal {
    /// Semantic class.
    pub kind: ContextItemKind,
    /// Included items of this class.
    pub items: u32,
    /// Their tokens.
    pub tokens: u64,
    /// Their bytes.
    pub bytes: u64,
}

/// Inspectable, content-free view of one composed packet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ContextInspection {
    /// View schema version.
    pub schema_version: u16,
    /// Inspected packet identity.
    pub context_packet_id: String,
    /// Inspected packet digest.
    pub packet_sha256: String,
    /// Exact token counter the budget used.
    pub token_counter_id: String,
    /// Inclusive token ceiling and use.
    pub max_tokens: u32,
    /// Admitted tokens.
    pub used_tokens: u32,
    /// Inclusive byte ceiling.
    pub max_bytes: u64,
    /// Admitted bytes.
    pub used_bytes: u64,
    /// Included retained constraints, in packet order.
    pub pinned: Vec<InspectedContextItem>,
    /// Other included sources, in packet order.
    pub selected: Vec<InspectedContextItem>,
    /// Omitted sources with their reasons, by item identity.
    pub omitted: Vec<InspectedContextItem>,
    /// Included use per semantic class.
    pub kind_totals: Vec<ContextKindTotal>,
}

/// Inspects one verified packet.
pub fn inspect_context(
    packet: &ComposedContextPacket,
) -> Result<ContextInspection, ContextInspectionError> {
    verify_composed_context(packet).map_err(|_| ContextInspectionError::InvalidPacket)?;
    let accounting: BTreeMap<&str, _> = packet
        .accounting
        .iter()
        .map(|item| (item.item_id.as_str(), item))
        .collect();
    let mut pinned = Vec::new();
    let mut selected = Vec::new();
    let mut totals: BTreeMap<ContextItemKind, ContextKindTotal> = BTreeMap::new();
    for item in &packet.items {
        let accounted = accounting
            .get(item.item_id.as_str())
            .ok_or(ContextInspectionError::InvalidPacket)?;
        let view = inspected(accounted);
        let total = totals.entry(item.kind).or_insert(ContextKindTotal {
            kind: item.kind,
            items: 0,
            tokens: 0,
            bytes: 0,
        });
        total.items += 1;
        total.tokens += u64::from(view.token_count);
        total.bytes += view.byte_count;
        if is_retained_constraint(item.kind, item.essential) {
            pinned.push(view);
        } else {
            selected.push(view);
        }
    }
    let omitted = packet
        .accounting
        .iter()
        .filter(|item| !item.included)
        .map(inspected)
        .collect();
    Ok(ContextInspection {
        schema_version: 1,
        context_packet_id: packet.context_packet_id.as_str().to_owned(),
        packet_sha256: packet.packet_sha256.clone(),
        token_counter_id: packet.token_counter_id.clone(),
        max_tokens: packet.max_tokens,
        used_tokens: packet.used_tokens,
        max_bytes: packet.max_bytes,
        used_bytes: packet.used_bytes,
        pinned,
        selected,
        omitted,
        kind_totals: totals.into_values().collect(),
    })
}

fn inspected(item: &agentmage_kernel_contracts::ContextItemAccounting) -> InspectedContextItem {
    InspectedContextItem {
        item_id: item.item_id.clone(),
        kind: item.kind,
        sensitivity: item.sensitivity,
        source_id: item.source_id.clone(),
        source_revision: item.source_revision.clone(),
        content_sha256: item.content_sha256.clone(),
        token_count: item.token_count,
        byte_count: item.byte_count,
        omission: item.omission,
    }
}

/// Bounded text for a person. Identities are shown escaped; no excerpt appears.
#[must_use]
pub fn render_context_inspection(view: &ContextInspection) -> String {
    let mut output = String::new();
    let _ = writeln!(
        output,
        "context {:?} sha256 {} counter {:?}",
        view.context_packet_id,
        &view.packet_sha256[..12],
        view.token_counter_id
    );
    let _ = writeln!(
        output,
        "tokens {}/{} bytes {}/{}",
        view.used_tokens, view.max_tokens, view.used_bytes, view.max_bytes
    );
    let sections = [
        ("pinned constraints", &view.pinned),
        ("selected sources", &view.selected),
        ("omitted sources", &view.omitted),
    ];
    let total: usize = sections.iter().map(|(_, items)| items.len()).sum();
    let mut shown = 0;
    for (title, items) in sections {
        let _ = writeln!(output, "{title}: {}", items.len());
        for item in items {
            let reason = item.omission.map_or(String::new(), |reason| {
                format!(" omitted={}", omission_label(reason))
            });
            let line = format!(
                "- {:?} {:?}@{:?} {} tokens{reason}\n",
                item.kind, item.source_id, item.source_revision, item.token_count
            );
            if output.len() + line.len() > MAX_CONTEXT_INSPECTION_RENDER_BYTES {
                let _ = writeln!(
                    output,
                    "... inspection truncated; {} of {total} sources not shown",
                    total - shown
                );
                return output;
            }
            output.push_str(&line);
            shown += 1;
        }
    }
    output
}

const fn omission_label(reason: ContextOmissionReason) -> &'static str {
    match reason {
        ContextOmissionReason::Duplicate => "duplicate",
        ContextOmissionReason::Budget => "budget",
        ContextOmissionReason::Stale => "stale",
        ContextOmissionReason::Denied => "denied",
    }
}

/// Closed reason a recomposition does not preserve the earlier packet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecompositionViolation {
    /// An included retained constraint is no longer included.
    ConstraintDropped,
    /// A source accounted earlier is absent from the later accounting.
    SourceLinkLost,
    /// The same item identity now names different source material.
    SourceChanged,
}

/// One violation for one item identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RecompositionFinding {
    /// Earlier item identity.
    pub item_id: String,
    /// Violation class.
    pub violation: RecompositionViolation,
}

/// Sealed comparison of two compositions of the same session state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RecompositionReport {
    /// Report schema version.
    pub schema_version: u16,
    /// Earlier packet digest.
    pub before_packet_sha256: String,
    /// Later packet digest.
    pub after_packet_sha256: String,
    /// Whether the token counter changed, as on a model switch.
    pub token_counter_changed: bool,
    /// Retained constraints included earlier and checked.
    pub retained_constraints: u32,
    /// Sources accounted earlier and checked.
    pub carried_sources: u32,
    /// Included earlier, omitted later, and not a constraint; with the later reason.
    pub newly_omitted: Vec<InspectedContextItem>,
    /// Every violation, sorted.
    pub findings: Vec<RecompositionFinding>,
    /// Whether no violation was found.
    pub survives: bool,
    /// Digest sealing every preceding field.
    pub report_sha256: String,
}

/// Compares an earlier and a later composition of the same session state.
pub fn verify_recomposition(
    before: &ComposedContextPacket,
    after: &ComposedContextPacket,
) -> Result<RecompositionReport, ContextInspectionError> {
    verify_composed_context(before).map_err(|_| ContextInspectionError::InvalidPacket)?;
    verify_composed_context(after).map_err(|_| ContextInspectionError::InvalidPacket)?;
    let later: BTreeMap<&str, _> = after
        .accounting
        .iter()
        .map(|item| (item.item_id.as_str(), item))
        .collect();
    let constraints: BTreeSet<&str> = before
        .items
        .iter()
        .filter(|item| is_retained_constraint(item.kind, item.essential))
        .map(|item| item.item_id.as_str())
        .collect();
    let mut findings = Vec::new();
    let mut newly_omitted = Vec::new();
    for earlier in &before.accounting {
        let Some(current) = later.get(earlier.item_id.as_str()) else {
            findings.push(RecompositionFinding {
                item_id: earlier.item_id.clone(),
                violation: RecompositionViolation::SourceLinkLost,
            });
            continue;
        };
        if current.kind != earlier.kind
            || current.source_id != earlier.source_id
            || current.source_revision != earlier.source_revision
            || current.content_sha256 != earlier.content_sha256
        {
            findings.push(RecompositionFinding {
                item_id: earlier.item_id.clone(),
                violation: RecompositionViolation::SourceChanged,
            });
            continue;
        }
        if earlier.included && !current.included {
            if constraints.contains(earlier.item_id.as_str()) {
                findings.push(RecompositionFinding {
                    item_id: earlier.item_id.clone(),
                    violation: RecompositionViolation::ConstraintDropped,
                });
            } else {
                newly_omitted.push(inspected(current));
            }
        }
    }
    findings.sort_by(|left, right| {
        (left.violation, &left.item_id).cmp(&(right.violation, &right.item_id))
    });
    let mut report = RecompositionReport {
        schema_version: 1,
        before_packet_sha256: before.packet_sha256.clone(),
        after_packet_sha256: after.packet_sha256.clone(),
        token_counter_changed: before.token_counter_id != after.token_counter_id,
        retained_constraints: u32::try_from(constraints.len())
            .map_err(|_| ContextInspectionError::InvalidPacket)?,
        carried_sources: u32::try_from(before.accounting.len())
            .map_err(|_| ContextInspectionError::InvalidPacket)?,
        newly_omitted,
        survives: findings.is_empty(),
        findings,
        report_sha256: "0".repeat(64),
    };
    let bytes = serde_json::to_vec(&report).map_err(|_| ContextInspectionError::Serialization)?;
    report.report_sha256 = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_management::{ContextCompositionBudget, compose_context};
    use agentmage_kernel_contracts::{ContextAdmission, ContextItemCandidate, ContextPacketId};

    fn candidate(
        id: &str,
        kind: ContextItemKind,
        essential: bool,
        text: &str,
    ) -> ContextItemCandidate {
        ContextItemCandidate {
            item_id: id.to_owned(),
            kind,
            sensitivity: ContextSensitivity::Internal,
            admission: ContextAdmission::Eligible,
            authoritative_evidence: false,
            essential,
            source_id: format!("source:{id}"),
            source_revision: "r1".to_owned(),
            content_sha256: Sha256::digest(text.as_bytes())
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
            bounded_excerpt: text.to_owned(),
            token_count: u32::try_from(text.len()).unwrap(),
        }
    }

    fn compose(
        candidates: &[ContextItemCandidate],
        max_tokens: u32,
        counter: &str,
    ) -> ComposedContextPacket {
        compose_context(
            ContextPacketId::from_raw("packet-inspection"),
            &ContextCompositionBudget {
                max_bytes: 1 << 20,
                max_tokens,
                max_items: 64,
                token_counter_id: counter.to_owned(),
            },
            candidates.to_vec(),
        )
        .unwrap()
    }

    fn session() -> Vec<ContextItemCandidate> {
        let mut stale = candidate(
            "stale",
            ContextItemKind::Supporting,
            false,
            "stale secret-ish text",
        );
        stale.admission = ContextAdmission::Stale;
        let mut denied = candidate(
            "denied",
            ContextItemKind::Memory,
            false,
            "denied private text",
        );
        denied.admission = ContextAdmission::Denied;
        vec![
            candidate(
                "system",
                ContextItemKind::Instruction,
                true,
                "instruction text",
            ),
            candidate(
                "request",
                ContextItemKind::NewestRequest,
                true,
                "request text",
            ),
            candidate(
                "correction",
                ContextItemKind::Correction,
                false,
                "correction",
            ),
            candidate(
                "support-a",
                ContextItemKind::Supporting,
                false,
                "supporting source a",
            ),
            candidate(
                "support-b",
                ContextItemKind::Supporting,
                false,
                "supporting source b, longer",
            ),
            stale,
            denied,
        ]
    }

    #[test]
    fn inspection_separates_pinned_selected_and_omitted_sources_without_content() {
        let packet = compose(&session(), 200, "counter-a");
        let view = inspect_context(&packet).unwrap();
        fn ids(items: &[InspectedContextItem]) -> Vec<&str> {
            items.iter().map(|item| item.item_id.as_str()).collect()
        }
        assert_eq!(ids(&view.pinned), ["system", "request", "correction"]);
        assert_eq!(ids(&view.selected), ["support-a", "support-b"]);
        assert_eq!(ids(&view.omitted), ["denied", "stale"]);
        assert_eq!(
            view.omitted[0].omission,
            Some(ContextOmissionReason::Denied)
        );
        assert_eq!(view.omitted[1].omission, Some(ContextOmissionReason::Stale));
        assert_eq!(
            view.kind_totals
                .iter()
                .map(|total| total.tokens)
                .sum::<u64>(),
            u64::from(view.used_tokens)
        );
        let text = render_context_inspection(&view);
        assert!(text.contains("pinned constraints: 3"));
        assert!(text.contains("omitted=stale"));
        for excerpt in [
            "instruction text",
            "secret-ish",
            "private text",
            "supporting source",
        ] {
            assert!(!text.contains(excerpt), "{excerpt}");
            assert!(!serde_json::to_string(&view).unwrap().contains(excerpt));
        }
    }

    #[test]
    fn rendering_escapes_identities_and_is_bounded() {
        let mut hostile = candidate("hostile", ContextItemKind::Supporting, false, "x");
        hostile.source_id = "source\u{202e}\u{2028}name".to_owned();
        let packet = compose(&[hostile], 10, "counter-a");
        let text = render_context_inspection(&inspect_context(&packet).unwrap());
        assert!(!text.contains('\u{202e}') && !text.contains('\u{2028}'));
        let many: Vec<_> = (0..1_000)
            .map(|index| {
                let mut item = candidate(
                    &format!("item-{index}"),
                    ContextItemKind::Supporting,
                    false,
                    "y",
                );
                item.source_id = format!("{index:0>200}");
                item.content_sha256 = format!("{index:0>64}");
                item
            })
            .collect();
        let packet = compose(&many, 100_000, "counter-a");
        let text = render_context_inspection(&inspect_context(&packet).unwrap());
        assert!(text.len() <= MAX_CONTEXT_INSPECTION_RENDER_BYTES + 96);
        assert!(
            text.ends_with(" sources not shown\n"),
            "{}",
            &text[text.len() - 60..]
        );
    }

    #[test]
    fn reflow_and_model_switch_keep_constraints_and_every_source_link() {
        let session = session();
        let before = compose(&session, 200, "counter-a");
        // A smaller budget omits a supporting source; a new counter models a switch.
        let after = compose(&session, 60, "counter-b");
        let report = verify_recomposition(&before, &after).unwrap();
        assert!(report.survives, "{report:?}");
        assert!(report.token_counter_changed);
        assert_eq!(report.retained_constraints, 3);
        assert_eq!(report.carried_sources, 7);
        assert_eq!(
            report
                .newly_omitted
                .iter()
                .map(|item| (item.item_id.as_str(), item.omission))
                .collect::<Vec<_>>(),
            [("support-b", Some(ContextOmissionReason::Budget))]
        );
        assert_eq!(
            report,
            verify_recomposition(&before, &after).unwrap(),
            "deterministic"
        );
    }

    #[test]
    fn dropped_constraints_lost_links_and_changed_sources_are_found() {
        let session = session();
        let before = compose(&session, 200, "counter-a");
        // The non-essential correction no longer fits.
        let after = compose(&session, 30, "counter-a");
        let report = verify_recomposition(&before, &after).unwrap();
        assert!(!report.survives);
        assert!(report.findings.contains(&RecompositionFinding {
            item_id: "correction".to_owned(),
            violation: RecompositionViolation::ConstraintDropped,
        }));
        // A source that is no longer accounted, and one whose material changed.
        let mut changed = session.clone();
        changed.retain(|item| item.item_id != "support-a");
        changed
            .iter_mut()
            .find(|item| item.item_id == "support-b")
            .unwrap()
            .source_revision = "r2".to_owned();
        let report = verify_recomposition(&before, &compose(&changed, 200, "counter-a")).unwrap();
        assert_eq!(
            report.findings,
            [
                RecompositionFinding {
                    item_id: "support-a".to_owned(),
                    violation: RecompositionViolation::SourceLinkLost,
                },
                RecompositionFinding {
                    item_id: "support-b".to_owned(),
                    violation: RecompositionViolation::SourceChanged,
                },
            ]
        );
        let mut tampered = before.clone();
        tampered.used_tokens += 1;
        assert_eq!(
            verify_recomposition(&tampered, &before).err(),
            Some(ContextInspectionError::InvalidPacket)
        );
        assert_eq!(
            inspect_context(&tampered).err(),
            Some(ContextInspectionError::InvalidPacket)
        );
    }
}
