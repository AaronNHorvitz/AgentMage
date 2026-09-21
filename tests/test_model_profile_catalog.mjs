import assert from "node:assert/strict";
import test from "node:test";

import { buildCatalog, synchronizeCatalog, validateCatalog } from "../scripts/model_profile_catalog.mjs";

test("catalog preserves every historical candidate record and enables nothing", () => {
  synchronizeCatalog();
  const catalog = buildCatalog();
  assert.equal(catalog.enabled_profile_count, 0);
  assert.equal(catalog.historical_records.length, 9);
  assert.equal(catalog.historical_records.every((record) => record.preserved), true);
  assert.deepEqual(new Set(catalog.historical_records.map((record) => record.profile_family)), new Set([
    "gemma-4-e4b",
    "gemma-4-12b-unified",
    "muse-glimmer-30b-text-8k",
  ]));
});

test("real Muse quality and diagnostic tuples remain separate and disabled", () => {
  const catalog = validateCatalog();
  const profiles = catalog.profiles.filter(
    (profile) => profile.family === "muse_glimmer" && profile.context.max_context_tokens === 8192,
  );
  assert.equal(profiles.length, 2);
  assert.notEqual(profiles[0].profile_id, profiles[1].profile_id);
  assert.notEqual(profiles[0].manifest_sha256, profiles[1].manifest_sha256);
  assert.notEqual(profiles[0].decoding.profile_id, profiles[1].decoding.profile_id);
  for (const profile of profiles) {
    assert.equal(profile.enabled, false);
    assert.equal(profile.automatic_fallback, false);
    assert.equal(profile.lifecycle, "candidate");
    assert.equal(profile.modalities.length, 1);
    assert.equal(profile.modalities[0], "text");
    assert.equal(profile.context.max_context_tokens, 8192);
    assert.equal(profile.capabilities.every((capability) => capability.state === "blocked"), true);
  }
});

test("32k coding candidates preserve exact family boundaries without enabling admission", () => {
  const catalog = validateCatalog();
  const profiles = catalog.profiles.filter((profile) =>
    profile.profile_id.endsWith("text-32k-fedora-coding-development"),
  );
  assert.deepEqual(profiles.map((profile) => profile.family), ["muse_glimmer", "gpt_oss"]);
  for (const profile of profiles) {
    assert.equal(profile.context.max_context_tokens, 32768);
    assert.equal(profile.decoding.max_output_tokens, 4096);
    assert.equal(profile.codec.reasoning_enabled, true);
    assert.equal(profile.enabled, false);
    assert.equal(profile.automatic_fallback, false);
    assert.equal(profile.capabilities.every((capability) => capability.state === "blocked"), true);
    assert.equal(
      profile.capabilities.every((capability) =>
        capability.limitations.includes("coding-campaign-not-run")),
      true,
    );
  }
  assert.equal(profiles[0].codec.tool_protocol_version.includes("atem"), true);
  assert.equal(profiles[1].codec.tool_protocol_version.includes("harmony"), true);
  assert.equal(
    profiles[1].capabilities.every((capability) =>
      capability.limitations.includes("conversion-equivalence-not-established")),
    true,
  );
});

test("fake Muse and Gemma entries are synthetic, exact, disabled, and non-fallback", () => {
  const catalog = validateCatalog();
  const profiles = catalog.profiles.filter((profile) => profile.runtime.kind === "deterministic_fake");
  assert.deepEqual(profiles.map((profile) => profile.family), [
    "deterministic_fake_muse",
    "deterministic_fake_gemma",
  ]);
  for (const profile of profiles) {
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
