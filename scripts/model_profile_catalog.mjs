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
  "model-profiles/candidates/muse-glimmer-30b-text-8k/source-admission.json",
  "model-profiles/candidates/muse-glimmer-30b-text-8k/runtime-support.json",
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

function museProfile(mode) {
  const diagnostic = mode === "diagnostic-repeatability";
  const profileId = `muse-glimmer-30b-q4-k-m-text-8k-fedora-${mode}`;
  const manifestPreimage = JSON.stringify({
    profile_id: profileId,
    source_admission_sha256: sha256(
      "model-profiles/candidates/muse-glimmer-30b-text-8k/source-admission.json",
    ),
    runtime_support_sha256: sha256(
      "model-profiles/candidates/muse-glimmer-30b-text-8k/runtime-support.json",
    ),
    codec_sha256: sha256("platforms/linux-inference/src/muse_atem_codec.rs"),
    decoding: diagnostic
      ? { sampler_order: ["greedy"], temperature: 0, top_p: 1, top_k: 1, seed: 42 }
      : { sampler_order: ["top_k", "top_p", "temperature"], temperature: 1, top_p: 0.95, top_k: 64, seed: 0 },
  });
  const manifestSha256 = crypto.createHash("sha256").update(manifestPreimage).digest("hex");
  return {
    schema_version: 2,
    profile_id: profileId,
    manifest_id: `${profileId}-manifest-v1`,
    manifest_sha256: manifestSha256,
    display_name: `Muse Glimmer 30B Q4_K_M text 8k Fedora ${mode}`,
    family: "muse_glimmer",
    publisher_control: "Meta",
    lineage: [
      "meta-models/Muse-Glimmer-30B@a4e59da52a7bc87ae7251dd5545c0dd437c44b68",
      "meta-models/Muse-Glimmer-30B-GGUF@43c7eadd41352a299ea8e0a36b3157978dd63596",
      "Muse-Glimmer-30B-KQuant-17GB-Q4_K_M.gguf",
    ],
    license_spdx: "Apache-2.0",
    license_terms_sha256: "cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30",
    artifact: {
      artifact_id: "meta-muse-glimmer-30b-kquant-17gb-q4-k-m",
      publisher: "Meta",
      source_revision: "43c7eadd41352a299ea8e0a36b3157978dd63596",
      format: "GGUF",
      bytes: 16_756_683_904,
      sha256: "4cc57c0f51040a226e5a72cc47b7613f7772950e460a665f7083de89f183f60e",
    },
    transformations: [],
    codec: {
      codec_id: "muse-glimmer-atem-closed-proposal-v1",
      codec_version: "1.0.0",
      codec_sha256: sha256("platforms/linux-inference/src/muse_atem_codec.rs"),
      tokenizer: "meta-models/Muse-Glimmer-30B tokenizer",
      tokenizer_sha256: "c9dbee66967b58f31a7c27f723c3760da3526ccd0427578e8905b0abb0031c4d",
      template: "meta-models/Muse-Glimmer-30B ATEM chat template",
      template_sha256: "cfc67e5f349f37690dfd31ed1f18bc4442a9dd32fe39a648f993cb4eb3cae678",
      tool_protocol_version: "atem-v1",
      end_tokens: [200001, 200008],
      reasoning_enabled: false,
    },
    runtime: {
      adapter_id: "linux-native-llama-cpp-b10423-vulkan-x86_64",
      kind: "native_llama_cpp",
      contract_version: 1,
      runtime_build: "llama.cpp b10423 a94d563ed801d1da1b8c2432946de07d0231bb3d",
      runtime_sha256: "3b1194ef38f4b02b6329d698e29532435a5a7c3567c84b8bb822459ca0893286",
      platform: "fedora",
      architecture: "x86_64",
    },
    quantization: "Q4_K_M first-party 17GB GGUF",
    modalities: ["text"],
    context: {
      max_context_tokens: 8192,
      max_input_bytes: 131072,
      max_messages: 128,
      token_counter: "llama.cpp b10423 Muse tokenizer",
      token_counter_sha256: "c9dbee66967b58f31a7c27f723c3760da3526ccd0427578e8905b0abb0031c4d",
    },
    decoding: diagnostic
      ? {
          profile_id: "diagnostic-repeatability-v1",
          sampler_order: ["greedy"],
          temperature: 0,
          top_p: 1,
          top_k: 1,
          repeat_penalty: 1,
          seed: 42,
          max_output_tokens: 2048,
        }
      : {
          profile_id: "first-party-recommended-quality-v1",
          sampler_order: ["top_k", "top_p", "temperature"],
          temperature: 1,
          top_p: 0.95,
          top_k: 64,
          repeat_penalty: 1,
          seed: 0,
          max_output_tokens: 2048,
        },
    hardware: [{
      platform: "fedora",
      architecture: "x86_64",
      minimum_system_memory_bytes: 32 * 1024 * 1024 * 1024,
      minimum_accelerator_memory_bytes: 20 * 1024 * 1024 * 1024,
      accelerator: "NVIDIA Vulkan",
      driver_constraint: "NVIDIA 610.43.03 evaluation tuple; changes require re-evaluation",
    }],
    capabilities: ["dialogue", "coding_planner", "tool_selection"].map((role) => ({
      role,
      state: "blocked",
      evaluation_profile: null,
      result_sha256: null,
      limitations: [
        "artifact-not-locally-verified",
        "runtime-package-not-admitted",
        "quality-not-measured",
        "synthetic-evaluation-only",
      ],
    })),
    policy_sha256: sha256("MODEL-PROVENANCE-POLICY.md"),
    lifecycle: "candidate",
    enabled: false,
    automatic_fallback: false,
  };
}

