#!/usr/bin/env node

import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import Ajv2020 from "ajv/dist/2020.js";


export const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
export const SCHEMA_PATH = "schemas/platform/macos-release-manifest.schema.json";
export const FIXTURE_PATH = "release/platform-manifests/macos/v1/contract-fixture.json";
export const REPORT_PATH = "artifacts/sprints/sprint-7/story-7.1/macos-release-manifest-contract.json";
export const COMPONENTS = Object.freeze([
  "kernel_host",
  "vscode_bridge",
  "xpc_tool_helper",
  "metal_inference_service",
]);
export const SOURCE_PATHS = Object.freeze([
  "release/platform-manifests/README.md",
  SCHEMA_PATH,
  FIXTURE_PATH,
  "scripts/macos_release_manifest_contract.mjs",
  "tests/test_macos_release_manifest_contract.mjs",
]);
export const BLOCKERS = Object.freeze([
  "macos-native-components-not-implemented",
  "release-derived-identities-and-component-hashes-unavailable",
  "signing-notarization-stapling-and-gatekeeper-not-executed",
  "apple-silicon-m5-installed-product-evidence-unavailable",
  "independent-critical-boundary-review-not-performed",
]);


function readJson(relativePath) {
  return JSON.parse(fs.readFileSync(path.join(ROOT, relativePath), "utf8"));
}


function sha256(value) {
  return crypto.createHash("sha256").update(value).digest("hex");
}


function canonicalReport(value) {
  const order = (item) => {
    if (Array.isArray(item)) return item.map(order);
    if (item !== null && typeof item === "object") {
      return Object.fromEntries(Object.keys(item).sort().map((key) => [key, order(item[key])]));
    }
    return item;
  };
  return `${JSON.stringify(order(value), null, 2)}\n`;
}


export function createValidator() {
  const ajv = new Ajv2020({
    allErrors: true,
    coerceTypes: false,
    removeAdditional: false,
    strict: true,
    useDefaults: false,
  });
  return ajv.compile(readJson(SCHEMA_PATH));
}


export function validateManifest(value) {
  const validator = createValidator();
  const failures = [];
  if (!validator(value)) {
    failures.push(`schema validation failed: ${JSON.stringify(validator.errors)}`);
    return failures;
  }
  if (
    value.status !== "contract-fixture" ||
    value.identity_class !== "synthetic-contract-fixture" ||
    value.code_identity.team_id !== "AAAAAAAAAA" ||
    value.macos_execution_performed !== false ||
    value.release_claim !== "none"
  ) {
    failures.push("contract fixture identity or no-release disposition changed");
  }
  if (
    value.platform.minimum_macos_version !== "15.0" ||
    value.platform.architecture !== "arm64" ||
    value.toolchain.swift_toolchain.version !== "6.0"
  ) {
    failures.push("minimum platform or toolchain contract changed");
  }
  const componentSets = [
    value.code_identity.bundle_identifiers,
    value.code_identity.designated_requirements,
    value.entitlements,
    value.component_hashes,
  ];
  for (const record of componentSets) {
    if (JSON.stringify(Object.keys(record).sort()) !== JSON.stringify([...COMPONENTS].sort())) {
      failures.push("macOS component closure changed");
    }
  }
  const bundles = Object.values(value.code_identity.bundle_identifiers);
  if (
    new Set(bundles).size !== COMPONENTS.length ||
    bundles.some((item) => !item.startsWith("com.example.agentmage.contractfixture.")) ||
    value.code_identity.app_group_identifier !== "group.com.example.agentmage.contractfixture"
  ) {
    failures.push("synthetic bundle or App Group identity changed");
  }
  for (const component of COMPONENTS) {
    const requirement = value.code_identity.designated_requirements[component];
    if (
      !requirement.includes(`identifier ${value.code_identity.bundle_identifiers[component]}`) ||
      !requirement.includes("certificate leaf[subject.OU] = AAAAAAAAAA")
    ) {
      failures.push(`designated requirement is not bundle and Team ID bound: ${component}`);
    }
  }
  const forbiddenEntitlements = new Set([
    "com.apple.security.network.client",
    "com.apple.security.network.server",
    "com.apple.security.get-task-allow",
    "com.apple.security.cs.allow-jit",
    "com.apple.security.cs.disable-library-validation",
  ]);
  const entitlementValues = Object.values(value.entitlements).flat();
  if (
    entitlementValues.some((item) => forbiddenEntitlements.has(item)) ||
    !value.entitlements.kernel_host.includes("com.apple.security.files.user-selected.read-only") ||
    !value.entitlements.kernel_host.includes("keychain-access-groups") ||
    JSON.stringify(value.entitlements.xpc_tool_helper) !== JSON.stringify([
      "com.apple.security.app-sandbox",
      "com.apple.security.files.bookmarks.app-scope",
    ]) ||
    JSON.stringify(value.entitlements.metal_inference_service) !== JSON.stringify([
      "com.apple.security.app-sandbox",
    ])
  ) {
    failures.push("minimal entitlement or no-network contract changed");
  }
  const hashes = [
    value.platform.tested_macos_build.sha256,
    value.toolchain.apple_sdk.sha256,
    value.toolchain.swift_toolchain.sha256,
    value.vscode.sha256,
    ...Object.values(value.component_hashes),
    value.package.sha256,
  ];
  if (new Set(hashes).size !== hashes.length) {
    failures.push("synthetic identity hashes are not independently bound");
  }
  if (value.credential_values_present || value.private_environment_values_present) {
    failures.push("contract fixture retained a credential or private environment value");
  }
  return failures;
}


