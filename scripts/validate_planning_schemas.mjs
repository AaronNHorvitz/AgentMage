#!/usr/bin/env node

import fs from "node:fs";
import crypto from "node:crypto";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

export const RECORD_TYPES = Object.freeze([
  "decision-record",
  "risk-register",
  "change-log",
  "release-manifest",
  "requirement-supersession",
]);

export const TEMPLATE_TYPES = Object.freeze([
  "decision-record",
  "requirement-supersession",
]);

export const TEST_RECORD_TYPES = Object.freeze([
  "platform-result",
  "fixture-provenance-ledger",
  "fuzz-result",
]);

export const CONFIGURATION_SECTION_TYPES = Object.freeze([
  "core",
  "platform",
  "model",
  "workspace",
  "tool",
  "permission",
  "budget",
  "logging",
  "retention",
  "skill",
  "shell",
]);

export const CONFIGURATION_BUNDLE_TYPE = "agent-configuration";
export const CONFIGURATION_PROFILE_CATALOG_TYPE = "profile-catalog";
export const CONFIGURATION_RESULT_TYPE = "configuration-result";
export const CONFIGURATION_REVIEW_TYPES = Object.freeze([
  "configuration-diff",
  "configuration-rollback-report",
]);
export const RUNTIME_RECORD_TYPES = Object.freeze([
  "single-agent-state-machine",
  "agent-progress-event",
  "session-environment-capture",
  "write-aware-checkpoint",
  "command-preview",
  "command-receipt",
]);
const CONFIGURATION_REPORT_PATH =
  "artifacts/sprints/sprint-3/story-3.1/configuration-schema-report.json";

function readJson(relativePath) {
  return JSON.parse(fs.readFileSync(path.join(ROOT, relativePath), "utf8"));
}

function duplicateValues(values) {
  const seen = new Set();
  const duplicates = new Set();
  for (const value of values) {
    if (seen.has(value)) {
      duplicates.add(value);
    }
    seen.add(value);
  }
  return [...duplicates].sort();
}

function sha256File(relativePath) {
  return crypto
    .createHash("sha256")
    .update(fs.readFileSync(path.join(ROOT, relativePath)))
    .digest("hex");
}

export function createPlanningValidators() {
  const ajv = new Ajv2020({
    allErrors: true,
    coerceTypes: false,
    removeAdditional: false,
    strict: true,
    useDefaults: false,
  });
  addFormats(ajv);

  const common = readJson("schemas/planning/common.schema.json");
  ajv.addSchema(common);

  return Object.fromEntries(
    RECORD_TYPES.map((recordType) => {
      const schema = readJson(`schemas/planning/${recordType}.schema.json`);
      return [recordType, ajv.compile(schema)];
    }),
  );
}

export function createTestingValidators() {
  const ajv = new Ajv2020({
    allErrors: true,
    coerceTypes: false,
    removeAdditional: false,
    strict: true,
    useDefaults: false,
  });
  addFormats(ajv);

  return Object.fromEntries(
    TEST_RECORD_TYPES.map((recordType) => {
      const schema = readJson(`schemas/testing/${recordType}.schema.json`);
      return [recordType, ajv.compile(schema)];
    }),
  );
}

export function createConfigurationValidators() {
  const ajv = new Ajv2020({
    allErrors: true,
    coerceTypes: false,
    removeAdditional: false,
    strict: true,
    useDefaults: false,
  });
  addFormats(ajv);

  const common = readJson("schemas/configuration/common.schema.json");
  ajv.addSchema(common);
  for (const recordType of CONFIGURATION_SECTION_TYPES) {
    ajv.addSchema(
      readJson(`schemas/configuration/${recordType}.schema.json`),
    );
  }
  ajv.addSchema(
    readJson(
      `schemas/configuration/${CONFIGURATION_BUNDLE_TYPE}.schema.json`,
    ),
  );
  for (const recordType of CONFIGURATION_REVIEW_TYPES) {
    ajv.addSchema(
      readJson(`schemas/configuration/${recordType}.schema.json`),
    );
  }
  ajv.addSchema(
    readJson(
      `schemas/configuration/${CONFIGURATION_PROFILE_CATALOG_TYPE}.schema.json`,
    ),
  );
  ajv.addSchema(
    readJson(
      `schemas/configuration/${CONFIGURATION_RESULT_TYPE}.schema.json`,
    ),
  );

  return Object.fromEntries(
    [
      ...CONFIGURATION_SECTION_TYPES,
      CONFIGURATION_BUNDLE_TYPE,
      CONFIGURATION_PROFILE_CATALOG_TYPE,
      CONFIGURATION_RESULT_TYPE,
      ...CONFIGURATION_REVIEW_TYPES,
    ].map(
      (recordType) => {
        const schemaId =
          `https://agentmage.dev/schemas/configuration/${recordType}.schema.json`;
        const validator = ajv.getSchema(schemaId);
        if (!validator) {
          throw new Error(`configuration schema did not register: ${recordType}`);
        }
        return [recordType, validator];
      },
    ),
  );
}

