#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import Ajv2020 from "ajv/dist/2020.js";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const OUT = path.join(ROOT, "schemas/model");
const BASE = "https://agentmage.dev/schemas/model";

const identifier = { type: "string", pattern: "^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$" };
const bounded = { type: "string", minLength: 1, maxLength: 256 };
const digest = { type: "string", pattern: "^[0-9a-f]{64}$" };
const uint = { type: "integer", minimum: 0 };
const positive = { type: "integer", minimum: 1 };
const nullable = (schema) => ({ oneOf: [schema, { type: "null" }] });
const array = (items, minItems = 0, maxItems = 64) => ({
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

const schemaReference = closed({
  schema_id: identifier,
  schema_version: positive,
  schema_sha256: digest,
});
const payload = closed({
  schema: schemaReference,
  media_type: bounded,
  bytes: array({ type: "integer", minimum: 0, maximum: 255 }, 0, 1048576),
  sha256: digest,
});
const runtime = closed({
  adapter_id: identifier,
  kind: { enum: ["deterministic_fake", "native_llama_cpp", "docker_model_runner", "macos_metal_llama_cpp"] },
  contract_version: positive,
  runtime_build: bounded,
  runtime_sha256: digest,
  platform: { enum: ["deterministic_fake", "fedora", "ubuntu", "mac_os_apple_silicon"] },
  architecture: { enum: ["x86_64", "aarch64"] },
});
const capability = closed({
  role: { enum: ["dialogue", "coding_planner", "tool_selection", "safety_classification", "embedding", "reranking", "multimodal"] },
  state: { enum: ["not_evaluated", "blocked", "failed", "passed", "not_applicable"] },
  evaluation_profile: nullable(bounded),
  result_sha256: nullable(digest),
  limitations: array(bounded),
});
const modelProfile = closed({
  schema_version: positive,
  profile_id: identifier,
  manifest_id: identifier,
  manifest_sha256: digest,
  display_name: bounded,
  family: bounded,
  publisher_control: bounded,
  lineage: array(bounded, 1, 32),
  license_spdx: bounded,
  license_terms_sha256: digest,
  artifact: closed({
    artifact_id: bounded,
    publisher: bounded,
    source_revision: bounded,
    format: bounded,
    bytes: positive,
    sha256: digest,
  }),
  transformations: array(closed({
    transformation_id: bounded,
    tool: bounded,
    arguments_sha256: digest,
    input_sha256: digest,
    output_sha256: digest,
    reproducible: { type: "boolean" },
  }), 0, 32),
  codec: closed({
    codec_id: identifier,
    codec_version: bounded,
    codec_sha256: digest,
    tokenizer: bounded,
    tokenizer_sha256: digest,
    template: bounded,
    template_sha256: digest,
    tool_protocol_version: bounded,
    end_tokens: array(uint, 1, 32),
    reasoning_enabled: { type: "boolean" },
  }),
  runtime,
  quantization: bounded,
  modalities: array({ enum: ["text", "image", "audio", "embedding"] }, 1, 4),
  context: closed({
    max_context_tokens: positive,
    max_input_bytes: positive,
    max_messages: positive,
    token_counter: bounded,
    token_counter_sha256: digest,
  }),
  decoding: closed({
    profile_id: bounded,
    sampler_order: array(bounded, 1, 16),
    temperature: { type: "number", minimum: 0 },
    top_p: { type: "number", minimum: 0, maximum: 1 },
    top_k: uint,
    repeat_penalty: { type: "number", exclusiveMinimum: 0 },
    seed: uint,
    max_output_tokens: positive,
  }),
  hardware: array(closed({
    platform: { enum: ["deterministic_fake", "fedora", "ubuntu", "mac_os_apple_silicon"] },
    architecture: { enum: ["x86_64", "aarch64"] },
    minimum_system_memory_bytes: positive,
    minimum_accelerator_memory_bytes: uint,
    accelerator: bounded,
    driver_constraint: bounded,
  }), 1, 16),
  capabilities: array(capability, 1, 7),
  policy_sha256: digest,
  lifecycle: { enum: ["candidate", "blocked", "rejected", "admitted", "quarantined", "disabled"] },
  enabled: { type: "boolean" },
  automatic_fallback: { const: false },
});
const message = closed({
  message_id: identifier,
  role: { enum: ["system", "user", "assistant", "tool"] },
  content: payload,
});
const contextPacket = closed({
  schema_version: positive,
  context_packet_id: identifier,
  session_id: identifier,
  task_id: identifier,
  profile_id: identifier,
  manifest_sha256: digest,
  tool_catalog_id: identifier,
  messages: array(message, 1, 4096),
  input_bytes: uint,
  input_tokens: uint,
  packet_sha256: digest,
});
const runRequest = closed({
  schema_version: positive,
  model_run_id: identifier,
  correlation_id: identifier,
  context_packet_id: identifier,
  profile_id: identifier,
  manifest_sha256: digest,
  adapter_id: identifier,
  decoding_profile_id: bounded,
  max_output_tokens: positive,
  timeout_ms: positive,
});
const streamFragment = closed({
  schema_version: positive,
  stream_id: identifier,
  model_run_id: identifier,
  correlation_id: identifier,
  sequence: uint,
  bytes: array({ type: "integer", minimum: 0, maximum: 255 }, 1, 33554432),
  sha256: digest,
  terminal: { type: "boolean" },
});
const toolCall = closed({
  tool_call_id: identifier,
  tool_id: identifier,
  tool_version: bounded,
  arguments: payload,
});
const proposal = closed({
  schema_version: positive,
  proposal_id: identifier,
  model_run_id: identifier,
  context_packet_id: identifier,
  profile_id: identifier,
  codec_id: identifier,
  correlation_id: identifier,
  kind: { enum: ["text", "evidence_request", "tool_call", "user_question", "blocked", "completion_candidate"] },
  payload: nullable(payload),
  tool_call: nullable(toolCall),
  proposal_sha256: digest,
});
proposal.allOf = [
  {
    if: { properties: { kind: { const: "tool_call" } }, required: ["kind"] },
    then: { properties: { tool_call: toolCall } },
    else: { properties: { tool_call: { type: "null" } } },
  },
];
const contractError = closed({
  schema_version: positive,
  error_id: identifier,
  code: bounded,
  category: { enum: ["validation", "policy", "dependency", "resource", "cancellation", "timeout", "uncertain", "internal"] },
  message: { type: "string", maxLength: 1024 },
  field_path: array({ type: "string", maxLength: 128 }, 0, 64),
  retry: { enum: ["never", "after_correction", "after_dependency_recovery", "after_user_decision"] },
  caused_by: nullable(identifier),
});
const runtimeFailure = closed({
  code: bounded,
  retryable_after_correction: { type: "boolean" },
  dependency_recovery_required: { type: "boolean" },
  contract_error: nullable(contractError),
});
const resourceReport = closed({
  adapter_id: identifier,
  profile_id: identifier,
  model_run_id: nullable(identifier),
  resident_memory_bytes: uint,
  accelerator_memory_bytes: uint,
  input_tokens: uint,
  output_tokens: uint,
  elapsed_ms: uint,
});
const runResult = closed({
  schema_version: positive,
  model_run_id: identifier,
  stream_id: identifier,
  correlation_id: identifier,
  terminal_state: { enum: ["proposed", "advisory_text", "cancelled", "timed_out", "resource_exhausted", "failed", "rejected"] },
  fragment_count: positive,
  response_sha256: digest,
  proposal: nullable(proposal),
  failure: nullable(runtimeFailure),
  resources: resourceReport,
});
runResult.allOf = [
  {
    if: { properties: { terminal_state: { const: "proposed" } }, required: ["terminal_state"] },
    then: {
      properties: { proposal, failure: { type: "null" } },
    },
    else: {
      properties: { proposal: { type: "null" } },
    },
  },
  {
    if: {
      properties: { terminal_state: { enum: ["failed", "rejected", "resource_exhausted"] } },
      required: ["terminal_state"],
    },
    then: { properties: { failure: runtimeFailure } },
  },
];
const toolResult = closed({
  schema_version: positive,
  tool_call_id: identifier,
  correlation_id: identifier,
  outcome: { enum: ["succeeded", "denied", "failed", "cancelled", "timed_out", "uncertain"] },
  output: nullable(payload),
  validation_issues: array(closed({
    code: bounded,
    severity: { enum: ["error", "warning"] },
    field_path: array({ type: "string", maxLength: 128 }),
    message: { type: "string", maxLength: 1024 },
  })),
  evidence: array(closed({
    evidence_id: identifier,
    evidence_kind: bounded,
    sha256: digest,
    source_component: identifier,
  })),
  error: nullable(contractError),
  elapsed_ms: uint,
  state_change: { enum: ["not_changed", "changed", "uncertain"] },
});
const clientSchemas = closed(Object.fromEntries([
  "model_profile", "message", "capability", "context_packet", "run_request",
  "stream_fragment", "proposal", "tool_call", "tool_result", "run_result",
  "terminal_claim", "correlation",
].map((name) => [name, schemaReference])));

export const MODEL_SCHEMAS = Object.freeze({
  "exact-profile": modelProfile,
  capability,
  message,
  "context-packet": contextPacket,
  "run-request": runRequest,
  "stream-fragment": streamFragment,
  "closed-proposal": proposal,
  "tool-call-candidate": toolCall,
  "tool-result": toolResult,
  "run-result": runResult,
  "terminal-claim": runResult,
  correlation: identifier,
  "client-schemas": clientSchemas,
});

export function modelSchemaDocument(name, schema = MODEL_SCHEMAS[name]) {
  if (schema === undefined) {
    throw new Error(`unknown model schema: ${name}`);
  }
  return {
    $schema: "https://json-schema.org/draft/2020-12/schema",
    $id: `${BASE}/${name}.schema.json`,
    title: `AgentMage ${name}`,
    ...schema,
  };
}

export function renderedSchemas() {
  return Object.fromEntries(Object.entries(MODEL_SCHEMAS).map(([name, schema]) => [
    `${name}.schema.json`,
    `${JSON.stringify(modelSchemaDocument(name, schema), null, 2)}\n`,
  ]));
}

export function validateGeneratedSchemas() {
  const ajv = new Ajv2020({ allErrors: true, strict: true });
  for (const [name, schema] of Object.entries(MODEL_SCHEMAS)) {
    ajv.compile(modelSchemaDocument(name, schema));
  }
}

export function synchronize({ write = false } = {}) {
  validateGeneratedSchemas();
  const mismatches = [];
  for (const [name, expected] of Object.entries(renderedSchemas())) {
    const target = path.join(OUT, name);
    if (write) {
      fs.mkdirSync(OUT, { recursive: true });
      fs.writeFileSync(target, expected);
    } else if (!fs.existsSync(target) || fs.readFileSync(target, "utf8") !== expected) {
      mismatches.push(path.relative(ROOT, target));
    }
  }
  if (mismatches.length > 0) {
    throw new Error(`model schemas differ from generator: ${mismatches.join(", ")}`);
  }
}

if (path.resolve(process.argv[1] ?? "") === fileURLToPath(import.meta.url)) {
  synchronize({ write: process.argv.includes("--write") });
  process.stdout.write(`model contract schemas: ${process.argv.includes("--write") ? "written" : "valid"}\n`);
}
