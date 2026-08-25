#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const OUT = path.join(ROOT, "schemas/engineering-runtime");
const BASE = "https://agentmage.dev/schemas/engineering-runtime";

const identifier = { type: "string", pattern: "^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$" };
const digest = { type: "string", pattern: "^[0-9a-f]{64}$" };
const bounded = { type: "string", minLength: 1, maxLength: 512 };
const timestamp = { type: "string", format: "date-time" };
const uint = { type: "integer", minimum: 0 };
const positive = { type: "integer", minimum: 1 };
const nullable = (schema) => ({ oneOf: [schema, { type: "null" }] });
const list = (items, minItems = 0, maxItems = 256) => ({
  type: "array",
  items,
  minItems,
  maxItems,
});
const closed = (properties, required = Object.keys(properties)) => ({
  type: "object",
  additionalProperties: false,
  required,
  properties,
});
const schemaRef = closed({
  schema_id: identifier,
  schema_version: positive,
  schema_sha256: digest,
});
const artifactRef = closed({
  artifact_id: identifier,
  sha256: digest,
  byte_length: uint,
});
const range = closed({
  start_byte: uint,
  end_byte_exclusive: uint,
});
const lineRange = closed({
  start_line: uint,
  end_line_exclusive: uint,
});
const budgets = closed({
  turns: positive,
  tokens: positive,
  duration_ms: positive,
  tool_calls: positive,
  attempts: positive,
  no_progress_events: positive,
  output_bytes: positive,
  memory_bytes: positive,
  cost_minor_units: uint,
});
const commit = { type: "string", pattern: "^[0-9a-f]{40,64}$" };
const evidenceReference = closed({
  kind: bounded,
  uri: { type: "string", minLength: 1, maxLength: 4096 },
  sha256: digest,
});

const artifactEnvelope = closed({
  schema_version: positive,
  artifact_id: identifier,
  request_id: identifier,
  authority_id: identifier,
  origin: { enum: ["paste", "request_reference", "file", "uri", "directory", "archive", "tool_output"] },
  media_type: bounded,
  classification: { enum: ["public", "internal", "confidential", "restricted"] },
  capture_state: { enum: ["captured", "unavailable", "unsupported", "denied", "failed"] },
  byte_length: nullable(uint),
  sha256: nullable(digest),
  collected_at: timestamp,
});
artifactEnvelope.allOf = [{
  if: { properties: { capture_state: { const: "captured" } }, required: ["capture_state"] },
  then: { properties: { byte_length: uint, sha256: digest } },
  else: { properties: { byte_length: { type: "null" }, sha256: { type: "null" } } },
}];

const artifactTransformation = closed({
  schema_version: positive,
  transformation_id: identifier,
  artifact_id: identifier,
  transformer_id: identifier,
  transformer_version: bounded,
  input_sha256: digest,
  output_sha256: nullable(digest),
  source_ranges: list(range),
  warnings: list(bounded),
  reproducible: { type: "boolean" },
});

const artifactIngestionResult = closed({
  schema_version: positive,
  ingestion_id: identifier,
  artifact_id: identifier,
  disposition: { enum: ["captured", "parsed", "partial", "unsupported", "denied", "unavailable", "failed", "omitted"] },
  source: nullable(artifactRef),
  transformation_ids: list(identifier),
  warnings: list(bounded),
  error_code: nullable(identifier),
  terminal: { const: true },
});

