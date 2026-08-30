#!/usr/bin/env node

import assert from "node:assert/strict";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

import { schemaDocument } from "./engineering_runtime_schemas.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const OUT = path.join(ROOT, "fixtures/engineering-runtime/v2");
const SHA = "a".repeat(64);
const RECORD_SCHEMA_VERSION = 2;

export const RECORD_SCHEMAS = Object.freeze([
  "artifact-envelope",
  "artifact-transformation",
  "artifact-ingestion-result",
  "context-manifest",
  "workflow-definition",
  "workflow-state",
  "tool-observation",
  "verification-result",
  "terminal-result",
]);

const budgets = {
  turns: 1,
  tokens: 1,
  duration_ms: 1,
  tool_calls: 1,
  attempts: 1,
  no_progress_events: 1,
  output_bytes: 1,
  memory_bytes: 1,
  cost_minor_units: 0,
};

export function validRecords() {
  return {
    "artifact-envelope": {
      schema_version: RECORD_SCHEMA_VERSION,
      artifact_id: "artifact:1",
      request_id: "request:1",
      authority_id: "authority:1",
      origin: "paste",
      media_type: "text/plain",
      classification: "internal",
      capture_state: "captured",
      byte_length: 4,
      sha256: SHA,
      collected_at: "2026-08-29T12:00:00Z",
    },
    "artifact-transformation": {
      schema_version: RECORD_SCHEMA_VERSION,
      transformation_id: "transformation:1",
      artifact_id: "artifact:1",
      transformer_id: "parser:1",
      transformer_version: "1",
      input_sha256: SHA,
      output_sha256: SHA,
      source_ranges: [{ start_byte: 0, end_byte_exclusive: 4 }],
      warnings: [],
      reproducible: true,
    },
    "artifact-ingestion-result": {
      schema_version: RECORD_SCHEMA_VERSION,
      ingestion_id: "ingestion:1",
      artifact_id: "artifact:1",
      disposition: "parsed",
      source: { artifact_id: "artifact:1", sha256: SHA, byte_length: 4 },
      transformation_ids: ["transformation:1"],
      warnings: [],
      error_code: null,
      terminal: true,
    },
    "context-manifest": {
      schema_version: RECORD_SCHEMA_VERSION,
      context_manifest_id: "context:1",
      session_id: "session:1",
      turn_id: "turn:1",
      model_profile_id: "model:1",
      source_artifact_count: 1,
      items: [
        {
          artifact_id: "artifact:1",
          disposition: "included",
          ranges: [{ start_byte: 0, end_byte_exclusive: 4 }],
          token_count: 1,
          reason_code: null,
          reason: null,
        },
      ],
      total_input_tokens: 1,
      reserved_output_tokens: 1,
      safety_margin_tokens: 1,
      manifest_sha256: SHA,
    },
    "workflow-definition": {
      schema_version: RECORD_SCHEMA_VERSION,
      workflow_id: "workflow:1",
      workflow_version: 1,
      input_schema: {
        schema_id: "schema:input",
        schema_version: 1,
        schema_sha256: SHA,
      },
      output_schema: {
        schema_id: "schema:output",
        schema_version: 1,
        schema_sha256: SHA,
      },
      steps: [
        {
          step_id: "step:1",
          depends_on: [],
          model_role: null,
          tool_id: "tool:1",
          effect_class: "read_only",
          retry_class: "recoverable_read",
          verifier_ids: ["verifier:1"],
          budgets,
        },
      ],
      entry_step_ids: ["step:1"],
      definition_sha256: SHA,
    },
    "workflow-state": {
      schema_version: RECORD_SCHEMA_VERSION,
      workflow_id: "workflow:1",
      workflow_version: 1,
      sequence: 5,
      state: "succeeded",
      active_step_id: null,
      completed_step_ids: ["step:1"],
      attempt_ids: ["attempt:1"],
      consumed_budget_sha256: SHA,
      terminal_result_id: "terminal:1",
    },
    "tool-observation": {
      schema_version: RECORD_SCHEMA_VERSION,
      observation_id: "observation:1",
      tool_call_id: "tool-call:1",
      attempt_id: "attempt:1",
      task_id: "task:1",
      step_id: "step:1",
      tool_id: "tool:1",
      tool_version: "1",
      tool_schema_sha256: SHA,
      arguments_sha256: SHA,
      authority_id: "authority:1",
      started_at: "2026-08-29T12:00:00Z",
      completed_at: "2026-08-29T12:00:01Z",
      outcome: "succeeded",
      exit_code: 0,
      signal: null,
      stdout: null,
      stderr: null,
      stdout_excerpt: "",
      stderr_excerpt: "",
      stdout_truncated: false,
      stderr_truncated: false,
      generated_artifact_ids: [],
      state_change: "not_changed",
      descendants_cleaned: true,
      resource_usage_sha256: SHA,
      retry_disposition: "not_eligible",
      receipt_sha256: SHA,
      terminal: true,
    },
    "verification-result": {
      schema_version: RECORD_SCHEMA_VERSION,
      verification_result_id: "verification:1",
      workflow_id: "workflow:1",
      step_id: "step:1",
      verifier_id: "verifier:1",
      verifier_version: "1",
      subject_sha256: SHA,
      observed_evidence_sha256s: [SHA],
      preserved_invariants: ["invariant:1"],
      prohibited_effects_observed: [],
      outcome: "passed",
      current: true,
      result_sha256: SHA,
    },
    "terminal-result": {
      schema_version: RECORD_SCHEMA_VERSION,
      terminal_result_id: "terminal:1",
      workflow_id: "workflow:1",
      outcome: "verified_success",
      verification_result_ids: ["verification:1"],
      last_verified_state_sha256: SHA,
      diagnostic_code: null,
      safe_next_action: null,
      established_by: "agentmage-runtime-verifier",
      result_sha256: SHA,
    },
  };
}

