import assert from "node:assert/strict";
import test from "node:test";

import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

import {
  CONTEXT_MANIFEST_SCHEMA_VERSION,
  ENGINEERING_RUNTIME_SCHEMAS,
  RUNTIME_ARTIFACT_KINDS,
  SOURCE_LOCATOR_KINDS,
  SOURCE_LOCATOR_PAYLOAD_FIELDS,
  SOURCE_LOCATOR_RESOLVED_STATES,
  SOURCE_LOCATOR_SCHEMA_VERSION,
  SOURCE_LOCATOR_UNRESOLVED_STATES,
  SOURCE_RETENTION_PHYSICAL_STORE,
  SOURCE_RETENTION_SCHEMA_VERSION,
  STEP_APPROVAL_REQUIRING_EFFECTS,
  STEP_AUTOMATIC_RETRY_CLASSES,
  STEP_EFFECT_CLASSES,
  STEP_EFFECT_RETRY_MATRIX,
  STEP_EXECUTION_POLICY_IDENTITIES,
  STEP_EXECUTION_POLICY_SCHEMA_VERSION,
  STEP_RETRY_CLASSES,
  ENGINEERING_RUNTIME_SEMANTIC_INVARIANTS,
  ENGINEERING_RUNTIME_SEMANTIC_VALIDATORS,
  REUSED_SCHEMA_CONTRACTS,
  createEngineeringRuntimeValidator,
  schemaDocument,
  synchronize,
  validateEngineeringRuntimeRecord,
} from "../scripts/engineering_runtime_schemas.mjs";

const SHA = "a".repeat(64);

function ajvInstance() {
  const ajv = new Ajv2020({ allErrors: true, strict: true });
  addFormats(ajv);
  return ajv;
}

function validator(name) {
  return ajvInstance().compile(schemaDocument(name));
}

function combinedValidator(name) {
  const compiled = validator(name);
  return (candidate) => validateEngineeringRuntimeRecord(compiled, name, candidate);
}

