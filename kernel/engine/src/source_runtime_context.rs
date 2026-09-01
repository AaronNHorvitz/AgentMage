//! Prepared-source delivery through the interface-neutral runtime context boundary.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::sync::Arc;

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, ContextPacketId, ContractPayload, EvidenceReference,
    ModelContextPacket, ModelMessage, ModelMessageId, ModelMessageRole, RuntimeRunRequest,
    SchemaId, SchemaReference, ToolResult, to_canonical_json,
};
use sha2::{Digest, Sha256};

use crate::model_orchestration_profile::ModelContextWindowPlan;
use crate::runtime_loop::{RuntimeContextPort, RuntimePortFailure};
use crate::source_preparation::{
    ExactSourceTokenCounter, PreparedSourceContextManifest, SourceContextDisposition,
    SourcePreparationService, verify_prepared_source_context_manifest,
};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const TASK_SCHEMA: &str = "runtime.prepared-source.task";
const SOURCE_SCHEMA: &str = "runtime.prepared-source.section";
const TOOL_SCHEMA: &str = "runtime.prepared-source.tool-result";
const EVIDENCE_SCHEMA: &str = "runtime.prepared-source.evidence";
const JSON_OBJECT_SCHEMA: &[u8] = br#"{"type":"object"}"#;
const TEXT_SCHEMA: &[u8] = br#"{"type":"string"}"#;

/// Runtime context built from one immutable prepared-source projection and exact tokenizer.
///
/// The adapter owns neither acquisition nor tool authority. It recompiles source accounting on
/// every turn, refuses stale or omitted required sources, and retains the exact accounting
/// manifests and bounded packets that explain every model-visible source byte.
pub struct PreparedSourceRuntimeContext<C>
where
    C: ExactSourceTokenCounter,
{
    sources: Arc<SourcePreparationService>,
    plan: ModelContextWindowPlan,
    counter: C,
    required_source_ids: BTreeSet<String>,
    disclose_private: bool,
    manifests: Vec<PreparedSourceContextManifest>,
}

