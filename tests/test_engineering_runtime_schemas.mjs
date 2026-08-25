import assert from "node:assert/strict";
import test from "node:test";

import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

import {
  ENGINEERING_RUNTIME_SCHEMAS,
  REUSED_SCHEMA_CONTRACTS,
  schemaDocument,
  synchronize,
} from "../scripts/engineering_runtime_schemas.mjs";

const SHA = "a".repeat(64);

function validator(name) {
  const ajv = new Ajv2020({ allErrors: true, strict: true });
  addFormats(ajv);
  return ajv.compile(schemaDocument(name));
}

test("generated Engineering Runtime schemas are current, closed, and compile", () => {
  assert.equal(synchronize(), 26);
  assert.equal(Object.keys(REUSED_SCHEMA_CONTRACTS).length, 6);
  assert.equal(Object.keys(ENGINEERING_RUNTIME_SCHEMAS).length, 26);
});

test("source-artifact family schemas reject missing, extra, malformed, stale, oversized, and unsupported-version envelopes", () => {
  const timestamp = "2026-08-25T12:00:00Z";
  const captured = {
    schema_version: 1,
    source_artifact_id: "source-1",
    request_id: "request-1",
    authority_id: "authority-1",
    reference_id: "reference-1",
    origin_id: "origin-1",
    provenance_sha256: SHA,
    declared_media_type: "text/plain",
    classification: "internal",
    freshness_state: "fresh",
    capture_state: "captured",
    byte_length: 3,
    sha256: SHA,
    collected_at: timestamp,
    source_artifact_sha256: SHA,
  };
  const validateArtifact = validator("source-artifact");
  assert.equal(validateArtifact(captured), true, JSON.stringify(validateArtifact.errors));
  const { schema_version: _omitted, ...missingVersion } = captured;
  assert.equal(validateArtifact(missingVersion), false);
  assert.equal(validateArtifact({ ...captured, unknown_field: true }), false);
  assert.equal(validateArtifact({ ...captured, sha256: "not-a-digest" }), false);
  assert.equal(validateArtifact({ ...captured, freshness_state: "stale" }), false);
  assert.equal(validateArtifact({ ...captured, byte_length: 200 * 1024 * 1024 }), false);
  assert.equal(validateArtifact({ ...captured, schema_version: 2 }), false);
  const unavailable = { ...captured, capture_state: "unavailable", byte_length: null, sha256: null };
  assert.equal(validateArtifact(unavailable), true, JSON.stringify(validateArtifact.errors));
  assert.equal(validateArtifact({ ...unavailable, byte_length: 3 }), false);

  const origin = {
    schema_version: 1,
    origin_id: "origin-1",
    request_id: "request-1",
    authority_id: "authority-1",
    origin_class: "request_reference",
    captured_at: timestamp,
    origin_sha256: SHA,
  };
  const validateOrigin = validator("origin");
  assert.equal(validateOrigin(origin), true, JSON.stringify(validateOrigin.errors));
  assert.equal(validateOrigin({ ...origin, origin_class: "ambient" }), false);
  assert.equal(validateOrigin({ ...origin, schema_version: 0 }), false);

  const reference = {
    schema_version: 1,
    reference_id: "reference-1",
    request_id: "request-1",
    authority_id: "authority-1",
    reference_class: "request_reference",
    reference_display: "supplied editor selection",
    support_state: "supported",
    reference_sha256: SHA,
    collected_at: timestamp,
  };
  const validateReference = validator("source-reference");
  assert.equal(validateReference(reference), true, JSON.stringify(validateReference.errors));
  assert.equal(validateReference({ ...reference, reference_display: "" }), false);
  assert.equal(validateReference({ ...reference, extra: "field" }), false);

  const provenance = {
    schema_version: 1,
    provenance_id: "provenance-1",
    source_artifact_id: "source-1",
    reference_id: "reference-1",
    origin_id: "origin-1",
    classification: "internal",
    freshness_state: "fresh",
    observed_at: timestamp,
    collected_at: timestamp,
    provenance_sha256: SHA,
  };
  const validateProvenance = validator("source-provenance");
  assert.equal(validateProvenance(provenance), true, JSON.stringify(validateProvenance.errors));
  assert.equal(validateProvenance({ ...provenance, freshness_state: "elsewhere" }), false);
  assert.equal(validateProvenance({ ...provenance, provenance_sha256: "short" }), false);

  const extraction = {
    schema_version: 1,
    extraction_id: "extraction-1",
    source_artifact_id: "source-1",
    extractor_id: "extractor-1",
    extractor_version: "1.0.0",
    source_sha256: SHA,
    output_sha256: SHA,
    media_type: "text/plain",
    disposition: "parsed",
    section_ids: ["section-1"],
    warnings: [],
    truncated: false,
    reproducible: true,
    terminal: true,
  };
  const validateExtraction = validator("extraction-result");
  assert.equal(validateExtraction(extraction), true, JSON.stringify(validateExtraction.errors));
  assert.equal(validateExtraction({ ...extraction, terminal: false }), false);
  assert.equal(validateExtraction({ ...extraction, disposition: "queued" }), false);
  const bigWarnings = Array.from({ length: 257 }, (_, index) => `warning-${index}`);
  assert.equal(validateExtraction({ ...extraction, warnings: bigWarnings }), false);

  const section = {
    schema_version: 1,
    section_id: "section-1",
    source_artifact_id: "source-1",
    extraction_id: "extraction-1",
    parent_section_id: null,
    ordinal: 0,
    kind: "heading",
    byte_range: { start_byte: 0, end_byte_exclusive: 32 },
    line_range: { start_byte: 1, end_byte_exclusive: 2 },
    token_count: 8,
    title: "Overview",
    content_sha256: SHA,
  };
  const validateSection = validator("structural-section");
  assert.equal(validateSection(section), true, JSON.stringify(validateSection.errors));
  assert.equal(validateSection({ ...section, kind: "footnote" }), false);
  assert.equal(validateSection({ ...section, schema_version: 99 }), false);

  const disposition = {
    schema_version: 1,
    disposition_id: "disposition-1",
    context_manifest_id: "context-1",
    source_artifact_id: "source-1",
    section_id: null,
    disposition: "stale",
    ranges: [],
    token_count: 0,
    reason_code: "source-mutated",
    reason: null,
    terminal: true,
  };
  const validateDisposition = validator("context-disposition");
  assert.equal(validateDisposition(disposition), true, JSON.stringify(validateDisposition.errors));
  assert.equal(validateDisposition({ ...disposition, disposition: "queued" }), false);
  assert.equal(validateDisposition({ ...disposition, terminal: false }), false);
  const overRanges = Array.from({ length: 257 }, () => ({ start_byte: 0, end_byte_exclusive: 1 }));
  assert.equal(validateDisposition({ ...disposition, ranges: overRanges }), false);
});

