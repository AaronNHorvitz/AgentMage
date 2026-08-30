import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";
import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

import { createEngineeringRuntimeValidator } from "../scripts/engineering_runtime_schemas.mjs";

const suite = JSON.parse(
  fs.readFileSync("fixtures/artifact-admission/v1/context-accounting-manifests.json", "utf8"),
);

function validator(name) {
  const ajv = new Ajv2020({ allErrors: true, strict: true });
  addFormats(ajv);
  return createEngineeringRuntimeValidator(name, ajv);
}

test("every scenario emits one authoritative complete context manifest", () => {
  const validate = validator("context-manifest");
  for (const scenario of suite.scenarios) {
    assert.equal(validate(scenario.context_manifest), true, scenario.scenario_id);
    assert.equal(scenario.complete_manifest_count, 1);
    assert.equal(scenario.context_manifest.items.length, suite.source_count_per_manifest);
  }
});

test("non-admitting dispositions never smuggle ranges or tokens", () => {
  const nonAdmitting = new Set(["duplicate", "stale", "unsupported", "unavailable", "restricted", "omitted"]);
  for (const scenario of suite.scenarios) {
    for (const item of scenario.context_manifest.items) {
      if (nonAdmitting.has(item.disposition)) {
        assert.deepEqual(item.ranges, [], `${scenario.scenario_id}/${item.artifact_id}`);
        assert.equal(item.token_count, 0, `${scenario.scenario_id}/${item.artifact_id}`);
        assert.notEqual(item.reason_code, null, `${scenario.scenario_id}/${item.artifact_id}`);
      }
    }
  }
});
