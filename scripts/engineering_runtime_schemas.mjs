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
export const ENGINEERING_RUNTIME_RECORD_SCHEMA_VERSION = 2;
const supportedRuntimeRecordVersion = {
  type: "integer",
  const: ENGINEERING_RUNTIME_RECORD_SCHEMA_VERSION,
};
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
  schema_version: supportedRuntimeRecordVersion,
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
  schema_version: supportedRuntimeRecordVersion,
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
  schema_version: supportedRuntimeRecordVersion,
  ingestion_id: identifier,
  artifact_id: identifier,
  disposition: { enum: ["captured", "parsed", "partial", "unsupported", "denied", "unavailable", "failed", "omitted"] },
  source: nullable(artifactRef),
  transformation_ids: list(identifier),
  warnings: list(bounded),
  error_code: nullable(identifier),
  terminal: { const: true },
});

export const CONTEXT_MANIFEST_SCHEMA_VERSION = ENGINEERING_RUNTIME_RECORD_SCHEMA_VERSION;
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

// Closed effect and retry families shared by the workflow step definition and the
// companion step-execution policy. Keeping one source of truth means widening either
// family widens both surfaces at once and the two can never drift apart.
export const STEP_EFFECT_CLASSES = Object.freeze([
  "read_only",
  "idempotent_write",
  "conditional",
  "non_idempotent",
  "destructive",
  "external",
  "unknown",
]);
export const STEP_RETRY_CLASSES = Object.freeze([
  "never",
  "recoverable_read",
  "conditional_after_reconciliation",
  "user_decision_required",
]);
const effectClass = { enum: [...STEP_EFFECT_CLASSES] };
const retryClass = { enum: [...STEP_RETRY_CLASSES] };

const workflowStep = closed({
  step_id: identifier,
  depends_on: list(identifier),
  model_role: nullable(identifier),
  tool_id: nullable(identifier),
  effect_class: effectClass,
  retry_class: retryClass,
  verifier_ids: list(identifier, 1, 32),
  budgets,
});
const workflowDefinition = closed({
  schema_version: supportedRuntimeRecordVersion,
  workflow_id: identifier,
  workflow_version: positive,
  input_schema: schemaRef,
  output_schema: schemaRef,
  steps: list(workflowStep, 1, 256),
  entry_step_ids: list(identifier, 1, 32),
  definition_sha256: digest,
});

const workflowState = closed({
  schema_version: supportedRuntimeRecordVersion,
  workflow_id: identifier,
  workflow_version: positive,
  sequence: uint,
  state: { enum: ["created", "validating", "ready", "running", "verifying", "waiting_for_dependency", "waiting_for_approval", "paused", "reconciling", "recovering", "succeeded", "no_op", "blocked", "denied", "failed", "cancelled", "timed_out", "resource_exhausted", "uncertain"] },
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
  schema_version: supportedRuntimeRecordVersion,
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
  schema_version: supportedRuntimeRecordVersion,
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
  schema_version: supportedRuntimeRecordVersion,
  terminal_result_id: identifier,
  workflow_id: identifier,
  outcome: { enum: ["verified_success", "verified_no_op", "blocked", "denied", "failed", "cancelled", "timed_out", "resource_exhausted", "uncertain"] },
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
  reason_code: nullable(identifier),
  reason: nullable(bounded),
  terminal: { const: true },
});
contextDisposition.allOf = [nonAdmittingDispositionConstraint, reasonCodeRequiredConstraint];

export const SOURCE_RETENTION_SCHEMA_VERSION = 1;
// Exact existing RuntimeArtifactKind variants. Source retention reuses this closed set;
// adding a source-specific physical family here would widen the kernel contract.
export const RUNTIME_ARTIFACT_KINDS = Object.freeze([
  "patch",
  "standard_output",
  "standard_error",
  "test_log",
  "generated_file",
  "report",
  "model_output",
]);
export const SOURCE_RETENTION_PHYSICAL_STORE = "runtime_artifact_backend";
export const SOURCE_RETENTION_PERSISTING_CLASSES = Object.freeze(["policy_persisted"]);
export const SOURCE_RETENTION_NON_PERSISTING_CLASSES = Object.freeze([
  "memory_only",
  "released",
  "deleted",
]);

const physicalArtifactBinding = closed({
  artifact_kind: { enum: [...RUNTIME_ARTIFACT_KINDS] },
  artifact_id: identifier,
  payload_sha256: digest,
  byte_length: boundedSourceBytes,
});

