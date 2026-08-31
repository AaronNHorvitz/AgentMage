import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import Ajv2020 from "ajv/dist/2020.js";

const schema = JSON.parse(
  await readFile(new URL("../schemas/model/orchestration-profile.schema.json", import.meta.url), "utf8"),
);
const validate = new Ajv2020({ allErrors: true, strict: true }).compile(schema);
const sha = (value) => value.repeat(64);

function record() {
  return {
    schema_version: 1,
    orchestration_profile_id: "orchestration-1",
    model_profile_id: "model-1",
    model_manifest_sha256: sha("a"),
    context: {
      schema_version: 1,
      model_profile_id: "model-1",
      model_manifest_sha256: sha("a"),
      model_runtime_sha256: sha("b"),
      tokenizer_sha256: sha("c"),
      token_counter_sha256: sha("d"),
      total_window_tokens: 1024,
      system_and_tool_tokens: 100,
      user_input_tokens: 100,
      source_artifacts: {
        requested_tokens: 300,
        minimum_tokens: 200,
        allocated_tokens: 300,
        disposition: "included",
        reason_code: null,
      },
      retrieved_context: {
        requested_tokens: 200,
        minimum_tokens: 50,
        allocated_tokens: 200,
        disposition: "included",
        reason_code: null,
      },
      workflow_recovery_reserve_tokens: 96,
      output_reserve_tokens: 128,
      safety_margin_tokens: 100,
      unallocated_tokens: 0,
      plan_sha256: sha("e"),
    },
    shape: {
      plan_horizon: 4,
      visible_tool_ids: ["tool.read"],
      max_observation_tokens: 128,
      parser_repair: "one_targeted_repair",
      recovery_scaffolding: "bounded_history",
      diagnostic_verbosity: "bounded_explanation",
    },
    invariant_controls: {
      policy_sha256: sha("1"),
      grant_contract_sha256: sha("2"),
      approval_policy_sha256: sha("3"),
      side_effect_policy_sha256: sha("4"),
      retry_policy_sha256: sha("5"),
      verifier_policy_sha256: sha("6"),
      budget_policy_sha256: sha("7"),
      completion_policy_sha256: sha("8"),
    },
    qualifications: [
      { role_id: "role-coding", workflow_id: "workflow-read-only", evidence_sha256: sha("9"), passed: true },
    ],
    enabled: false,
    automatic_fallback: false,
    profile_sha256: sha("f"),
  };
}

test("compiled orchestration profile schema accepts the closed record", () => {
  assert.equal(validate(record()), true, JSON.stringify(validate.errors));
});

test("schema rejects authority widening and stale qualification shapes", () => {
  const cases = [
    (value) => { value.automatic_fallback = true; },
    (value) => { value.qualifications[0].passed = false; },
    (value) => { value.shape.visible_tool_ids.push("tool.read"); },
    (value) => { value.context.source_artifacts.disposition = "silently_dropped"; },
    (value) => { value.invariant_controls.execute = true; },
    (value) => { delete value.context.tokenizer_sha256; },
  ];
  for (const mutate of cases) {
    const value = record();
    mutate(value);
    assert.equal(validate(value), false);
  }
});