export function createRuntimeValidators() {
  const ajv = new Ajv2020({
    allErrors: true,
    coerceTypes: false,
    removeAdditional: false,
    strict: true,
    useDefaults: false,
  });
  addFormats(ajv);

  return Object.fromEntries(
    RUNTIME_RECORD_TYPES.map((recordType) => {
      const schema = readJson(`schemas/runtime/${recordType}.schema.json`);
      return [recordType, ajv.compile(schema)];
    }),
  );
}

export function validateRuntimeRecord(recordType, data, validators) {
  const validator = validators[recordType];
  if (!validator) {
    throw new Error(`unknown runtime record type: ${recordType}`);
  }
  const valid = validator(data);
  return {
    valid,
    schemaErrors: valid
      ? []
      : (validator.errors ?? []).map((error) =>
        `${error.instancePath || "/"} ${error.message}`,
      ),
  };
}

export function validateRuntimeFixtures() {
  const validators = createRuntimeValidators();
  return RUNTIME_RECORD_TYPES.map((recordType) => ({
    recordType,
    ...validateRuntimeRecord(
      recordType,
      readJson(`schemas/runtime/examples/${recordType}.valid.json`),
      validators,
    ),
  }));
}

export function planningSemanticErrors(recordType, data) {
  const errors = [];

  if (recordType === "decision-record") {
    if (data.supersedes?.includes(data.decision_id)) {
      errors.push("decision record cannot supersede itself");
    }
  } else if (recordType === "risk-register") {
    for (const riskId of duplicateValues((data.risks ?? []).map((risk) => risk.risk_id))) {
      errors.push(`duplicate risk identifier: ${riskId}`);
    }
    for (const risk of data.risks ?? []) {
      if (risk.score !== risk.likelihood * risk.impact) {
        errors.push(
          `${risk.risk_id}: score must equal likelihood multiplied by impact`,
        );
      }
    }
  } else if (recordType === "change-log") {
    for (const changeId of duplicateValues(
      (data.entries ?? []).map((entry) => entry.change_id),
    )) {
      errors.push(`duplicate change identifier: ${changeId}`);
    }
    for (let index = 1; index < (data.entries ?? []).length; index += 1) {
      const previous = data.entries[index - 1];
      const current = data.entries[index];
      if (Date.parse(current.timestamp) < Date.parse(previous.timestamp)) {
        errors.push(
          `${current.change_id}: timestamp precedes ${previous.change_id}`,
        );
      }
    }
  } else if (recordType === "release-manifest") {
    const expectedReleaseId = `agentmage-${data.version}`;
    if (data.release_id !== expectedReleaseId) {
      errors.push(`release_id must equal ${expectedReleaseId}`);
    }
    for (const name of duplicateValues(
      (data.components ?? []).map((component) => component.name),
    )) {
      errors.push(`duplicate component name: ${name}`);
    }
    for (const profileId of duplicateValues(
      (data.model_profiles ?? []).map((profile) => profile.profile_id),
    )) {
      errors.push(`duplicate model profile identifier: ${profileId}`);
    }
  } else if (recordType === "requirement-supersession") {
    const originals = new Set(data.original_requirement_ids ?? []);
    const overlaps = (data.replacement_requirement_ids ?? [])
      .filter((requirementId) => originals.has(requirementId))
      .sort();
    for (const requirementId of overlaps) {
      errors.push(
        `requirement cannot be both original and replacement: ${requirementId}`,
      );
    }
  }

  return errors;
}

export function validatePlanningRecord(recordType, data, validators) {
  const validator = validators[recordType];
  if (!validator) {
    throw new Error(`unknown planning record type: ${recordType}`);
  }

  const schemaValid = validator(data);
  const schemaErrors = schemaValid
    ? []
    : (validator.errors ?? []).map((error) =>
      `${error.instancePath || "/"} ${error.message}`,
    );
  const semanticErrors = schemaValid
    ? planningSemanticErrors(recordType, data)
    : [];

  return {
    valid: schemaErrors.length === 0 && semanticErrors.length === 0,
    schemaErrors,
    semanticErrors,
  };
}