const sourceRetention = closed({
  schema_version: { type: "integer", const: SOURCE_RETENTION_SCHEMA_VERSION },
  retention_id: identifier,
  source_artifact_id: identifier,
  request_id: identifier,
  authority_id: identifier,
  owner_class: { enum: ["session", "task", "request"] },
  owner_id: identifier,
  retention_class: {
    enum: [...SOURCE_RETENTION_PERSISTING_CLASSES, ...SOURCE_RETENTION_NON_PERSISTING_CLASSES],
  },
  retention_policy_id: nullable(identifier),
  retention_expires_at: nullable(timestamp),
  physical_store: { const: SOURCE_RETENTION_PHYSICAL_STORE },
  physical_binding: nullable(physicalArtifactBinding),
  encryption_state: { enum: ["not_persisted", "encrypted_at_rest"] },
  protected_metadata_sha256: digest,
  lifecycle_state: { enum: ["active", "quarantined", "released", "deleted"] },
  reason_code: nullable(identifier),
  recorded_at: timestamp,
  source_retention_sha256: digest,
});
sourceRetention.allOf = [
  {
    if: {
      properties: { retention_class: { enum: [...SOURCE_RETENTION_PERSISTING_CLASSES] } },
      required: ["retention_class"],
    },
    then: {
      properties: {
        physical_binding: physicalArtifactBinding,
        retention_policy_id: identifier,
        retention_expires_at: timestamp,
        encryption_state: { const: "encrypted_at_rest" },
      },
    },
    else: {
      properties: {
        physical_binding: { type: "null" },
        retention_policy_id: { type: "null" },
        retention_expires_at: { type: "null" },
        encryption_state: { const: "not_persisted" },
      },
    },
  },
  {
    if: {
      properties: { lifecycle_state: { const: "active" } },
      required: ["lifecycle_state"],
    },
    then: { properties: { reason_code: { type: "null" } } },
    else: { properties: { reason_code: identifier } },
  },
];

export const SOURCE_LOCATOR_SCHEMA_VERSION = 1;
// Each locator kind is authoritative for exactly one coordinate space. The payload field
// listed here MUST be present for a resolvable state and every other payload MUST be null.
export const SOURCE_LOCATOR_PAYLOAD_FIELDS = Object.freeze({
  byte: ["byte_range"],
  line: ["line_range"],
  page: ["page_number"],
  sheet: ["sheet_name"],
  cell: ["sheet_name", "cell_reference"],
  image_region: ["image_region"],
  section: ["section_id"],
});
export const SOURCE_LOCATOR_KINDS = Object.freeze(Object.keys(SOURCE_LOCATOR_PAYLOAD_FIELDS));
// States that name an exact position. Every other state cannot honestly carry one.
export const SOURCE_LOCATOR_RESOLVED_STATES = Object.freeze(["complete", "partial", "truncated"]);
export const SOURCE_LOCATOR_UNRESOLVED_STATES = Object.freeze([
  "encrypted",
  "unsupported",
  "unavailable",
]);

const cellReference = closed({
  sheet_row: positive,
  sheet_column: positive,
});

const imageRegion = closed({
  origin_x: uint,
  origin_y: uint,
  width: positive,
  height: positive,
});

const SOURCE_LOCATOR_ALL_PAYLOADS = Object.freeze([
  "byte_range",
  "line_range",
  "page_number",
  "sheet_name",
  "cell_reference",
  "image_region",
  "section_id",
]);

const sourceLocator = closed({
  schema_version: { type: "integer", const: SOURCE_LOCATOR_SCHEMA_VERSION },
  locator_id: identifier,
  source_artifact_id: identifier,
  provenance_id: identifier,
  extraction_id: nullable(identifier),
  locator_kind: { enum: [...SOURCE_LOCATOR_KINDS] },
  availability_state: {
    enum: [...SOURCE_LOCATOR_RESOLVED_STATES, ...SOURCE_LOCATOR_UNRESOLVED_STATES],
  },
  byte_range: nullable(range),
  line_range: nullable(lineRange),
  page_number: nullable(positive),
  sheet_name: nullable(bounded),
  cell_reference: nullable(cellReference),
  image_region: nullable(imageRegion),
  section_id: nullable(identifier),
  reason_code: nullable(identifier),
  observed_at: timestamp,
  locator_sha256: digest,
});

const nullPayloads = (fields) => Object.fromEntries(fields.map((field) => [field, { type: "null" }]));

sourceLocator.allOf = [
  // An unresolved state cannot name a position in content it never read.
  {
    if: {
      properties: { availability_state: { enum: [...SOURCE_LOCATOR_UNRESOLVED_STATES] } },
      required: ["availability_state"],
    },
    then: { properties: nullPayloads(SOURCE_LOCATOR_ALL_PAYLOADS) },
  },
  // Only a complete locator may omit a deterministic reason_code.
  {
    if: {
      properties: { availability_state: { const: "complete" } },
      required: ["availability_state"],
    },
    then: { properties: { reason_code: { type: "null" } } },
    else: { properties: { reason_code: identifier } },
  },
  // Each resolved kind carries exactly its own payload and nothing else.
  ...SOURCE_LOCATOR_KINDS.map((kind) => ({
    if: {
      properties: {
        locator_kind: { const: kind },
        availability_state: { enum: [...SOURCE_LOCATOR_RESOLVED_STATES] },
      },
      required: ["locator_kind", "availability_state"],
    },
    then: {
      required: [...SOURCE_LOCATOR_PAYLOAD_FIELDS[kind]],
      properties: nullPayloads(
        SOURCE_LOCATOR_ALL_PAYLOADS.filter(
          (field) => !SOURCE_LOCATOR_PAYLOAD_FIELDS[kind].includes(field),
        ),
      ),
    },
  })),
  // A resolved locator must actually populate its declared payload.
  ...SOURCE_LOCATOR_KINDS.flatMap((kind) => SOURCE_LOCATOR_PAYLOAD_FIELDS[kind].map((field) => ({
    if: {
      properties: {
        locator_kind: { const: kind },
        availability_state: { enum: [...SOURCE_LOCATOR_RESOLVED_STATES] },
      },
      required: ["locator_kind", "availability_state"],
    },
    then: { properties: { [field]: { not: { type: "null" } } } },
  }))),
];

