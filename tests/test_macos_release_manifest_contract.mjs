import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

import {
  BLOCKERS,
  FIXTURE_PATH,
  REPORT_PATH,
  ROOT,
  buildReport,
  createValidator,
  resolveRevision,
  validateManifest,
  validateReport,
} from "../scripts/macos_release_manifest_contract.mjs";


const fixture = () => JSON.parse(fs.readFileSync(path.join(ROOT, FIXTURE_PATH), "utf8"));


test("closed schema accepts only the exact synthetic contract fixture", () => {
  const value = fixture();
  const validator = createValidator();
  assert.equal(validator(value), true, JSON.stringify(validator.errors));
  assert.deepEqual(validateManifest(value), []);
  for (const mutate of [
    (item) => { delete item.platform; },
    (item) => { item.unknown = true; },
    (item) => { item.platform.architecture = "x86_64"; },
    (item) => { item.code_identity.team_id = "SHORT"; },
    (item) => { item.component_hashes.kernel_host = "0".repeat(64); },
  ]) {
    const changed = fixture();
    mutate(changed);
    assert.equal(validator(changed), false);
  }
});


test("every component identity and hash remains complete and distinct", () => {
  for (const mutate of [
    (item) => { delete item.code_identity.bundle_identifiers.vscode_bridge; },
    (item) => { item.code_identity.bundle_identifiers.vscode_bridge = item.code_identity.bundle_identifiers.kernel_host; },
    (item) => { item.code_identity.designated_requirements.kernel_host = "anchor apple generic"; },
    (item) => { item.component_hashes.xpc_tool_helper = item.component_hashes.kernel_host; },
    (item) => { item.package.sha256 = item.component_hashes.kernel_host; },
  ]) {
    const changed = fixture();
    mutate(changed);
    assert.notDeepEqual(validateManifest(changed), []);
  }
});


test("minimal entitlements exclude network and release promotion", () => {
  for (const mutate of [
    (item) => { item.entitlements.kernel_host.push("com.apple.security.network.client"); },
    (item) => { item.entitlements.kernel_host = ["com.apple.security.app-sandbox"]; },
    (item) => { item.entitlements.xpc_tool_helper.push("com.apple.security.inherit"); },
    (item) => { item.entitlements.xpc_tool_helper.push("com.apple.security.application-groups"); },
    (item) => { item.entitlements.metal_inference_service.push("com.apple.security.application-groups"); },
    (item) => { item.entitlements.metal_inference_service = ["com.apple.security.application-groups"]; },
    (item) => { item.status = "signed-release"; },
    (item) => { item.macos_execution_performed = true; },
    (item) => { item.release_claim = "signed-package-candidate"; },
    (item) => { item.credential_values_present = true; },
  ]) {
    const changed = fixture();
    mutate(changed);
    assert.notDeepEqual(validateManifest(changed), []);
  }
});


test("retained report matches immutable sources and preserves every blocker", () => {
  const report = JSON.parse(fs.readFileSync(path.join(ROOT, REPORT_PATH), "utf8"));
  assert.deepEqual(report, buildReport(resolveRevision(report.source_revision)));
  assert.deepEqual(report.blockers, BLOCKERS);
  assert.deepEqual(validateReport(report), []);
});


test("report rejects signing execution product and release overclaims", () => {
  const baseline = JSON.parse(fs.readFileSync(path.join(ROOT, REPORT_PATH), "utf8"));
  for (const [field, value] of [
    ["signing_performed", true],
    ["notarization_performed", true],
    ["installation_performed", true],
    ["m5_execution_performed", true],
    ["product_acceptance_claim", "pass"],
    ["release_claim", "pass"],
  ]) {
    const changed = structuredClone(baseline);
    changed[field] = value;
    assert.notDeepEqual(validateReport(changed, false), []);
  }
  const blockers = structuredClone(baseline);
  blockers.blockers = [];
  assert.notDeepEqual(validateReport(blockers, false), []);
});