export function validateTestingRecord(recordType, data, validators) {
  const validator = validators[recordType];
  if (!validator) {
    throw new Error(`unknown testing record type: ${recordType}`);
  }
  const valid = validator(data);
  return {
    valid,
    schemaErrors: valid
      ? []
      : (validator.errors ?? []).map((error) =>
        `${error.instancePath || "/"} ${error.message}`,
      ),
  };
}

export function configurationSemanticErrors(recordType, data) {
  if (recordType !== CONFIGURATION_BUNDLE_TYPE) {
    return [];
  }
  const errors = [];
  for (const section of CONFIGURATION_SECTION_TYPES) {
    if (data[section]?.schema_version !== data.schema_version) {
      errors.push(`${section}: schema version differs from bundle`);
    }
  }
  for (const rootId of duplicateValues(
    (data.workspace?.roots ?? []).map((root) => root.root_id),
  )) {
    errors.push(`duplicate workspace root identifier: ${rootId}`);
  }
  for (const toolId of duplicateValues(
    (data.tool?.tools ?? []).map((tool) => tool.tool_id),
  )) {
    errors.push(`duplicate tool identifier: ${toolId}`);
  }
  const allowed = new Set(data.permission?.allowed_capabilities ?? []);
  for (const tool of data.tool?.tools ?? []) {
    if (!tool.enabled) {
      continue;
    }
    for (const capability of tool.required_capabilities ?? []) {
      if (!allowed.has(capability)) {
        errors.push(
          `${tool.tool_id}: required capability is outside permission ceiling: ${capability}`,
        );
      }
    }
  }
  for (const capability of data.skill?.capability_ceiling ?? []) {
    if (!allowed.has(capability)) {
      errors.push(
        `skill capability is outside permission ceiling: ${capability}`,
      );
    }
  }
  if (
    data.model?.decoding?.maximum_context_tokens
      > data.budget?.maximum_context_tokens
  ) {
    errors.push("model context exceeds the global context budget");
  }
  if (
    data.model?.decoding?.maximum_output_tokens
      > data.budget?.maximum_output_tokens
  ) {
    errors.push("model output exceeds the global output-token budget");
  }
  return errors;
}

export function validateConfigurationRecord(recordType, data, validators) {
  const validator = validators[recordType];
  if (!validator) {
    throw new Error(`unknown configuration record type: ${recordType}`);
  }
  const schemaValid = validator(data);
  const schemaErrors = schemaValid
    ? []
    : (validator.errors ?? []).map((error) =>
      `${error.instancePath || "/"} ${error.message}`,
    );
  const semanticErrors = schemaValid
    ? configurationSemanticErrors(recordType, data)
    : [];
  return {
    valid: schemaErrors.length === 0 && semanticErrors.length === 0,
    schemaErrors,
    semanticErrors,
  };
}

export function validateConfigurationFixtures() {
  const validators = createConfigurationValidators();
  const bundle = readJson(
    "schemas/configuration/examples/agent-configuration.valid.json",
  );
  const results = CONFIGURATION_SECTION_TYPES.map((recordType) => ({
    recordType,
    ...validateConfigurationRecord(recordType, bundle[recordType], validators),
  }));
  results.push({
    recordType: CONFIGURATION_BUNDLE_TYPE,
    ...validateConfigurationRecord(
      CONFIGURATION_BUNDLE_TYPE,
      bundle,
      validators,
    ),
  });
  return results;
}

export function validateConfigurationProfiles() {
  const validators = createConfigurationValidators();
  const catalog = readJson("configuration/profiles/catalog.json");
  const results = [
    {
      recordType: CONFIGURATION_PROFILE_CATALOG_TYPE,
      ...validateConfigurationRecord(
        CONFIGURATION_PROFILE_CATALOG_TYPE,
        catalog,
        validators,
      ),
    },
  ];
  for (const profile of catalog.profiles ?? []) {
    results.push({
      recordType: `configuration-profile[${profile.profile_id}]`,
      ...validateConfigurationRecord(
        CONFIGURATION_BUNDLE_TYPE,
        readJson(profile.configuration_path),
        validators,
      ),
    });
  }
  return results;
}

