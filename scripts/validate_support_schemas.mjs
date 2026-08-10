#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

export const SUPPORT_RECORD_TYPES = Object.freeze([
  "vulnerability-support-policy",
]);

function readJson(relativePath) {
  return JSON.parse(fs.readFileSync(path.join(ROOT, relativePath), "utf8"));
}

export function createSupportValidators() {
  const ajv = new Ajv2020({
    allErrors: true,
    coerceTypes: false,
    removeAdditional: false,
    strict: true,
    useDefaults: false,
  });
  addFormats(ajv);

  return Object.fromEntries(
    SUPPORT_RECORD_TYPES.map((recordType) => {
      const schema = readJson(`schemas/support/${recordType}.schema.json`);
      return [recordType, ajv.compile(schema)];
    }),
  );
}

export function validateSupportRecord(recordType, data, validators) {
  const validator = validators[recordType];
  if (!validator) {
    throw new Error(`unknown support record type: ${recordType}`);
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

export function validateSupportRecords() {
  const validators = createSupportValidators();
  return SUPPORT_RECORD_TYPES.map((recordType) => ({
    recordType,
    ...validateSupportRecord(
      recordType,
      readJson(`support/${recordType}.json`),
      validators,
    ),
  }));
}

function main() {
  const failures = validateSupportRecords().filter((result) => !result.valid);
  if (failures.length > 0) {
    for (const failure of failures) {
      console.error(`${failure.recordType}: validation failed`);
      for (const error of failure.schemaErrors) {
        console.error(`- ${error}`);
      }
    }
    return 1;
  }
  console.log(`Validated ${SUPPORT_RECORD_TYPES.length} support record(s).`);
  return 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exitCode = main();
}