export const CONTEXT_MANIFEST_SCHEMA_VERSION = 2;
const supportedContextManifestVersion = {
  type: "integer",
  const: CONTEXT_MANIFEST_SCHEMA_VERSION,
};
export const CONTEXT_ADMITTING_DISPOSITIONS = Object.freeze(["included", "summarized", "truncated"]);
export const CONTEXT_NON_ADMITTING_DISPOSITIONS = Object.freeze(["duplicate", "stale", "unsupported", "unavailable", "restricted", "omitted"]);
export const CONTEXT_COMPLETE_DISPOSITIONS = Object.freeze(["included"]);
export const CONTEXT_REASON_CODE_REQUIRED_DISPOSITIONS = Object.freeze([
  "summarized",
  "truncated",
  ...CONTEXT_NON_ADMITTING_DISPOSITIONS,
]);
const nonAdmittingDispositionConstraint = {
  if: {
    properties: { disposition: { enum: [...CONTEXT_NON_ADMITTING_DISPOSITIONS] } },
    required: ["disposition"],
  },
  then: {
    properties: {
      ranges: { type: "array", maxItems: 0 },
      token_count: { const: 0 },
    },
  },
};
const reasonCodeRequiredConstraint = {
  if: {
    properties: { disposition: { enum: [...CONTEXT_REASON_CODE_REQUIRED_DISPOSITIONS] } },
    required: ["disposition"],
  },
  then: {
    properties: { reason_code: identifier },
    required: ["reason_code"],
  },
  else: {
    properties: { reason_code: { type: "null" } },
  },
};
const contextItem = closed({
  artifact_id: identifier,
  disposition: { enum: [...CONTEXT_ADMITTING_DISPOSITIONS, ...CONTEXT_NON_ADMITTING_DISPOSITIONS] },
  ranges: list(range),
  token_count: uint,
  reason_code: nullable(identifier),
  reason: nullable(bounded),
});
contextItem.allOf = [nonAdmittingDispositionConstraint, reasonCodeRequiredConstraint];
const contextManifest = closed({
  schema_version: supportedContextManifestVersion,
  context_manifest_id: identifier,
  session_id: identifier,
  turn_id: identifier,
  model_profile_id: identifier,
  source_artifact_count: uint,
  items: list(contextItem, 0, 4096),
  total_input_tokens: uint,
  reserved_output_tokens: uint,
  safety_margin_tokens: uint,
  manifest_sha256: digest,
});

const deliveredRange = closed({
  artifact_id: identifier,
  sha256: digest,
  range,
  token_count: uint,
});
const contextDeliveryReceipt = closed({
  schema_version: positive,
  receipt_id: identifier,
  context_manifest_id: identifier,
  context_manifest_sha256: digest,
  model_request_id: identifier,
  route_decision_id: identifier,
  delivered: list(deliveredRange, 0, 4096),
  required_unseen_artifact_ids: list(identifier),
  outcome: { enum: ["delivered", "blocked"] },
  receipt_sha256: digest,
});
contextDeliveryReceipt.allOf = [{
  if: { properties: { outcome: { const: "delivered" } }, required: ["outcome"] },
  then: { properties: { required_unseen_artifact_ids: { type: "array", maxItems: 0 } } },
  else: { properties: { required_unseen_artifact_ids: { type: "array", minItems: 1 } } },
}];

const workflowStep = closed({
  step_id: identifier,
  depends_on: list(identifier),
  model_role: nullable(identifier),
  tool_id: nullable(identifier),
  effect_class: { enum: ["read_only", "idempotent_write", "conditional", "non_idempotent", "destructive", "external", "unknown"] },
  retry_class: { enum: ["never", "recoverable_read", "conditional_after_reconciliation", "user_decision_required"] },
  verifier_ids: list(identifier, 1, 32),
  budgets,
});
const workflowDefinition = closed({
  schema_version: positive,
  workflow_id: identifier,
  workflow_version: positive,
  input_schema: schemaRef,
  output_schema: schemaRef,
  steps: list(workflowStep, 1, 256),
  entry_step_ids: list(identifier, 1, 32),
  definition_sha256: digest,
});

const workflowState = closed({
  schema_version: positive,
  workflow_id: identifier,
  workflow_version: positive,
  sequence: uint,
  state: { enum: ["created", "validating", "ready", "running", "verifying", "waiting_for_dependency", "waiting_for_approval", "paused", "reconciling", "recovering", "succeeded", "no_op", "blocked", "failed", "cancelled", "timed_out", "resource_exhausted", "uncertain"] },
  active_step_id: nullable(identifier),
  completed_step_ids: list(identifier),
  attempt_ids: list(identifier),
  consumed_budget_sha256: digest,
  terminal_result_id: nullable(identifier),
});