test("generated Engineering Runtime schemas are current, closed, and compile", () => {
  assert.equal(synchronize(), 29);
  assert.equal(Object.keys(REUSED_SCHEMA_CONTRACTS).length, 6);
  assert.equal(Object.keys(ENGINEERING_RUNTIME_SCHEMAS).length, 29);
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
    line_range: { start_line: 1, end_line_exclusive: 2 },
    token_count: 8,
    title: "Overview",
    content_sha256: SHA,
  };
  const validateSection = validator("structural-section");
  assert.equal(validateSection(section), true, JSON.stringify(validateSection.errors));
  assert.equal(validateSection({ ...section, kind: "footnote" }), false);
  assert.equal(validateSection({ ...section, schema_version: 99 }), false);
  assert.equal(
    validateSection({ ...section, line_range: { start_byte: 1, end_byte_exclusive: 2 } }),
    false,
    "byte-named fields inside line_range must be rejected structurally",
  );

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

test("context-manifest binds schema_version to the authoritative Rust contract version", () => {
  assert.equal(CONTEXT_MANIFEST_SCHEMA_VERSION, 2);
  const manifest = {
    schema_version: CONTEXT_MANIFEST_SCHEMA_VERSION,
    context_manifest_id: "context-1",
    session_id: "session-1",
    turn_id: "turn-1",
    model_profile_id: "profile-1",
    source_artifact_count: 0,
    items: [],
    total_input_tokens: 0,
    reserved_output_tokens: 0,
    safety_margin_tokens: 0,
    manifest_sha256: SHA,
  };
  const validateManifest = validator("context-manifest");
  assert.equal(validateManifest(manifest), true, JSON.stringify(validateManifest.errors));
  const { schema_version: _dropped, ...withoutVersion } = manifest;
  assert.equal(validateManifest(withoutVersion), false);
  for (const rejected of [0, CONTEXT_MANIFEST_SCHEMA_VERSION - 1, CONTEXT_MANIFEST_SCHEMA_VERSION + 1, 999]) {
    assert.equal(validateManifest({ ...manifest, schema_version: rejected }), false, `version ${rejected} must fail`);
  }
});

test("structural sections and context dispositions reject reversed ranges through trusted semantic validation", () => {
  const section = {
    schema_version: 1,
    section_id: "section-1",
    source_artifact_id: "source-1",
    extraction_id: "extraction-1",
    parent_section_id: null,
    ordinal: 0,
    kind: "paragraph",
    byte_range: { start_byte: 0, end_byte_exclusive: 32 },
    line_range: { start_line: 1, end_line_exclusive: 2 },
    token_count: 8,
    title: null,
    content_sha256: SHA,
  };
  const validateSection = combinedValidator("structural-section");
  assert.equal(validateSection(section), true);
  assert.equal(validateSection({ ...section, byte_range: { start_byte: 32, end_byte_exclusive: 32 } }), true);
  assert.equal(validateSection({ ...section, byte_range: { start_byte: 10, end_byte_exclusive: 3 } }), false);
  assert.equal(
    validateSection({ ...section, line_range: { start_line: 5, end_line_exclusive: 1 } }),
    false,
    "reversed line coordinates must be rejected",
  );
  assert.equal(
    validateSection({ ...section, line_range: { start_byte: 1, end_byte_exclusive: 2 } }),
    false,
    "byte-named fields inside line_range must be rejected",
  );
  assert.equal(
    validateSection({ ...section, line_range: { start_line: 3, end_line_exclusive: 3 } }),
    true,
    "zero-length line ranges are admitted",
  );

  const disposition = {
    schema_version: 1,
    disposition_id: "disposition-1",
    context_manifest_id: "context-1",
    source_artifact_id: "source-1",
    section_id: null,
    disposition: "included",
    ranges: [{ start_byte: 0, end_byte_exclusive: 16 }],
    token_count: 4,
    reason_code: null,
    reason: null,
    terminal: true,
  };
  const validateDisposition = combinedValidator("context-disposition");
  assert.equal(validateDisposition(disposition), true);
  assert.equal(
    validateDisposition({ ...disposition, ranges: [{ start_byte: 0, end_byte_exclusive: 0 }] }),
    true,
    "zero-length ranges are admitted",
  );
  assert.equal(
    validateDisposition({ ...disposition, ranges: [{ start_byte: 12, end_byte_exclusive: 4 }] }),
    false,
  );
});

test("extraction terminal states forbid payload contradicting the declared disposition", () => {
  const validateExtraction = validator("extraction-result");
  const base = {
    schema_version: 1,
    extraction_id: "extraction-1",
    source_artifact_id: "source-1",
    extractor_id: "extractor-1",
    extractor_version: "1.0.0",
    source_sha256: SHA,
    media_type: "text/plain",
    section_ids: [],
    warnings: [],
    truncated: false,
    reproducible: true,
    terminal: true,
  };
  const producing = { ...base, disposition: "parsed", output_sha256: SHA, section_ids: ["section-1"] };
  assert.equal(validateExtraction(producing), true, JSON.stringify(validateExtraction.errors));
  assert.equal(validateExtraction({ ...producing, output_sha256: null }), false);
  assert.equal(
    validateExtraction({ ...base, disposition: "partially_parsed", output_sha256: SHA, section_ids: ["section-1"] }),
    true,
  );
  assert.equal(
    validateExtraction({ ...base, disposition: "captured", output_sha256: SHA, section_ids: [] }),
    true,
  );
  for (const state of ["unavailable", "denied", "unsupported", "failed", "omitted"]) {
    const consistent = { ...base, disposition: state, output_sha256: null, section_ids: [] };
    assert.equal(validateExtraction(consistent), true, `${state} consistent payload must pass`);
    assert.equal(
      validateExtraction({ ...consistent, output_sha256: SHA }),
      false,
      `${state} must not carry output_sha256`,
    );
    assert.equal(
      validateExtraction({ ...consistent, section_ids: ["section-1"] }),
      false,
      `${state} must not claim sections`,
    );
  }
});

test("context-manifest semantic validation reconciles source_artifact_count, artifact_id uniqueness, and ordered ranges", () => {
  const validateManifest = combinedValidator("context-manifest");
  const buildItem = (id, overrides = {}) => ({
    artifact_id: id,
    disposition: "included",
    ranges: [{ start_byte: 0, end_byte_exclusive: 4 }],
    token_count: 2,
    reason_code: null,
    reason: null,
    ...overrides,
  });
  const base = {
    schema_version: CONTEXT_MANIFEST_SCHEMA_VERSION,
    context_manifest_id: "context-1",
    session_id: "session-1",
    turn_id: "turn-1",
    model_profile_id: "profile-1",
    source_artifact_count: 2,
    items: [buildItem("artifact-1"), buildItem("artifact-2")],
    total_input_tokens: 4,
    reserved_output_tokens: 0,
    safety_margin_tokens: 0,
    manifest_sha256: SHA,
  };
  assert.equal(validateManifest(base), true);
  assert.equal(
    validateManifest({ ...base, source_artifact_count: 2, items: [buildItem("artifact-1")] }),
    false,
    "item count below declared source_artifact_count must fail",
  );
  assert.equal(
    validateManifest({ ...base, source_artifact_count: 1 }),
    false,
    "item count above declared source_artifact_count must fail",
  );
  assert.equal(
    validateManifest({ ...base, items: [buildItem("artifact-1"), buildItem("artifact-1")] }),
    false,
    "duplicate artifact_id must fail",
  );
  assert.equal(
    validateManifest({
      ...base,
      items: [buildItem("artifact-1"), buildItem("artifact-2", { ranges: [{ start_byte: 12, end_byte_exclusive: 4 }] })],
    }),
    false,
    "reversed range inside a manifest item must fail",
  );
  assert.equal(
    validateManifest({
      ...base,
      items: [buildItem("artifact-1"), buildItem("artifact-2", { ranges: [{ start_byte: 4, end_byte_exclusive: 4 }] })],
    }),
    true,
    "zero-length range inside a manifest item is admitted",
  );
  assert.equal(
    validateManifest({
      ...base,
      items: [buildItem("artifact-1", { token_count: 100 }), buildItem("artifact-2")],
      total_input_tokens: 4,
    }),
    false,
    "item token_count sum exceeding total_input_tokens must fail",
  );
  assert.equal(
    validateManifest({
      ...base,
      source_artifact_count: 1,
      items: [buildItem("artifact-1", { token_count: 100 })],
      total_input_tokens: 0,
    }),
    false,
    "single item token_count exceeding total_input_tokens must fail",
  );
  assert.equal(
    validateManifest({ ...base, total_input_tokens: 10 }),
    true,
    "total_input_tokens above the item sum (non-artifact input included) remains valid",
  );
  assert.equal(
    validateManifest({ ...base }),
    true,
    "total_input_tokens equal to the item sum remains valid",
  );
});

test("context-manifest items reject non-admitting dispositions carrying ranges or tokens", () => {
  const validateManifest = validator("context-manifest");
  const base = {
    schema_version: CONTEXT_MANIFEST_SCHEMA_VERSION,
    context_manifest_id: "context-1",
    session_id: "session-1",
    turn_id: "turn-1",
    model_profile_id: "profile-1",
    source_artifact_count: 1,
    items: [],
    total_input_tokens: 0,
    reserved_output_tokens: 0,
    safety_margin_tokens: 0,
    manifest_sha256: SHA,
  };
  for (const state of ["duplicate", "stale", "unsupported", "unavailable", "restricted", "omitted"]) {
    const empty = {
      ...base,
      items: [{
        artifact_id: `artifact-${state}`,
        disposition: state,
        ranges: [],
        token_count: 0,
        reason_code: `code-${state}`,
        reason: null,
      }],
    };
    assert.equal(validateManifest(empty), true, `${state} empty item must pass structural validation`);
    assert.equal(
      validateManifest({
        ...empty,
        items: [{ ...empty.items[0], ranges: [{ start_byte: 0, end_byte_exclusive: 4 }] }],
      }),
      false,
      `${state} item must not carry ranges`,
    );
    assert.equal(
      validateManifest({
        ...empty,
        items: [{ ...empty.items[0], token_count: 1 }],
      }),
      false,
      `${state} item must not carry nonzero token_count`,
    );
  }
  for (const state of ["included", "summarized", "truncated"]) {
    const admitting = {
      ...base,
      items: [{
        artifact_id: `artifact-${state}`,
        disposition: state,
        ranges: [{ start_byte: 0, end_byte_exclusive: 4 }],
        token_count: 2,
        reason_code: state === "included" ? null : `code-${state}`,
        reason: null,
      }],
    };
    assert.equal(validateManifest(admitting), true, `${state} admitting item must pass`);
  }
});

test("context-manifest items require a deterministic reason_code for every non-complete disposition", () => {
  const validateManifest = validator("context-manifest");
  const base = {
    schema_version: CONTEXT_MANIFEST_SCHEMA_VERSION,
    context_manifest_id: "context-1",
    session_id: "session-1",
    turn_id: "turn-1",
    model_profile_id: "profile-1",
    source_artifact_count: 1,
    items: [],
    total_input_tokens: 0,
    reserved_output_tokens: 0,
    safety_margin_tokens: 0,
    manifest_sha256: SHA,
  };
  const admittingRanges = [{ start_byte: 0, end_byte_exclusive: 4 }];
  const buildItem = (state, reason_code) => {
    const admitting = state === "included" || state === "summarized" || state === "truncated";
    return {
      artifact_id: `artifact-${state}`,
      disposition: state,
      ranges: admitting ? admittingRanges : [],
      token_count: admitting ? 2 : 0,
      reason_code,
      reason: null,
    };
  };
  for (const state of ["summarized", "truncated", "duplicate", "stale", "unsupported", "unavailable", "restricted", "omitted"]) {
    assert.equal(
      validateManifest({ ...base, items: [buildItem(state, null)] }),
      false,
      `${state} without reason_code must fail`,
    );
    assert.equal(
      validateManifest({ ...base, items: [buildItem(state, `reason-${state}`)] }),
      true,
      `${state} with deterministic reason_code must pass`,
    );
    assert.equal(
      validateManifest({ ...base, items: [buildItem(state, "not a code")] }),
      false,
      `${state} with malformed reason_code must fail`,
    );
  }
  assert.equal(
    validateManifest({ ...base, items: [buildItem("included", null)] }),
    true,
    "included item may have null reason_code",
  );
  assert.equal(
    validateManifest({ ...base, items: [buildItem("included", "unexpected-code")] }),
    false,
    "included item must not carry a reason_code",
  );
});

test("createEngineeringRuntimeValidator is the mandatory public path that binds structural and semantic contracts", () => {
  const ajv = ajvInstance();
  for (const name of Object.keys(ENGINEERING_RUNTIME_SEMANTIC_INVARIANTS)) {
    const document = schemaDocument(name);
    assert.equal(
      typeof document.description === "string" && document.description.startsWith("Semantic invariants"),
      true,
      `${name} schema document must publish its semantic invariant`,
    );
    const validate = createEngineeringRuntimeValidator(name, ajv);
    assert.equal(validate.schemaName, name);
    assert.equal(validate.semantic, ENGINEERING_RUNTIME_SEMANTIC_VALIDATORS[name]);
  }
  const validateSection = createEngineeringRuntimeValidator("structural-section", ajv);
  const section = {
    schema_version: 1,
    section_id: "section-1",
    source_artifact_id: "source-1",
    extraction_id: "extraction-1",
    parent_section_id: null,
    ordinal: 0,
    kind: "paragraph",
    byte_range: { start_byte: 0, end_byte_exclusive: 32 },
    line_range: { start_line: 1, end_line_exclusive: 2 },
    token_count: 8,
    title: null,
    content_sha256: SHA,
  };
  assert.equal(validateSection(section), true);
  assert.equal(validateSection({ ...section, byte_range: { start_byte: 32, end_byte_exclusive: 0 } }), false);
  assert.equal(validateSection({ ...section, line_range: { start_line: 8, end_line_exclusive: 2 } }), false);

  const validateDisposition = createEngineeringRuntimeValidator("context-disposition", ajv);
  const disposition = {
    schema_version: 1,
    disposition_id: "disposition-1",
    context_manifest_id: "context-1",
    source_artifact_id: "source-1",
    section_id: null,
    disposition: "included",
    ranges: [{ start_byte: 0, end_byte_exclusive: 16 }],
    token_count: 4,
    reason_code: null,
    reason: null,
    terminal: true,
  };
  assert.equal(validateDisposition(disposition), true);
  assert.equal(
    validateDisposition({ ...disposition, ranges: [{ start_byte: 16, end_byte_exclusive: 0 }] }),
    false,
    "reversed disposition range is rejected via the mandatory public path",
  );

  assert.throws(() => createEngineeringRuntimeValidator("nonexistent-schema", ajv));
  assert.throws(() => validateEngineeringRuntimeRecord(() => true, "nonexistent-schema", {}));
});

test("createEngineeringRuntimeValidator refuses to reuse a mismatched schema registered under the authoritative $id", () => {
  const ajv = ajvInstance();
  const document = schemaDocument("context-manifest");
  const stale = {
    $schema: "https://json-schema.org/draft/2020-12/schema",
    $id: document.$id,
    type: "object",
    additionalProperties: true,
    properties: {
      schema_version: { type: "integer", minimum: 1 },
    },
  };
  ajv.addSchema(stale, document.$id);
  assert.throws(
    () => createEngineeringRuntimeValidator("context-manifest", ajv),
    /refusing to reuse mismatched schema/,
  );

  const fresh = ajvInstance();
  fresh.compile(document);
  const validate = createEngineeringRuntimeValidator("context-manifest", fresh);
  assert.equal(validate.compiled, fresh.getSchema(document.$id));
  const rejected = {
    schema_version: 1,
    context_manifest_id: "context-1",
    session_id: "session-1",
    turn_id: "turn-1",
    model_profile_id: "profile-1",
    source_artifact_count: 0,
    items: [],
    total_input_tokens: 0,
    reserved_output_tokens: 0,
    safety_margin_tokens: 0,
    manifest_sha256: SHA,
  };
  assert.equal(
    validate(rejected),
    false,
    "reused authoritative schema must still reject schema_version 1",
  );
  assert.equal(
    validate({ ...rejected, schema_version: CONTEXT_MANIFEST_SCHEMA_VERSION }),
    true,
    "reused authoritative schema must still admit the current schema_version",
  );
});

test("source-reference records reject contradictory reference_class and support_state combinations", () => {
  const timestamp = "2026-08-25T12:00:00Z";
  const validateReference = validator("source-reference");
  const base = {
    schema_version: 1,
    reference_id: "reference-1",
    request_id: "request-1",
    authority_id: "authority-1",
    reference_display: "supplied editor selection",
    reference_sha256: SHA,
    collected_at: timestamp,
  };
  const supportedClasses = ["paste", "request_reference", "file_path", "virtual_uri", "remote_uri", "directory", "archive"];
  for (const reference_class of supportedClasses) {
    for (const support_state of ["supported", "ambient_prohibited"]) {
      assert.equal(
        validateReference({ ...base, reference_class, support_state }),
        true,
        `${reference_class}/${support_state} must pass`,
      );
    }
    assert.equal(
      validateReference({ ...base, reference_class, support_state: "unsupported" }),
      false,
      `${reference_class}/unsupported must fail`,
    );
  }
  assert.equal(
    validateReference({ ...base, reference_class: "unsupported", support_state: "unsupported" }),
    true,
    "unsupported/unsupported must pass",
  );
  assert.equal(
    validateReference({ ...base, reference_class: "unsupported", support_state: "supported" }),
    false,
    "unsupported/supported must fail",
  );
  assert.equal(
    validateReference({ ...base, reference_class: "unsupported", support_state: "ambient_prohibited" }),
    false,
    "unsupported/ambient_prohibited must fail",
  );
});

test("context-disposition terminal states forbid ranges and tokens outside admitting states", () => {
  const validateDisposition = validator("context-disposition");
  const base = {
    schema_version: 1,
    disposition_id: "disposition-1",
    context_manifest_id: "context-1",
    source_artifact_id: "source-1",
    section_id: null,
    reason: null,
    terminal: true,
  };
  const admittedIncluded = {
    ...base,
    disposition: "included",
    ranges: [{ start_byte: 0, end_byte_exclusive: 16 }],
    token_count: 4,
    reason_code: null,
  };
  assert.equal(validateDisposition(admittedIncluded), true, "included admitting payload must pass");
  for (const state of ["summarized", "truncated"]) {
    const admitted = {
      ...base,
      disposition: state,
      ranges: [{ start_byte: 0, end_byte_exclusive: 16 }],
      token_count: 4,
      reason_code: "reason",
    };
    assert.equal(validateDisposition(admitted), true, `${state} admitting payload must pass`);
  }
  for (const state of ["duplicate", "stale", "unsupported", "unavailable", "restricted", "omitted"]) {
    const empty = { ...base, disposition: state, ranges: [], token_count: 0, reason_code: "reason" };
    assert.equal(validateDisposition(empty), true, `${state} empty payload must pass`);
    assert.equal(
      validateDisposition({ ...empty, ranges: [{ start_byte: 0, end_byte_exclusive: 4 }] }),
      false,
      `${state} must not claim admitted ranges`,
    );
    assert.equal(
      validateDisposition({ ...empty, token_count: 1 }),
      false,
      `${state} must not claim admitted tokens`,
    );
  }
});

test("context-disposition and context-manifest impose matching reason_code rules for each disposition", () => {
  const validateDisposition = validator("context-disposition");
  const validateManifest = validator("context-manifest");
  const dispositionBase = {
    schema_version: 1,
    disposition_id: "disposition-1",
    context_manifest_id: "context-1",
    source_artifact_id: "source-1",
    section_id: null,
    reason: null,
    terminal: true,
  };
  const manifestBase = {
    schema_version: CONTEXT_MANIFEST_SCHEMA_VERSION,
    context_manifest_id: "context-1",
    session_id: "session-1",
    turn_id: "turn-1",
    model_profile_id: "model-1",
    source_artifact_count: 1,
    items: [],
    total_input_tokens: 32,
    reserved_output_tokens: 8,
    safety_margin_tokens: 4,
    manifest_sha256: SHA,
  };
  for (const state of ["included", "summarized", "truncated", "duplicate", "stale", "unsupported", "unavailable", "restricted", "omitted"]) {
    const admitting = state === "included" || state === "summarized" || state === "truncated";
    const ranges = admitting ? [{ start_byte: 0, end_byte_exclusive: 16 }] : [];
    const tokens = admitting ? 4 : 0;
    const dispositionWithNull = {
      ...dispositionBase,
      disposition: state,
      ranges,
      token_count: tokens,
      reason_code: null,
    };
    const dispositionWithCode = {
      ...dispositionWithNull,
      reason_code: "code",
    };
    const manifestItem = {
      artifact_id: "source-1",
      disposition: state,
      ranges,
      token_count: tokens,
      reason_code: null,
      reason: null,
    };
    const manifestItemWithCode = { ...manifestItem, reason_code: "code" };
    if (state === "included") {
      assert.equal(validateDisposition(dispositionWithNull), true, `disposition ${state} accepts reason_code=null`);
      assert.equal(validateDisposition(dispositionWithCode), false, `disposition ${state} rejects reason_code string`);
      assert.equal(validateManifest({ ...manifestBase, items: [manifestItem] }), true, `manifest ${state} accepts reason_code=null`);
      assert.equal(validateManifest({ ...manifestBase, items: [manifestItemWithCode] }), false, `manifest ${state} rejects reason_code string`);
    } else {
      assert.equal(validateDisposition(dispositionWithNull), false, `disposition ${state} rejects reason_code=null`);
      assert.equal(validateDisposition(dispositionWithCode), true, `disposition ${state} accepts reason_code string`);
      assert.equal(validateManifest({ ...manifestBase, items: [manifestItem] }), false, `manifest ${state} rejects reason_code=null`);
      assert.equal(validateManifest({ ...manifestBase, items: [manifestItemWithCode] }), true, `manifest ${state} accepts reason_code string`);
    }
  }
});

const RETENTION_TIMESTAMP = "2026-08-25T12:00:00Z";
const PAYLOAD_SHA = "b".repeat(64);
const PROTECTED_SHA = "c".repeat(64);

function memoryOnlyRetention() {
  return {
    schema_version: SOURCE_RETENTION_SCHEMA_VERSION,
    retention_id: "retention-1",
    source_artifact_id: "source-1",
    request_id: "request-1",
    authority_id: "authority-1",
    owner_class: "session",
    owner_id: "session-1",
    retention_class: "memory_only",
    retention_policy_id: null,
    retention_expires_at: null,
    physical_store: SOURCE_RETENTION_PHYSICAL_STORE,
    physical_binding: null,
    encryption_state: "not_persisted",
    protected_metadata_sha256: PROTECTED_SHA,
    lifecycle_state: "active",
    reason_code: null,
    recorded_at: RETENTION_TIMESTAMP,
    source_retention_sha256: SHA,
  };
}

function persistedRetention() {
  return {
    ...memoryOnlyRetention(),
    retention_id: "retention-2",
    retention_class: "policy_persisted",
    retention_policy_id: "policy-1",
    retention_expires_at: "2026-09-25T12:00:00Z",
    physical_binding: {
      artifact_kind: "generated_file",
      artifact_id: "artifact-1",
      payload_sha256: PAYLOAD_SHA,
      byte_length: 4096,
    },
    encryption_state: "encrypted_at_rest",
  };
}

test("source-retention binds schema_version to the authoritative Rust contract version", () => {
  assert.equal(SOURCE_RETENTION_SCHEMA_VERSION, 1);
  const validate = validator("source-retention");
  const record = memoryOnlyRetention();
  assert.equal(validate(record), true, JSON.stringify(validate.errors));
  const { schema_version: _dropped, ...withoutVersion } = record;
  assert.equal(validate(withoutVersion), false, "missing schema_version must fail");
  for (const rejected of [0, SOURCE_RETENTION_SCHEMA_VERSION + 1, 2, 999, "1", null]) {
    assert.equal(
      validate({ ...record, schema_version: rejected }),
      false,
      `version ${JSON.stringify(rejected)} must fail`,
    );
  }
});

test("source-retention rejects missing, extra, malformed, and oversized fields", () => {
  const validate = validator("source-retention");
  const record = memoryOnlyRetention();
  for (const field of Object.keys(record)) {
    const { [field]: _removed, ...missing } = record;
    assert.equal(validate(missing), false, `missing ${field} must fail`);
  }
  assert.equal(
    validate({ ...record, source_uri: "file:///home/user/secret.txt" }),
    false,
    "unknown field must fail",
  );
  assert.equal(validate({ ...record, protected_metadata_sha256: "not-a-digest" }), false);
  assert.equal(validate({ ...record, recorded_at: "2026-13-45" }), false);
  assert.equal(validate({ ...record, owner_class: "workspace" }), false);
  assert.equal(validate({ ...record, retention_class: "forever" }), false);
  assert.equal(validate({ ...record, lifecycle_state: "archived" }), false);
  const persisted = persistedRetention();
  assert.equal(validate(persisted), true, JSON.stringify(validate.errors));
  assert.equal(
    validate({
      ...persisted,
      physical_binding: { ...persisted.physical_binding, byte_length: 104857601 },
    }),
    false,
    "oversized payload must fail",
  );
  assert.equal(
    validate({
      ...persisted,
      physical_binding: { ...persisted.physical_binding, byte_length: -1 },
    }),
    false,
    "negative payload must fail",
  );
});

test("source-retention cannot widen RuntimeArtifactKind or name a second physical store", () => {
  const validate = validator("source-retention");
  const persisted = persistedRetention();
  assert.deepEqual(RUNTIME_ARTIFACT_KINDS, [
    "patch",
    "standard_output",
    "standard_error",
    "test_log",
    "generated_file",
    "report",
    "model_output",
  ]);
  for (const kind of RUNTIME_ARTIFACT_KINDS) {
    assert.equal(
      validate({ ...persisted, physical_binding: { ...persisted.physical_binding, artifact_kind: kind } }),
      true,
      `existing kind ${kind} must be accepted`,
    );
  }
  for (const widened of ["source_payload", "source_artifact", "attachment", "ingested_source", ""]) {
    assert.equal(
      validate({ ...persisted, physical_binding: { ...persisted.physical_binding, artifact_kind: widened } }),
      false,
      `source-specific kind ${widened} must be rejected`,
    );
  }
  for (const store of ["source_store", "ingestion_store", "runtime_artifact_backend_v2", ""]) {
    assert.equal(
      validate({ ...persisted, physical_store: store }),
      false,
      `second physical store ${store} must be rejected`,
    );
  }
  assert.equal(
    validate({
      ...persisted,
      physical_binding: { ...persisted.physical_binding, source_kind: "paste" },
    }),
    false,
    "unknown binding field must fail",
  );
});

test("source-retention forbids persistence contradicting the declared retention class", () => {
  const validate = validator("source-retention");
  const memoryOnly = memoryOnlyRetention();
  const persisted = persistedRetention();
  assert.equal(
    validate({ ...memoryOnly, physical_binding: persisted.physical_binding }),
    false,
    "memory-only must not bind a durable payload",
  );
  assert.equal(
    validate({ ...memoryOnly, encryption_state: "encrypted_at_rest" }),
    false,
    "memory-only must not claim encryption at rest",
  );
  assert.equal(
    validate({ ...memoryOnly, retention_policy_id: "policy-1" }),
    false,
    "memory-only must not carry a retention policy",
  );
  assert.equal(
    validate({ ...memoryOnly, retention_expires_at: RETENTION_TIMESTAMP }),
    false,
    "memory-only must not carry an expiry",
  );
  assert.equal(
    validate({ ...persisted, physical_binding: null }),
    false,
    "persisted retention requires a durable binding",
  );
  assert.equal(
    validate({ ...persisted, retention_policy_id: null }),
    false,
    "persisted retention requires an approving policy",
  );
  assert.equal(
    validate({ ...persisted, retention_expires_at: null }),
    false,
    "persisted retention requires an expiry",
  );
  assert.equal(
    validate({ ...persisted, encryption_state: "not_persisted" }),
    false,
    "persisted retention must be encrypted at rest",
  );
});

test("source-retention requires a deterministic reason_code for every non-active lifecycle state", () => {
  const validate = validator("source-retention");
  const record = memoryOnlyRetention();
  assert.equal(validate({ ...record, lifecycle_state: "active", reason_code: null }), true);
  assert.equal(
    validate({ ...record, lifecycle_state: "active", reason_code: "owner_released" }),
    false,
    "active must not carry a reason_code",
  );
  assert.equal(
    validate({ ...record, lifecycle_state: "quarantined", reason_code: null }),
    false,
    "quarantined requires a reason_code",
  );
  assert.equal(
    validate({ ...record, lifecycle_state: "quarantined", reason_code: "integrity_failed" }),
    true,
  );
});

test("source-retention semantic validation reconciles retention_class with lifecycle_state", () => {
  const validate = combinedValidator("source-retention");
  const structural = validator("source-retention");
  const record = memoryOnlyRetention();
  assert.equal(validate(record), true);
  const releasedShape = {
    ...record,
    retention_class: "released",
    lifecycle_state: "released",
    reason_code: "owner_released",
  };
  assert.equal(validate(releasedShape), true);
  const deletedShape = {
    ...record,
    retention_class: "deleted",
    lifecycle_state: "deleted",
    reason_code: "retention_expired",
  };
  assert.equal(validate(deletedShape), true);
  const contradictions = [
    { retention_class: "released", lifecycle_state: "active", reason_code: null },
    { retention_class: "deleted", lifecycle_state: "quarantined", reason_code: "integrity_failed" },
    { retention_class: "memory_only", lifecycle_state: "deleted", reason_code: "retention_expired" },
    { retention_class: "memory_only", lifecycle_state: "released", reason_code: "owner_released" },
  ];
  for (const contradiction of contradictions) {
    const candidate = { ...record, ...contradiction };
    assert.equal(
      validate(candidate),
      false,
      `${contradiction.retention_class}/${contradiction.lifecycle_state} must fail combined validation`,
    );
    if (structural(candidate)) {
      assert.equal(
        ENGINEERING_RUNTIME_SEMANTIC_VALIDATORS["source-retention"](candidate),
        false,
        "semantic validator must reject what the structural schema admits",
      );
    }
  }
  assert.ok(ENGINEERING_RUNTIME_SEMANTIC_INVARIANTS["source-retention"].includes("retention_class"));
});

test("source-retention carries protected metadata only as a digest", () => {
  const validate = validator("source-retention");
  const record = memoryOnlyRetention();
  for (const leaked of [
    "absolute_path",
    "source_path",
    "original_uri",
    "workspace_path",
    "file_uri",
  ]) {
    assert.equal(
      validate({ ...record, [leaked]: "/home/user/private/report.docx" }),
      false,
      `${leaked} must never be an admitted field`,
    );
  }
  const properties = ENGINEERING_RUNTIME_SCHEMAS["source-retention"].properties;
  for (const name of Object.keys(properties)) {
    assert.ok(
      !/(^|_)(path|uri|url|filename)(_|$)/.test(name),
      `${name} must not expose a path or URI surface`,
    );
  }
});

const LOCATOR_PAYLOAD_VALUES = {
  byte_range: { start_byte: 0, end_byte_exclusive: 128 },
  line_range: { start_line: 0, end_line_exclusive: 12 },
  page_number: 3,
  sheet_name: "Q3 Summary",
  cell_reference: { sheet_row: 4, sheet_column: 7 },
  image_region: { origin_x: 10, origin_y: 20, width: 640, height: 480 },
  section_id: "section-1",
};
const ALL_LOCATOR_PAYLOADS = Object.keys(LOCATOR_PAYLOAD_VALUES);

function locator(kind, availability_state = "complete") {
  const carried = SOURCE_LOCATOR_PAYLOAD_FIELDS[kind];
  const resolved = SOURCE_LOCATOR_RESOLVED_STATES.includes(availability_state);
  const payloads = Object.fromEntries(
    ALL_LOCATOR_PAYLOADS.map((field) => [
      field,
      resolved && carried.includes(field) ? LOCATOR_PAYLOAD_VALUES[field] : null,
    ]),
  );
  return {
    schema_version: SOURCE_LOCATOR_SCHEMA_VERSION,
    locator_id: "locator-1",
    source_artifact_id: "source-1",
    provenance_id: "provenance-1",
    extraction_id: "extraction-1",
    locator_kind: kind,
    availability_state,
    ...payloads,
    reason_code: availability_state === "complete" ? null : "declared_bound_reached",
    observed_at: "2026-08-25T12:00:00Z",
    locator_sha256: SHA,
  };
}

test("source-locator binds schema_version and admits every authoritative coordinate space", () => {
  assert.equal(SOURCE_LOCATOR_SCHEMA_VERSION, 1);
  assert.deepEqual(SOURCE_LOCATOR_KINDS, [
    "byte",
    "line",
    "page",
    "sheet",
    "cell",
    "image_region",
    "section",
  ]);
  const validate = combinedValidator("source-locator");
  for (const kind of SOURCE_LOCATOR_KINDS) {
    assert.equal(validate(locator(kind)), true, `${kind} locator must be admitted`);
  }
  const structural = validator("source-locator");
  const record = locator("byte");
  const { schema_version: _dropped, ...withoutVersion } = record;
  assert.equal(structural(withoutVersion), false);
  for (const rejected of [0, 2, 999, "1", null]) {
    assert.equal(structural({ ...record, schema_version: rejected }), false);
  }
});

test("source-locator carries exactly the payload of its declared kind", () => {
  const validate = combinedValidator("source-locator");
  for (const kind of SOURCE_LOCATOR_KINDS) {
    const carried = SOURCE_LOCATOR_PAYLOAD_FIELDS[kind];
    for (const foreign of ALL_LOCATOR_PAYLOADS.filter((f) => !carried.includes(f))) {
      assert.equal(
        validate({ ...locator(kind), [foreign]: LOCATOR_PAYLOAD_VALUES[foreign] }),
        false,
        `${kind} locator must not carry ${foreign}`,
      );
    }
    for (const required of carried) {
      assert.equal(
        validate({ ...locator(kind), [required]: null }),
        false,
        `${kind} locator must populate ${required}`,
      );
    }
  }
});

test("source-locator forbids naming a position it never resolved", () => {
  const validate = combinedValidator("source-locator");
  assert.deepEqual(SOURCE_LOCATOR_UNRESOLVED_STATES, ["encrypted", "unsupported", "unavailable"]);
  for (const state of SOURCE_LOCATOR_UNRESOLVED_STATES) {
    for (const kind of SOURCE_LOCATOR_KINDS) {
      const record = locator(kind, state);
      assert.equal(validate(record), true, `${kind}/${state} with no payload must be admitted`);
      for (const field of ALL_LOCATOR_PAYLOADS) {
        assert.equal(
          validate({ ...record, [field]: LOCATOR_PAYLOAD_VALUES[field] }),
          false,
          `${state} must not claim ${field}`,
        );
      }
    }
  }
  for (const state of ["partial", "truncated"]) {
    assert.equal(
      validate(locator("byte", state)),
      true,
      `${state} keeps its known position`,
    );
  }
});

test("source-locator requires a deterministic reason_code for every non-complete state", () => {
  const validate = combinedValidator("source-locator");
  assert.equal(validate({ ...locator("byte"), reason_code: "declared_bound_reached" }), false);
  for (const state of ["partial", "truncated", "encrypted", "unsupported", "unavailable"]) {
    assert.equal(
      validate({ ...locator("byte", state), reason_code: null }),
      false,
      `${state} requires a reason_code`,
    );
  }
});

test("source-locator semantic validation rejects reversed ranges and degenerate coordinates", () => {
  const validate = combinedValidator("source-locator");
  const structural = validator("source-locator");
  const degenerate = [
    ["byte", { byte_range: { start_byte: 128, end_byte_exclusive: 0 } }],
    ["line", { line_range: { start_line: 12, end_line_exclusive: 4 } }],
    ["cell", { cell_reference: { sheet_row: 0, sheet_column: 7 } }],
    ["cell", { cell_reference: { sheet_row: 4, sheet_column: 0 } }],
    ["image_region", { image_region: { origin_x: 0, origin_y: 0, width: 0, height: 480 } }],
    ["image_region", { image_region: { origin_x: 0, origin_y: 0, width: 640, height: 0 } }],
  ];
  for (const [kind, override] of degenerate) {
    const candidate = { ...locator(kind), ...override };
    assert.equal(validate(candidate), false, `${kind} ${JSON.stringify(override)} must fail`);
  }
  const reversed = { ...locator("byte"), byte_range: { start_byte: 128, end_byte_exclusive: 0 } };
  assert.equal(structural(reversed), true, "reversed range is structurally well formed");
  assert.equal(
    ENGINEERING_RUNTIME_SEMANTIC_VALIDATORS["source-locator"](reversed),
    false,
    "semantic validation must reject the reversed range",
  );
  assert.ok(ENGINEERING_RUNTIME_SEMANTIC_INVARIANTS["source-locator"].includes("locator_kind"));
});

test("source-locator rejects missing, extra, malformed, and out-of-range fields", () => {
  const validate = validator("source-locator");
  const record = locator("page");
  for (const field of Object.keys(record)) {
    const { [field]: _removed, ...missing } = record;
    assert.equal(validate(missing), false, `missing ${field} must fail`);
  }
  assert.equal(validate({ ...record, source_path: "/home/user/report.pdf" }), false);
  assert.equal(validate({ ...record, locator_kind: "paragraph" }), false);
  assert.equal(validate({ ...record, availability_state: "redacted" }), false);
  assert.equal(validate({ ...record, observed_at: "yesterday" }), false);
  assert.equal(validate({ ...record, locator_sha256: "short" }), false);
  assert.equal(validate({ ...record, page_number: 0 }), false, "pages are one-indexed");
  assert.equal(validate({ ...record, page_number: -1 }), false);
  assert.equal(
    validate({ ...locator("sheet"), sheet_name: "x".repeat(513) }),
    false,
    "oversized sheet name must fail",
  );
  assert.equal(
    validate({ ...locator("byte"), byte_range: { start_byte: 0, end_byte_exclusive: 1, extra: 1 } }),
    false,
    "unknown range field must fail",
  );
});

const POLICY_TIMESTAMP = "2026-08-26T12:00:00Z";
const POLICY_BUDGETS = {
  turns: 4,
  tokens: 32000,
  duration_ms: 60000,
  tool_calls: 8,
  attempts: 1,
  no_progress_events: 2,
  output_bytes: 1048576,
  memory_bytes: 268435456,
  cost_minor_units: 0,
};

function policy(overrides = {}) {
  return {
    schema_version: 1,
    policy_id: "policy-1",
    plan_id: "plan-0001",
    plan_step_id: "plan-0001:step:0002",
    plan_revision: 3,
    preflight_policy_id: "preflight-policy-1",
    required_preflight_ids: ["workspace-trust", "repository-clean"],
    side_effect_policy_id: "side-effect-policy-1",
    effect_class: "read_only",
    approval_policy_id: "approval-policy-1",
    approval_requirement: "not_required",
    idempotency_policy_id: "idempotency-policy-1",
    idempotency_key_requirement: "not_applicable",
    verifier_policy_id: "verifier-policy-1",
    verification_requirement: "verifier_evidence_required",
    required_verifier_ids: ["exit-status"],
    deferral_reason_code: null,
    retry_policy_id: "retry-policy-1",
    retry_class: "never",
    budget_policy_id: "budget-policy-1",
    budgets: { ...POLICY_BUDGETS },
    diagnostic_policy_id: "diagnostic-policy-1",
    diagnostic_disclosure: "content_free_codes",
    recorded_at: POLICY_TIMESTAMP,
    policy_sha256: SHA,
    ...overrides,
  };
}

// Smallest admitted policy for one effect class, so a matrix case fails for the reason
// under test rather than for an unrelated approval or idempotency rule.
function policyFor(effect, overrides = {}) {
  let approval = "not_required";
  if (effect === "destructive") approval = "required_per_attempt";
  else if (STEP_APPROVAL_REQUIRING_EFFECTS.includes(effect)) approval = "required_once";
  const idempotency = effect === "idempotent_write" ? "required" : "not_applicable";
  return policy({
    effect_class: effect,
    approval_requirement: approval,
    idempotency_key_requirement: idempotency,
    ...overrides,
  });
}

test("step-execution-policy binds schema_version and names eight identities keyed to the existing plan step", () => {
  assert.equal(STEP_EXECUTION_POLICY_SCHEMA_VERSION, 1);
  const validate = combinedValidator("step-execution-policy");
  assert.equal(validate(policy()), true);
  const schema = ENGINEERING_RUNTIME_SCHEMAS["step-execution-policy"];
  assert.ok(schema.required.includes("plan_step_id"), "the policy is keyed to the plan step");
  assert.ok(schema.required.includes("plan_revision"), "policy is bound to an exact revision");
  assert.equal(STEP_EXECUTION_POLICY_IDENTITIES.length, 8);
  for (const identity of STEP_EXECUTION_POLICY_IDENTITIES) {
    assert.ok(schema.required.includes(identity), `${identity} must be required`);
    const { [identity]: _dropped, ...missing } = policy();
    assert.equal(validate(missing), false, `${identity} must be mandatory`);
  }
  const structural = validator("step-execution-policy");
  for (const rejected of [0, 2, 999, "1", null]) {
    assert.equal(structural(policy({ schema_version: rejected })), false);
  }
});

test("step-execution-policy is a companion record and restates no plan, tool, grant, receipt, or completion state", () => {
  const structural = validator("step-execution-policy");
  for (const ownedElsewhere of [
    "description",
    "ordinal",
    "depends_on",
    "expected_evidence",
    "state",
    "arguments",
    "tool_arguments",
    "grant_id",
    "capability_grant",
    "receipt_id",
    "exit_status",
    "changed_resources",
    "completed",
    "verified_completion",
  ]) {
    assert.equal(
      structural(policy({ [ownedElsewhere]: "x" })),
      false,
      `${ownedElsewhere} belongs to an existing contract and must not be admitted`,
    );
  }
  const schema = ENGINEERING_RUNTIME_SCHEMAS["step-execution-policy"];
  assert.equal(schema.additionalProperties, false);
  for (const field of Object.keys(schema.properties)) {
    for (const forbidden of ["path", "uri", "url", "command", "secret", "token", "credential"]) {
      assert.ok(!field.includes(forbidden), `${field} must not expose a ${forbidden} surface`);
    }
  }
});

test("step-execution-policy admits exactly the retry classes its effect class permits", () => {
  const validate = combinedValidator("step-execution-policy");
  assert.deepEqual(Object.keys(STEP_EFFECT_RETRY_MATRIX), [...STEP_EFFECT_CLASSES]);
  for (const effect of STEP_EFFECT_CLASSES) {
    const permitted = STEP_EFFECT_RETRY_MATRIX[effect];
    for (const retry of STEP_RETRY_CLASSES) {
      const attempts = retry === "never" ? 1 : 3;
      const candidate = policyFor(effect, {
        retry_class: retry,
        budgets: { ...POLICY_BUDGETS, attempts },
      });
      assert.equal(
        validate(candidate),
        permitted.includes(retry),
        `${effect} with ${retry} must be ${permitted.includes(retry)}`,
      );
    }
  }
});

test("step-execution-policy never automatically retries a destructive, external, non-idempotent, or unclassified effect", () => {
  const validate = combinedValidator("step-execution-policy");
  assert.deepEqual(
    [...STEP_APPROVAL_REQUIRING_EFFECTS],
    ["non_idempotent", "destructive", "external", "unknown"],
  );
  assert.deepEqual(
    [...STEP_AUTOMATIC_RETRY_CLASSES],
    ["recoverable_read", "conditional_after_reconciliation"],
  );
  for (const effect of STEP_APPROVAL_REQUIRING_EFFECTS) {
    for (const retry of STEP_AUTOMATIC_RETRY_CLASSES) {
      assert.equal(
        validate(policyFor(effect, {
          retry_class: retry,
          budgets: { ...POLICY_BUDGETS, attempts: 3 },
        })),
        false,
        `${effect} must never carry automatic ${retry}`,
      );
    }
    assert.equal(
      validate(policyFor(effect, { approval_requirement: "not_required" })),
      false,
      `${effect} must fail toward approval`,
    );
  }
  assert.equal(
    validate(policyFor("destructive", { approval_requirement: "required_once" })),
    false,
    "a destructive step needs a fresh narrow approval for every attempt",
  );
});

test("step-execution-policy retries an idempotent write only against a verified key or desired state", () => {
  const validate = combinedValidator("step-execution-policy");
  assert.equal(
    validate(policyFor("idempotent_write", { idempotency_key_requirement: "not_applicable" })),
    false,
  );
  for (const requirement of ["required", "verified_desired_state"]) {
    assert.equal(
      validate(policyFor("idempotent_write", { idempotency_key_requirement: requirement })),
      true,
      `${requirement} must admit an idempotent write`,
    );
    assert.equal(
      validate(policyFor("read_only", { idempotency_key_requirement: requirement })),
      false,
      "a read changes nothing and cannot claim an idempotency key",
    );
  }
});

test("step-execution-policy resolves completion to verifier evidence or exactly one recorded deferral", () => {
  const validate = combinedValidator("step-execution-policy");
  assert.equal(validate(policy({ required_verifier_ids: [] })), false, "evidence needs a verifier");
  assert.equal(
    validate(policy({ deferral_reason_code: "deferred_to_release_gate" })),
    false,
    "a verified step must not also record a deferral",
  );
  const deferred = policy({
    verification_requirement: "policy_deferred",
    required_verifier_ids: [],
    deferral_reason_code: "deferred_to_release_gate",
  });
  assert.equal(validate(deferred), true);
  assert.equal(
    validate({ ...deferred, deferral_reason_code: null }),
    false,
    "a deferral must record its reason",
  );
  assert.equal(
    validate({ ...deferred, required_verifier_ids: ["exit-status"] }),
    false,
    "a deferred step must not also claim verifier evidence",
  );
});

test("step-execution-policy gives an unretried step exactly one attempt", () => {
  const validate = combinedValidator("step-execution-policy");
  assert.equal(validate(policy({ budgets: { ...POLICY_BUDGETS, attempts: 2 } })), false);
  const retried = policyFor("read_only", {
    retry_class: "recoverable_read",
    budgets: { ...POLICY_BUDGETS, attempts: 3 },
  });
  assert.equal(validate(retried), true);
  assert.equal(
    validate({ ...retried, budgets: { ...POLICY_BUDGETS, attempts: 0 } }),
    false,
    "every step gets at least one attempt",
  );
});

test("step-execution-policy rejects missing, malformed, oversized, and duplicated identities", () => {
  const structural = validator("step-execution-policy");
  const validate = combinedValidator("step-execution-policy");
  const record = policy();
  for (const field of Object.keys(record)) {
    const { [field]: _removed, ...missing } = record;
    assert.equal(structural(missing), false, `missing ${field} must fail`);
  }
  assert.equal(structural(policy({ effect_class: "maybe_safe" })), false);
  assert.equal(structural(policy({ retry_class: "always" })), false);
  assert.equal(structural(policy({ approval_requirement: "best_effort" })), false);
  assert.equal(structural(policy({ verification_requirement: "model_asserted" })), false);
  assert.equal(structural(policy({ diagnostic_disclosure: "model_prose" })), false);
  assert.equal(structural(policy({ recorded_at: "yesterday" })), false);
  assert.equal(structural(policy({ policy_sha256: "short" })), false);
  assert.equal(structural(policy({ plan_revision: -1 })), false);
  assert.equal(
    structural(policy({ required_preflight_ids: [] })),
    false,
    "every step declares at least one deterministic preflight",
  );
  assert.equal(
    structural(policy({ required_preflight_ids: ["x".repeat(129)] })),
    false,
    "oversized identity must fail",
  );
  assert.equal(structural(policy({ budgets: { ...POLICY_BUDGETS, extra: 1 } })), false);
  for (const field of ["required_preflight_ids", "required_verifier_ids"]) {
    const repeated = field === "required_preflight_ids"
      ? policy({ required_preflight_ids: ["workspace-trust", "workspace-trust"] })
      : policy({ required_verifier_ids: ["exit-status", "exit-status"] });
    assert.equal(structural(repeated), true, `a repeated ${field} is structurally well formed`);
    assert.equal(validate(repeated), false, `a repeated ${field} must fail semantic validation`);
  }
  assert.ok(ENGINEERING_RUNTIME_SEMANTIC_INVARIANTS["step-execution-policy"].includes("effect_class"));
});
