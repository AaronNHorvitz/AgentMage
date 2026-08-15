import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  CONFIGURATION_BUNDLE_TYPE,
  CONFIGURATION_PROFILE_CATALOG_TYPE,
  CONFIGURATION_RESULT_TYPE,
  CONFIGURATION_REVIEW_TYPES,
  CONFIGURATION_SECTION_TYPES,
  RECORD_TYPES,
  RUNTIME_RECORD_TYPES,
  TEST_RECORD_TYPES,
  TEMPLATE_TYPES,
  buildConfigurationSchemaReport,
  createConfigurationValidators,
  createPlanningValidators,
  createRuntimeValidators,
  createTestingValidators,
  validateConfigurationFixtures,
  validateConfigurationRecord,
  validateConfigurationProfiles,
  validateConfigurationResultFixtures,
  validateConfigurationReviewFixtures,
  validateConfigurationSchemaReport,
  validatePlanningRecord,
  validatePlanningTemplates,
  validateRuntimeFixtures,
  validateRuntimeRecord,
  validateTestingFixtures,
  validateTestingRecord,
} from "../scripts/validate_planning_schemas.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const validators = createPlanningValidators();
const testingValidators = createTestingValidators();
const configurationValidators = createConfigurationValidators();
const runtimeValidators = createRuntimeValidators();

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

function configurationFixture() {
  return JSON.parse(
    fs.readFileSync(
      path.join(
        ROOT,
        "schemas/configuration/examples/agent-configuration.valid.json",
      ),
      "utf8",
    ),
  );
}

function configurationResultFixture(resultKind) {
  return JSON.parse(
    fs.readFileSync(
      path.join(
        ROOT,
        `schemas/configuration/examples/configuration-${resultKind}-result.valid.json`,
      ),
      "utf8",
    ),
  );
}

function validateConfiguration(recordType, data) {
  return validateConfigurationRecord(recordType, data, configurationValidators);
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
  assert.ok(
    result.semanticErrors.some((error) => error.includes("duplicate risk")),
  );
  assert.ok(
    result.semanticErrors.some((error) => error.includes("score must equal")),
  );
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
  assert.ok(
    result.semanticErrors.some((error) => error.includes("duplicate change")),
  );
  assert.ok(
    result.semanticErrors.some((error) => error.includes("timestamp precedes")),
  );
});

test("release manifests bind release IDs and require unique component and profile identities", () => {
  const release = fixture("release-manifest");
  release.version = "0.1.1";
  release.components.push(structuredClone(release.components[0]));
  release.model_profiles.push(structuredClone(release.model_profiles[0]));
  const result = validate("release-manifest", release);
  assert.equal(result.valid, false);
  assert.ok(
    result.semanticErrors.some((error) => error.includes("release_id")),
  );
  assert.ok(
    result.semanticErrors.some((error) => error.includes("component name")),
  );
  assert.ok(
    result.semanticErrors.some((error) => error.includes("model profile")),
  );
});

test("released manifests require a signer and release evidence", () => {
  const release = fixture("release-manifest");
  release.signer = null;
  release.evidence = [];
  assert.equal(validate("release-manifest", release).valid, false);
});

test("release manifests require one exact configuration identity", () => {
  const missingConfiguration = fixture("release-manifest");
  delete missingConfiguration.configuration;
  assert.equal(validate("release-manifest", missingConfiguration).valid, false);

  const invalidHash = fixture("release-manifest");
  invalidHash.configuration.sha256 = "A".repeat(64);
  assert.equal(validate("release-manifest", invalidHash).valid, false);
});

