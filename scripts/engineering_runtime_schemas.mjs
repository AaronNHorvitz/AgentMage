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

const contextItem = closed({
  artifact_id: identifier,
  disposition: { enum: ["included", "summarized", "truncated", "duplicate", "stale", "unsupported", "unavailable", "restricted", "omitted"] },
  ranges: list(range),
  token_count: uint,
  reason: nullable(bounded),
});
const contextManifest = closed({
  schema_version: positive,
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

export const ENGINEERING_RUNTIME_SCHEMAS = Object.freeze({
  "artifact-envelope": artifactEnvelope,
  "artifact-transformation": artifactTransformation,
  "artifact-ingestion-result": artifactIngestionResult,
  "context-manifest": contextManifest,
  "context-delivery-receipt": contextDeliveryReceipt,
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
  return {
    $schema: "https://json-schema.org/draft/2020-12/schema",
    $id: `${BASE}/${name}.schema.json`,
    title: `AgentMage ${name}`,
    ...schema,
  };
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
