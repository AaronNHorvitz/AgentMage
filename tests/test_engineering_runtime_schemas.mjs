import assert from "node:assert/strict";
import test from "node:test";

import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

import {
  CONTEXT_MANIFEST_SCHEMA_VERSION,
  ENGINEERING_RUNTIME_SCHEMAS,
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
    reason_code: "included",
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
      items: [{ artifact_id: `artifact-${state}`, disposition: state, ranges: [], token_count: 0, reason: null }],
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
        reason: null,
      }],
    };
    assert.equal(validateManifest(admitting), true, `${state} admitting item must pass`);
  }
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
    reason_code: "included",
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

test("context-disposition terminal states forbid ranges and tokens outside admitting states", () => {
  const validateDisposition = validator("context-disposition");
  const base = {
    schema_version: 1,
    disposition_id: "disposition-1",
    context_manifest_id: "context-1",
    source_artifact_id: "source-1",
    section_id: null,
    reason_code: "reason",
    reason: null,
    terminal: true,
  };
  for (const state of ["included", "summarized", "truncated"]) {
    const admitted = {
      ...base,
      disposition: state,
      ranges: [{ start_byte: 0, end_byte_exclusive: 16 }],
      token_count: 4,
    };
    assert.equal(validateDisposition(admitted), true, `${state} admitting payload must pass`);
  }
  for (const state of ["duplicate", "stale", "unsupported", "unavailable", "restricted", "omitted"]) {
    const empty = { ...base, disposition: state, ranges: [], token_count: 0 };
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