test("multi-agent schemas require bounded workers and verifier-owned completion evidence", () => {
  const lease = {
    schema_version: 1,
    campaign_id: "campaign-1",
    lease_id: "lease-1",
    task_id: "task-1",
    agent_id: "agent-1",
    session_id: "session-1",
    model_profile_id: "model-1",
    endpoint_profile_id: "endpoint-1",
    base_commit: "a".repeat(40),
    worktree_id: "worktree-1",
    branch: "agentmage/task-1",
    path_leases: ["kernel/engine"],
    test_resource_leases: ["cargo-workspace"],
    state: "implementing",
    correction_limit: 3,
    correction_count: 0,
    candidate_commit: null,
    lease_sha256: SHA,
  };
  const validateLease = validator("agent-lease");
  assert.equal(validateLease(lease), true, JSON.stringify(validateLease.errors));
  assert.equal(validateLease({ ...lease, state: "self_approved" }), false);

  const campaign = {
    schema_version: 1,
    campaign_id: "campaign-1",
    coordinator_session_id: "session-1",
    objective: "Implement an approved plan",
    approved_plan_id: "approval-1",
    approved_plan_sha256: SHA,
    campaign_branch: "agentmage/campaign-1",
    starting_commit: "a".repeat(40),
    campaign_head: "b".repeat(40),
    max_workers: 5,
    maximum_concurrent_workers: 1,
    state: "success",
    task_ids: ["task-1"],
    leases: [lease],
    integrations: [],
    reason_codes: [],
    final_evidence: [{ kind: "test", uri: "artifact:test-1", sha256: SHA }],
    campaign_sha256: SHA,
  };
  const validateCampaign = validator("multi-agent-campaign");
  assert.equal(validateCampaign(campaign), true, JSON.stringify(validateCampaign.errors));
  assert.equal(validateCampaign({ ...campaign, max_workers: 6 }), false);
  assert.equal(validateCampaign({ ...campaign, final_evidence: [] }), false);
});

