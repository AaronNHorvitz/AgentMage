import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import test from "node:test";

import {
  parseModelPickerSnapshot,
  renderModelAcquisitionReview,
  renderModelManagementReport,
  selectableModelInformation,
} from "../src/model_discovery.js";

const SHA = "a".repeat(64);

void test("zero-profile discovery is valid and yields no native picker entry", () => {
  const snapshot = signedSnapshot([]);
  const parsed = parseModelPickerSnapshot(snapshot);
  assert.deepEqual(parsed.entries, []);
  assert.deepEqual(selectableModelInformation(parsed), []);
  assert.match(
    renderModelManagementReport(parsed),
    /No exact local model profile/,
  );
});

void test("acquisition review separates preflight and license without effects", () => {
  const parsed = parseModelPickerSnapshot(
    signedSnapshot([entry("muse-review-1", "muse")]),
  );
  const review = renderModelAcquisitionReview(parsed, "muse-review-1");
  assert.match(review ?? "", /## Preflight Review/);
  assert.match(review ?? "", /## License Review/);
  assert.match(review ?? "", /Source opened: no/);
  assert.match(review ?? "", /Destination changed: no/);
  assert.match(review ?? "", /Acquisition started: no/);
  assert.match(review ?? "", /required before any acquisition/);
  assert.match(review ?? "", /Import, download, activation, rollback/);
  assert.equal(renderModelAcquisitionReview(parsed, "missing-profile"), undefined);
  assert.equal(renderModelAcquisitionReview(parsed, "../invalid"), undefined);
});

void test("Muse, Gemma, and additional admitted families map without prerequisites", () => {
  const parsed = parseModelPickerSnapshot(
    signedSnapshot([
      entry("additional-1", "additional"),
      entry("gemma-1", "gemma"),
      entry("muse-1", "muse"),
    ]),
  );
  const models = selectableModelInformation(parsed);
  assert.deepEqual(
    models.map((model) => model.id),
    ["additional-1", "gemma-1", "muse-1"],
  );
  assert.ok(models.every((model) => model.version === SHA));
  assert.ok(models.every((model) => model.capabilities.toolCalling));
  assert.ok(
    models.every((model) => model.tooltip.includes("never substitutes")),
  );
  const report = renderModelManagementReport(parsed);
  for (const expected of [
    "Fixture Publisher",
    "fixture\\-source@revision -> fixture\\-artifact\\.gguf",
    "Apache-2.0",
    "GGUF, 1,048,576 bytes",
    "fixture-source-revision",
    "Q4\\_K\\_M",
  ]) {
    assert.ok(report.includes(expected), expected);
  }
});

void test("every unavailable lifecycle remains visible only in management output", () => {
  const states = [
    "candidate",
    "evaluating",
    "quarantined",
    "rejected",
    "retired",
  ] as const;
  const blocked = states.map((state, index) =>
    entry(`blocked-${index.toString()}`, "family", {
      lifecycle: state,
      runtime_health: state === "quarantined" ? "quarantined" : "ready",
      activation: "blocked",
      disposition: "management_only",
      limitations: [`model.lifecycle.${state.replaceAll("_", "-")}`],
    }),
  );
  const parsed = parseModelPickerSnapshot(signedSnapshot(blocked));
  assert.deepEqual(selectableModelInformation(parsed), []);
  const report = renderModelManagementReport(parsed);
  for (const state of states) {
    assert.match(report, new RegExp(state.replaceAll("_", " ")));
  }
  assert.doesNotMatch(report, /\/var\/home|token=|password=/i);
});

void test("identity mutation and digest tampering fail before picker display", () => {
  const baseline = signedSnapshot([entry("selected-1", "muse")]);
  const fields = [
    "family",
    "display_name",
    "artifact_sha256",
    "publisher",
    "publisher_control",
    "lineage",
    "license_spdx",
    "license_terms_sha256",
    "source_revision",
    "artifact_format",
    "artifact_bytes",
    "quantization",
    "tokenizer_sha256",
    "template_sha256",
    "codec_sha256",
    "runtime_build",
    "runtime_contract_version",
    "runtime_sha256",
    "max_context_tokens",
    "context_sha256",
    "max_output_tokens",
    "decoding_sha256",
    "hardware_sha256",
    "policy_sha256",
    "activation",
    "support",
  ] as const;
  for (const field of fields) {
    const changed = structuredClone(baseline);
    const target = changed.entries[0] as Record<string, unknown>;
    target[field] =
      typeof target[field] === "number"
        ? target[field] + 1
        : field === "activation"
          ? "stale"
          : field === "support"
            ? "stale"
            : field.endsWith("sha256")
              ? "b".repeat(64)
              : "changed";
    assert.throws(() => parseModelPickerSnapshot(changed));
  }
});

void test("self-consistent blocked entry cannot claim selectable disposition", () => {
  const forged = entry("blocked-1", "muse", {
    lifecycle: "candidate",
    activation: "blocked",
    disposition: "selectable",
    limitations: ["model.activation.blocked"],
  });
  assert.throws(() => parseModelPickerSnapshot(signedSnapshot([forged])));
});

function entry(
  id: string,
  family: string,
  overrides: Record<string, unknown> = {},
): Record<string, unknown> {
  const unsigned: Record<string, unknown> = {
    profile_id: id,
    display_name: `${family} exact profile`,
    family,
    manifest_sha256: SHA,
    artifact_sha256: SHA,
    publisher: "Fixture Publisher",
    publisher_control: "fixture-policy",
    lineage: ["fixture-source@revision", "fixture-artifact.gguf"],
    license_spdx: "Apache-2.0",
    license_terms_sha256: SHA,
    source_revision: "fixture-source-revision",
    artifact_format: "GGUF",
    artifact_bytes: 1_048_576,
    quantization: "Q4_K_M",
    codec_id: `${id}-codec`,
    codec_sha256: SHA,
    tokenizer_sha256: SHA,
    template_sha256: SHA,
    runtime_adapter_id: `${id}-adapter`,
    runtime_kind: "native_llama_cpp",
    runtime_contract_version: 1,
    runtime_build: "fixture-runtime-v1",
    runtime_sha256: SHA,
    platform: "fedora",
    architecture: "x86_64",
    modalities: ["text"],
    max_context_tokens: 8_192,
    max_input_bytes: 32_768,
    max_messages: 32,
    context_sha256: SHA,
    max_output_tokens: 256,
    decoding_sha256: SHA,
    hardware_sha256: SHA,
    policy_sha256: SHA,
    tool_calling: true,
    capabilities: [
      { role: "dialogue", state: "passed", limitations: [] },
      { role: "tool_selection", state: "passed", limitations: [] },
    ],
    lifecycle: "approved",
    runtime_health: "ready",
    activation: "activated",
    compatibility: "compatible",
    support: "supported",
    limitations: [],
    requires_user_decision: true,
    disposition: "selectable",
    ...overrides,
  };
  return { ...unsigned, entry_sha256: digest(unsigned) };
}

function signedSnapshot(entries: readonly Record<string, unknown>[]): {
  schema_version: number;
  catalog_sha256: string;
  catalog_signature_verified: boolean;
  observed_at_ms: number;
  entries: readonly Record<string, unknown>[];
  snapshot_sha256: string;
} {
  const unsigned = {
    schema_version: 1,
    catalog_sha256: SHA,
    catalog_signature_verified: true,
    observed_at_ms: 10,
    entries,
  };
  return { ...unsigned, snapshot_sha256: digest(unsigned) };
}

function digest(value: unknown): string {
  return createHash("sha256")
    .update(JSON.stringify(value), "utf8")
    .digest("hex");
}