function clone(value) {
  return structuredClone(value);
}

function without(record, field) {
  const copy = clone(record);
  delete copy[field];
  return copy;
}

function invalidCases(valid) {
  const required = {
    "artifact-envelope": "artifact_id",
    "artifact-transformation": "transformation_id",
    "artifact-ingestion-result": "ingestion_id",
    "context-manifest": "context_manifest_id",
    "workflow-definition": "workflow_id",
    "workflow-state": "workflow_id",
    "tool-observation": "observation_id",
    "verification-result": "verification_result_id",
    "terminal-result": "terminal_result_id",
  };
  const malformed = {
    "artifact-envelope": {
      ...clone(valid["artifact-envelope"]),
      sha256: "not-a-digest",
    },
    "artifact-transformation": {
      ...clone(valid["artifact-transformation"]),
      reproducible: "yes",
    },
    "artifact-ingestion-result": {
      ...clone(valid["artifact-ingestion-result"]),
      terminal: false,
    },
    "context-manifest": {
      ...clone(valid["context-manifest"]),
      manifest_sha256: "short",
    },
    "workflow-definition": {
      ...clone(valid["workflow-definition"]),
      workflow_version: 0,
    },
    "workflow-state": { ...clone(valid["workflow-state"]), sequence: -1 },
    "tool-observation": {
      ...clone(valid["tool-observation"]),
      started_at: "not-a-time",
    },
    "verification-result": {
      ...clone(valid["verification-result"]),
      result_sha256: "short",
    },
    "terminal-result": {
      ...clone(valid["terminal-result"]),
      established_by: "untrusted-client",
    },
  };
  const oversized = {
    "artifact-envelope": {
      ...clone(valid["artifact-envelope"]),
      media_type: "x".repeat(513),
    },
    "artifact-transformation": {
      ...clone(valid["artifact-transformation"]),
      warnings: Array(257).fill("warning"),
    },
    "artifact-ingestion-result": {
      ...clone(valid["artifact-ingestion-result"]),
      warnings: Array(257).fill("warning"),
    },
    "context-manifest": {
      ...clone(valid["context-manifest"]),
      context_manifest_id: `c${"x".repeat(128)}`,
    },
    "workflow-definition": {
      ...clone(valid["workflow-definition"]),
      workflow_id: `w${"x".repeat(128)}`,
    },
    "workflow-state": {
      ...clone(valid["workflow-state"]),
      attempt_ids: Array.from({ length: 257 }, (_, i) => `attempt:${i}`),
    },
    "tool-observation": {
      ...clone(valid["tool-observation"]),
      stdout_excerpt: "x".repeat(8193),
    },
    "verification-result": {
      ...clone(valid["verification-result"]),
      observed_evidence_sha256s: Array(257).fill(SHA),
    },
    "terminal-result": {
      ...clone(valid["terminal-result"]),
      outcome: "blocked",
      verification_result_ids: [],
      diagnostic_code: "blocked",
      safe_next_action: "x".repeat(513),
    },
  };
  const cases = [];
  for (const schemaName of RECORD_SCHEMAS) {
    cases.push(
      {
        schemaName,
        category: "missing",
        record: without(valid[schemaName], required[schemaName]),
      },
      {
        schemaName,
        category: "extra",
        record: { ...clone(valid[schemaName]), unexpected: true },
      },
      { schemaName, category: "malformed", record: malformed[schemaName] },
      { schemaName, category: "oversized", record: oversized[schemaName] },
      {
        schemaName,
        category: "unsupported-version",
        record: { ...clone(valid[schemaName]), schema_version: 1 },
      },
    );
  }

  const cyclic = clone(valid["workflow-definition"]);
  cyclic.steps[0].depends_on = ["step:1"];
  cases.push({
    schemaName: "workflow-definition",
    category: "cyclic",
    record: cyclic,
    expectedBoundary: "reject-semantic",
    expectedCode: "engineering.workflow.dependency_cycle",
  });

  cases.push({
    schemaName: "artifact-envelope",
    category: "stale",
    record: {
      ...clone(valid["artifact-envelope"]),
      request_id: "request:stale",
    },
    expectedBoundary: "admit-individual-reject-record-set",
    expectedCode: "engineering.record.binding_mismatch",
  });
  return cases;
}

