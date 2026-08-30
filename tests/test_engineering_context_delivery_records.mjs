import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";
import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

import { createEngineeringRuntimeValidator } from "../scripts/engineering_runtime_schemas.mjs";

const suite = JSON.parse(
  fs.readFileSync("fixtures/artifact-admission/v1/context-delivery-receipts.json", "utf8"),
);

function validator(name) {
  const ajv = new Ajv2020({ allErrors: true, strict: true });
  addFormats(ajv);
  return createEngineeringRuntimeValidator(name, ajv);
}

test("all reconstruction receipts satisfy the authoritative delivery schema", () => {
  const validate = validator("context-delivery-receipt");
  for (const scenario of suite.scenarios) {
    assert.equal(validate(scenario.receipt), true, scenario.scenario_id);
    assert.equal(scenario.receipt.context_manifest_id, scenario.context_manifest_id);
    assert.equal(scenario.receipt.context_manifest_sha256, scenario.context_manifest_sha256);
  }
});

test("blocked receipts name required unseen authority and never allow completion", () => {
  for (const scenario of suite.scenarios) {
    if (scenario.receipt.outcome === "blocked") {
      assert.ok(scenario.receipt.required_unseen_artifact_ids.length > 0);
      assert.equal(scenario.completion_allowed, false);
    } else {
      assert.deepEqual(scenario.receipt.required_unseen_artifact_ids, []);
      assert.equal(scenario.completion_allowed, true);
    }
  }
});