test("accepted supersessions require approval and disjoint requirement sets", () => {
  const missingApproval = fixture("requirement-supersession");
  missingApproval.decision_id = null;
  missingApproval.approved_by = [];
  missingApproval.approved_at = null;
  missingApproval.evidence = [];
  assert.equal(
    validate("requirement-supersession", missingApproval).valid,
    false,
  );

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

test("platform, provenance, and fuzz results satisfy their testing schemas", () => {
  const results = validateTestingFixtures();
  assert.equal(results.length, 10);
  assert.deepEqual(
    results.map((result) => result.valid),
    [true, true, true, true, true, true, true, true, true, true],
  );
  assert.deepEqual(TEST_RECORD_TYPES, [
    "platform-result",
    "fixture-provenance-ledger",
    "fuzz-result",
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
    validateTestingRecord("platform-result", platformRecord, testingValidators)
      .valid,
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

  const fuzzResult = JSON.parse(
    fs.readFileSync(
      path.join(ROOT, "schemas/testing/examples/fuzz-result.valid.json"),
      "utf8",
    ),
  );
  fuzzResult.failure = null;
  fuzzResult.redaction.secret_canary_values_recorded = true;
  fuzzResult.unreviewed_extension = true;
  assert.equal(
    validateTestingRecord("fuzz-result", fuzzResult, testingValidators).valid,
    false,
  );
});

test("unknown testing record types fail explicitly", () => {
  assert.throws(
    () => validateTestingRecord("unknown-record", {}, testingValidators),
    /unknown testing record type/,
  );
});

test("runtime state event and environment fixtures satisfy closed schemas", () => {
  const results = validateRuntimeFixtures();
  assert.deepEqual(RUNTIME_RECORD_TYPES, [
    "single-agent-state-machine",
    "agent-progress-event",
    "session-environment-capture",
    "write-aware-checkpoint",
    "command-preview",
    "command-receipt",
    "validation-receipt",
    "logical-commit-plan",
    "local-review-packet",
    "pinned-commit-signer",
    "candidate-tree-plan",
    "candidate-tree-receipt",
    "local-commit-plan",
    "manual-commit-approval-receipt",
    "local-commit-receipt",
    "repository-preservation-manifest",
    "repository-operation-plan",
    "repository-operation-receipt",
    "worktree-ownership",
    "change-intent-record",
    "reproduction-record",
    "thin-client-request",
    "thin-client-event",
    "frontier-tier-decision",
    "frontier-recommendation-receipt",
  ]);
  assert.deepEqual(
    results.map((result) => result.valid),
    Array(RUNTIME_RECORD_TYPES.length).fill(true),
  );
});

test("validation receipts reject false pass, partial ambiguity, secrets, and unsorted evidence", () => {
  const source = JSON.parse(
    fs.readFileSync(
      path.join(ROOT, "schemas/runtime/examples/validation-receipt.valid.json"),
      "utf8",
    ),
  );
  const zeroTests = structuredClone(source);
  zeroTests.passed = 0;
  assert.equal(
    validateRuntimeRecord("validation-receipt", zeroTests, runtimeValidators)
      .valid,
    false,
  );

  const falsePartial = structuredClone(source);
  falsePartial.coverage = "partial";
  assert.equal(
    validateRuntimeRecord("validation-receipt", falsePartial, runtimeValidators)
      .valid,
    false,
  );

  const leakedSecret = structuredClone(source);
  leakedSecret.secret_match_count = 1;
  assert.equal(
    validateRuntimeRecord("validation-receipt", leakedSecret, runtimeValidators)
      .valid,
    false,
  );

  const unsorted = structuredClone(source);
  unsorted.environment_names = ["TZ", "LANG"];
  assert.equal(
    validateRuntimeRecord("validation-receipt", unsorted, runtimeValidators)
      .valid,
    false,
  );

  const rawOutput = structuredClone(source);
  rawOutput.stdout = "forged green output";
  assert.equal(
    validateRuntimeRecord("validation-receipt", rawOutput, runtimeValidators)
      .valid,
    false,
  );
});

test("logical commit plans reject authority, duplicate operations, and noncanonical groups", () => {
  const source = JSON.parse(
    fs.readFileSync(
      path.join(
        ROOT,
        "schemas/runtime/examples/logical-commit-plan.valid.json",
      ),
      "utf8",
    ),
  );
  for (const mutation of ["authority", "duplicate", "message", "order"]) {
    const changed = structuredClone(source);
    if (mutation === "authority") {
      changed.automatic_commit = true;
    } else if (mutation === "duplicate") {
      changed.groups[1].operation_ids = ["operation-01"];
    } else if (mutation === "message") {
      changed.groups[0].proposed_message =
        "feat: include unrelated work\n\nFiles: 1";
    } else {
      changed.groups.reverse();
    }
    assert.equal(
      validateRuntimeRecord("logical-commit-plan", changed, runtimeValidators)
        .valid,
      false,
    );
  }
});

test("local review packets reject false review, validation, and change accounting", () => {
  const source = JSON.parse(
    fs.readFileSync(
      path.join(
        ROOT,
        "schemas/runtime/examples/local-review-packet.valid.json",
      ),
      "utf8",
    ),
  );
  for (const mutation of [
    "authority",
    "false-pass",
    "unrun",
    "blocking",
    "change-set",
    "same-image",
  ]) {
    const changed = structuredClone(source);
    if (mutation === "authority") {
      changed.commit_authority = true;
    } else if (mutation === "false-pass") {
      changed.validations[0].coverage = "partial";
    } else if (mutation === "unrun") {
      changed.checks_not_run = ["unit"];
    } else if (mutation === "blocking") {
      changed.blocking_finding_count = 0;
    } else if (mutation === "change-set") {
      changed.commit_plan.groups[0].paths = ["user/notes.md"];
    } else {
      changed.change_set.files[0].postimage_sha256 =
        changed.change_set.files[0].preimage_sha256;
    }
    assert.equal(
      validateRuntimeRecord("local-review-packet", changed, runtimeValidators)
        .valid,
      false,
    );
  }
});

test("local commit records reject signer, approval, index, publication, and signature drift", () => {
  const load = (recordType) =>
    JSON.parse(
      fs.readFileSync(
        path.join(ROOT, `schemas/runtime/examples/${recordType}.valid.json`),
        "utf8",
      ),
    );
  const cases = [
    [
      "pinned-commit-signer",
      (record) => {
        record.repository_selected_program_used = true;
      },
    ],
    [
      "pinned-commit-signer",
      (record) => {
        record.unsigned_fallback = true;
      },
    ],
    [
      "candidate-tree-plan",
      (record) => {
        record.network_authority = true;
      },
    ],
    [
      "candidate-tree-plan",
      (record) => {
        record.files.push(structuredClone(record.files[0]));
      },
    ],
    [
      "candidate-tree-receipt",
      (record) => {
        record.ref_update_authority = true;
      },
    ],
    [
      "local-commit-plan",
      (record) => {
        record.push_authority = true;
      },
    ],
    [
      "local-commit-plan",
      (record) => {
        record.message += "\nforged";
      },
    ],
    [
      "manual-commit-approval-receipt",
      (record) => {
        record.model_confirmed = true;
      },
    ],
    [
      "manual-commit-approval-receipt",
      (record) => {
        record.expires_at_epoch_ms += 600001;
      },
    ],
    [
      "local-commit-receipt",
      (record) => {
        record.signature_verified = false;
      },
    ],
    [
      "local-commit-receipt",
      (record) => {
        record.user_index_unchanged = false;
      },
    ],
    [
      "local-commit-receipt",
      (record) => {
        record.network_used = true;
      },
    ],
  ];
  for (const [recordType, mutate] of cases) {
    const changed = load(recordType);
    mutate(changed);
    assert.equal(
      validateRuntimeRecord(recordType, changed, runtimeValidators).valid,
      false,
      recordType,
    );
  }
});

test("change intent runtime schema rejects ambiguity and authority forgery", () => {
  const source = JSON.parse(
    fs.readFileSync(
      path.join(
        ROOT,
        "schemas/runtime/examples/change-intent-record.valid.json",
      ),
      "utf8",
    ),
  );
  const falseReady = structuredClone(source);
  falseReady.input.clarifications.push({
    question_id: "question-scope",
    dimension: "scope",
    question: "Should another package change?",
    evidence_fact_ids: [],
    material: true,
  });
  assert.equal(
    validateRuntimeRecord("change-intent-record", falseReady, runtimeValidators)
      .valid,
    false,
  );

  const instructionTarget = structuredClone(source);
  instructionTarget.input.target_fact_ids = ["5".repeat(64)];
  instructionTarget.input.current_behavior_fact_ids = ["5".repeat(64)];
  assert.equal(
    validateRuntimeRecord(
      "change-intent-record",
      instructionTarget,
      runtimeValidators,
    ).valid,
    false,
  );
});

test("reproduction runtime schema rejects false outcomes and inherited authority", () => {
  const source = JSON.parse(
    fs.readFileSync(
      path.join(
        ROOT,
        "schemas/runtime/examples/reproduction-record.valid.json",
      ),
      "utf8",
    ),
  );
  const falseOutcome = structuredClone(source);
  falseOutcome.input.failure_signature_observed = false;
  assert.equal(
    validateRuntimeRecord(
      "reproduction-record",
      falseOutcome,
      runtimeValidators,
    ).valid,
    false,
  );

  const inheritedWrite = structuredClone(source);
  inheritedWrite.input.steps[0].write_authority = true;
  assert.equal(
    validateRuntimeRecord(
      "reproduction-record",
      inheritedWrite,
      runtimeValidators,
    ).valid,
    false,
  );

  const equalResults = structuredClone(source);
  equalResults.input.observed_result_sha256 =
    equalResults.input.expected_result_sha256;
  assert.equal(
    validateRuntimeRecord(
      "reproduction-record",
      equalResults,
      runtimeValidators,
    ).valid,
    false,
  );
});

test("repository runtime schemas reject preservation and authority ambiguity", () => {
  const manifest = JSON.parse(
    fs.readFileSync(
      path.join(
        ROOT,
        "schemas/runtime/examples/repository-preservation-manifest.valid.json",
      ),
      "utf8",
    ),
  );
  const unsafe = structuredClone(manifest);
  unsafe.safe_ownership = false;
  assert.equal(
    validateRuntimeRecord(
      "repository-preservation-manifest",
      unsafe,
      runtimeValidators,
    ).valid,
    false,
  );

  const plan = JSON.parse(
    fs.readFileSync(
      path.join(
        ROOT,
        "schemas/runtime/examples/repository-operation-plan.valid.json",
      ),
      "utf8",
    ),
  );
  const pull = structuredClone(plan);
  pull.invocations[0].arguments.push("pull");
  assert.equal(
    validateRuntimeRecord("repository-operation-plan", pull, runtimeValidators)
      .valid,
    false,
  );
  const networkWorktree = structuredClone(plan);
  networkWorktree.invocations[0].network = true;
  assert.equal(
    validateRuntimeRecord(
      "repository-operation-plan",
      networkWorktree,
      runtimeValidators,
    ).valid,
    false,
  );

  const receipt = JSON.parse(
    fs.readFileSync(
      path.join(
        ROOT,
        "schemas/runtime/examples/repository-operation-receipt.valid.json",
      ),
      "utf8",
    ),
  );
  receipt.cleanup_verified = false;
  assert.equal(
    validateRuntimeRecord(
      "repository-operation-receipt",
      receipt,
      runtimeValidators,
    ).valid,
    false,
  );

  const ownership = JSON.parse(
    fs.readFileSync(
      path.join(ROOT, "schemas/runtime/examples/worktree-ownership.valid.json"),
      "utf8",
    ),
  );
  ownership.live_process_count = 1;
  assert.equal(
    validateRuntimeRecord("worktree-ownership", ownership, runtimeValidators)
      .valid,
    false,
  );
});

test("command runtime schemas reject authority and outcome ambiguity", () => {
  const preview = JSON.parse(
    fs.readFileSync(
      path.join(ROOT, "schemas/runtime/examples/command-preview.valid.json"),
      "utf8",
    ),
  );
  const inherited = structuredClone(preview);
  inherited.inherit_environment = true;
  assert.equal(
    validateRuntimeRecord("command-preview", inherited, runtimeValidators)
      .valid,
    false,
  );
  const shell = structuredClone(preview);
  shell.executable = "/usr/bin/bash";
  assert.equal(
    validateRuntimeRecord("command-preview", shell, runtimeValidators).valid,
    false,
  );

  const receipt = JSON.parse(
    fs.readFileSync(
      path.join(ROOT, "schemas/runtime/examples/command-receipt.valid.json"),
      "utf8",
    ),
  );
  const falseSuccess = structuredClone(receipt);
  falseSuccess.termination = "timed_out";
  assert.equal(
    validateRuntimeRecord("command-receipt", falseSuccess, runtimeValidators)
      .valid,
    false,
  );
  const missingCleanup = structuredClone(receipt);
  missingCleanup.termination = "cancelled";
  missingCleanup.outcome = "cancelled";
  missingCleanup.exit_code = null;
  missingCleanup.descendants_terminated = false;
  assert.equal(
    validateRuntimeRecord("command-receipt", missingCleanup, runtimeValidators)
      .valid,
    false,
  );
  const nonzeroSuccess = structuredClone(receipt);
  nonzeroSuccess.exit_code = 2;
  assert.equal(
    validateRuntimeRecord("command-receipt", nonzeroSuccess, runtimeValidators)
      .valid,
    false,
  );
});

test("write-aware checkpoint schema rejects ambiguous completion", () => {
  const source = JSON.parse(
    fs.readFileSync(
      path.join(
        ROOT,
        "schemas/runtime/examples/write-aware-checkpoint.valid.json",
      ),
      "utf8",
    ),
  );
  const incomplete = structuredClone(source);
  incomplete.receipt_chain_verified = false;
  assert.equal(
    validateRuntimeRecord(
      "write-aware-checkpoint",
      incomplete,
      runtimeValidators,
    ).valid,
    false,
  );
  const replayable = structuredClone(source);
  replayable.consumed_grant_id = null;
  assert.equal(
    validateRuntimeRecord(
      "write-aware-checkpoint",
      replayable,
      runtimeValidators,
    ).valid,
    false,
  );
});

test("runtime schemas reject missing and unknown fields", () => {
  for (const recordType of RUNTIME_RECORD_TYPES) {
    const source = JSON.parse(
      fs.readFileSync(
        path.join(ROOT, `schemas/runtime/examples/${recordType}.valid.json`),
        "utf8",
      ),
    );
    const missing = structuredClone(source);
    delete missing.schema_version;
    assert.equal(
      validateRuntimeRecord(recordType, missing, runtimeValidators).valid,
      false,
    );
    const unknown = structuredClone(source);
    unknown.model_instruction = "broaden authority";
    assert.equal(
      validateRuntimeRecord(recordType, unknown, runtimeValidators).valid,
      false,
    );
  }
});

test("thin client requests reject hidden authority and binding drift", () => {
  const source = JSON.parse(
    fs.readFileSync(
      path.join(
        ROOT,
        "schemas/runtime/examples/thin-client-request.valid.json",
      ),
      "utf8",
    ),
  );
  const mutations = [
    (record) => {
      record.authority = {
        kind: "interactive",
        approval_channel_sha256: "7".repeat(64),
      };
    },
    (record) => {
      record.authority.grant.operation = "database_write";
    },
    (record) => {
      record.authority.grant.policy_sha256 = "8".repeat(64);
    },
    (record) => {
      record.authority.grant.arguments_sha256 = "8".repeat(64);
    },
    (record) => {
      record.authority.grant.single_use = false;
    },
    (record) => {
      record.authority.grant.expires_at_epoch_ms =
        record.authority.grant.issued_at_epoch_ms;
    },
    (record) => {
      record.max_output_bytes = record.max_event_bytes - 1;
    },
    (record) => {
      record.direct_storage = true;
    },
  ];
  for (const mutate of mutations) {
    const changed = structuredClone(source);
    mutate(changed);
    assert.equal(
      validateRuntimeRecord("thin-client-request", changed, runtimeValidators)
        .valid,
      false,
    );
  }
});

test("thin client request commands remain closed and semantically bounded", () => {
  const source = JSON.parse(
    fs.readFileSync(
      path.join(
        ROOT,
        "schemas/runtime/examples/thin-client-request.valid.json",
      ),
      "utf8",
    ),
  );
  const reversed = structuredClone(source);
  reversed.command = {
    command: "conversations",
    action: { action: "list", from: "2026-12-31", to: "2026-01-01" },
  };
  reversed.authority.grant.operation = "database_read";
  assert.equal(
    validateRuntimeRecord("thin-client-request", reversed, runtimeValidators)
      .valid,
    false,
  );

  const ambient = structuredClone(source);
  ambient.command = {
    command: "vault",
    action: { action: "tasks", repository_path: "/ambient" },
  };
  assert.equal(
    validateRuntimeRecord("thin-client-request", ambient, runtimeValidators)
      .valid,
    false,
  );
});

test("thin client events reject malformed payloads and output drift", () => {
  const source = JSON.parse(
    fs.readFileSync(
      path.join(ROOT, "schemas/runtime/examples/thin-client-event.valid.json"),
      "utf8",
    ),
  );
  const mutations = [
    (record) => {
      record.schema_version = 2;
    },
    (record) => {
      record.previous_event_sha256 = "7".repeat(64);
    },
    (record) => {
      record.cumulative_output_bytes = 4194305;
    },
    (record) => {
      record.kind = {
        event: "content",
        channel: "content",
        text: "visible text",
        text_sha256: "7".repeat(64),
      };
    },
    (record) => {
      record.kind = {
        event: "completed",
        final_state_sha256: "7".repeat(64),
        authority: true,
      };
    },
    (record) => {
      record.host_socket = "/run/agentmage.sock";
    },
  ];
  for (const mutate of mutations) {
    const changed = structuredClone(source);
    mutate(changed);
    assert.equal(
      validateRuntimeRecord("thin-client-event", changed, runtimeValidators)
        .valid,
      false,
    );
  }
});

test("frontier decisions reject trigger, authority, clarification, and evidence drift", () => {
  const source = JSON.parse(
    fs.readFileSync(
      path.join(
        ROOT,
        "schemas/runtime/examples/frontier-tier-decision.valid.json",
      ),
      "utf8",
    ),
  );
  const mutations = [
    (record) => {
      record.trigger = null;
    },
    (record) => {
      record.reason_code = "frontier.recommend.exhausted-budget";
    },
    (record) => {
      record.external_effect_allowed = true;
    },
    (record) => {
      record.user_clarification_required = true;
    },
    (record) => {
      record.recommendation_only = false;
    },
    (record) => {
      record.evidence_sha256.reverse();
    },
  ];
  for (const mutate of mutations) {
    const changed = structuredClone(source);
    mutate(changed);
    assert.equal(
      validateRuntimeRecord(
        "frontier-tier-decision",
        changed,
        runtimeValidators,
      ).valid,
      false,
    );
  }
});

test("frontier receipts reject delivery and implicit or sensitive destinations", () => {
  const source = JSON.parse(
    fs.readFileSync(
      path.join(
        ROOT,
        "schemas/runtime/examples/frontier-recommendation-receipt.valid.json",
      ),
      "utf8",
    ),
  );
  const mutations = [
    (record) => {
      record.external_delivery_attempted = true;
    },
    (record) => {
      record.user_recorded_destination = "Unapproved endpoint";
    },
    (record) => {
      record.destination_recording_requested = true;
    },
    (record) => {
      record.destination_recording_requested = true;
      record.user_recorded_destination = "token=private-value";
    },
  ];
  for (const mutate of mutations) {
    const changed = structuredClone(source);
    mutate(changed);
    assert.equal(
      validateRuntimeRecord(
        "frontier-recommendation-receipt",
        changed,
        runtimeValidators,
      ).valid,
      false,
    );
  }
});

test("state-machine schema rejects phase edge and authority drift", () => {
  const source = JSON.parse(
    fs.readFileSync(
      path.join(
        ROOT,
        "schemas/runtime/examples/single-agent-state-machine.valid.json",
      ),
      "utf8",
    ),
  );
  for (const mutation of ["phase", "edge", "authority", "terminal"]) {
    const changed = structuredClone(source);
    if (mutation === "phase") {
      changed.phases[2] = "execute";
    } else if (mutation === "edge") {
      changed.transitions[4].to = "act";
    } else if (mutation === "authority") {
      changed.authority = "model-authorized";
    } else {
      changed.terminal_phases = ["review"];
    }
    assert.equal(
      validateRuntimeRecord(
        "single-agent-state-machine",
        changed,
        runtimeValidators,
      ).valid,
      false,
    );
  }
});

test("progress events enforce sequence revision step identity and content-free shape", () => {
  const source = JSON.parse(
    fs.readFileSync(
      path.join(
        ROOT,
        "schemas/runtime/examples/agent-progress-event.valid.json",
      ),
      "utf8",
    ),
  );
  const cases = [];
  const zeroSequence = structuredClone(source);
  zeroSequence.sequence = 0;
  cases.push(zeroSequence);
  const zeroRevision = structuredClone(source);
  zeroRevision.plan_revision = 0;
  cases.push(zeroRevision);
  const planWithStep = structuredClone(source);
  planWithStep.plan_step_id = "step-unexpected";
  cases.push(planWithStep);
  const stepWithoutIdentity = structuredClone(source);
  stepWithoutIdentity.kind = "step_started";
  cases.push(stepWithoutIdentity);
  const content = structuredClone(source);
  content.message = "private progress content";
  cases.push(content);
  for (const changed of cases) {
    assert.equal(
      validateRuntimeRecord("agent-progress-event", changed, runtimeValidators)
        .valid,
      false,
    );
  }
});

test("session environment schema rejects syntax bounds variants and authority drift", () => {
  const source = JSON.parse(
    fs.readFileSync(
      path.join(
        ROOT,
        "schemas/runtime/examples/session-environment-capture.valid.json",
      ),
      "utf8",
    ),
  );
  const cases = [];
  const fractionalTimestamp = structuredClone(source);
  fractionalTimestamp.captured_at_utc = "2026-08-13T20:00:00.001Z";
  cases.push(fractionalTimestamp);
  const invalidTimezone = structuredClone(source);
  invalidTimezone.timezone_id = "America//Chicago";
  cases.push(invalidTimezone);
  const nonRootWorkspace = structuredClone(source);
  nonRootWorkspace.workspace_roots[0].components = ["src"];
  cases.push(nonRootWorkspace);
  const traversal = structuredClone(source);
  traversal.current_directory.components = [".."];
  cases.push(traversal);
  const shortHead = structuredClone(source);
  shortHead.repository.head_commit = "c".repeat(39);
  cases.push(shortHead);
  const detachedWithValue = structuredClone(source);
  detachedWithValue.repository.head = { kind: "detached", value: "unexpected" };
  cases.push(detachedWithValue);
  const oversizedAttachment = structuredClone(source);
  oversizedAttachment.attachments[0].byte_len = 1099511627777;
  cases.push(oversizedAttachment);
  const digestCase = structuredClone(source);
  digestCase.capture_sha256 = "A".repeat(64);
  cases.push(digestCase);
  const platform = structuredClone(source);
  platform.platform_family = "windows";
  cases.push(platform);
  const authority = structuredClone(source);
  authority.authority = "model-authorized";
  cases.push(authority);
  for (const changed of cases) {
    assert.equal(
      validateRuntimeRecord(
        "session-environment-capture",
        changed,
        runtimeValidators,
      ).valid,
      false,
    );
  }
});

test("unknown runtime record types fail explicitly", () => {
  assert.throws(
    () => validateRuntimeRecord("unknown-record", {}, runtimeValidators),
    /unknown runtime record type/,
  );
});

test("all configuration sections and the bundle satisfy closed versioned schemas", () => {
  const results = validateConfigurationFixtures();
  assert.equal(results.length, CONFIGURATION_SECTION_TYPES.length + 1);
  assert.deepEqual(
    results.map((result) => result.valid),
    Array(results.length).fill(true),
  );
  assert.equal(results.at(-1).recordType, CONFIGURATION_BUNDLE_TYPE);
});

test("session and release result examples bind an exact configuration identity", () => {
  const results = validateConfigurationResultFixtures();
  assert.equal(results.length, 2);
  assert.deepEqual(
    results.map((result) => result.valid),
    [true, true],
  );
  assert.deepEqual(
    results.map((result) => result.recordType),
    [
      `${CONFIGURATION_RESULT_TYPE}[session]`,
      `${CONFIGURATION_RESULT_TYPE}[release]`,
    ],
  );
});

test("configuration-bound results reject incomplete or broadened records", () => {
  const mutations = [];
  const missingIdentity = configurationResultFixture("session");
  delete missingIdentity.configuration;
  mutations.push(missingIdentity);

  const unknownField = configurationResultFixture("release");
  unknownField.raw_configuration = {};
  mutations.push(unknownField);

  const invalidKind = configurationResultFixture("session");
  invalidKind.result_kind = "task";
  mutations.push(invalidKind);

  const invalidHash = configurationResultFixture("release");
  invalidHash.payload_sha256 = "not-a-hash";
  mutations.push(invalidHash);

  for (const changed of mutations) {
    assert.equal(
      validateConfiguration(CONFIGURATION_RESULT_TYPE, changed).valid,
      false,
    );
  }
});

test("configuration diff and rollback review examples satisfy closed schemas", () => {
  const results = validateConfigurationReviewFixtures();
  assert.deepEqual(
    results.map((result) => result.recordType),
    CONFIGURATION_REVIEW_TYPES,
  );
  assert.deepEqual(
    results.map((result) => result.valid),
    [true, true],
  );
});

test("configuration review schemas reject raw values and unsafe rollback claims", () => {
  const diff = JSON.parse(
    fs.readFileSync(
      path.join(
        ROOT,
        "schemas/configuration/examples/configuration-diff.valid.json",
      ),
      "utf8",
    ),
  );
  diff.changes[0].before_value = "private-value";
  assert.equal(
    validateConfiguration(CONFIGURATION_REVIEW_TYPES[0], diff).valid,
    false,
  );

  const rollback = JSON.parse(
    fs.readFileSync(
      path.join(
        ROOT,
        "schemas/configuration/examples/configuration-rollback-report.valid.json",
      ),
      "utf8",
    ),
  );
  rollback.backup_retained = false;
  rollback.private_path_persisted = true;
  assert.equal(
    validateConfiguration(CONFIGURATION_REVIEW_TYPES[1], rollback).valid,
    false,
  );
});

test("configuration schemas reject missing versions and unknown fields", () => {
  const bundle = configurationFixture();
  for (const recordType of CONFIGURATION_SECTION_TYPES) {
    const missing = structuredClone(bundle[recordType]);
    delete missing.schema_version;
    assert.equal(validateConfiguration(recordType, missing).valid, false);

    const unknown = structuredClone(bundle[recordType]);
    unknown.unreviewed_extension = true;
    assert.equal(validateConfiguration(recordType, unknown).valid, false);
  }
  const unknownBundle = structuredClone(bundle);
  unknownBundle.unreviewed_extension = true;
  assert.equal(
    validateConfiguration(CONFIGURATION_BUNDLE_TYPE, unknownBundle).valid,
    false,
  );
});

test("configuration validation does not coerce values or apply defaults", () => {
  const bundle = configurationFixture();
  const before = structuredClone(bundle);
  bundle.budget.maximum_concurrency = "1";
  assert.equal(
    validateConfiguration(CONFIGURATION_BUNDLE_TYPE, bundle).valid,
    false,
  );
  assert.equal(bundle.budget.maximum_concurrency, "1");
  before.budget.maximum_concurrency = "1";
  assert.deepEqual(bundle, before);
});

test("configuration bundle enforces authority, network, and logging controls", () => {
  const baseline = configurationFixture();
  const mutations = [];
  const permission = structuredClone(baseline);
  permission.permission.default_effect = "allow";
  mutations.push(permission);
  const modelNetwork = structuredClone(baseline);
  modelNetwork.model.network_access = true;
  mutations.push(modelNetwork);
  const shellNetwork = structuredClone(baseline);
  shellNetwork.shell.network_access = true;
  mutations.push(shellNetwork);
  const promptLog = structuredClone(baseline);
  promptLog.logging.record_prompts = true;
  mutations.push(promptLog);
  const modelTool = structuredClone(baseline);
  modelTool.shell.model_to_tool_channel = "allowed";
  mutations.push(modelTool);
  for (const changed of mutations) {
    assert.equal(
      validateConfiguration(CONFIGURATION_BUNDLE_TYPE, changed).valid,
      false,
    );
  }
});

test("configuration bundle rejects authority and resource relationships that broaden scope", () => {
  const capability = configurationFixture();
  capability.tool.tools[0].required_capabilities.push("workspace.write");
  const capabilityResult = validateConfiguration(
    CONFIGURATION_BUNDLE_TYPE,
    capability,
  );
  assert.equal(capabilityResult.valid, false);
  assert.match(
    capabilityResult.semanticErrors[0],
    /outside permission ceiling/,
  );

  const budget = configurationFixture();
  budget.model.decoding.maximum_output_tokens = 2048;
  const budgetResult = validateConfiguration(CONFIGURATION_BUNDLE_TYPE, budget);
  assert.equal(budgetResult.valid, false);
  assert.match(budgetResult.semanticErrors[0], /exceeds the global/);
});

test("current configuration schemas retain complete fail-closed mutation coverage", () => {
  const report = buildConfigurationSchemaReport();
  assert.equal(report.section_schema_count, 11);
  assert.equal(report.schemas.length, 13);
  assert.equal(report.mutation_count, 48);
  assert.equal(report.rejected_mutation_count, 48);
  assert.equal(report.product_configuration_loader_claim, "none");
  assert.equal(report.macos_execution_status, "blocked-macos");
});

test("retained configuration report currentness remains an explicit legacy check", () => {
  const result = validateConfigurationSchemaReport();
  assert.ok(Array.isArray(result));
  assert.ok(result.length <= 1);
});

test("unknown configuration record types fail explicitly", () => {
  assert.throws(
    () =>
      validateConfigurationRecord(
        "unknown-record",
        {},
        configurationValidators,
      ),
    /unknown configuration record type/,
  );
});

test("configuration profile catalog and all seven profiles satisfy formal schemas", () => {
  const results = validateConfigurationProfiles();
  assert.equal(results.length, 8);
  assert.equal(results[0].recordType, CONFIGURATION_PROFILE_CATALOG_TYPE);
  assert.deepEqual(
    results.map((result) => result.valid),
    Array(8).fill(true),
  );
});

test("future profile catalog entries cannot claim registration, network, or macOS support", () => {
  const catalog = JSON.parse(
    fs.readFileSync(
      path.join(ROOT, "configuration/profiles/catalog.json"),
      "utf8",
    ),
  );
  for (const mutation of [
    ["product_registration", true],
    ["network_effective", true],
    ["macos_support_claim", "supported"],
  ]) {
    const changed = structuredClone(catalog);
    changed.profiles.at(-1)[mutation[0]] = mutation[1];
    assert.equal(
      validateConfiguration(CONFIGURATION_PROFILE_CATALOG_TYPE, changed).valid,
      false,
    );
  }
});