export function validateConfigurationResultFixtures() {
  const validators = createConfigurationValidators();
  return ["session", "release"].map((resultKind) => ({
    recordType: `${CONFIGURATION_RESULT_TYPE}[${resultKind}]`,
    ...validateConfigurationRecord(
      CONFIGURATION_RESULT_TYPE,
      readJson(
        `schemas/configuration/examples/configuration-${resultKind}-result.valid.json`,
      ),
      validators,
    ),
  }));
}

export function validateConfigurationReviewFixtures() {
  const validators = createConfigurationValidators();
  return CONFIGURATION_REVIEW_TYPES.map((recordType) => ({
    recordType,
    ...validateConfigurationRecord(
      recordType,
      readJson(`schemas/configuration/examples/${recordType}.valid.json`),
      validators,
    ),
  }));
}

export function buildConfigurationSchemaReport() {
  const validators = createConfigurationValidators();
  const bundlePath =
    "schemas/configuration/examples/agent-configuration.valid.json";
  const bundle = readJson(bundlePath);
  const recordTypes = [
    ...CONFIGURATION_SECTION_TYPES,
    CONFIGURATION_BUNDLE_TYPE,
  ];
  const mutationResults = [];
  for (const recordType of recordTypes) {
    const source = recordType === CONFIGURATION_BUNDLE_TYPE
      ? bundle
      : bundle[recordType];
    const mutations = [];

    const missing = structuredClone(source);
    delete missing.schema_version;
    mutations.push(["missing-schema-version", missing]);

    const unknown = structuredClone(source);
    unknown.unreviewed_extension = true;
    mutations.push(["unknown-field", unknown]);

    const wrongType = structuredClone(source);
    wrongType.schema_version = "1";
    mutations.push(["wrong-type", wrongType]);

    const unsupported = structuredClone(source);
    unsupported.schema_version = source.schema_version + 1;
    mutations.push(["unsupported-version", unsupported]);

    for (const [mutation, changed] of mutations) {
      const result = validateConfigurationRecord(
        recordType,
        changed,
        validators,
      );
      mutationResults.push({
        record_type: recordType,
        mutation,
        rejected: !result.valid,
      });
    }
  }
  if (mutationResults.some((item) => !item.rejected)) {
    throw new Error("configuration schema mutation did not fail closed");
  }
  const schemaPaths = [
    "schemas/configuration/common.schema.json",
    ...recordTypes.map(
      (recordType) =>
        `schemas/configuration/${recordType}.schema.json`,
    ),
  ];
  return {
    schema_version: 1,
    task_id: "3.1.1.1",
    status: "pass-versioned-closed-configuration-schemas",
    formal_validator: {
      engine: "ajv-draft-2020-12",
      coerce_types: false,
      remove_additional: false,
      strict: true,
      use_defaults: false,
    },
    schemas: schemaPaths.map((schemaPath) => ({
      path: schemaPath,
      sha256: sha256File(schemaPath),
    })),
    canonical_fixture: {
      path: bundlePath,
      sha256: sha256File(bundlePath),
    },
    implementation: {
      path: "scripts/validate_planning_schemas.mjs",
      sha256: sha256File("scripts/validate_planning_schemas.mjs"),
    },
    tests: {
      path: "tests/test_planning_schemas.mjs",
      sha256: sha256File("tests/test_planning_schemas.mjs"),
    },
    section_types: [...CONFIGURATION_SECTION_TYPES],
    section_schema_count: CONFIGURATION_SECTION_TYPES.length,
    bundle_schema_count: 1,
    shared_definition_schema_count: 1,
    canonical_validation_count: recordTypes.length,
    mutation_results: mutationResults,
    mutation_count: mutationResults.length,
    rejected_mutation_count: mutationResults.filter((item) => item.rejected).length,
    baseline_controls: {
      startup_failure_policy: "fail-closed",
      default_permission_effect: "deny",
      authority_inheritance: "restrict-only",
      administrator_required: false,
      model_network_access: false,
      shell_network_access: false,
      automatic_model_routing: false,
      model_to_tool_channel: "prohibited",
      prompt_logging: false,
      environment_value_logging: false,
    },
    product_configuration_loader_claim: "none",
    macos_execution_status: "blocked-macos",
    macos_support_claim: "none",
  };
}