test("artifact capture and context delivery fail closed", () => {
  const envelope = {
    schema_version: 1,
    artifact_id: "artifact-1",
    request_id: "request-1",
    authority_id: "authority-1",
    origin: "paste",
    media_type: "text/plain",
    classification: "internal",
    capture_state: "captured",
    byte_length: 3,
    sha256: SHA,
    collected_at: "2026-08-22T12:00:00Z",
  };
  const validateEnvelope = validator("artifact-envelope");
  assert.equal(validateEnvelope(envelope), true, JSON.stringify(validateEnvelope.errors));
  assert.equal(validateEnvelope({ ...envelope, unknown: true }), false);
  assert.equal(validateEnvelope({ ...envelope, capture_state: "failed" }), false);

  const receipt = {
    schema_version: 1,
    receipt_id: "delivery-1",
    context_manifest_id: "context-1",
    context_manifest_sha256: SHA,
    model_request_id: "model-request-1",
    route_decision_id: "route-decision-1",
    delivered: [],
    required_unseen_artifact_ids: ["artifact-1"],
    outcome: "blocked",
    receipt_sha256: SHA,
  };
  const validateReceipt = validator("context-delivery-receipt");
  assert.equal(validateReceipt(receipt), true, JSON.stringify(validateReceipt.errors));
  assert.equal(validateReceipt({ ...receipt, outcome: "delivered" }), false);
});

test("remote endpoint and route records cannot enable or silently fallback", () => {
  const endpoint = {
    schema_version: 1,
    endpoint_profile_id: "endpoint-1",
    profile_class: "remote_private",
    operator_id: "operator-1",
    endpoint_reference: "secret-reference:endpoint-1",
    protocol_codec_id: "codec-1",
    tls_policy: "mutual_tls",
    host_policy_sha256: SHA,
    credential_reference: "secret-1",
    region: "declared-region",
    retention_policy: "declared",
    logging_policy: "declared",
    training_use_policy: "declared",
    quota_policy_sha256: SHA,
    cost_policy_sha256: SHA,
    qualification_sha256: null,
    enabled: false,
    automatic_fallback: false,
  };
  const validateEndpoint = validator("model-endpoint-profile");
  assert.equal(validateEndpoint(endpoint), true, JSON.stringify(validateEndpoint.errors));
  assert.equal(validateEndpoint({ ...endpoint, enabled: true }), false);
  assert.equal(validateEndpoint({ ...endpoint, automatic_fallback: true }), false);

  const decision = {
    schema_version: 1,
    route_decision_id: "route-decision-1",
    request_id: "request-1",
    policy_sha256: SHA,
    disclosure_class: "private_remote",
    considered_routes: [{ route_id: "route-1", qualified: false, admitted: false, reason: "not-qualified" }],
    selected_route_id: null,
    fallback_used: false,
    fallback_policy_sha256: null,
    reason: "no-admitted-route",
    decision_sha256: SHA,
  };
  const validateDecision = validator("model-route-decision");
  assert.equal(validateDecision(decision), true, JSON.stringify(validateDecision.errors));
  assert.equal(validateDecision({ ...decision, fallback_used: true }), false);
});

test("tool observations and terminal results preserve runtime authority", () => {
  const observation = {
    schema_version: 1,
    observation_id: "observation-1",
    tool_call_id: "call-1",
    attempt_id: "attempt-1",
    task_id: "task-1",
    step_id: "step-1",
    tool_id: "tool-1",
    tool_version: "1",
    tool_schema_sha256: SHA,
    arguments_sha256: SHA,
    authority_id: "authority-1",
    started_at: "2026-08-22T12:00:00Z",
    completed_at: "2026-08-22T12:00:01Z",
    outcome: "uncertain",
    exit_code: null,
    signal: null,
    stdout: null,
    stderr: null,
    stdout_excerpt: "",
    stderr_excerpt: "",
    stdout_truncated: false,
    stderr_truncated: false,
    generated_artifact_ids: [],
    state_change: "uncertain",
    descendants_cleaned: true,
    resource_usage_sha256: SHA,
    retry_disposition: "reconcile_first",
    receipt_sha256: SHA,
    terminal: true,
  };
  const validateObservation = validator("tool-observation");
  assert.equal(validateObservation(observation), true, JSON.stringify(validateObservation.errors));
  assert.equal(validateObservation({ ...observation, terminal: false }), false);

  const terminal = {
    schema_version: 1,
    terminal_result_id: "terminal-1",
    workflow_id: "workflow-1",
    outcome: "verified_success",
    verification_result_ids: ["verification-1"],
    last_verified_state_sha256: SHA,
    diagnostic_code: null,
    safe_next_action: null,
    established_by: "agentmage-runtime-verifier",
    result_sha256: SHA,
  };
  const validateTerminal = validator("terminal-result");
  assert.equal(validateTerminal(terminal), true, JSON.stringify(validateTerminal.errors));
  assert.equal(validateTerminal({ ...terminal, established_by: "model" }), false);
  assert.equal(validateTerminal({ ...terminal, verification_result_ids: [] }), false);
});