export const STEP_EXECUTION_POLICY_SCHEMA_VERSION = 1;
// Retry classes that let the runtime open a fresh attempt on its own authority.
// Every other class needs a human decision, or no further attempt at all.
export const STEP_AUTOMATIC_RETRY_CLASSES = Object.freeze([
  "recoverable_read",
  "conditional_after_reconciliation",
]);
// Decision 0042 section 5 and the runtime retry table as one closed matrix. An effect
// class admits exactly these retry classes and no others, so a destructive, external,
// non-idempotent, or unclassified step can never be scheduled for an automatic retry.
export const STEP_EFFECT_RETRY_MATRIX = Object.freeze({
  read_only: Object.freeze(["never", "recoverable_read"]),
  idempotent_write: Object.freeze(["never", "conditional_after_reconciliation"]),
  conditional: Object.freeze(["never", "conditional_after_reconciliation"]),
  non_idempotent: Object.freeze(["never", "user_decision_required"]),
  destructive: Object.freeze(["never", "user_decision_required"]),
  external: Object.freeze(["never", "user_decision_required"]),
  unknown: Object.freeze(["never", "user_decision_required"]),
});
// Effect classes that must never reach an executor without a recorded approval. An
// unclassified effect is included deliberately: it fails toward approval, not past it.
export const STEP_APPROVAL_REQUIRING_EFFECTS = Object.freeze([
  "non_idempotent",
  "destructive",
  "external",
  "unknown",
]);
export const STEP_APPROVAL_REQUIREMENTS = Object.freeze([
  "not_required",
  "required_once",
  "required_per_attempt",
]);
export const STEP_IDEMPOTENCY_REQUIREMENTS = Object.freeze([
  "not_applicable",
  "required",
  "verified_desired_state",
]);
export const STEP_VERIFICATION_REQUIREMENTS = Object.freeze([
  "verifier_evidence_required",
  "policy_deferred",
]);
// Terminal diagnostics carry deterministic codes and artifact references only. Neither
// disclosure admits model prose, prompts, credentials, or environment values.
export const STEP_DIAGNOSTIC_DISCLOSURES = Object.freeze([
  "content_free_codes",
  "content_free_codes_with_artifact_reference",
]);
// The eight policy identities this companion record is required to name.
export const STEP_EXECUTION_POLICY_IDENTITIES = Object.freeze([
  "preflight_policy_id",
  "side_effect_policy_id",
  "approval_policy_id",
  "idempotency_policy_id",
  "verifier_policy_id",
  "retry_policy_id",
  "budget_policy_id",
  "diagnostic_policy_id",
]);

// A companion record keyed to one existing PlanStepId. It carries execution policy and
// nothing else: the step's own description, ordinal, dependencies, and state stay in
// Plan/PlanStep, arguments stay in ToolCall, authority stays in CapabilityGrant,
// observed outcome stays in OperationReceipt, and completion stays in
// VerifiedCompletion. This record replaces none of them.
const stepExecutionPolicy = closed({
  schema_version: { type: "integer", const: STEP_EXECUTION_POLICY_SCHEMA_VERSION },
  policy_id: identifier,
  plan_id: identifier,
  plan_step_id: identifier,
  // Policy is bound to the exact plan revision that proposed the step, so a replanned
  // step cannot silently inherit a policy written for different work.
  plan_revision: uint,
  preflight_policy_id: identifier,
  required_preflight_ids: list(identifier, 1, 32),
  side_effect_policy_id: identifier,
  effect_class: effectClass,
  approval_policy_id: identifier,
  approval_requirement: { enum: [...STEP_APPROVAL_REQUIREMENTS] },
  idempotency_policy_id: identifier,
  idempotency_key_requirement: { enum: [...STEP_IDEMPOTENCY_REQUIREMENTS] },
  verifier_policy_id: identifier,
  verification_requirement: { enum: [...STEP_VERIFICATION_REQUIREMENTS] },
  required_verifier_ids: list(identifier, 0, 32),
  deferral_reason_code: nullable(identifier),
  retry_policy_id: identifier,
  retry_class: retryClass,
  budget_policy_id: identifier,
  budgets,
  diagnostic_policy_id: identifier,
  diagnostic_disclosure: { enum: [...STEP_DIAGNOSTIC_DISCLOSURES] },
  recorded_at: timestamp,
  policy_sha256: digest,
});

