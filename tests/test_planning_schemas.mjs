import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  RECORD_TYPES,
  TEST_RECORD_TYPES,
  TEMPLATE_TYPES,
  createPlanningValidators,
  createTestingValidators,
  validatePlanningRecord,
  validatePlanningTemplates,
  validateTestingFixtures,
  validateTestingRecord,
} from "../scripts/validate_planning_schemas.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const validators = createPlanningValidators();
const testingValidators = createTestingValidators();

function fixture(recordType) {
  const fixturePath = path.join(
    ROOT,
    "schemas",
    "planning",
    "examples",
    `${recordType}.valid.json`,
  );
  return JSON.parse(fs.readFileSync(fixturePath, "utf8"));
}

function validate(recordType, data) {
  return validatePlanningRecord(recordType, data, validators);
}

function template(recordType) {
  const templatePath = path.join(
    ROOT,
    "schemas",
    "planning",
    "templates",
    `${recordType}.template.json`,
  );
  return JSON.parse(fs.readFileSync(templatePath, "utf8"));
}

test("all canonical examples satisfy their schema and semantic contract", () => {
  for (const recordType of RECORD_TYPES) {
    assert.deepEqual(validate(recordType, fixture(recordType)), {
      valid: true,
      schemaErrors: [],
      semanticErrors: [],
    });
  }
});

test("decision and supersession templates are valid unapproved drafts", () => {
  assert.equal(validatePlanningTemplates().length, TEMPLATE_TYPES.length);
  for (const recordType of TEMPLATE_TYPES) {
    const draft = template(recordType);
    assert.deepEqual(validate(recordType, draft), {
      valid: true,
      schemaErrors: [],
      semanticErrors: [],
    });
    assert.equal(draft.status, "proposed");
    assert.deepEqual(draft.approved_by, []);
    assert.deepEqual(draft.evidence, []);
    assert.equal(draft.date, "1970-01-01");
  }
});

test("templates cannot be represented as accepted without approval evidence", () => {
  const decision = template("decision-record");
  decision.status = "accepted";
  assert.equal(validate("decision-record", decision).valid, false);

  const supersession = template("requirement-supersession");
  supersession.status = "accepted";
  assert.equal(validate("requirement-supersession", supersession).valid, false);
});

test("validation does not mutate its input", () => {
  for (const recordType of RECORD_TYPES) {
    const input = fixture(recordType);
    const before = structuredClone(input);
    validate(recordType, input);
    assert.deepEqual(input, before);
  }
});

test("every record rejects missing required fields and unknown properties", () => {
  for (const recordType of RECORD_TYPES) {
    const missingVersion = fixture(recordType);
    delete missingVersion.schema_version;
    assert.equal(validate(recordType, missingVersion).valid, false);

    const unknownProperty = fixture(recordType);
    unknownProperty.unreviewed_extension = true;
    assert.equal(validate(recordType, unknownProperty).valid, false);
  }
});

test("shared identities, paths, hashes, dates, and commits are strict", () => {
  const decision = fixture("decision-record");
  decision.decision_id = "adr-2";
  decision.date = "08/10/2026";
  decision.evidence[0].path = "../outside.json";
  decision.evidence[0].sha256 = "not-a-hash";
  const decisionResult = validate("decision-record", decision);
  assert.equal(decisionResult.valid, false);
  assert.ok(decisionResult.schemaErrors.length >= 4);

  const changeLog = fixture("change-log");
  changeLog.entries[0].commit = "abc123";
  assert.equal(validate("change-log", changeLog).valid, false);
});

test("accepted decisions require approval evidence and cannot supersede themselves", () => {
  const missingApproval = fixture("decision-record");
  missingApproval.approved_by = [];
  missingApproval.evidence = [];
  assert.equal(validate("decision-record", missingApproval).valid, false);

  const selfSupersession = fixture("decision-record");
  selfSupersession.supersedes = [selfSupersession.decision_id];
  const result = validate("decision-record", selfSupersession);
  assert.equal(result.valid, false);
  assert.match(result.semanticErrors[0], /cannot supersede itself/);
});

test("risk registers reject duplicate identities and incorrect calculated scores", () => {
  const riskRegister = fixture("risk-register");
  const duplicate = structuredClone(riskRegister.risks[0]);
  duplicate.score = 7;
  riskRegister.risks.push(duplicate);
  const result = validate("risk-register", riskRegister);
  assert.equal(result.valid, false);
  assert.ok(result.semanticErrors.some((error) => error.includes("duplicate risk")));
  assert.ok(result.semanticErrors.some((error) => error.includes("score must equal")));
});

