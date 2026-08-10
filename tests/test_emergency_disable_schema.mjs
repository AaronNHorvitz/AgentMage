import assert from "node:assert/strict";
import test from "node:test";

import {
  createValidator,
  readFixture,
  validateRecord,
} from "../scripts/validate_emergency_disable_schema.mjs";

const validator = createValidator();

test("canonical emergency-disable policy satisfies the closed schema", () => {
  assert.deepEqual(validateRecord(readFixture(), validator), {
    valid: true,
    errors: [],
  });
});

test("missing, unknown, malformed, and traversal fields fail closed", () => {
  const missing = readFixture();
  delete missing.policy.policy_sequence;
  const unknown = readFixture();
  unknown.installation.remote_endpoint = "fixture";
  const malformed = readFixture();
  malformed.entries[0].subject_sha256 = "invalid";
  const traversal = readFixture();
  traversal.signing.detached_signatures[0].path = "../outside.sig";
  for (const changed of [missing, unknown, malformed, traversal]) {
    assert.equal(validateRecord(changed, validator).valid, false);
  }
});

test("network, remote, authority, recovery, and product claims fail closed", () => {
  const network = readFixture();
  network.installation.network_allowed = true;
  const remote = readFixture();
  remote.installation.remote_trigger = true;
  const authority = readFixture();
  authority.evaluation.may_create_authority = true;
  const recovery = readFixture();
  recovery.recovery.expiration_silently_unblocks = true;
  const product = readFixture();
  product.claims.implemented_startup_hook = true;
  const macos = readFixture();
  macos.claims.macos_support = true;
  for (const changed of [network, remote, authority, recovery, product, macos]) {
    assert.equal(validateRecord(changed, validator).valid, false);
  }
});

test("formal emergency-disable validation is non-mutating", () => {
  const fixture = readFixture();
  const before = structuredClone(fixture);
  validateRecord(fixture, validator);
  assert.deepEqual(fixture, before);
});
