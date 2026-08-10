import assert from "node:assert/strict";
import test from "node:test";

import {
  createValidator,
  readFixture,
  validateRecord,
} from "../scripts/validate_manual_patch_schema.mjs";

const validator = createValidator();

test("canonical signed manual patch metadata satisfies the closed schema", () => {
  assert.deepEqual(validateRecord(readFixture(), validator), {
    valid: true,
    errors: [],
  });
});

test("missing, unknown, malformed, and traversal fields fail closed", () => {
  const missing = readFixture();
  delete missing.release.release_id;
  const unknown = readFixture();
  unknown.delivery.unreviewed_channel = true;
  const malformed = readFixture();
  malformed.artifacts[0].sha256 = "not-a-hash";
  const traversal = readFixture();
  traversal.artifacts[0].path = "../outside/package";
  for (const changed of [missing, unknown, malformed, traversal]) {
    assert.equal(validateRecord(changed, validator).valid, false);
  }
});

test("network, remote, signer, and production overclaims fail closed", () => {
  const network = readFixture();
  network.delivery.transaction_network_allowed = true;
  const remote = readFixture();
  remote.delivery.remote_control = true;
  const algorithm = readFixture();
  algorithm.signing.algorithm = "unknown";
  const release = readFixture();
  release.claims.releasable_package = true;
  const macos = readFixture();
  macos.claims.macos_support = true;
  for (const changed of [network, remote, algorithm, release, macos]) {
    assert.equal(validateRecord(changed, validator).valid, false);
  }
});

test("formal validation is non-mutating", () => {
  const fixture = readFixture();
  const before = structuredClone(fixture);
  validateRecord(fixture, validator);
  assert.deepEqual(fixture, before);
});