test("accepted risks require a decision and evidence", () => {
  const riskRegister = fixture("risk-register");
  riskRegister.risks[0].status = "accepted";
  riskRegister.risks[0].evidence = [];
  const result = validate("risk-register", riskRegister);
  assert.equal(result.valid, false);
  assert.ok(result.schemaErrors.length >= 2);
});

test("change logs reject duplicate identities and decreasing timestamps", () => {
  const changeLog = fixture("change-log");
  const duplicate = structuredClone(changeLog.entries[0]);
  duplicate.timestamp = "2026-08-09T18:00:00Z";
  changeLog.entries.push(duplicate);
  const result = validate("change-log", changeLog);
  assert.equal(result.valid, false);
  assert.ok(result.semanticErrors.some((error) => error.includes("duplicate change")));
  assert.ok(result.semanticErrors.some((error) => error.includes("timestamp precedes")));
});

test("release manifests bind release IDs and require unique component and profile identities", () => {
  const release = fixture("release-manifest");
  release.version = "0.1.1";
  release.components.push(structuredClone(release.components[0]));
  release.model_profiles.push(structuredClone(release.model_profiles[0]));
  const result = validate("release-manifest", release);
  assert.equal(result.valid, false);
  assert.ok(result.semanticErrors.some((error) => error.includes("release_id")));
  assert.ok(result.semanticErrors.some((error) => error.includes("component name")));
  assert.ok(result.semanticErrors.some((error) => error.includes("model profile")));
});

test("released manifests require a signer and release evidence", () => {
  const release = fixture("release-manifest");
  release.signer = null;
  release.evidence = [];
  assert.equal(validate("release-manifest", release).valid, false);
});

test("accepted supersessions require approval and disjoint requirement sets", () => {
  const missingApproval = fixture("requirement-supersession");
  missingApproval.decision_id = null;
  missingApproval.approved_by = [];
  missingApproval.approved_at = null;
  missingApproval.evidence = [];
  assert.equal(validate("requirement-supersession", missingApproval).valid, false);

  const overlap = fixture("requirement-supersession");
  overlap.replacement_requirement_ids = [...overlap.original_requirement_ids];
  const result = validate("requirement-supersession", overlap);
  assert.equal(result.valid, false);
  assert.match(result.semanticErrors[0], /both original and replacement/);
});

test("unknown planning record types fail explicitly", () => {
  assert.throws(
    () => validatePlanningRecord("unknown-record", {}, validators),
    /unknown planning record type/,
  );
});

test("platform results and the fixture ledger satisfy their testing schemas", () => {
  const results = validateTestingFixtures();
  assert.equal(results.length, 3);
  assert.deepEqual(
    results.map((result) => result.valid),
    [true, true, true],
  );
  assert.deepEqual(TEST_RECORD_TYPES, [
    "platform-result",
    "fixture-provenance-ledger",
  ]);
});

test("testing schemas reject weakened platform and provenance controls", () => {
  const platformReport = JSON.parse(
    fs.readFileSync(
      path.join(
        ROOT,
        "artifacts/sprints/sprint-2/story-2.1/platform-result-recorder-report.json",
      ),
      "utf8",
    ),
  );
  const platformRecord = structuredClone(
    platformReport.synthetic_record_set.records[0],
  );
  platformRecord.environment.hostname = "fixture-host";
  platformRecord.macos_support_claim = "supported";
  assert.equal(
    validateTestingRecord(
      "platform-result",
      platformRecord,
      testingValidators,
    ).valid,
    false,
  );

  const ledger = JSON.parse(
    fs.readFileSync(
      path.join(ROOT, "fixtures/corpus/v1/provenance-ledger.json"),
      "utf8",
    ),
  );
  ledger.controls.acyclic = false;
  ledger.scope.private_user_data = true;
  assert.equal(
    validateTestingRecord(
      "fixture-provenance-ledger",
      ledger,
      testingValidators,
    ).valid,
    false,
  );
});

test("unknown testing record types fail explicitly", () => {
  assert.throws(
    () => validateTestingRecord("unknown-record", {}, testingValidators),
    /unknown testing record type/,
  );
});