stepExecutionPolicy.allOf = [
  // Each effect class admits exactly its own retry classes.
  ...STEP_EFFECT_CLASSES.map((effect) => ({
    if: { properties: { effect_class: { const: effect } }, required: ["effect_class"] },
    then: { properties: { retry_class: { enum: [...STEP_EFFECT_RETRY_MATRIX[effect]] } } },
  })),
  // A step that is never retried gets exactly one attempt.
  {
    if: { properties: { retry_class: { const: "never" } }, required: ["retry_class"] },
    then: { properties: { budgets: { type: "object", properties: { attempts: { const: 1 } } } } },
  },
  // A destructive step needs a fresh narrow approval for every attempt.
  {
    if: { properties: { effect_class: { const: "destructive" } }, required: ["effect_class"] },
    then: { properties: { approval_requirement: { const: "required_per_attempt" } } },
  },
  // Non-idempotent, external, and unclassified effects fail toward approval.
  {
    if: {
      properties: { effect_class: { enum: [...STEP_APPROVAL_REQUIRING_EFFECTS] } },
      required: ["effect_class"],
    },
    then: {
      properties: { approval_requirement: { enum: ["required_once", "required_per_attempt"] } },
    },
  },
  // An idempotent write is retried only against a verified key or a verified desired state.
  {
    if: {
      properties: { effect_class: { const: "idempotent_write" } },
      required: ["effect_class"],
    },
    then: {
      properties: {
        idempotency_key_requirement: { enum: ["required", "verified_desired_state"] },
      },
    },
  },
  // A read changes nothing, so claiming an idempotency key would be a false claim.
  {
    if: { properties: { effect_class: { const: "read_only" } }, required: ["effect_class"] },
    then: { properties: { idempotency_key_requirement: { const: "not_applicable" } } },
  },
  // Completion resolves to verifier evidence, or to exactly one recorded deferral reason.
  {
    if: {
      properties: { verification_requirement: { const: "verifier_evidence_required" } },
      required: ["verification_requirement"],
    },
    then: {
      properties: {
        required_verifier_ids: { type: "array", minItems: 1 },
        deferral_reason_code: { type: "null" },
      },
    },
    else: {
      properties: {
        required_verifier_ids: { type: "array", maxItems: 0 },
        deferral_reason_code: identifier,
      },
    },
  },
];

export const EXECUTION_ENVELOPE_SCHEMA_VERSION = 1;
// The five contracts this family must never replace. Each envelope reaches them by
// identity and digest only, so execution truth composes with them instead of
// restating — and eventually contradicting — what they already own.
export const PROTECTED_EXECUTION_CONTRACTS = Object.freeze({
  Plan: "plan_id",
  ToolCall: "tool_call_id",
  CapabilityGrant: "grant_id",
  OperationReceipt: "receipt_id",
  VerifiedCompletion: "verification_result_id",
});
// Content those contracts own. No envelope in this family may admit any of it.
export const PROTECTED_EXECUTION_FIELDS = Object.freeze([
  "steps",
  "description",
  "ordinal",
  "depends_on",
  "expected_evidence",
  "arguments",
  "tool_arguments",
  "parameters",
  "scope",
  "capability",
  "granted_operations",
  "exit_code",
  "stdout",
  "stderr",
  "resource_usage",
  "preserved_invariants",
  "observed_evidence_sha256s",
]);
// Deterministic failure families from the runtime retry table.
export const EXECUTION_FAILURE_CLASSES = Object.freeze([
  "transport",
  "rate",
  "timeout",
  "crash",
  "unavailable_service",
  "missing_command",
  "invalid_arguments",
  "authentication",
  "permission",
  "policy_denial",
  "deterministic_verification_failure",
  "malformed_model_output",
  "context_overflow",
  "user_rejection",
]);
// Only a classified transient failure may be retried by the runtime on its own.
export const EXECUTION_TRANSIENT_FAILURE_CLASSES = Object.freeze([
  "transport",
  "rate",
  "timeout",
  "unavailable_service",
]);
export const RECOVERY_DECISIONS = Object.freeze([
  "retry_new_attempt",
  "reconcile_then_decide",
  "request_approval",
  "request_user_decision",
  "replan",
  "defer",
  "stop",
]);
export const ATTEMPT_STATES = Object.freeze([
  "started",
  "succeeded",
  "failed",
  "denied",
  "cancelled",
  "timed_out",
  "uncertain",
]);
// Only the runtime establishes execution truth. A model proposal never does.
const RUNTIME_AUTHORITY = "agentmage-runtime";

// One bounded workflow execution. It references the existing Plan by identity and
// revision and never restates the plan's steps.
const workflowExecution = closed({
  schema_version: { type: "integer", const: EXECUTION_ENVELOPE_SCHEMA_VERSION },
  execution_id: identifier,
  workflow_id: identifier,
  workflow_version: positive,
  definition_sha256: digest,
  plan_id: identifier,
  plan_revision: uint,
  task_id: identifier,
  session_id: identifier,
  policy_sha256: digest,
  lifecycle: { enum: ["created", "validating", "ready", "running", "verifying", "waiting_for_dependency", "waiting_for_approval", "paused", "reconciling", "recovering", "succeeded", "no_op", "blocked", "denied", "failed", "cancelled", "timed_out", "resource_exhausted", "uncertain"] },
  sequence: uint,
  started_at: timestamp,
  established_by: { const: RUNTIME_AUTHORITY },
  execution_sha256: digest,
});

// One bounded step execution, keyed to the existing PlanStepId and governed by the
// companion step-execution policy.
const stepExecution = closed({
  schema_version: { type: "integer", const: EXECUTION_ENVELOPE_SCHEMA_VERSION },
  step_execution_id: identifier,
  execution_id: identifier,
  plan_step_id: identifier,
  policy_id: identifier,
  state: { enum: ["ready", "preflighting", "awaiting_approval", "running", "verifying", "reconciling", "completed", "deferred", "blocked", "failed", "cancelled"] },
  attempt_ids: list(identifier, 0, 64),
  verification_ids: list(identifier, 0, 32),
  sequence: uint,
  started_at: timestamp,
  ended_at: nullable(timestamp),
  established_by: { const: RUNTIME_AUTHORITY },
  step_execution_sha256: digest,
});

