//! Deterministic schema registry for every canonical knowledge record kind.

use crate::KnowledgeRecordKind;

/// Closed field registry for one canonical record kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeRecordSchema {
    /// Record kind governed by this schema.
    pub kind: KnowledgeRecordKind,
    /// Required kind-specific field names.
    pub required_fields: &'static [&'static str],
    /// Optional kind-specific field names.
    pub optional_fields: &'static [&'static str],
}

impl KnowledgeRecordSchema {
    /// Returns whether this schema admits the exact field name.
    #[must_use]
    pub fn permits(&self, name: &str) -> bool {
        self.required_fields.contains(&name) || self.optional_fields.contains(&name)
    }
}

const SCHEMAS: &[KnowledgeRecordSchema] = &[
    schema(
        KnowledgeRecordKind::Person,
        &["name"],
        &["role", "timezone", "notes"],
    ),
    schema(
        KnowledgeRecordKind::Organization,
        &["name"],
        &["purpose", "website", "notes"],
    ),
    schema(
        KnowledgeRecordKind::Project,
        &["purpose", "status"],
        &["owner", "next_action", "notes"],
    ),
    schema(
        KnowledgeRecordKind::Meeting,
        &["occurred_at"],
        &["status", "attendees", "raw_notes"],
    ),
    schema(
        KnowledgeRecordKind::Task,
        &["status", "owner"],
        &["due_at", "next_action", "notes"],
    ),
    schema(
        KnowledgeRecordKind::Decision,
        &["status", "decided_at"],
        &["decision", "rationale", "notes"],
    ),
    schema(
        KnowledgeRecordKind::Commitment,
        &["promisor", "promisee", "status"],
        &["due_at", "commitment", "notes"],
    ),
    schema(
        KnowledgeRecordKind::Document,
        &["document_type", "status"],
        &["version", "owner", "notes"],
    ),
    schema(
        KnowledgeRecordKind::Correspondence,
        &["occurred_at", "direction"],
        &["sender", "recipients", "subject"],
    ),
    schema(
        KnowledgeRecordKind::Deadline,
        &["due_at", "status"],
        &["owner", "source", "notes"],
    ),
    schema(
        KnowledgeRecordKind::Approval,
        &["status", "approver"],
        &["requested_at", "responded_at", "conditions"],
    ),
    schema(
        KnowledgeRecordKind::Risk,
        &["status", "impact"],
        &["likelihood", "owner", "mitigation"],
    ),
    schema(
        KnowledgeRecordKind::Question,
        &["status"],
        &["question", "answer", "owner"],
    ),
    schema(
        KnowledgeRecordKind::Handoff,
        &["status", "recipient"],
        &["objective", "next_action", "notes"],
    ),
];

const fn schema(
    kind: KnowledgeRecordKind,
    required_fields: &'static [&'static str],
    optional_fields: &'static [&'static str],
) -> KnowledgeRecordSchema {
    KnowledgeRecordSchema {
        kind,
        required_fields,
        optional_fields,
    }
}

/// Returns the complete immutable schema registry in stable kind order.
#[must_use]
pub const fn knowledge_schemas() -> &'static [KnowledgeRecordSchema] {
    SCHEMAS
}

/// Returns the one schema for a closed record kind.
#[must_use]
pub fn knowledge_schema(kind: KnowledgeRecordKind) -> &'static KnowledgeRecordSchema {
    SCHEMAS
        .iter()
        .find(|schema| schema.kind == kind)
        .expect("every closed knowledge kind has one static schema")
}

/// Verifies field closure, uniqueness, and complete record-kind coverage.
#[must_use]
pub fn verify_schema_registry() -> bool {
    use std::collections::BTreeSet;

    let expected = [
        KnowledgeRecordKind::Person,
        KnowledgeRecordKind::Organization,
        KnowledgeRecordKind::Project,
        KnowledgeRecordKind::Meeting,
        KnowledgeRecordKind::Task,
        KnowledgeRecordKind::Decision,
        KnowledgeRecordKind::Commitment,
        KnowledgeRecordKind::Document,
        KnowledgeRecordKind::Correspondence,
        KnowledgeRecordKind::Deadline,
        KnowledgeRecordKind::Approval,
        KnowledgeRecordKind::Risk,
        KnowledgeRecordKind::Question,
        KnowledgeRecordKind::Handoff,
    ];
    if SCHEMAS.len() != expected.len() || SCHEMAS.iter().map(|schema| schema.kind).ne(expected) {
        return false;
    }
    SCHEMAS.iter().all(|schema| {
        !schema.required_fields.is_empty()
            && schema
                .required_fields
                .iter()
                .chain(schema.optional_fields)
                .all(|field| {
                    !field.is_empty()
                        && field
                            .bytes()
                            .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
                })
            && schema
                .required_fields
                .iter()
                .chain(schema.optional_fields)
                .collect::<BTreeSet<_>>()
                .len()
                == schema.required_fields.len() + schema.optional_fields.len()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_complete_closed_and_unique() {
        assert!(verify_schema_registry());
        assert_eq!(knowledge_schemas().len(), 14);
    }

    #[test]
    fn required_and_optional_fields_are_distinct() {
        for schema in knowledge_schemas() {
            for field in schema.required_fields {
                assert!(schema.permits(field));
                assert!(!schema.optional_fields.contains(field));
            }
        }
    }
}