function jsonBytes(value) {
  return Buffer.from(`${JSON.stringify(value, null, 2)}\n`, "utf8");
}

function digest(bytes) {
  return crypto.createHash("sha256").update(bytes).digest("hex");
}

export function expectedFiles() {
  const valid = validRecords();
  const files = new Map();
  const cases = [];
  for (const schemaName of RECORD_SCHEMAS) {
    const relative = `valid/${schemaName}.json`;
    const bytes = jsonBytes(valid[schemaName]);
    files.set(relative, bytes);
    cases.push({
      case_id: `${schemaName}:valid`,
      schema_name: schemaName,
      category: "valid",
      path: relative,
      expected_boundary: "admit",
      expected_code: null,
      sha256: digest(bytes),
    });
  }
  for (const item of invalidCases(valid)) {
    const relative = `invalid/${item.category}/${item.schemaName}.json`;
    const bytes = jsonBytes(item.record);
    files.set(relative, bytes);
    cases.push({
      case_id: `${item.schemaName}:${item.category}`,
      schema_name: item.schemaName,
      category: item.category,
      path: relative,
      expected_boundary: item.expectedBoundary ?? "reject-schema",
      expected_code: item.expectedCode ?? "contract.validation.failed",
      sha256: digest(bytes),
    });
  }
  const manifest = {
    schema_version: 1,
    corpus_id: "engineering-runtime-canonical-records-v2",
    synthetic: true,
    contains_real_user_data: false,
    canonical_record_schema_version: RECORD_SCHEMA_VERSION,
    schemas: RECORD_SCHEMAS,
    required_categories_per_schema: [
      "valid",
      "missing",
      "extra",
      "malformed",
      "oversized",
      "unsupported-version",
    ],
    relationship_categories: ["cyclic", "stale"],
    cases,
  };
  files.set("manifest.json", jsonBytes(manifest));
  return files;
}

function validators() {
  const ajv = new Ajv2020({ allErrors: true, strict: true });
  addFormats(ajv);
  return Object.fromEntries(
    RECORD_SCHEMAS.map((name) => [name, ajv.compile(schemaDocument(name))]),
  );
}

export function verifyFixtureSemantics(files = expectedFiles()) {
  const compiled = validators();
  const manifest = JSON.parse(files.get("manifest.json"));
  assert.deepEqual(manifest.schemas, RECORD_SCHEMAS);
  assert.equal(manifest.cases.length, 56);
  for (const item of manifest.cases) {
    const record = JSON.parse(files.get(item.path));
    const structurallyValid = compiled[item.schema_name](record);
    const shouldPassSchema = [
      "admit",
      "reject-semantic",
      "admit-individual-reject-record-set",
    ].includes(item.expected_boundary);
    assert.equal(structurallyValid, shouldPassSchema, item.case_id);
  }
  for (const schemaName of RECORD_SCHEMAS) {
    const categories = manifest.cases
      .filter((item) => item.schema_name === schemaName)
      .map((item) => item.category);
    for (const category of manifest.required_categories_per_schema) {
      assert.ok(
        categories.includes(category),
        `${schemaName} lacks ${category}`,
      );
    }
  }
  return manifest;
}

export function synchronize({ write = false } = {}) {
  const files = expectedFiles();
  verifyFixtureSemantics(files);
  if (write) {
    fs.rmSync(OUT, { recursive: true, force: true });
    for (const [relative, bytes] of files) {
      const target = path.join(OUT, relative);
      fs.mkdirSync(path.dirname(target), { recursive: true });
      fs.writeFileSync(target, bytes);
    }
  } else {
    const observed = fs.existsSync(OUT)
      ? fs
          .readdirSync(OUT, { recursive: true, withFileTypes: true })
          .filter((entry) => entry.isFile())
          .map((entry) =>
            path
              .relative(OUT, path.join(entry.parentPath, entry.name))
              .replaceAll(path.sep, "/"),
          )
          .sort()
      : [];
    assert.deepEqual(
      observed,
      [...files.keys()].sort(),
      "fixture file closure changed",
    );
    for (const [relative, bytes] of files) {
      assert.deepEqual(
        fs.readFileSync(path.join(OUT, relative)),
        bytes,
        `${relative} drifted`,
      );
    }
  }
  return files.size;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const write = process.argv.includes("--write");
  const count = synchronize({ write });
  console.log(
    `${write ? "Wrote" : "Validated"} ${count} deterministic Engineering Runtime fixture files.`,
  );
}