impl<C> PreparedSourceRuntimeContext<C>
where
    C: ExactSourceTokenCounter,
{
    /// Binds one already-admitted immutable source projection to an exact window plan.
    pub fn new(
        sources: Arc<SourcePreparationService>,
        plan: ModelContextWindowPlan,
        counter: C,
        required_source_ids: impl IntoIterator<Item = String>,
        disclose_private: bool,
    ) -> Result<Self, RuntimePortFailure> {
        let required_source_ids = required_source_ids.into_iter().collect::<BTreeSet<_>>();
        if required_source_ids.is_empty()
            || required_source_ids
                .iter()
                .any(|source_id| source_id.is_empty() || source_id.len() > 256)
        {
            return Err(RuntimePortFailure::Invalid);
        }
        let current = sources
            .manifests()
            .into_iter()
            .map(|manifest| manifest.source_id)
            .collect::<BTreeSet<_>>();
        if !required_source_ids.is_subset(&current) {
            return Err(RuntimePortFailure::Unavailable);
        }
        Ok(Self {
            sources,
            plan,
            counter,
            required_source_ids,
            disclose_private,
            manifests: Vec::new(),
        })
    }

    /// Returns the immutable source authority shared with read-only artifact adapters.
    #[must_use]
    pub fn sources(&self) -> Arc<SourcePreparationService> {
        Arc::clone(&self.sources)
    }

    /// Returns every exact source-accounting manifest in turn order.
    #[must_use]
    pub fn context_manifests(&self) -> &[PreparedSourceContextManifest] {
        &self.manifests
    }

    fn compile(
        &mut self,
        request: &RuntimeRunRequest,
        context_packet_id: ContextPacketId,
        turn: u32,
        tool_results: &[ToolResult],
        evidence: &[EvidenceReference],
    ) -> Result<ModelContextPacket, RuntimePortFailure> {
        if turn == 0 || self.manifests.len() >= request.limits.max_context_refreshes as usize {
            return Err(RuntimePortFailure::ResourceExhausted);
        }
        let counter_binding = self.counter.binding();
        if counter_binding.token_counter_id != request.context_budget.token_counter
            || counter_binding.token_counter_sha256 != request.context_budget.token_counter_sha256
            || self.plan.model_profile_id != request.model_profile.profile_id.as_str()
            || self.plan.model_manifest_sha256 != request.model_profile.manifest_sha256
            || self.plan.tokenizer_sha256 != request.model_profile.codec.tokenizer_sha256
            || self.plan.token_counter_sha256 != counter_binding.token_counter_sha256
            || self.plan.source_artifacts.allocated_tokens
                > request.context_budget.max_context_tokens
        {
            return Err(RuntimePortFailure::Invalid);
        }
        let manifest = self
            .sources
            .compile_context(
                format!("source-context-{}-{turn}", request.run_id.as_str()),
                context_packet_id.clone(),
                &self.plan,
                request.context_budget.max_input_bytes,
                &mut self.counter,
                self.disclose_private,
            )
            .map_err(|_| RuntimePortFailure::Invalid)?;
        verify_prepared_source_context_manifest(&manifest)
            .map_err(|_| RuntimePortFailure::Invalid)?;
        for required in &self.required_source_ids {
            let delivered = manifest.records.iter().any(|record| {
                record.source_id == *required
                    && record.section_id.is_some()
                    && record.disposition == SourceContextDisposition::Included
            });
            if !delivered {
                return Err(RuntimePortFailure::Unavailable);
            }
        }

        let mut messages = Vec::new();
        messages.push(message(
            format!("runtime-task-{turn}"),
            ModelMessageRole::User,
            TASK_SCHEMA,
            TEXT_SCHEMA,
            "text/plain",
            request.task.objective.as_bytes().to_vec(),
        ));
        for item in &manifest.packet.items {
            messages.push(message(
                format!("runtime-source-{}-{turn}", item.item_id),
                ModelMessageRole::User,
                SOURCE_SCHEMA,
                TEXT_SCHEMA,
                "text/plain",
                item.bounded_excerpt.as_bytes().to_vec(),
            ));
        }
        for (index, result) in tool_results.iter().enumerate() {
            messages.push(message(
                format!("runtime-tool-{turn}-{index}"),
                ModelMessageRole::Tool,
                TOOL_SCHEMA,
                JSON_OBJECT_SCHEMA,
                "application/json",
                to_canonical_json(result).map_err(|_| RuntimePortFailure::Invalid)?,
            ));
        }
        let result_evidence = tool_results
            .iter()
            .flat_map(|result| result.evidence.iter())
            .map(|reference| reference.evidence_id.as_str())
            .collect::<BTreeSet<_>>();
        for (index, reference) in evidence
            .iter()
            .filter(|reference| !result_evidence.contains(reference.evidence_id.as_str()))
            .enumerate()
        {
            messages.push(message(
                format!("runtime-evidence-{turn}-{index}"),
                ModelMessageRole::Tool,
                EVIDENCE_SCHEMA,
                JSON_OBJECT_SCHEMA,
                "application/json",
                to_canonical_json(reference).map_err(|_| RuntimePortFailure::Invalid)?,
            ));
        }
        if messages.is_empty() || messages.len() > request.context_budget.max_messages as usize {
            return Err(RuntimePortFailure::ResourceExhausted);
        }

        let mut input_bytes = 0_u64;
        let mut input_tokens = 0_u32;
        for item in &messages {
            input_bytes = input_bytes
                .checked_add(
                    u64::try_from(item.content.bytes.len())
                        .map_err(|_| RuntimePortFailure::ResourceExhausted)?,
                )
                .ok_or(RuntimePortFailure::ResourceExhausted)?;
            let content = std::str::from_utf8(&item.content.bytes)
                .map_err(|_| RuntimePortFailure::Invalid)?;
            input_tokens = input_tokens
                .checked_add(
                    self.counter
                        .count_tokens(content)
                        .map_err(|_| RuntimePortFailure::Invalid)?,
                )
                .ok_or(RuntimePortFailure::ResourceExhausted)?;
        }
        if input_bytes == 0
            || input_bytes > request.context_budget.max_input_bytes
            || input_tokens == 0
            || input_tokens > request.context_budget.max_context_tokens
        {
            return Err(RuntimePortFailure::ResourceExhausted);
        }
        let mut packet = ModelContextPacket {
            schema_version: CONTRACT_SCHEMA_VERSION,
            context_packet_id,
            session_id: request.session_id.clone(),
            task_id: request.task.task_id.clone(),
            profile_id: request.model_profile.profile_id.clone(),
            manifest_sha256: request.model_profile.manifest_sha256.clone(),
            tool_catalog_id: request.tool_catalog_id.clone(),
            messages,
            input_bytes,
            input_tokens,
            packet_sha256: ZERO_SHA256.to_owned(),
        };
        packet.packet_sha256 =
            sha256(&to_canonical_json(&packet).map_err(|_| RuntimePortFailure::Invalid)?);
        self.manifests.push(manifest);
        Ok(packet)
    }
}

impl<C> RuntimeContextPort for PreparedSourceRuntimeContext<C>
where
    C: ExactSourceTokenCounter,
{
    fn build_context(
        &mut self,
        request: &RuntimeRunRequest,
        context_packet_id: ContextPacketId,
        turn: u32,
        tool_results: &[ToolResult],
        evidence: &[EvidenceReference],
    ) -> Result<ModelContextPacket, RuntimePortFailure> {
        self.compile(request, context_packet_id, turn, tool_results, evidence)
    }
}

fn message(
    message_id: String,
    role: ModelMessageRole,
    schema_id: &str,
    schema: &[u8],
    media_type: &str,
    bytes: Vec<u8>,
) -> ModelMessage {
    ModelMessage {
        message_id: ModelMessageId::from_raw(message_id),
        role,
        content: ContractPayload {
            schema: SchemaReference {
                schema_id: SchemaId::from_raw(schema_id),
                schema_version: 1,
                schema_sha256: sha256(schema),
            },
            media_type: media_type.to_owned(),
            sha256: sha256(&bytes),
            bytes,
        },
    }
}

fn sha256(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}