const workflowCheckpoint = closed({
  schema_version: positive,
  checkpoint_id: identifier,
  workflow_id: identifier,
  state_sha256: digest,
  journal_sequence: uint,
  source_manifest_sha256: digest,
  policy_sha256: digest,
  environment_sha256: digest,
  route_sha256: digest,
  tool_catalog_sha256: digest,
  receipt_sha256s: list(digest),
  consumed_grant_sha256s: list(digest),
  created_at: timestamp,
});

const toolObservation = closed({
  schema_version: positive,
  observation_id: identifier,
  tool_call_id: identifier,
  attempt_id: identifier,
  task_id: identifier,
  step_id: identifier,
  tool_id: identifier,
  tool_version: bounded,
  tool_schema_sha256: digest,
  arguments_sha256: digest,
  authority_id: identifier,
  started_at: timestamp,
  completed_at: timestamp,
  outcome: { enum: ["succeeded", "denied", "failed", "cancelled", "timed_out", "resource_exhausted", "uncertain"] },
  exit_code: nullable({ type: "integer" }),
  signal: nullable(bounded),
  stdout: nullable(artifactRef),
  stderr: nullable(artifactRef),
  stdout_excerpt: { type: "string", maxLength: 8192 },
  stderr_excerpt: { type: "string", maxLength: 8192 },
  stdout_truncated: { type: "boolean" },
  stderr_truncated: { type: "boolean" },
  generated_artifact_ids: list(identifier),
  state_change: { enum: ["not_changed", "changed", "uncertain"] },
  descendants_cleaned: { type: "boolean" },
  resource_usage_sha256: digest,
  retry_disposition: { enum: ["not_eligible", "eligible_fresh_attempt", "reconcile_first", "user_decision_required"] },
  receipt_sha256: digest,
  terminal: { const: true },
});

const modelEndpointProfile = closed({
  schema_version: positive,
  endpoint_profile_id: identifier,
  profile_class: { enum: ["strict_local", "local_network_private", "remote_private", "remote_managed"] },
  operator_id: identifier,
  endpoint_reference: bounded,
  protocol_codec_id: identifier,
  tls_policy: { enum: ["not_applicable_local", "verified_tls", "mutual_tls"] },
  host_policy_sha256: digest,
  credential_reference: nullable(identifier),
  region: nullable(bounded),
  retention_policy: bounded,
  logging_policy: bounded,
  training_use_policy: bounded,
  quota_policy_sha256: digest,
  cost_policy_sha256: digest,
  qualification_sha256: nullable(digest),
  enabled: { const: false },
  automatic_fallback: { const: false },
});

const consideredRoute = closed({
  route_id: identifier,
  qualified: { type: "boolean" },
  admitted: { type: "boolean" },
  reason: bounded,
});
const modelRouteDecision = closed({
  schema_version: positive,
  route_decision_id: identifier,
  request_id: identifier,
  policy_sha256: digest,
  disclosure_class: { enum: ["none", "private_network", "private_remote", "managed_remote"] },
  considered_routes: list(consideredRoute, 1, 64),
  selected_route_id: nullable(identifier),
  fallback_used: { type: "boolean" },
  fallback_policy_sha256: nullable(digest),
  reason: bounded,
  decision_sha256: digest,
});
modelRouteDecision.allOf = [{
  if: { properties: { fallback_used: { const: false } }, required: ["fallback_used"] },
  then: { properties: { fallback_policy_sha256: { type: "null" } } },
  else: { properties: { fallback_policy_sha256: digest } },
}];

const capabilityManifest = closed({
  schema_version: positive,
  capability_id: identifier,
  capability_version: positive,
  publisher_id: identifier,
  title: bounded,
  purpose: bounded,
  lifecycle: { enum: ["draft", "admitted", "enabled", "degraded", "disabled", "quarantined", "retired"] },
  input_schema: schemaRef,
  output_schema: schemaRef,
  workflow_definition_sha256: digest,
  required_tool_ids: list(identifier),
  required_model_roles: list(identifier),
  requested_authority_ids: list(identifier),
  prohibited_authority_ids: list(identifier),
  budgets,
  verifier_ids: list(identifier, 1, 64),
  fixture_manifest_sha256: digest,
  migration_policy_sha256: digest,
  removal_policy_sha256: digest,
  manifest_sha256: digest,
});

