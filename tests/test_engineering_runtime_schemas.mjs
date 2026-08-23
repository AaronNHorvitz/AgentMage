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
  assert.equal(synchronize(), 14);
  assert.equal(Object.keys(REUSED_SCHEMA_CONTRACTS).length, 6);
  assert.equal(Object.keys(ENGINEERING_RUNTIME_SCHEMAS).length, 14);
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
