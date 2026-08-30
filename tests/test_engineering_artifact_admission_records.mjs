import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";
import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

import { createEngineeringRuntimeValidator } from "../scripts/engineering_runtime_schemas.mjs";

const manifest = JSON.parse(
  fs.readFileSync("fixtures/artifact-admission/v1/manifest.json", "utf8"),
);

function validator(name) {
  const ajv = new Ajv2020({ allErrors: true, strict: true });
  addFormats(ajv);
  return createEngineeringRuntimeValidator(name, ajv);
}

test("all admission cases use the authoritative closed Engineering Runtime records", () => {
  const validators = {
    origin: validator("origin"),
    source_reference: validator("source-reference"),
    source_provenance: validator("source-provenance"),
    source_artifact: validator("source-artifact"),
  };
  assert.equal(manifest.cases.length, 16);
  for (const fixture of manifest.cases) {
    for (const [kind, validate] of Object.entries(validators)) {
      assert.equal(validate(fixture.records[kind]), true, `${fixture.fixture_id}/${kind}`);
    }
  }
});

test("unavailable and unsupported records cannot be changed into captured records without bytes", () => {
  const validate = validator("source-artifact");
  for (const category of ["inaccessible", "unsupported"]) {
    const fixture = manifest.cases.find((candidate) => candidate.category === category);
    assert.ok(fixture);
    assert.equal(validate({ ...fixture.records.source_artifact, capture_state: "captured" }), false);
  }
});

test("stale provenance cannot masquerade as a captured current source artifact", () => {
  const validate = validator("source-artifact");
  const fixture = manifest.cases.find((candidate) => candidate.category === "stale");
  assert.ok(fixture);
  const claimed = {
    ...fixture.records.source_artifact,
    capture_state: "captured",
    byte_length: fixture.offered_byte_identity.byte_length,
    sha256: fixture.offered_byte_identity.sha256,
  };
  assert.equal(validate(claimed), false);
});