const verificationResult = closed({
  schema_version: positive,
  verification_result_id: identifier,
  workflow_id: identifier,
  step_id: nullable(identifier),
  verifier_id: identifier,
  verifier_version: bounded,
  subject_sha256: digest,
  observed_evidence_sha256s: list(digest, 1, 256),
  preserved_invariants: list(identifier),
  prohibited_effects_observed: list(identifier),
  outcome: { enum: ["passed", "failed", "blocked", "uncertain"] },
  current: { type: "boolean" },
  result_sha256: digest,
});
verificationResult.allOf = [{
  if: { properties: { outcome: { const: "passed" } }, required: ["outcome"] },
  then: { properties: { current: { const: true }, prohibited_effects_observed: { type: "array", maxItems: 0 } } },
}];

const terminalResult = closed({
  schema_version: positive,
  terminal_result_id: identifier,
  workflow_id: identifier,
  outcome: { enum: ["verified_success", "verified_no_op", "blocked", "failed", "cancelled", "timed_out", "resource_exhausted", "uncertain"] },
  verification_result_ids: list(identifier),
  last_verified_state_sha256: digest,
  diagnostic_code: nullable(identifier),
  safe_next_action: nullable(bounded),
  established_by: { const: "agentmage-runtime-verifier" },
  result_sha256: digest,
});
terminalResult.allOf = [{
  if: { properties: { outcome: { enum: ["verified_success", "verified_no_op"] } }, required: ["outcome"] },
  then: {
    properties: {
      verification_result_ids: { type: "array", minItems: 1 },
      diagnostic_code: { type: "null" },
      safe_next_action: { type: "null" },
    },
  },
  else: { properties: { diagnostic_code: identifier, safe_next_action: bounded } },
}];

const agentLease = closed({
  schema_version: positive,
  campaign_id: identifier,
  lease_id: identifier,
  task_id: identifier,
  agent_id: identifier,
  session_id: identifier,
  model_profile_id: identifier,
  endpoint_profile_id: identifier,
  base_commit: commit,
  worktree_id: identifier,
  branch: { type: "string", minLength: 1, maxLength: 512 },
  path_leases: { ...list({ type: "string", minLength: 1, maxLength: 4096 }, 0, 256), uniqueItems: true },
  test_resource_leases: { ...list(identifier, 0, 256), uniqueItems: true },
  state: { enum: ["ready", "leased", "implementing", "gating", "reviewing", "correcting", "merge_ready", "integration_queued", "integrating", "merged", "blocked", "disputed", "failed", "cancelled"] },
  correction_limit: { type: "integer", minimum: 0, maximum: 255 },
  correction_count: { type: "integer", minimum: 0, maximum: 255 },
  candidate_commit: nullable(commit),
  lease_sha256: digest,
});

const reviewFinding = closed({
  schema_version: positive,
  review_id: identifier,
  campaign_id: identifier,
  lease_id: identifier,
  reviewer_id: identifier,
  base_commit: commit,
  candidate_commit: commit,
  outcome: { enum: ["PASS", "CHANGES_REQUIRED", "BLOCKED", "DISPUTED"] },
  severity: bounded,
  code: identifier,
  summary: { type: "string", minLength: 1, maxLength: 4096 },
  diff_sha256: digest,
  finding_sha256: digest,
});

const integrationRecord = closed({
  schema_version: positive,
  integration_id: identifier,
  campaign_id: identifier,
  lease_id: identifier,
  prior_campaign_head: commit,
  candidate_commit: commit,
  resulting_campaign_head: nullable(commit),
  state: { enum: ["queued", "revalidating", "integrating", "integrated", "blocked", "failed"] },
  gate_evidence_sha256: nullable(digest),
  reason_codes: { ...list(identifier, 0, 256), uniqueItems: true },
  integration_sha256: digest,
});

