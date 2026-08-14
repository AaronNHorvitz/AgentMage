import assert from "node:assert/strict";
import test from "node:test";

import Ajv2020 from "ajv/dist/2020.js";

import {
  MODEL_SCHEMAS,
  modelSchemaDocument,
  synchronize,
} from "../scripts/model_contract_schemas.mjs";

const SHA = "a".repeat(64);
const schemaReference = (id) => ({
  schema_id: id,
  schema_version: 1,
  schema_sha256: SHA,
});
const payload = () => ({
  schema: schemaReference("fixture.payload"),
  media_type: "application/json",
  bytes: [123, 125],
  sha256: SHA,
});
const capability = () => ({
  role: "dialogue",
  state: "not_evaluated",
  evaluation_profile: null,
  result_sha256: null,
  limitations: ["fixture-only"],
});
const profile = () => ({
  schema_version: 2,
  profile_id: "fixture-profile",
  manifest_id: "fixture-manifest",
  manifest_sha256: SHA,
  display_name: "Fixture profile",
  family: "fixture",
  publisher_control: "fixture",
  lineage: ["fixture-source"],
  license_spdx: "Apache-2.0",
  license_terms_sha256: SHA,
  artifact: {
    artifact_id: "fixture-artifact",
    publisher: "fixture",
    source_revision: "fixture-revision",
    format: "fixture",
    bytes: 1,
    sha256: SHA,
  },
  transformations: [],
  codec: {
    codec_id: "fixture-codec",
    codec_version: "1",
    codec_sha256: SHA,
    tokenizer: "fixture-tokenizer",
    tokenizer_sha256: SHA,
    template: "fixture-template",
    template_sha256: SHA,
    tool_protocol_version: "1",
    end_tokens: [1],
    reasoning_enabled: false,
  },
  runtime: {
    adapter_id: "fixture-adapter",
    kind: "deterministic_fake",
    contract_version: 1,
    runtime_build: "fixture-runtime",
    runtime_sha256: SHA,
    platform: "deterministic_fake",
    architecture: "x86_64",
  },
  quantization: "fixture",
  modalities: ["text"],
  context: {
    max_context_tokens: 8192,
    max_input_bytes: 32768,
    max_messages: 32,
    token_counter: "fixture-counter",
    token_counter_sha256: SHA,
  },
  decoding: {
    profile_id: "fixture-decoding",
    sampler_order: ["temperature"],
    temperature: 0,
    top_p: 1,
    top_k: 1,
    repeat_penalty: 1,
    seed: 1,
    max_output_tokens: 256,
  },
  hardware: [{
    platform: "deterministic_fake",
    architecture: "x86_64",
    minimum_system_memory_bytes: 1,
    minimum_accelerator_memory_bytes: 0,
    accelerator: "none",
    driver_constraint: "none",
  }],
  capabilities: [capability()],
  policy_sha256: SHA,
  lifecycle: "candidate",
  enabled: false,
  automatic_fallback: false,
});
const proposal = (kind = "completion_candidate") => ({
  schema_version: 2,
  proposal_id: "proposal-1",
  model_run_id: "run-1",
  context_packet_id: "context-1",
  profile_id: "fixture-profile",
  codec_id: "fixture-codec",
  correlation_id: "correlation-1",
  kind,
  payload: null,
  tool_call: kind === "tool_call" ? {
    tool_call_id: "call-1",
    tool_id: "fixture.read",
    tool_version: "1",
    arguments: payload(),
  } : null,
  proposal_sha256: SHA,
});
const resources = () => ({
  adapter_id: "fixture-adapter",
  profile_id: "fixture-profile",
  model_run_id: "run-1",
  resident_memory_bytes: 1,
  accelerator_memory_bytes: 0,
  input_tokens: 1,
  output_tokens: 1,
  elapsed_ms: 1,
});

function validator(name) {
  return new Ajv2020({ allErrors: true, strict: true }).compile(modelSchemaDocument(name));
}

test("generated model schema files are current and every schema compiles", () => {
  synchronize();
  assert.equal(Object.keys(MODEL_SCHEMAS).length, 14);
});

test("exact profile is closed, hash-bound, bounded, and has no fallback state", () => {
  const validate = validator("exact-profile");
  assert.equal(validate(profile()), true, JSON.stringify(validate.errors));
  for (const mutate of [
    (item) => { item.unknown = true; },
    (item) => { delete item.codec; },
    (item) => { item.manifest_sha256 = "A".repeat(64); },
    (item) => { item.automatic_fallback = true; },
    (item) => { item.runtime.kind = "cloud"; },
    (item) => { item.decoding.top_p = 2; },
  ]) {
    const changed = profile();
    mutate(changed);
    assert.equal(validate(changed), false);
  }
});

test("proposal schema requires exact nullable fields and kind-consistent tool calls", () => {
  const validate = validator("closed-proposal");
  assert.equal(validate(proposal()), true, JSON.stringify(validate.errors));
  assert.equal(validate(proposal("tool_call")), true, JSON.stringify(validate.errors));
  const missing = proposal();
  delete missing.tool_call;
  assert.equal(validate(missing), false);
  const authority = proposal();
  authority.grant = true;
  assert.equal(validate(authority), false);
  const mismatched = proposal("tool_call");
  mismatched.tool_call = null;
  assert.equal(validate(mismatched), false);
  const unexpected = proposal();
  unexpected.tool_call = proposal("tool_call").tool_call;
  assert.equal(validate(unexpected), false);
});

test("terminal claim cannot conflate success, failure, or absent evidence", () => {
  const validate = validator("terminal-claim");
  const success = {
    schema_version: 2,
    model_run_id: "run-1",
    stream_id: "stream-1",
    correlation_id: "correlation-1",
    terminal_state: "proposed",
    fragment_count: 1,
    response_sha256: SHA,
    proposal: proposal(),
    failure: null,
    resources: resources(),
  };
  assert.equal(validate(success), true, JSON.stringify(validate.errors));
  const falseCompletion = structuredClone(success);
  falseCompletion.proposal = null;
  assert.equal(validate(falseCompletion), false);
  const conflated = structuredClone(success);
  conflated.failure = {
    code: "fixture.failed",
    retryable_after_correction: false,
    dependency_recovery_required: false,
    contract_error: null,
  };
  assert.equal(validate(conflated), false);
  const failed = structuredClone(success);
  failed.terminal_state = "failed";
  failed.proposal = null;
  failed.failure = conflated.failure;
  assert.equal(validate(failed), true, JSON.stringify(validate.errors));
});

test("all client schema references are explicit and hash-bound", () => {
  const validate = validator("client-schemas");
  const names = [
    "model_profile", "message", "capability", "context_packet", "run_request",
    "stream_fragment", "proposal", "tool_call", "tool_result", "run_result",
    "terminal_claim", "correlation",
  ];
  const value = Object.fromEntries(names.map((name) => [name, schemaReference(`model.${name}`)]));
  assert.equal(validate(value), true, JSON.stringify(validate.errors));
  delete value.correlation;
  assert.equal(validate(value), false);
});