export function validateConfigurationSchemaReport() {
  const expected = buildConfigurationSchemaReport();
  let actual;
  try {
    actual = readJson(CONFIGURATION_REPORT_PATH);
  } catch (error) {
    return [`cannot read configuration schema report: ${error.message}`];
  }
  return JSON.stringify(actual) === JSON.stringify(expected)
    ? []
    : ["configuration schema report is stale or non-deterministic"];
}

export function validateTestingFixtures() {
  const validators = createTestingValidators();
  const platformReport = readJson(
    "artifacts/sprints/sprint-2/story-2.1/platform-result-recorder-report.json",
  );
  const results = platformReport.synthetic_record_set.records.map(
    (record, index) => ({
      recordType: `platform-result[${index}]`,
      ...validateTestingRecord("platform-result", record, validators),
    }),
  );
  const ledger = readJson("fixtures/corpus/v1/provenance-ledger.json");
  results.push({
    recordType: "fixture-provenance-ledger",
    ...validateTestingRecord(
      "fixture-provenance-ledger",
      ledger,
      validators,
    ),
  });
  const fuzzResult = readJson(
    "schemas/testing/examples/fuzz-result.valid.json",
  );
  results.push({
    recordType: "fuzz-result",
    ...validateTestingRecord("fuzz-result", fuzzResult, validators),
  });
  const seededFailureReport = readJson(
    "artifacts/sprints/sprint-2/story-2.2/seeded-failure-report.json",
  );
  results.push(
    ...seededFailureReport.results.map((record, index) => ({
      recordType: `seeded-fuzz-result[${index}]`,
      ...validateTestingRecord("fuzz-result", record, validators),
    })),
  );
  return results;
}

export function validatePlanningFixtures() {
  const validators = createPlanningValidators();
  const results = [];
  for (const recordType of RECORD_TYPES) {
    const fixture = readJson(
      `schemas/planning/examples/${recordType}.valid.json`,
    );
    results.push({
      recordType,
      ...validatePlanningRecord(recordType, fixture, validators),
    });
  }
  return results;
}

export function validatePlanningTemplates() {
  const validators = createPlanningValidators();
  const results = [];
  for (const recordType of TEMPLATE_TYPES) {
    const template = readJson(
      `schemas/planning/templates/${recordType}.template.json`,
    );
    results.push({
      recordType,
      ...validatePlanningRecord(recordType, template, validators),
    });
  }
  return results;
}

function main() {
  if (process.argv.includes("--write-configuration-report")) {
    const report = buildConfigurationSchemaReport();
    const reportPath = path.join(ROOT, CONFIGURATION_REPORT_PATH);
    fs.mkdirSync(path.dirname(reportPath), { recursive: true });
    fs.writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);
  }
  const planningResults = [
    ...validatePlanningFixtures(),
    ...validatePlanningTemplates(),
  ];
  const testingResults = validateTestingFixtures();
  const configurationResults = validateConfigurationFixtures();
  const configurationProfileResults = validateConfigurationProfiles();
  const configurationBoundResultResults =
    validateConfigurationResultFixtures();
  const configurationReviewResults = validateConfigurationReviewFixtures();
  const runtimeResults = validateRuntimeFixtures();
  const results = [
    ...planningResults,
    ...testingResults,
    ...configurationResults,
    ...configurationProfileResults,
    ...configurationBoundResultResults,
    ...configurationReviewResults,
    ...runtimeResults,
  ];
  const failures = results.filter((result) => !result.valid);
  if (process.argv.includes("--legacy-report-currentness")) {
    for (const failure of validateConfigurationSchemaReport()) {
      failures.push({
        recordType: "configuration-schema-report",
        schemaErrors: [failure],
        semanticErrors: [],
        valid: false,
      });
    }
  }
  if (failures.length > 0) {
    for (const failure of failures) {
      console.error(`${failure.recordType}: validation failed`);
      for (const error of [
        ...failure.schemaErrors,
        ...(failure.semanticErrors ?? []),
      ]) {
        console.error(`- ${error}`);
      }
    }
    return 1;
  }

  console.log(
    `Validated ${planningResults.length} planning and `
      + `${testingResults.length} testing and `
      + `${configurationResults.length} configuration schema fixture(s), plus `
      + `${configurationProfileResults.length} profile record(s) and `
      + `${configurationBoundResultResults.length} configuration-bound result(s) and `
      + `${configurationReviewResults.length} review record(s) and `
      + `${runtimeResults.length} runtime record(s).`,
  );
  return 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exitCode = main();
}
