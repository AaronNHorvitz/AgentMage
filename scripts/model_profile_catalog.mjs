#!/usr/bin/env node

import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import Ajv2020 from "ajv/dist/2020.js";

import { modelSchemaDocument } from "./model_contract_schemas.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const TARGET = path.join(ROOT, "model-profiles/exact-profile-catalog.json");
const SHA = "a".repeat(64);

const historicalPaths = [
  "model-profiles/candidates/gemma-4-e4b/artifact-admission.json",
  "model-profiles/candidates/gemma-4-e4b/feasibility-disposition.json",
  "model-profiles/candidates/gemma-4-e4b/source-admission.json",
  "model-profiles/candidates/gemma-4-12b-unified/artifact-admission.json",
  "model-profiles/candidates/gemma-4-12b-unified/evaluation-plan.json",
  "model-profiles/candidates/gemma-4-12b-unified/feasibility-disposition.json",
  "model-profiles/candidates/gemma-4-12b-unified/source-admission.json",
];

function sha256(relativePath) {
  return crypto.createHash("sha256").update(fs.readFileSync(path.join(ROOT, relativePath))).digest("hex");
}

function fakeProfile(family) {
  const label = family === "muse" ? "Muse" : "Gemma";
  return {
    schema_version: 2,
    profile_id: `deterministic-fake-${family}-v1`,
    manifest_id: `deterministic-fake-${family}-manifest-v1`,
    manifest_sha256: sha256(`kernel/engine/src/model_runtime.rs`),
    display_name: `Deterministic fake ${label} contract profile`,
    family: `deterministic_fake_${family}`,
    publisher_control: "AgentMage test fixture",
    lineage: ["synthetic-code-fixture", `deterministic-fake-${family}-v1`],
    license_spdx: "Apache-2.0",
    license_terms_sha256: sha256("LICENSE"),
    artifact: {
      artifact_id: `deterministic-fake-${family}-bytes-v1`,
      publisher: "AgentMage test fixture",
      source_revision: "source-tree-bound",
      format: "synthetic-test-fixture",
      bytes: 1,
      sha256: SHA,
    },
    transformations: [],
    codec: {
      codec_id: `deterministic-fake-${family}-codec-v1`,
      codec_version: "1.0.0",
      codec_sha256: sha256("kernel/contracts/src/model.rs"),
      tokenizer: `deterministic-fake-${family}-tokenizer-v1`,
      tokenizer_sha256: SHA,
      template: `deterministic-fake-${family}-template-v1`,
      template_sha256: SHA,
      tool_protocol_version: "closed-proposal-v1",
      end_tokens: [0],
      reasoning_enabled: false,
    },
    runtime: {
      adapter_id: `deterministic-fake-${family}-adapter-v1`,
      kind: "deterministic_fake",
      contract_version: 1,
      runtime_build: "agentmage-kernel-engine-test-fixture",
      runtime_sha256: sha256("kernel/engine/src/model_runtime.rs"),
      platform: "deterministic_fake",
      architecture: "x86_64",
    },
    quantization: "not-applicable-synthetic-fixture",
    modalities: ["text"],
    context: {
      max_context_tokens: 8192,
      max_input_bytes: 32768,
      max_messages: 32,
      token_counter: "deterministic-byte-counter-v1",
      token_counter_sha256: SHA,
    },
    decoding: {
      profile_id: "deterministic-contract-v1",
      sampler_order: ["fixture_exact"],
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
    capabilities: [{
      role: "dialogue",
      state: "not_evaluated",
      evaluation_profile: null,
      result_sha256: null,
      limitations: ["contract-testing-only", "not-a-model", "no-product-selection"],
    }],
    policy_sha256: sha256("MODEL-PROVENANCE-POLICY.md"),
    lifecycle: "candidate",
    enabled: false,
    automatic_fallback: false,
  };
}

export function buildCatalog() {
  return {
    schema_version: 1,
    policy_path: "MODEL-PROVENANCE-POLICY.md",
    policy_sha256: sha256("MODEL-PROVENANCE-POLICY.md"),
    enabled_profile_count: 0,
    historical_records: historicalPaths.map((recordPath) => ({
      profile_family: recordPath.includes("e4b") ? "gemma-4-e4b" : "gemma-4-12b-unified",
      path: recordPath,
      sha256: sha256(recordPath),
      disposition: recordPath.includes("feasibility-disposition") ? "REJECTED" : "BLOCKED",
      preserved: true,
    })),
    profiles: [fakeProfile("muse"), fakeProfile("gemma")],
  };
}

export function validateCatalog(catalog = buildCatalog()) {
  const validate = new Ajv2020({ allErrors: true, strict: true }).compile(
    modelSchemaDocument("profile-catalog"),
  );
  if (!validate(catalog)) {
    throw new Error(`invalid exact-profile catalog: ${JSON.stringify(validate.errors)}`);
  }
  const actualEnabled = catalog.profiles.filter((profile) => profile.enabled).length;
  if (actualEnabled !== catalog.enabled_profile_count) {
    throw new Error("enabled profile count does not match exact entries");
  }
  const expectedHistorical = new Set(historicalPaths);
  const observedHistorical = new Set(catalog.historical_records.map((record) => record.path));
  if (
    expectedHistorical.size !== observedHistorical.size ||
    [...expectedHistorical].some((recordPath) => !observedHistorical.has(recordPath))
  ) {
    throw new Error("historical Gemma record set is incomplete or substituted");
  }
  if (new Set(catalog.profiles.map((profile) => profile.profile_id)).size !== catalog.profiles.length) {
    throw new Error("exact-profile catalog contains duplicate profile identities");
  }
  for (const record of catalog.historical_records) {
    if (sha256(record.path) !== record.sha256) {
      throw new Error(`historical model record changed: ${record.path}`);
    }
  }
  return catalog;
}

export function synchronizeCatalog({ write = false } = {}) {
  const expected = `${JSON.stringify(validateCatalog(), null, 2)}\n`;
  if (write) {
    fs.writeFileSync(TARGET, expected);
  } else if (!fs.existsSync(TARGET) || fs.readFileSync(TARGET, "utf8") !== expected) {
    throw new Error("exact-profile catalog differs from source records");
  }
}

if (path.resolve(process.argv[1] ?? "") === fileURLToPath(import.meta.url)) {
  synchronizeCatalog({ write: process.argv.includes("--write") });
  process.stdout.write(`exact-profile catalog: ${process.argv.includes("--write") ? "written" : "valid"}\n`);
}