// One validated call proposed for a step. The call envelope binds the existing
// ToolCall by identity and carries only the digest of its validated arguments, never
// the argument values.
const callEnvelope = closed({
  schema_version: { type: "integer", const: EXECUTION_ENVELOPE_SCHEMA_VERSION },
  call_id: identifier,
  step_execution_id: identifier,
  tool_call_id: identifier,
  tool_id: identifier,
  tool_version: bounded,
  tool_schema_sha256: digest,
  validated_arguments_sha256: nullable(digest),
  validation_state: { enum: ["validated", "repaired_then_validated", "rejected"] },
  repair_count: uint,
  rejection_reason_code: nullable(identifier),
  proposed_at: timestamp,
  established_by: { const: RUNTIME_AUTHORITY },
  call_sha256: digest,
});
callEnvelope.allOf = [
  // A rejected call never produced validated arguments and always says why.
  {
    if: { properties: { validation_state: { const: "rejected" } }, required: ["validation_state"] },
    then: {
      properties: {
        validated_arguments_sha256: { type: "null" },
        rejection_reason_code: identifier,
      },
    },
    else: {
      properties: {
        validated_arguments_sha256: digest,
        rejection_reason_code: { type: "null" },
      },
    },
  },
  // Only a repaired call may report a repair.
  {
    if: {
      properties: { validation_state: { const: "repaired_then_validated" } },
      required: ["validation_state"],
    },
    then: { properties: { repair_count: { const: 1 } } },
    else: { properties: { repair_count: { const: 0 } } },
  },
];

// One operation attempt. The attempt binds the existing CapabilityGrant and
// OperationReceipt by identity and digest and never restates their contents.
const operationAttempt = closed({
  schema_version: { type: "integer", const: EXECUTION_ENVELOPE_SCHEMA_VERSION },
  attempt_id: identifier,
  call_id: identifier,
  step_execution_id: identifier,
  // One-based and strictly increasing. Attempt two is a new attempt, never a replay.
  attempt_ordinal: positive,
  supersedes_attempt_id: nullable(identifier),
  grant_id: identifier,
  grant_sha256: digest,
  executor_identity: identifier,
  sandbox_identity: nullable(identifier),
  state: { enum: [...ATTEMPT_STATES] },
  started_at: timestamp,
  ended_at: nullable(timestamp),
  receipt_id: nullable(identifier),
  receipt_sha256: nullable(digest),
  observation_id: nullable(identifier),
  established_by: { const: RUNTIME_AUTHORITY },
  attempt_sha256: digest,
});
operationAttempt.allOf = [
  // A started attempt has not ended and cannot yet carry a receipt.
  {
    if: { properties: { state: { const: "started" } }, required: ["state"] },
    then: {
      properties: {
        ended_at: { type: "null" },
        receipt_id: { type: "null" },
        receipt_sha256: { type: "null" },
      },
    },
    else: { properties: { ended_at: timestamp } },
  },
  // A succeeded attempt resolves to an executor receipt, never to a model claim.
  {
    if: { properties: { state: { const: "succeeded" } }, required: ["state"] },
    then: {
      properties: { receipt_id: identifier, receipt_sha256: digest },
    },
  },
  // The first attempt supersedes nothing; every later attempt names what it follows.
  {
    if: { properties: { attempt_ordinal: { const: 1 } }, required: ["attempt_ordinal"] },
    then: { properties: { supersedes_attempt_id: { type: "null" } } },
    else: { properties: { supersedes_attempt_id: identifier } },
  },
];

// Binds one verification to the step execution it judges. The verdict itself stays in
// the existing verification-result record.
const verificationEnvelope = closed({
  schema_version: { type: "integer", const: EXECUTION_ENVELOPE_SCHEMA_VERSION },
  verification_id: identifier,
  step_execution_id: identifier,
  attempt_id: nullable(identifier),
  verification_result_id: identifier,
  verifier_policy_id: identifier,
  required: { type: "boolean" },
  current: { type: "boolean" },
  observed_at: timestamp,
  established_by: { const: RUNTIME_AUTHORITY },
  verification_sha256: digest,
});

// What the runtime decided after a non-success, and why.
const recoveryDecision = closed({
  schema_version: { type: "integer", const: EXECUTION_ENVELOPE_SCHEMA_VERSION },
  decision_id: identifier,
  step_execution_id: identifier,
  attempt_id: identifier,
  policy_id: identifier,
  failure_class: { enum: [...EXECUTION_FAILURE_CLASSES] },
  // True when the attempt may have produced an effect the runtime cannot yet confirm.
  uncertain_outcome: { type: "boolean" },
  decision: { enum: [...RECOVERY_DECISIONS] },
  reason_code: identifier,
  decided_at: timestamp,
  established_by: { const: RUNTIME_AUTHORITY },
  decision_sha256: digest,
});
recoveryDecision.allOf = [
  // An outcome the runtime cannot confirm is reconciled before anything else is decided.
  {
    if: { properties: { uncertain_outcome: { const: true } }, required: ["uncertain_outcome"] },
    then: { properties: { decision: { const: "reconcile_then_decide" } } },
  },
  // The runtime opens a fresh attempt on its own only for a classified transient failure
  // with a confirmed outcome. Every other failure needs approval, replanning, or a stop.
  {
    if: { properties: { decision: { const: "retry_new_attempt" } }, required: ["decision"] },
    then: {
      properties: {
        failure_class: { enum: [...EXECUTION_TRANSIENT_FAILURE_CLASSES] },
        uncertain_outcome: { const: false },
      },
    },
  },
];