function codingDevelopmentProfile(family) {
  const muse = family === "muse";
  const profileId = muse
    ? "muse-glimmer-30b-q4-k-m-text-32k-fedora-coding-development"
    : "gpt-oss-20b-mxfp4-text-32k-fedora-coding-development";
  const codecPath = muse
    ? "platforms/linux-inference/src/muse_atem_codec.rs"
    : "platforms/linux-inference/src/gpt_oss_harmony_codec.rs";
  const preparationProfileSha256 = sha256("model-profiles/development/coding-model-lab.json");
  const preparationEvidenceSha256 = sha256(
    "docs/verification/coding-model-preparation-2026-09-21.md",
  );
  const manifestPreimage = JSON.stringify({
    profile_id: profileId,
    preparation_profile_sha256: preparationProfileSha256,
    preparation_evidence_sha256: preparationEvidenceSha256,
    codec_sha256: sha256(codecPath),
    context_tokens: 32768,
    output_tokens: 4096,
    reasoning: "medium",
  });
  const commonLimitations = [
    "coding-campaign-not-run",
    "product-transport-not-demonstrated",
    "repeated-run-admission-not-evaluated",
  ];
  const conversionLimitations = muse
    ? commonLimitations
    : [...commonLimitations, "conversion-equivalence-not-established"];
  return {
    schema_version: 2,
    profile_id: profileId,
    manifest_id: `${profileId}-manifest-v1`,
    manifest_sha256: crypto.createHash("sha256").update(manifestPreimage).digest("hex"),
    display_name: muse
      ? "Muse Glimmer 30B Q4_K_M text 32k Fedora coding development"
      : "OpenAI gpt-oss-20b MXFP4 ggml-org text 32k Fedora coding development",
    family: muse ? "muse_glimmer" : "gpt_oss",
    publisher_control: muse ? "Meta" : "OpenAI; GGUF conversion published by ggml-org",
    lineage: muse
      ? [
          "meta-models/Muse-Glimmer-30B@a4e59da52a7bc87ae7251dd5545c0dd437c44b68",
          "meta-models/Muse-Glimmer-30B-GGUF@43c7eadd41352a299ea8e0a36b3157978dd63596",
          "Muse-Glimmer-30B-KQuant-17GB-Q4_K_M.gguf",
        ]
      : [
          "openai/gpt-oss-20b@6cee5e81ee83917806bbde320786a8fb61efebee",
          "ggml-org/gpt-oss-20b-GGUF@ef9b12f2ff56c69cf32153a02784e7a3c88bf524",
          "gpt-oss-20b-MXFP4.gguf",
        ],
    license_spdx: "Apache-2.0",
    license_terms_sha256: muse
      ? "cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30"
      : "58d1e17ffe5109a7ae296caafcadfdbe6a7d176f0bc4ab01e12a689b0499d8bd",
    artifact: muse
      ? {
          artifact_id: "meta-muse-glimmer-30b-kquant-17gb-q4-k-m",
          publisher: "Meta",
          source_revision: "43c7eadd41352a299ea8e0a36b3157978dd63596",
          format: "GGUF",
          bytes: 16_756_683_904,
          sha256: "4cc57c0f51040a226e5a72cc47b7613f7772950e460a665f7083de89f183f60e",
        }
      : {
          artifact_id: "ggml-org-gpt-oss-20b-mxfp4",
          publisher: "ggml-org",
          source_revision: "ef9b12f2ff56c69cf32153a02784e7a3c88bf524",
          format: "GGUF",
          bytes: 12_109_566_624,
          sha256: "27cd6c432c7672cb812a92f611cf3ba7bbc35928262bb1e1253ff4ee6ae35901",
        },
    transformations: [],
    codec: {
      codec_id: muse
        ? "muse-glimmer-atem-reasoning-medium-closed-proposal-v1"
        : "gpt-oss-harmony-closed-proposal-v1",
      codec_version: "1.0.0",
      codec_sha256: sha256(codecPath),
      tokenizer: muse
        ? "meta-models/Muse-Glimmer-30B tokenizer"
        : "gpt-oss-20b MXFP4 GGUF embedded tokenizer",
      tokenizer_sha256: muse
        ? "c9dbee66967b58f31a7c27f723c3760da3526ccd0427578e8905b0abb0031c4d"
        : "27cd6c432c7672cb812a92f611cf3ba7bbc35928262bb1e1253ff4ee6ae35901",
      template: muse
        ? "meta-models/Muse-Glimmer-30B ATEM chat template"
        : "openai/gpt-oss-20b Harmony chat template at 6cee5e81",
      template_sha256: muse
        ? "cfc67e5f349f37690dfd31ed1f18bc4442a9dd32fe39a648f993cb4eb3cae678"
        : "a4c9919cbbd4acdd51ccffe22da049264b1b73e59055fa58811a99efbd7c8146",
      tool_protocol_version: muse
        ? "atem-reasoning-medium-closed-proposal-v1"
        : "harmony-closed-proposal-v1",
      end_tokens: muse ? [200001, 200008] : [200002, 200012],
      reasoning_enabled: true,
    },
    runtime: {
      adapter_id: "linux-native-llama-cpp-b10423-vulkan-x86_64",
      kind: "native_llama_cpp",
      contract_version: 1,
      runtime_build: "llama.cpp b10423 a94d563ed801d1da1b8c2432946de07d0231bb3d",
      runtime_sha256: "3b1194ef38f4b02b6329d698e29532435a5a7c3567c84b8bb822459ca0893286",
      platform: "fedora",
      architecture: "x86_64",
    },
    quantization: muse ? "Q4_K_M first-party 17GB GGUF" : "MXFP4 ggml-org GGUF conversion",
    modalities: ["text"],
    context: {
      max_context_tokens: 32768,
      max_input_bytes: 524288,
      max_messages: 4096,
      token_counter: muse
        ? "llama.cpp b10423 Muse tokenizer"
        : "llama.cpp b10423 GPT-OSS embedded tokenizer",
      token_counter_sha256: muse
        ? "c9dbee66967b58f31a7c27f723c3760da3526ccd0427578e8905b0abb0031c4d"
        : "27cd6c432c7672cb812a92f611cf3ba7bbc35928262bb1e1253ff4ee6ae35901",
    },
    decoding: {
      profile_id: muse
        ? "coding-development-muse-medium-32k-v1"
        : "coding-development-gpt-oss-medium-32k-v1",
      sampler_order: ["greedy"],
      temperature: 0,
      top_p: 1,
      top_k: 1,
      repeat_penalty: 1,
      seed: 42,
      max_output_tokens: 4096,
    },
    hardware: [{
      platform: "fedora",
      architecture: "x86_64",
      minimum_system_memory_bytes: 16 * 1024 * 1024 * 1024,
      minimum_accelerator_memory_bytes: 21_000 * 1024 * 1024,
      accelerator: "NVIDIA Vulkan",
      driver_constraint: "NVIDIA 610.43.03 preparation tuple; changes require re-evaluation",
    }],
    capabilities: ["dialogue", "coding_planner", "tool_selection"].map((role) => ({
      role,
      state: "blocked",
      evaluation_profile: null,
      result_sha256: null,
      limitations: conversionLimitations,
    })),
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
      profile_family: recordPath.includes("muse-glimmer")
        ? "muse-glimmer-30b-text-8k"
        : recordPath.includes("e4b")
          ? "gemma-4-e4b"
          : "gemma-4-12b-unified",
      path: recordPath,
      sha256: sha256(recordPath),
      disposition: recordPath.includes("feasibility-disposition") ? "REJECTED" : "BLOCKED",
      preserved: true,
    })),
    profiles: [
      fakeProfile("muse"),
      fakeProfile("gemma"),
      museProfile("first-party-quality"),
      museProfile("diagnostic-repeatability"),
      codingDevelopmentProfile("muse"),
      codingDevelopmentProfile("gpt-oss"),
    ],
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
    throw new Error("historical candidate record set is incomplete or substituted");
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