const multiAgentCampaign = closed({
  schema_version: positive,
  campaign_id: identifier,
  coordinator_session_id: identifier,
  objective: { type: "string", minLength: 1, maxLength: 16384 },
  approved_plan_id: identifier,
  approved_plan_sha256: digest,
  campaign_branch: { type: "string", minLength: 1, maxLength: 512 },
  starting_commit: commit,
  campaign_head: commit,
  max_workers: { type: "integer", minimum: 1, maximum: 5 },
  maximum_concurrent_workers: { type: "integer", minimum: 0, maximum: 5 },
  state: { enum: ["planned", "ready", "running", "paused", "blocked", "cancelled", "failed", "success"] },
  task_ids: { ...list(identifier, 1, 4096), uniqueItems: true },
  leases: list(agentLease, 0, 4096),
  integrations: list(integrationRecord, 0, 4096),
  reason_codes: { ...list(identifier, 0, 256), uniqueItems: true },
  final_evidence: list(evidenceReference, 0, 4096),
  campaign_sha256: digest,
});
multiAgentCampaign.allOf = [
  {
    if: { properties: { state: { const: "success" } }, required: ["state"] },
    then: { properties: { final_evidence: { type: "array", minItems: 1 } } },
  },
  {
    if: { properties: { state: { enum: ["blocked", "failed"] } }, required: ["state"] },
    then: { properties: { reason_codes: { type: "array", minItems: 1 } } },
  },
];

const completionEvidence = closed({
  schema_version: positive,
  task_id: identifier,
  attempt_id: identifier,
  source_sha256: digest,
  policy_sha256: digest,
  verifier_id: identifier,
  verifier_version: { type: "string", pattern: "^[0-9]+\\.[0-9]+\\.[0-9]+$" },
  outcome: { enum: ["verified_success", "verified_no_op", "not_verified"] },
  evidence: list(evidenceReference, 1, 4096),
  verified_at: timestamp,
  completion_evidence_sha256: digest,
});

export const SOURCE_ARTIFACT_SCHEMA_VERSION = 1;
export const MAX_SOURCE_ARTIFACT_BYTE_LENGTH = 104857600;
const supportedSourceVersion = { type: "integer", const: SOURCE_ARTIFACT_SCHEMA_VERSION };
const boundedSourceBytes = { type: "integer", minimum: 0, maximum: MAX_SOURCE_ARTIFACT_BYTE_LENGTH };
const originClass = { enum: ["paste", "request_reference", "file", "uri", "directory", "archive", "tool_output", "unsupported"] };
const referenceClass = { enum: ["paste", "request_reference", "file_path", "virtual_uri", "remote_uri", "directory", "archive", "unsupported"] };
const freshnessState = { enum: ["fresh", "stale", "renamed", "replaced", "missing", "unavailable", "unsupported"] };
const captureStateEnum = { enum: ["captured", "unavailable", "unsupported", "denied", "failed"] };
const classificationEnum = { enum: ["public", "internal", "confidential", "restricted"] };
const dispositionEnum = { enum: ["included", "summarized", "truncated", "duplicate", "stale", "unsupported", "unavailable", "restricted", "omitted"] };
const sectionKindEnum = { enum: ["document_root", "heading", "paragraph", "list_item", "table", "code_block", "image_region", "page", "sheet", "cell", "log_cluster", "unknown"] };

const origin = closed({
  schema_version: supportedSourceVersion,
  origin_id: identifier,
  request_id: identifier,
  authority_id: identifier,
  origin_class: originClass,
  captured_at: timestamp,
  origin_sha256: digest,
});

const sourceReference = closed({
  schema_version: supportedSourceVersion,
  reference_id: identifier,
  request_id: identifier,
  authority_id: identifier,
  reference_class: referenceClass,
  reference_display: bounded,
  support_state: { enum: ["supported", "unsupported", "ambient_prohibited"] },
  reference_sha256: digest,
  collected_at: timestamp,
});
sourceReference.allOf = [{
  if: { properties: { reference_class: { const: "unsupported" } }, required: ["reference_class"] },
  then: { properties: { support_state: { const: "unsupported" } } },
  else: { properties: { support_state: { enum: ["supported", "ambient_prohibited"] } } },
}];