function gitOutput(args) {
  return execFileSync("git", args, { cwd: ROOT, encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] }).trim();
}


function gitBytes(revision, relativePath) {
  return execFileSync("git", ["show", `${revision}:${relativePath}`], {
    cwd: ROOT,
    encoding: null,
    stdio: ["ignore", "pipe", "ignore"],
  });
}


export function resolveRevision(candidate = "HEAD") {
  const revision = gitOutput(["rev-parse", "--verify", `${candidate}^{commit}`]);
  if (!/^[0-9a-f]{40}$/.test(revision)) throw new Error("macOS manifest source revision is unavailable");
  return revision;
}


export function buildReport(sourceRevision) {
  const fixture = readJson(FIXTURE_PATH);
  const failures = validateManifest(fixture);
  if (failures.length) throw new Error(failures.join("; "));
  const sources = SOURCE_PATHS.map((relativePath) => {
    const committed = gitBytes(sourceRevision, relativePath);
    const current = fs.readFileSync(path.join(ROOT, relativePath));
    if (!committed.equals(current)) throw new Error(`source differs from review revision: ${relativePath}`);
    return { path: relativePath, bytes: committed.length, sha256: sha256(committed) };
  });
  return {
    schema_version: 1,
    task_id: "7.1.1.2",
    artifact_id: "macos-release-manifest-field-contract",
    status: "pass-frozen-contract-only",
    source_revision: sourceRevision,
    sources,
    field_closure: {
      top_level_field_count: Object.keys(fixture).length,
      component_count: COMPONENTS.length,
      designated_requirement_count: Object.keys(fixture.code_identity.designated_requirements).length,
      entitlement_assignment_count: Object.values(fixture.entitlements).flat().length,
      component_hash_count: Object.keys(fixture.component_hashes).length,
    },
    platform: {
      family: fixture.platform.family,
      minimum_macos_version: fixture.platform.minimum_macos_version,
      architecture: fixture.platform.architecture,
      swift_version: fixture.toolchain.swift_toolchain.version,
    },
    synthetic_identity_markers: {
      team_id: fixture.code_identity.team_id,
      bundle_prefix: "com.example.agentmage.contractfixture.",
      app_group_identifier: fixture.code_identity.app_group_identifier,
      identity_class: fixture.identity_class,
    },
    blockers: [...BLOCKERS],
    credential_values_present: false,
    private_environment_values_present: false,
    macos_build_performed: false,
    signing_performed: false,
    notarization_performed: false,
    installation_performed: false,
    m5_execution_performed: false,
    product_acceptance_claim: "none",
    release_claim: "none",
  };
}


