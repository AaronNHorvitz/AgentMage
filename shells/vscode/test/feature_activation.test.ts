import assert from "node:assert/strict";
import test from "node:test";

import { resolveExtensionFeatureActivation } from "../src/feature_activation.js";

void test("compatibility registrations require independent exact true flags", () => {
  const values = new Map<string, unknown>([
    ["nativeParticipant", true],
    ["nativeProviderCompatibility", false],
  ]);
  assert.deepEqual(
    resolveExtensionFeatureActivation((key) => values.get(key)),
    {
      nativeParticipant: true,
      nativeProviderCompatibility: false,
    },
  );
});

void test("missing malformed or truthy values fail disabled", () => {
  for (const value of [undefined, null, 1, "true", {}, []]) {
    const flags = resolveExtensionFeatureActivation(() => value);
    assert.equal(flags.nativeParticipant, false);
    assert.equal(flags.nativeProviderCompatibility, false);
  }
});
