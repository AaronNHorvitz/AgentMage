import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";
import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

import { createEngineeringRuntimeValidator } from "../scripts/engineering_runtime_schemas.mjs";

const manifest = JSON.parse(
  fs.readFileSync("fixtures/artifact-admission/v1/adversarial-manifest.json", "utf8"),
);

function validator(name) {
  const ajv = new Ajv2020({ allErrors: true, strict: true });
  addFormats(ajv);
  return createEngineeringRuntimeValidator(name, ajv);
}

test("hostile cases remain valid source records while their content stays untrusted", () => {
  const validators = {
    origin: validator("origin"),
    source_reference: validator("source-reference"),
    source_provenance: validator("source-provenance"),
    source_artifact: validator("source-artifact"),
  };
  for (const fixture of manifest.cases) {
    for (const [kind, validate] of Object.entries(validators)) {
      assert.equal(validate(fixture.records[kind]), true, `${fixture.fixture_id}/${kind}`);
    }
    assert.equal(fixture.expected_result.may_claim_complete, false);
  }
});

test("replacement and cancellation cannot masquerade as captured sources", () => {
  const validate = validator("source-artifact");
  for (const category of ["concurrent_replacement", "cancellation"]) {
    const fixture = manifest.cases.find((candidate) => candidate.category === category);
    assert.ok(fixture);
    assert.equal(validate({ ...fixture.records.source_artifact, capture_state: "captured" }), false);
  }
});