const sourceProvenance = closed({
  schema_version: supportedSourceVersion,
  provenance_id: identifier,
  source_artifact_id: identifier,
  reference_id: identifier,
  origin_id: identifier,
  classification: classificationEnum,
  freshness_state: freshnessState,
  observed_at: timestamp,
  collected_at: timestamp,
  provenance_sha256: digest,
});

const sourceArtifact = closed({
  schema_version: supportedSourceVersion,
  source_artifact_id: identifier,
  request_id: identifier,
  authority_id: identifier,
  reference_id: identifier,
  origin_id: identifier,
  provenance_sha256: digest,
  declared_media_type: bounded,
  classification: classificationEnum,
  freshness_state: freshnessState,
  capture_state: captureStateEnum,
  byte_length: nullable(boundedSourceBytes),
  sha256: nullable(digest),
  collected_at: timestamp,
  source_artifact_sha256: digest,
});
sourceArtifact.allOf = [{
  if: { properties: { capture_state: { const: "captured" } }, required: ["capture_state"] },
  then: {
    properties: {
      byte_length: boundedSourceBytes,
      sha256: digest,
      freshness_state: { enum: ["fresh", "renamed"] },
    },
  },
  else: { properties: { byte_length: { type: "null" }, sha256: { type: "null" } } },
}];

const EXTRACTION_PRODUCING_STATES = ["captured", "parsed", "partially_parsed"];
const EXTRACTION_NON_PRODUCING_STATES = ["unsupported", "denied", "unavailable", "failed", "omitted"];
const extractionResult = closed({
  schema_version: supportedSourceVersion,
  extraction_id: identifier,
  source_artifact_id: identifier,
  extractor_id: identifier,
  extractor_version: bounded,
  source_sha256: digest,
  output_sha256: nullable(digest),
  media_type: bounded,
  disposition: { enum: [...EXTRACTION_PRODUCING_STATES, ...EXTRACTION_NON_PRODUCING_STATES] },
  section_ids: list(identifier),
  warnings: list(bounded),
  truncated: { type: "boolean" },
  reproducible: { type: "boolean" },
  terminal: { const: true },
});
extractionResult.allOf = [{
  if: {
    properties: { disposition: { enum: EXTRACTION_PRODUCING_STATES } },
    required: ["disposition"],
  },
  then: { properties: { output_sha256: digest } },
  else: {
    properties: {
      output_sha256: { type: "null" },
      section_ids: { type: "array", maxItems: 0 },
    },
  },
}];

const structuralSection = closed({
  schema_version: supportedSourceVersion,
  section_id: identifier,
  source_artifact_id: identifier,
  extraction_id: identifier,
  parent_section_id: nullable(identifier),
  ordinal: uint,
  kind: sectionKindEnum,
  byte_range: range,
  line_range: nullable(lineRange),
  token_count: uint,
  title: nullable(bounded),
  content_sha256: digest,
});

const contextDisposition = closed({
  schema_version: supportedSourceVersion,
  disposition_id: identifier,
  context_manifest_id: identifier,
  source_artifact_id: identifier,
  section_id: nullable(identifier),
  disposition: dispositionEnum,
  ranges: list(range),
  token_count: uint,
  reason_code: identifier,
  reason: nullable(bounded),
  terminal: { const: true },
});
contextDisposition.allOf = [nonAdmittingDispositionConstraint];