export function validateReport(value, verifyCurrent = true) {
  const failures = [];
  if (
    value?.schema_version !== 1 ||
    value?.task_id !== "7.1.1.2" ||
    value?.artifact_id !== "macos-release-manifest-field-contract" ||
    value?.status !== "pass-frozen-contract-only" ||
    !/^[0-9a-f]{40}$/.test(value?.source_revision ?? "")
  ) failures.push("macOS manifest contract report identity changed");
  if (
    value?.field_closure?.top_level_field_count !== 16 ||
    value?.field_closure?.component_count !== 4 ||
    value?.field_closure?.designated_requirement_count !== 4 ||
    value?.field_closure?.entitlement_assignment_count !== 10 ||
    value?.field_closure?.component_hash_count !== 4
  ) failures.push("macOS manifest field closure changed");
  if (JSON.stringify(value?.blockers) !== JSON.stringify(BLOCKERS)) {
    failures.push("macOS manifest blockers changed");
  }
  for (const key of [
    "credential_values_present",
    "private_environment_values_present",
    "macos_build_performed",
    "signing_performed",
    "notarization_performed",
    "installation_performed",
    "m5_execution_performed",
  ]) {
    if (value?.[key] !== false) failures.push(`unsupported macOS claim: ${key}`);
  }
  if (value?.product_acceptance_claim !== "none" || value?.release_claim !== "none") {
    failures.push("macOS manifest report made a product or release claim");
  }
  if (verifyCurrent && failures.length === 0) {
    try {
      if (canonicalReport(value) !== canonicalReport(buildReport(value.source_revision))) {
        failures.push("macOS manifest contract report is stale or widened");
      }
    } catch (error) {
      failures.push(`cannot rebuild macOS manifest contract report: ${error.message}`);
    }
  }
  return failures;
}


function writeAtomic(relativePath, content) {
  const destination = path.join(ROOT, relativePath);
  fs.mkdirSync(path.dirname(destination), { recursive: true });
  const temporary = path.join(path.dirname(destination), `.agentmage-macos-manifest-${process.pid}-${crypto.randomUUID()}`);
  fs.writeFileSync(temporary, content, { encoding: "utf8", mode: 0o644, flag: "wx" });
  fs.renameSync(temporary, destination);
}


function main() {
  const args = process.argv.slice(2);
  const write = args.includes("--write");
  const validateOnly = args.includes("--validate-only");
  const sourceIndex = args.indexOf("--source-revision");
  const sourceRevision = resolveRevision(sourceIndex >= 0 ? args[sourceIndex + 1] : "HEAD");
  const fixtureFailures = validateManifest(readJson(FIXTURE_PATH));
  if (fixtureFailures.length) throw new Error(fixtureFailures.join("; "));
  if (validateOnly) {
    process.stdout.write("macOS release manifest schema and synthetic fixture validated without execution\n");
    return;
  }
  if (write) writeAtomic(REPORT_PATH, canonicalReport(buildReport(sourceRevision)));
  const report = readJson(REPORT_PATH);
  const failures = validateReport(report);
  if (failures.length) throw new Error(failures.join("; "));
  process.stdout.write("macOS release manifest field contract passed without platform or release promotion\n");
}


if (path.resolve(process.argv[1] ?? "") === fileURLToPath(import.meta.url)) {
  try {
    main();
  } catch (error) {
    process.stderr.write(`macOS release manifest contract failed: ${error.message}\n`);
    process.exitCode = 1;
  }
}
