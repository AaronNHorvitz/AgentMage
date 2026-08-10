import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  SUPPORT_RECORD_TYPES,
  createSupportValidators,
  validateSupportRecord,
  validateSupportRecords,
} from "../scripts/validate_support_schemas.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const supportValidators = createSupportValidators();

function policyFixture() {
  return JSON.parse(
    fs.readFileSync(
      path.join(ROOT, "support/vulnerability-support-policy.json"),
      "utf8",
    ),
  );
}

test("vulnerability support policy satisfies its closed formal schema", () => {
  const results = validateSupportRecords();
  assert.deepEqual(SUPPORT_RECORD_TYPES, ["vulnerability-support-policy"]);
  assert.equal(results.length, 1);
  assert.deepEqual(results.map((result) => result.valid), [true]);
});

test("vulnerability support schema rejects missing, unknown, and broadened records", () => {
  const policy = policyFixture();
  const missing = structuredClone(policy);
  delete missing.schema_version;
  const unknown = structuredClone(policy);
  unknown.unreviewed_extension = true;
  const network = structuredClone(policy);
  network.runtime_contact_authority.background_network_contact = true;
  const support = structuredClone(policy);
  support.supported_versions.production_binary_supported = true;
  for (const changed of [missing, unknown, network, support]) {
    assert.equal(
      validateSupportRecord(
        SUPPORT_RECORD_TYPES[0],
        changed,
        supportValidators,
      ).valid,
      false,
    );
  }
});

test("support validation is non-mutating and rejects unknown record types", () => {
  const policy = policyFixture();
  const before = structuredClone(policy);
  validateSupportRecord(SUPPORT_RECORD_TYPES[0], policy, supportValidators);
  assert.deepEqual(policy, before);
  assert.throws(
    () => validateSupportRecord("unknown-record", {}, supportValidators),
    /unknown support record type/,
  );
});