export const ENGINEERING_RUNTIME_SCHEMAS = Object.freeze({
  "artifact-envelope": artifactEnvelope,
  "artifact-transformation": artifactTransformation,
  "artifact-ingestion-result": artifactIngestionResult,
  "context-manifest": contextManifest,
  "context-delivery-receipt": contextDeliveryReceipt,
  "source-artifact": sourceArtifact,
  "origin": origin,
  "source-reference": sourceReference,
  "source-provenance": sourceProvenance,
  "extraction-result": extractionResult,
  "structural-section": structuralSection,
  "context-disposition": contextDisposition,
  "workflow-definition": workflowDefinition,
  "workflow-state": workflowState,
  "workflow-checkpoint": workflowCheckpoint,
  "tool-observation": toolObservation,
  "model-endpoint-profile": modelEndpointProfile,
  "model-route-decision": modelRouteDecision,
  "capability-manifest": capabilityManifest,
  "verification-result": verificationResult,
  "terminal-result": terminalResult,
  "agent-lease": agentLease,
  "review-finding": reviewFinding,
  "integration-record": integrationRecord,
  "multi-agent-campaign": multiAgentCampaign,
  "completion-evidence": completionEvidence,
});

function isOrderedRange(candidate) {
  return (
    candidate === null
    || candidate === undefined
    || (typeof candidate === "object"
      && typeof candidate.start_byte === "number"
      && typeof candidate.end_byte_exclusive === "number"
      && candidate.start_byte <= candidate.end_byte_exclusive)
  );
}

function isOrderedLineRange(candidate) {
  return (
    candidate === null
    || candidate === undefined
    || (typeof candidate === "object"
      && typeof candidate.start_line === "number"
      && typeof candidate.end_line_exclusive === "number"
      && candidate.start_line <= candidate.end_line_exclusive)
  );
}

function isOrderedRangeList(candidates) {
  return Array.isArray(candidates) && candidates.every(isOrderedRange);
}

function contextManifestSemantic(record) {
  if (!record || typeof record !== "object" || !Array.isArray(record.items)) return false;
  if (record.items.length !== record.source_artifact_count) return false;
  if (!Number.isSafeInteger(record.total_input_tokens) || record.total_input_tokens < 0) return false;
  const seen = new Set();
  let itemTokenSum = 0;
  for (const item of record.items) {
    if (!item || typeof item !== "object") return false;
    if (typeof item.artifact_id !== "string") return false;
    if (seen.has(item.artifact_id)) return false;
    seen.add(item.artifact_id);
    if (!isOrderedRangeList(item.ranges)) return false;
    if (!Number.isSafeInteger(item.token_count) || item.token_count < 0) return false;
    itemTokenSum += item.token_count;
    if (!Number.isSafeInteger(itemTokenSum)) return false;
    if (itemTokenSum > record.total_input_tokens) return false;
  }
  return true;
}

export const ENGINEERING_RUNTIME_SEMANTIC_VALIDATORS = Object.freeze({
  "structural-section": (record) => isOrderedRange(record.byte_range) && isOrderedLineRange(record.line_range),
  "context-disposition": (record) => isOrderedRangeList(record.ranges),
  "context-manifest": contextManifestSemantic,
});

export const ENGINEERING_RUNTIME_SEMANTIC_INVARIANTS = Object.freeze({
  "structural-section": "byte_range MUST satisfy start_byte <= end_byte_exclusive and line_range, when present, MUST satisfy start_line <= end_line_exclusive.",
  "context-disposition": "Every entry in ranges MUST satisfy start_byte <= end_byte_exclusive.",
  "context-manifest": "items.length MUST equal source_artifact_count, artifact_id values MUST be unique, every item ranges entry MUST satisfy start_byte <= end_byte_exclusive, and the sum of item token_count MUST NOT exceed total_input_tokens.",
});

export function validateEngineeringRuntimeRecord(compiledSchema, schemaName, candidate) {
  if (typeof schemaName !== "string" || !(schemaName in ENGINEERING_RUNTIME_SCHEMAS)) {
    throw new Error(`unknown Engineering Runtime schema: ${schemaName}`);
  }
  if (!compiledSchema(candidate)) return false;
  const semantic = ENGINEERING_RUNTIME_SEMANTIC_VALIDATORS[schemaName];
  return semantic === undefined ? true : Boolean(semantic(candidate));
}

