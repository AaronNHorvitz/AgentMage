#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function readJson(relativePath) {
  return JSON.parse(fs.readFileSync(path.join(ROOT, relativePath), "utf8"));
}

export function createValidator() {
  const ajv = new Ajv2020({
    allErrors: true,
    coerceTypes: false,
    removeAdditional: false,
    strict: true,
    useDefaults: false,
  });
  addFormats(ajv);
  return ajv.compile(
    readJson("schemas/support/emergency-disable-policy.schema.json"),
  );
}

export function readFixture() {
  return readJson("schemas/support/examples/emergency-disable-policy.valid.json");
}

export function validateRecord(data, validator = createValidator()) {
  const valid = validator(data);
  return {
    valid,
    errors: valid
      ? []
      : (validator.errors ?? []).map((error) =>
        `${error.instancePath || "/"} ${error.message}`,
      ),
  };
}

function main() {
  const result = validateRecord(readFixture());
  if (!result.valid) {
    for (const error of result.errors) {
      console.error(`emergency-disable-policy: ${error}`);
    }
    return 1;
  }
  console.log("Validated the synthetic local emergency-disable policy fixture.");
  return 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exitCode = main();
}
