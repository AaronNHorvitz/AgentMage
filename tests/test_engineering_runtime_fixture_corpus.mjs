import assert from "node:assert/strict";
import test from "node:test";

import {
  RECORD_SCHEMAS,
  expectedFiles,
  synchronize,
  validRecords,
  verifyFixtureSemantics,
} from "../scripts/engineering_runtime_fixture_corpus.mjs";

test("canonical record corpus is deterministic, closed, and current", () => {
  assert.equal(synchronize(), 57);
  const first = expectedFiles();
  const second = expectedFiles();
  assert.deepEqual([...first], [...second]);
});

test("every canonical schema has all required valid and invalid fixture categories", () => {
  const manifest = verifyFixtureSemantics();
  assert.equal(manifest.synthetic, true);
  assert.equal(manifest.contains_real_user_data, false);
  assert.equal(manifest.schemas.length, 9);
  assert.equal(manifest.cases.length, 56);
  for (const schemaName of RECORD_SCHEMAS) {
    const cases = manifest.cases.filter(
      (item) => item.schema_name === schemaName,
    );
    assert.ok(cases.some((item) => item.expected_boundary === "admit"));
    assert.equal(
      cases.filter((item) => item.expected_boundary === "reject-schema").length,
      5,
    );
  }
});

test("cyclic and stale fixtures remain structurally valid boundary attacks", () => {
  const manifest = verifyFixtureSemantics();
  const cyclic = manifest.cases.find((item) => item.category === "cyclic");
  const stale = manifest.cases.find((item) => item.category === "stale");
  assert.equal(cyclic.expected_boundary, "reject-semantic");
  assert.equal(cyclic.expected_code, "engineering.workflow.dependency_cycle");
  assert.equal(stale.expected_boundary, "admit-individual-reject-record-set");
  assert.equal(stale.expected_code, "engineering.record.binding_mismatch");
});

test("fixture builders return fresh values and preserve explicit nullable fields", () => {
  const first = validRecords();
  first["artifact-envelope"].artifact_id = "mutated";
  const second = validRecords();
  assert.equal(second["artifact-envelope"].artifact_id, "artifact:1");
  assert.equal(
    Object.hasOwn(second["terminal-result"], "diagnostic_code"),
    true,
  );
  assert.equal(Object.hasOwn(second["tool-observation"], "stdout"), true);
});
