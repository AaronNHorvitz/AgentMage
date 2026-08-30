import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";
import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

import { createEngineeringRuntimeValidator } from "../scripts/engineering_runtime_schemas.mjs";

const manifest = JSON.parse(
  fs.readFileSync("fixtures/artifact-admission/v1/lineage-manifest.json", "utf8"),
);

function validator(name) {
  const ajv = new Ajv2020({ allErrors: true, strict: true });
  addFormats(ajv);
  return createEngineeringRuntimeValidator(name, ajv);
}

test("lineage records satisfy every authoritative Engineering Runtime schema", () => {
  const validators = {
    transformation: validator("artifact-transformation"),
    extraction: validator("extraction-result"),
    section: validator("structural-section"),
    provenance: validator("source-provenance"),
    retention: validator("source-retention"),
    disposition: validator("context-disposition"),
  };
  for (const fixture of manifest.cases) {
    assert.equal(validators.provenance(fixture.source_records.source_provenance), true, `${fixture.fixture_id}/provenance`);
    assert.equal(validators.retention(fixture.source_records.source_retention), true, `${fixture.fixture_id}/retention`);
    assert.equal(validators.disposition(fixture.source_records.context_disposition), true, `${fixture.fixture_id}/disposition`);
    if (fixture.capture_state === "captured") {
      assert.equal(validators.transformation(fixture.records.transformation), true, `${fixture.fixture_id}/transformation`);
      assert.equal(validators.extraction(fixture.records.extraction), true, `${fixture.fixture_id}/extraction`);
      assert.equal(validators.section(fixture.records.sections[0]), true, `${fixture.fixture_id}/section`);
    } else {
      assert.equal(fixture.records.transformation, null);
      assert.equal(fixture.records.extraction, null);
      assert.deepEqual(fixture.records.sections, []);
    }
  }
});

test("no fixture context disposition claims model delivery", () => {
  for (const fixture of manifest.cases) {
    const disposition = fixture.source_records.context_disposition;
    assert.notEqual(disposition.disposition, "included");
    assert.deepEqual(disposition.ranges, []);
    assert.equal(disposition.token_count, 0);
  }
});