// One content-free terminal diagnostic.
const terminalDiagnostic = closed({
  schema_version: { type: "integer", const: EXECUTION_ENVELOPE_SCHEMA_VERSION },
  diagnostic_id: identifier,
  execution_id: identifier,
  terminal_result_id: identifier,
  diagnostic_code: identifier,
  failure_class: nullable({ enum: [...EXECUTION_FAILURE_CLASSES] }),
  safe_next_action_code: nullable(identifier),
  evidence_artifact_ids: list(identifier, 0, 64),
  disclosure: { enum: [...STEP_DIAGNOSTIC_DISCLOSURES] },
  reported_at: timestamp,
  established_by: { const: RUNTIME_AUTHORITY },
  diagnostic_sha256: digest,
});
terminalDiagnostic.allOf = [
  // A codes-only disclosure carries no artifact reference at all.
  {
    if: { properties: { disclosure: { const: "content_free_codes" } }, required: ["disclosure"] },
    then: { properties: { evidence_artifact_ids: { type: "array", maxItems: 0 } } },
  },
  // A successful terminal outcome names no failure class.
  {
    if: { properties: { failure_class: { type: "null" } }, required: ["failure_class"] },
    then: { properties: { safe_next_action_code: { type: "null" } } },
  },
];

export const RETRY_ADMISSION_SCHEMA_VERSION = 1;
// The identity pairs a successor attempt must not share with the attempt it follows.
// Reusing any one of them would make a retry a replay of an effect that already ran.
export const RETRY_FRESH_IDENTITY_PAIRS = Object.freeze([
  Object.freeze(["prior_attempt_id", "successor_attempt_id"]),
  Object.freeze(["prior_call_id", "successor_call_id"]),
  Object.freeze(["prior_tool_call_id", "successor_tool_call_id"]),
  Object.freeze(["prior_grant_id", "successor_grant_id"]),
]);
// The compatibility rules that keep the existing zero-hidden-retry behavior valid.
// Each rule is enforced by the structural schema, the semantic validator, or both.
export const RETRY_COMPATIBILITY_RULES = Object.freeze({
  fresh_attempt_identity: "A permitted retry opens a new operation-attempt identity. The successor never reuses the prior attempt identity.",
  fresh_call_identity: "A permitted retry issues a new call and a new tool-call identity. The prior validated call is never re-dispatched.",
  fresh_grant: "A permitted retry consumes a newly issued capability grant. A consumed grant is never replayed.",
  fresh_approval_when_required: "When policy requires approval per attempt, the successor names its own approval. A prior approval never covers a later attempt.",
  no_receipt_replay: "A retry admission carries no receipt. The successor attempt has not run, so no prior receipt may be presented as its outcome.",
  monotonic_attempt_ordinal: "The successor ordinal is exactly one greater than the prior ordinal, so attempts form an ordered chain rather than a repeated identity.",
  reconcile_before_reattempt: "When reconciliation is required, it is recorded as completed before the successor is admitted.",
  admission_requires_a_recorded_decision: "Every admission names the recovery decision that permitted it, so no retry occurs without a recorded runtime decision.",
});
// How existing behavior and existing readers migrate onto this contract.
export const RETRY_MIGRATION_RULES = Object.freeze({
  absent_admission_is_no_retry: "A runtime that performs no retry emits no retry-admission record. Absence is conformant and is never read as an implied replay.",
  additive_only: "Every record in this family is schema_version 1 and additive. No existing contract changes shape, so an existing caller keeps working unchanged.",
  unsupported_version_fails_closed: "An unrecognized schema_version is refused rather than coerced, so a newer admission is never reinterpreted under older rules.",
  downgrade_refuses_rather_than_replays: "A reader that does not understand retry-admission refuses the successor attempt. It never falls back to treating the successor as a replay of the prior attempt.",
  existing_zero_retry_behavior_remains_valid: "The current zero-hidden-retry runtime stays conformant without change, because this contract adds an admission requirement rather than a retry capability.",
});

// Admits exactly one successor attempt after a prior attempt.
//
// The record names both attempts and the recovery decision that permitted the successor.
// It carries no receipt, because the successor has not run.
const retryAdmission = closed({
  schema_version: { type: "integer", const: RETRY_ADMISSION_SCHEMA_VERSION },
  admission_id: identifier,
  step_execution_id: identifier,
  policy_id: identifier,
  decision_id: identifier,
  prior_attempt_id: identifier,
  prior_attempt_ordinal: positive,
  prior_call_id: identifier,
  prior_tool_call_id: identifier,
  prior_grant_id: identifier,
  successor_attempt_id: identifier,
  successor_attempt_ordinal: positive,
  successor_call_id: identifier,
  successor_tool_call_id: identifier,
  successor_grant_id: identifier,
  approval_requirement: { enum: [...STEP_APPROVAL_REQUIREMENTS] },
  successor_approval_id: nullable(identifier),
  reconciliation_required: { type: "boolean" },
  reconciled: { type: "boolean" },
  admitted_at: timestamp,
  established_by: { const: RUNTIME_AUTHORITY },
  admission_sha256: digest,
});
retryAdmission.allOf = [
  // A per-attempt approval covers exactly one attempt, so the successor names its own.
  {
    if: {
      properties: { approval_requirement: { const: "required_per_attempt" } },
      required: ["approval_requirement"],
    },
    then: { properties: { successor_approval_id: identifier } },
  },
  // A retry that requires reconciliation is admitted only once it has reconciled.
  {
    if: {
      properties: { reconciliation_required: { const: true } },
      required: ["reconciliation_required"],
    },
    then: { properties: { reconciled: { const: true } } },
  },
];

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
  "source-retention": sourceRetention,
  "source-locator": sourceLocator,
  "step-execution-policy": stepExecutionPolicy,
  "workflow-execution": workflowExecution,
  "step-execution": stepExecution,
  "call-envelope": callEnvelope,
  "operation-attempt": operationAttempt,
  "verification-envelope": verificationEnvelope,
  "recovery-decision": recoveryDecision,
  "terminal-diagnostic": terminalDiagnostic,
  "retry-admission": retryAdmission,
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