function canonicalSchemaKey(value) {
  if (Array.isArray(value)) {
    return `[${value.map(canonicalSchemaKey).join(",")}]`;
  }
  if (value !== null && typeof value === "object") {
    const keys = Object.keys(value).sort();
    return `{${keys.map((key) => `${JSON.stringify(key)}:${canonicalSchemaKey(value[key])}`).join(",")}}`;
  }
  return JSON.stringify(value);
}

export function createEngineeringRuntimeValidator(schemaName, ajv) {
  if (!(schemaName in ENGINEERING_RUNTIME_SCHEMAS)) {
    throw new Error(`unknown Engineering Runtime schema: ${schemaName}`);
  }
  const document = schemaDocument(schemaName);
  const existing = ajv.getSchema(document.$id);
  let compiled;
  if (existing === undefined) {
    compiled = ajv.compile(document);
  } else if (canonicalSchemaKey(existing.schema) === canonicalSchemaKey(document)) {
    compiled = existing;
  } else {
    throw new Error(
      `refusing to reuse mismatched schema at ${document.$id} for ${schemaName}`,
    );
  }
  const semantic = ENGINEERING_RUNTIME_SEMANTIC_VALIDATORS[schemaName];
  const predicate = (candidate) => {
    if (!compiled(candidate)) return false;
    return semantic === undefined ? true : Boolean(semantic(candidate));
  };
  predicate.schemaName = schemaName;
  predicate.compiled = compiled;
  predicate.semantic = semantic;
  return predicate;
}

export const REUSED_SCHEMA_CONTRACTS = Object.freeze({
  "action-proposal": "schemas/model/closed-proposal.schema.json",
  "model-event": "schemas/model/stream-fragment.schema.json",
  "model-profile": "schemas/model/exact-profile.schema.json",
  "model-request": "schemas/model/run-request.schema.json",
  "runtime-event": "schemas/runtime/runtime-event.schema.json",
  "tool-call": "schemas/model/tool-call-candidate.schema.json",
});

export function schemaDocument(name, schema = ENGINEERING_RUNTIME_SCHEMAS[name]) {
  if (schema === undefined) throw new Error(`unknown Engineering Runtime schema: ${name}`);
  const invariant = ENGINEERING_RUNTIME_SEMANTIC_INVARIANTS[name];
  const description = invariant === undefined
    ? undefined
    : `Semantic invariants enforced by createEngineeringRuntimeValidator: ${invariant}`;
  const header = { $schema: "https://json-schema.org/draft/2020-12/schema", $id: `${BASE}/${name}.schema.json`, title: `AgentMage ${name}` };
  if (description !== undefined) header.description = description;
  return { ...header, ...schema };
}

function render(value) {
  return `${JSON.stringify(value, null, 2)}\n`;
}

export function synchronize({ write = false } = {}) {
  fs.mkdirSync(OUT, { recursive: true });
  const ajv = new Ajv2020({ allErrors: true, strict: true });
  addFormats(ajv);
  const failures = [];
  for (const [name, schema] of Object.entries(ENGINEERING_RUNTIME_SCHEMAS)) {
    const document = schemaDocument(name, schema);
    ajv.compile(document);
    const target = path.join(OUT, `${name}.schema.json`);
    const expected = render(document);
    if (write) {
      fs.writeFileSync(target, expected);
    } else if (!fs.existsSync(target) || fs.readFileSync(target, "utf8") !== expected) {
      failures.push(path.relative(ROOT, target));
    }
  }
  for (const relative of Object.values(REUSED_SCHEMA_CONTRACTS)) {
    if (!fs.existsSync(path.join(ROOT, relative))) failures.push(relative);
  }
  if (failures.length > 0) {
    throw new Error(`stale or missing Engineering Runtime schemas: ${failures.join(", ")}`);
  }
  return Object.keys(ENGINEERING_RUNTIME_SCHEMAS).length;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  try {
    const write = process.argv.includes("--write");
    const count = synchronize({ write });
    console.log(`${write ? "Wrote" : "Validated"} ${count} Engineering Runtime schemas and ${Object.keys(REUSED_SCHEMA_CONTRACTS).length} reused contracts.`);
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
