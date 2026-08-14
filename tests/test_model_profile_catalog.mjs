import assert from "node:assert/strict";
import test from "node:test";

import { buildCatalog, synchronizeCatalog, validateCatalog } from "../scripts/model_profile_catalog.mjs";

test("catalog preserves every historical Gemma record and enables nothing", () => {
  synchronizeCatalog();
  const catalog = buildCatalog();
  assert.equal(catalog.enabled_profile_count, 0);
  assert.equal(catalog.historical_records.length, 7);
  assert.equal(catalog.historical_records.every((record) => record.preserved), true);
  assert.deepEqual(new Set(catalog.historical_records.map((record) => record.profile_family)), new Set([
    "gemma-4-e4b",
    "gemma-4-12b-unified",
  ]));
});

test("fake Muse and Gemma entries are synthetic, exact, disabled, and non-fallback", () => {
  const catalog = validateCatalog();
  assert.deepEqual(catalog.profiles.map((profile) => profile.family), [
    "deterministic_fake_muse",
    "deterministic_fake_gemma",
  ]);
  for (const profile of catalog.profiles) {
    assert.equal(profile.enabled, false);
    assert.equal(profile.automatic_fallback, false);
    assert.equal(profile.runtime.kind, "deterministic_fake");
    assert.equal(profile.capabilities[0].limitations.includes("not-a-model"), true);
  }
});

test("catalog rejects activation, fallback, hash drift, and historical removal", () => {
  for (const mutate of [
    (catalog) => { catalog.profiles[0].enabled = true; },
    (catalog) => { catalog.profiles[0].automatic_fallback = true; },
    (catalog) => { catalog.profiles[0].manifest_sha256 = "invalid"; },
    (catalog) => { catalog.historical_records.pop(); },
  ]) {
    const catalog = structuredClone(buildCatalog());
    mutate(catalog);
    assert.throws(() => validateCatalog(catalog));
  }
});