const SOURCE_RETENTION_LIFECYCLE_BY_CLASS = Object.freeze({
  memory_only: ["active", "quarantined"],
  policy_persisted: ["active", "quarantined"],
  released: ["released"],
  deleted: ["deleted"],
});

function sourceRetentionSemantic(record) {
  if (!record || typeof record !== "object") return false;
  const permitted = SOURCE_RETENTION_LIFECYCLE_BY_CLASS[record.retention_class];
  if (permitted === undefined) return false;
  if (!permitted.includes(record.lifecycle_state)) return false;
  const binding = record.physical_binding;
  if (binding === null || binding === undefined) return true;
  if (!RUNTIME_ARTIFACT_KINDS.includes(binding.artifact_kind)) return false;
  return Number.isSafeInteger(binding.byte_length) && binding.byte_length >= 0;
}

function sourceLocatorSemantic(record) {
  if (!record || typeof record !== "object") return false;
  const payloads = SOURCE_LOCATOR_PAYLOAD_FIELDS[record.locator_kind];
  if (payloads === undefined) return false;
  const resolved = SOURCE_LOCATOR_RESOLVED_STATES.includes(record.availability_state);
  if (!resolved && !SOURCE_LOCATOR_UNRESOLVED_STATES.includes(record.availability_state)) {
    return false;
  }
  for (const field of SOURCE_LOCATOR_ALL_PAYLOADS) {
    const present = record[field] !== null && record[field] !== undefined;
    const expected = resolved && payloads.includes(field);
    if (present !== expected) return false;
  }
  if (!resolved) return true;
  if (!isOrderedRange(record.byte_range)) return false;
  if (!isOrderedLineRange(record.line_range)) return false;
  const cell = record.cell_reference;
  if (cell !== null && cell !== undefined) {
    if (!Number.isSafeInteger(cell.sheet_row) || cell.sheet_row < 1) return false;
    if (!Number.isSafeInteger(cell.sheet_column) || cell.sheet_column < 1) return false;
  }
  const region = record.image_region;
  if (region !== null && region !== undefined) {
    for (const field of ["origin_x", "origin_y"]) {
      if (!Number.isSafeInteger(region[field]) || region[field] < 0) return false;
    }
    for (const field of ["width", "height"]) {
      if (!Number.isSafeInteger(region[field]) || region[field] < 1) return false;
    }
  }
  return true;
}

function hasUniqueIdentities(values) {
  return Array.isArray(values) && new Set(values).size === values.length;
}

function stepExecutionPolicySemantic(record) {
  if (!record || typeof record !== "object") return false;
  const permitted = STEP_EFFECT_RETRY_MATRIX[record.effect_class];
  if (permitted === undefined) return false;
  if (!permitted.includes(record.retry_class)) return false;
  // Belt and braces over the structural matrix: an effect class that fails toward
  // approval can never be paired with a retry the runtime performs by itself.
  if (
    STEP_AUTOMATIC_RETRY_CLASSES.includes(record.retry_class)
    && STEP_APPROVAL_REQUIRING_EFFECTS.includes(record.effect_class)
  ) {
    return false;
  }
  // A repeated identity would make a required preflight or verifier look satisfied twice.
  if (!hasUniqueIdentities(record.required_preflight_ids)) return false;
  if (!hasUniqueIdentities(record.required_verifier_ids)) return false;
  if (!Number.isSafeInteger(record.plan_revision) || record.plan_revision < 0) return false;
  const budgets = record.budgets;
  if (!budgets || typeof budgets !== "object") return false;
  if (!Number.isSafeInteger(budgets.attempts) || budgets.attempts < 1) return false;
  return !(record.retry_class === "never" && budgets.attempts !== 1);
}

function isOrderedInstant(startedAt, endedAt) {
  if (endedAt === null || endedAt === undefined) return true;
  if (typeof startedAt !== "string" || typeof endedAt !== "string") return false;
  const start = Date.parse(startedAt);
  const end = Date.parse(endedAt);
  return Number.isFinite(start) && Number.isFinite(end) && start <= end;
}

function stepExecutionSemantic(record) {
  if (!record || typeof record !== "object") return false;
  if (!hasUniqueIdentities(record.attempt_ids)) return false;
  if (!hasUniqueIdentities(record.verification_ids)) return false;
  return isOrderedInstant(record.started_at, record.ended_at);
}

function operationAttemptSemantic(record) {
  if (!record || typeof record !== "object") return false;
  // An attempt can never follow itself, so a superseding chain cannot close into a loop.
  if (
    record.supersedes_attempt_id !== null
    && record.supersedes_attempt_id !== undefined
    && record.supersedes_attempt_id === record.attempt_id
  ) {
    return false;
  }
  return isOrderedInstant(record.started_at, record.ended_at);
}

