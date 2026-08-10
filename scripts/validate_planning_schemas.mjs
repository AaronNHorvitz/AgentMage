#!/usr/bin/env node

import fs from "node:fs";
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
  const planningResults = [
    ...validatePlanningFixtures(),
    ...validatePlanningTemplates(),
  ];
  const testingResults = validateTestingFixtures();
  const results = [...planningResults, ...testingResults];
  const failures = results.filter((result) => !result.valid);
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
      + `${testingResults.length} testing schema fixture(s).`,
  );
  return 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exitCode = main();
}