function recoveryDecisionSemantic(record) {
  if (!record || typeof record !== "object") return false;
  if (!EXECUTION_FAILURE_CLASSES.includes(record.failure_class)) return false;
  if (!RECOVERY_DECISIONS.includes(record.decision)) return false;
  // An outcome the runtime cannot confirm is reconciled before anything else is decided.
  if (record.uncertain_outcome === true && record.decision !== "reconcile_then_decide") {
    return false;
  }
  if (record.decision !== "retry_new_attempt") return true;
  return (
    EXECUTION_TRANSIENT_FAILURE_CLASSES.includes(record.failure_class)
    && record.uncertain_outcome === false
  );
}

function terminalDiagnosticSemantic(record) {
  if (!record || typeof record !== "object") return false;
  return hasUniqueIdentities(record.evidence_artifact_ids);
}

function retryAdmissionSemantic(record) {
  if (!record || typeof record !== "object") return false;
  // No identity crosses from the prior attempt to the successor. Reusing any one of
  // them would make this a replay of an effect that already ran.
  for (const [prior, successor] of RETRY_FRESH_IDENTITY_PAIRS) {
    const previous = record[prior];
    const next = record[successor];
    if (typeof previous !== "string" || typeof next !== "string") return false;
    if (previous === next) return false;
  }
  // Attempts form an ordered chain rather than a repeated identity.
  if (!Number.isSafeInteger(record.prior_attempt_ordinal)) return false;
  if (!Number.isSafeInteger(record.successor_attempt_ordinal)) return false;
  if (record.prior_attempt_ordinal < 1) return false;
  if (record.successor_attempt_ordinal !== record.prior_attempt_ordinal + 1) return false;
  if (record.reconciliation_required === true && record.reconciled !== true) return false;
  if (record.approval_requirement === "required_per_attempt") {
    return typeof record.successor_approval_id === "string";
  }
  return true;
}

export const ENGINEERING_RUNTIME_SEMANTIC_VALIDATORS = Object.freeze({
  "structural-section": (record) => isOrderedRange(record.byte_range) && isOrderedLineRange(record.line_range),
  "context-disposition": (record) => isOrderedRangeList(record.ranges),
  "context-manifest": contextManifestSemantic,
  "source-retention": sourceRetentionSemantic,
  "source-locator": sourceLocatorSemantic,
  "step-execution-policy": stepExecutionPolicySemantic,
  "step-execution": stepExecutionSemantic,
  "operation-attempt": operationAttemptSemantic,
  "recovery-decision": recoveryDecisionSemantic,
  "terminal-diagnostic": terminalDiagnosticSemantic,
  "retry-admission": retryAdmissionSemantic,
});

export const ENGINEERING_RUNTIME_SEMANTIC_INVARIANTS = Object.freeze({
  "structural-section": "byte_range MUST satisfy start_byte <= end_byte_exclusive and line_range, when present, MUST satisfy start_line <= end_line_exclusive.",
  "context-disposition": "Every entry in ranges MUST satisfy start_byte <= end_byte_exclusive. reason_code MUST be null when disposition is included and MUST be a non-null identifier for every non-complete disposition.",
  "context-manifest": "items.length MUST equal source_artifact_count, artifact_id values MUST be unique, every item ranges entry MUST satisfy start_byte <= end_byte_exclusive, and the sum of item token_count MUST NOT exceed total_input_tokens.",
  "source-retention": "retention_class MUST agree with lifecycle_state: memory_only and policy_persisted permit only active or quarantined, released requires released, and deleted requires deleted. physical_binding, when present, MUST name an existing RuntimeArtifactKind and a non-negative byte_length; a source-specific physical family is prohibited.",
  "source-locator": "locator_kind MUST carry exactly its own payload fields and no others. complete, partial, and truncated require every declared payload; encrypted, unsupported, and unavailable MUST carry no payload at all. byte_range and line_range MUST be ordered, cell_reference row and column MUST be one-indexed, and image_region width and height MUST be positive.",
  "step-execution-policy": "effect_class MUST admit its retry_class under the closed effect/retry matrix, and an effect class that fails toward approval MUST NOT carry an automatic retry class. required_preflight_ids and required_verifier_ids MUST NOT repeat an identity. plan_revision MUST be a non-negative integer, budgets.attempts MUST be at least one, and a retry_class of never MUST allow exactly one attempt.",
  "step-execution": "attempt_ids and verification_ids MUST NOT repeat an identity, and ended_at, when present, MUST NOT precede started_at.",
  "operation-attempt": "supersedes_attempt_id MUST NOT equal attempt_id, and ended_at, when present, MUST NOT precede started_at.",
  "recovery-decision": "An uncertain outcome MUST decide reconcile_then_decide. retry_new_attempt MUST carry a classified transient failure class and a confirmed outcome, so the runtime never reopens a destructive, external, or unconfirmed operation on its own authority.",
  "terminal-diagnostic": "evidence_artifact_ids MUST NOT repeat an identity.",
  "retry-admission": "No identity may cross from the prior attempt to the successor: attempt, call, tool-call, and grant identities MUST all differ, so a permitted retry is a new attempt and never a replay. successor_attempt_ordinal MUST equal prior_attempt_ordinal plus one, required reconciliation MUST be recorded as completed, and an approval required per attempt MUST name the successor's own approval.",
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
