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

function sha256String(value) {
  return crypto.createHash("sha256").update(value, "utf8").digest("hex");
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
    ajv.addSchema(readJson(`schemas/configuration/${recordType}.schema.json`));
  }
  ajv.addSchema(
    readJson(`schemas/configuration/${CONFIGURATION_BUNDLE_TYPE}.schema.json`),
  );
  for (const recordType of CONFIGURATION_REVIEW_TYPES) {
    ajv.addSchema(readJson(`schemas/configuration/${recordType}.schema.json`));
  }
  ajv.addSchema(
    readJson(
      `schemas/configuration/${CONFIGURATION_PROFILE_CATALOG_TYPE}.schema.json`,
    ),
  );
  ajv.addSchema(
    readJson(`schemas/configuration/${CONFIGURATION_RESULT_TYPE}.schema.json`),
  );

  return Object.fromEntries(
    [
      ...CONFIGURATION_SECTION_TYPES,
      CONFIGURATION_BUNDLE_TYPE,
      CONFIGURATION_PROFILE_CATALOG_TYPE,
      CONFIGURATION_RESULT_TYPE,
      ...CONFIGURATION_REVIEW_TYPES,
    ].map((recordType) => {
      const schemaId = `https://agentmage.dev/schemas/configuration/${recordType}.schema.json`;
      const validator = ajv.getSchema(schemaId);
      if (!validator) {
        throw new Error(`configuration schema did not register: ${recordType}`);
      }
      return [recordType, validator];
    }),
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

  for (const recordType of RUNTIME_RECORD_TYPES) {
    ajv.addSchema(readJson(`schemas/runtime/${recordType}.schema.json`));
  }
  return Object.fromEntries(
    RUNTIME_RECORD_TYPES.map((recordType) => {
      const schemaId = `https://agentmage.dev/schemas/runtime/${recordType}.schema.json`;
      const validator = ajv.getSchema(schemaId);
      if (!validator) {
        throw new Error(`runtime schema did not register: ${recordType}`);
      }
      return [recordType, validator];
    }),
  );
}

export function validateRuntimeRecord(recordType, data, validators) {
  const validator = validators[recordType];
  if (!validator) {
    throw new Error(`unknown runtime record type: ${recordType}`);
  }
  const schemaValid = validator(data);
  const semanticErrors = runtimeSemanticErrors(recordType, data);
  const valid = schemaValid && semanticErrors.length === 0;
  return {
    valid,
    schemaErrors: valid
      ? []
      : [
          ...(validator.errors ?? []).map(
            (error) => `${error.instancePath || "/"} ${error.message}`,
          ),
          ...semanticErrors,
        ],
  };
}

function isStrictlySorted(values) {
  return values.every(
    (value, index) => index === 0 || values[index - 1] < value,
  );
}

function isStrictlySortedBy(values, key) {
  return values.every(
    (value, index) => index === 0 || key(values[index - 1]) < key(value),
  );
}

const VALIDATION_KIND_ORDER = Object.freeze([
  "unit",
  "integration",
  "end_to_end",
  "lint",
  "format",
  "types",
  "build",
  "packaging",
  "security",
]);
const REVIEW_MODE_ORDER = Object.freeze([
  "correctness",
  "simplicity",
  "maintainability",
  "security",
  "data_integrity",
  "accessibility",
  "performance",
  "tests",
  "documentation",
]);
const REVIEW_HOOK_ORDER = Object.freeze([
  "interface",
  "dependency",
  "migration",
  "security",
  "performance",
  "accessibility",
  "compatibility",
]);
const COMMIT_PURPOSE_ORDER = Object.freeze([
  "behavior",
  "formatting",
  "tests",
  "documentation",
  "migration",
  "generated_output",
]);
const FINDING_SEVERITY_ORDER = Object.freeze([
  "info",
  "low",
  "medium",
  "high",
  "critical",
]);

function enumIndex(values, value) {
  return values.indexOf(value);
}

function logicalCommitPlanSemanticErrors(data) {
  const errors = [];
  const groups = data.groups ?? [];
  if (
    !isStrictlySortedBy(groups, (group) =>
      enumIndex(COMMIT_PURPOSE_ORDER, group.purpose),
    )
  ) {
    errors.push("commit groups must be strictly purpose ordered");
  }
  const expectedMessage = {
    behavior: "feat: apply approved behavior change",
    formatting: "style: apply approved formatting",
    tests: "test: update approved tests",
    documentation: "docs: update approved documentation",
    migration: "chore(migration): apply approved migration",
    generated_output: "chore(generate): refresh approved output",
  };
  const operationIds = [];
  for (const [index, group] of groups.entries()) {
    const expectedGroupId = `commit-group-${String(index + 1).padStart(2, "0")}`;
    if (group.group_id !== expectedGroupId) {
      errors.push(
        `commit group ${index + 1} must use identity ${expectedGroupId}`,
      );
    }
    if (!isStrictlySorted(group.operation_ids ?? [])) {
      errors.push(`${expectedGroupId} operation_ids must be strictly sorted`);
    }
    if (!isStrictlySorted(group.paths ?? [])) {
      errors.push(`${expectedGroupId} paths must be strictly sorted`);
    }
    if ((group.operation_ids ?? []).length !== (group.paths ?? []).length) {
      errors.push(`${expectedGroupId} operation and path counts must agree`);
    }
    const message = `${expectedMessage[group.purpose]}\n\nFiles: ${(group.paths ?? []).length}`;
    if (group.proposed_message !== message) {
      errors.push(
        `${expectedGroupId} proposed message must match its exact approved group`,
      );
    }
    operationIds.push(...(group.operation_ids ?? []));
  }
  if (new Set(operationIds).size !== operationIds.length) {
    errors.push("an operation cannot appear in more than one commit group");
  }
  if (
    !isStrictlySortedBy(
      data.excluded_unrelated ?? [],
      (item) =>
        `${item.path_sha256}:${item.content_sha256}:${item.exclusion_code}`,
    )
  ) {
    errors.push("excluded unrelated evidence must be strictly sorted");
  }
  return errors;
}

function localReviewPacketSemanticErrors(data) {
  const errors = [];
  const changeSet = data.change_set ?? {};
  const files = changeSet.files ?? [];
  if (
    !isStrictlySortedBy(changeSet.review_hooks ?? [], (hook) =>
      enumIndex(REVIEW_HOOK_ORDER, hook),
    )
  ) {
    errors.push("review_hooks must be strictly ordered");
  }
  if (!isStrictlySortedBy(files, (file) => file.path)) {
    errors.push("changed files must be strictly path ordered");
  }
  const operationIds = files.map((file) => file.operation_id);
  if (new Set(operationIds).size !== operationIds.length) {
    errors.push("changed operation identities must be unique");
  }
  if (
    files.reduce(
      (total, file) => total + (file.complete_diff?.length ?? 0),
      0,
    ) > 8388608
  ) {
    errors.push("complete diff bytes exceed the packet bound");
  }
  const purposeMatches = {
    behavior: ["code", "configuration", false],
    formatting: ["code", "configuration", false],
    tests: ["test", false],
    documentation: ["documentation", false],
    migration: ["migration", false],
    generated_output: ["generated_output", true],
  };
  for (const file of files) {
    if (file.preimage_sha256 === file.postimage_sha256) {
      errors.push(
        `changed file ${file.operation_id} must have distinct images`,
      );
    }
    const allowed = purposeMatches[file.purpose] ?? [];
    const generated = allowed.at(-1);
    const artifacts = allowed.slice(0, -1);
    if (
      !artifacts.includes(file.artifact_class) ||
      file.generated !== generated
    ) {
      errors.push(
        `changed file ${file.operation_id} purpose and artifact class disagree`,
      );
    }
  }

  const validations = data.validations ?? [];
  if (
    !isStrictlySortedBy(
      validations,
      (validation) =>
        `${String(enumIndex(VALIDATION_KIND_ORDER, validation.kind)).padStart(2, "0")}:${validation.validation_id}`,
    )
  ) {
    errors.push("validations must be strictly kind and identity ordered");
  }
  if (
    !isStrictlySortedBy(data.checks_not_run ?? [], (kind) =>
      enumIndex(VALIDATION_KIND_ORDER, kind),
    )
  ) {
    errors.push("checks_not_run must be strictly kind ordered");
  }
  const executedKinds = new Set(
    validations.map((validation) => validation.kind),
  );
  if ((data.checks_not_run ?? []).some((kind) => executedKinds.has(kind))) {
    errors.push("an executed validation kind cannot also be marked not run");
  }

  const results = data.review_results ?? {};
  if (
    !isStrictlySortedBy(results.modes ?? [], (mode) =>
      enumIndex(REVIEW_MODE_ORDER, mode),
    )
  ) {
    errors.push("review modes must be strictly ordered");
  }
  const findingKey = (finding) =>
    [
      finding.path,
      String(finding.line).padStart(10, "0"),
      String(enumIndex(REVIEW_MODE_ORDER, finding.mode)).padStart(2, "0"),
      finding.code,
      finding.finding_id,
    ].join(":");
  if (!isStrictlySortedBy(results.findings ?? [], findingKey)) {
    errors.push("visible findings must be strictly source ordered");
  }
  if (
    !isStrictlySortedBy(results.suppressed ?? [], (entry) =>
      findingKey(entry.finding),
    )
  ) {
    errors.push("suppressed findings must be strictly source ordered");
  }
  const entries = [
    ...(results.findings ?? []).map((finding) => ({ finding, visible: true })),
    ...(results.suppressed ?? []).map((entry) => ({
      ...entry,
      visible: false,
    })),
  ];
  if (entries.length > 2048) {
    errors.push("total findings exceed the review bound");
  }
  const findingIds = entries.map((entry) => entry.finding.finding_id);
  if (new Set(findingIds).size !== findingIds.length) {
    errors.push(
      "finding identities must be unique across visible and suppressed evidence",
    );
  }
  const modes = new Set(results.modes ?? []);
  if (entries.some((entry) => !modes.has(entry.finding.mode))) {
    errors.push("every finding mode must have been requested");
  }
  const duplicateKey = (finding) =>
    [finding.mode, finding.code, finding.path, finding.line].join(":");
  const rank = (finding) => [
    enumIndex(FINDING_SEVERITY_ORDER, finding.severity),
    finding.confidence_bps,
    finding.finding_id,
  ];
  const compareRank = (left, right) =>
    left[0] - right[0] || left[1] - right[1] || left[2].localeCompare(right[2]);
  const byDuplicateKey = new Map();
  for (const entry of entries) {
    const key = duplicateKey(entry.finding);
    byDuplicateKey.set(key, [...(byDuplicateKey.get(key) ?? []), entry]);
  }
  for (const grouped of byDuplicateKey.values()) {
    const winner = grouped.reduce((best, entry) =>
      compareRank(rank(entry.finding), rank(best.finding)) > 0 ? entry : best,
    );
    for (const entry of grouped) {
      if (entry === winner) {
        if (entry.finding.confidence_bps >= 7000 && !entry.visible) {
          errors.push(
            `visible winner ${entry.finding.finding_id} was suppressed`,
          );
        } else if (
          entry.finding.confidence_bps < 7000 &&
          (entry.visible ||
            entry.reason !== "low_confidence" ||
            entry.superseded_by !== null)
        ) {
          errors.push(
            `low-confidence winner ${entry.finding.finding_id} is not preserved canonically`,
          );
        }
      } else if (
        entry.visible ||
        entry.reason !== "duplicate" ||
        entry.superseded_by !== winner.finding.finding_id
      ) {
        errors.push(
          `duplicate ${entry.finding.finding_id} does not name its canonical winner`,
        );
      }
    }
  }

  if (
    !isStrictlySortedBy(
      data.evidence_artifacts ?? [],
      (artifact) =>
        `${artifact.artifact_id}:${artifact.kind}:${artifact.sha256}`,
    )
  ) {
    errors.push("review evidence artifacts must be strictly ordered");
  }
  const blockingCount = (results.findings ?? []).filter((finding) =>
    ["high", "critical"].includes(finding.severity),
  ).length;
  if (data.blocking_finding_count !== blockingCount) {
    errors.push(
      "blocking_finding_count must equal visible high and critical findings",
    );
  }
  if (data.commit_plan?.change_set_sha256 !== changeSet.change_set_sha256) {
    errors.push(
      "logical commit plan must bind the exact reviewable change set",
    );
  }
  const filesByPurpose = new Map();
  for (const file of files) {
    filesByPurpose.set(file.purpose, [
      ...(filesByPurpose.get(file.purpose) ?? []),
      file,
    ]);
  }
  const expectedPurposes = COMMIT_PURPOSE_ORDER.filter((purpose) =>
    filesByPurpose.has(purpose),
  );
  const groups = data.commit_plan?.groups ?? [];
  if (groups.length !== expectedPurposes.length) {
    errors.push(
      "logical commit groups must account for every changed-file purpose",
    );
  } else {
    for (const [index, purpose] of expectedPurposes.entries()) {
      const group = groups[index];
      const purposeFiles = filesByPurpose.get(purpose);
      const expectedOperations = purposeFiles
        .map((file) => file.operation_id)
        .sort();
      const expectedPaths = purposeFiles.map((file) => file.path).sort();
      if (
        group.purpose !== purpose ||
        JSON.stringify(group.operation_ids) !==
          JSON.stringify(expectedOperations) ||
        JSON.stringify(group.paths) !== JSON.stringify(expectedPaths)
      ) {
        errors.push(
          `logical commit group ${index + 1} does not match the exact approved files`,
        );
      }
    }
  }
  return errors;
}

function runtimeSemanticErrors(recordType, data) {
  const errors = [];
  if (recordType === "change-intent-record") {
    for (const field of [
      "current_behavior_fact_ids",
      "target_fact_ids",
      "users",
      "acceptance_checks",
      "exclusions",
    ]) {
      if (
        Array.isArray(data.input?.[field]) &&
        !isStrictlySorted(data.input[field])
      ) {
        errors.push(`${field} must be strictly sorted`);
      }
    }
    for (const field of [
      "current_behavior_citation_ids",
      "target_citation_ids",
      "rejected_repository_instruction_fact_ids",
      "rejected_repository_instruction_citation_ids",
    ]) {
      if (Array.isArray(data[field]) && !isStrictlySorted(data[field])) {
        errors.push(`${field} must be strictly sorted`);
      }
    }
    const current = new Set(data.input?.current_behavior_fact_ids ?? []);
    if (
      !(data.input?.target_fact_ids ?? []).some((identity) =>
        current.has(identity),
      )
    ) {
      errors.push("target evidence must intersect current behavior evidence");
    }
    const material = (data.input?.clarifications ?? []).some(
      (clarification) => clarification.material === true,
    );
    const expectedStatus = material
      ? "clarification_required"
      : "ready_for_planning";
    if (data.status !== expectedStatus) {
      errors.push(`status must equal ${expectedStatus}`);
    }
    const rejected = new Set(
      data.rejected_repository_instruction_fact_ids ?? [],
    );
    if (
      (data.input?.target_fact_ids ?? []).some((identity) =>
        rejected.has(identity),
      )
    ) {
      errors.push(
        "rejected repository instructions cannot become change targets",
      );
    }
  } else if (recordType === "reproduction-record") {
    if (!isStrictlySorted(data.input?.log_sha256s ?? [])) {
      errors.push("log_sha256s must be strictly sorted");
    }
    const steps = data.input?.steps ?? [];
    if (!steps.every((step, index) => step.sequence === index + 1)) {
      errors.push(
        "reproduction steps must have contiguous one-based sequence values",
      );
    }
    const input = data.input ?? {};
    let expectedOutcome = "inconclusive";
    if (input.unsafe_reason !== null && input.unsafe_reason !== undefined) {
      expectedOutcome = "unsafe_to_reproduce";
    } else if (input.execution_status === "completed") {
      expectedOutcome = input.failure_signature_observed
        ? "reproduced"
        : "not_reproduced";
    }
    if (data.outcome !== expectedOutcome) {
      errors.push(`outcome must equal ${expectedOutcome}`);
    }
    if (
      expectedOutcome === "reproduced" &&
      input.observed_result_sha256 === input.expected_result_sha256
    ) {
      errors.push(
        "a reproduced failure must differ from the expected result identity",
      );
    }
  } else if (recordType === "validation-receipt") {
    for (const field of [
      "environment_names",
      "failed_names",
      "unverified_kinds",
    ]) {
      if (!isStrictlySorted(data[field] ?? [])) {
        errors.push(`${field} must be strictly sorted`);
      }
    }
    const artifactKeys = (data.artifacts ?? []).map(
      (artifact) => `${artifact.artifact_id}:${JSON.stringify(artifact.path)}`,
    );
    if (!isStrictlySorted(artifactKeys)) {
      errors.push("artifacts must be strictly sorted by identity and path");
    }
    const affectedKeys = (data.affected_files ?? []).map((file) =>
      JSON.stringify(file.path),
    );
    if (!isStrictlySorted(affectedKeys)) {
      errors.push("affected_files must be strictly sorted by path");
    }
    if ((data.unverified_kinds ?? []).includes(data.kind)) {
      errors.push("the executed validation kind cannot be unverified");
    }
    if (
      data.reported_duration_ms !== null &&
      data.reported_duration_ms > data.duration_ms + 1000
    ) {
      errors.push(
        "reported duration exceeds the bounded process duration allowance",
      );
    }
    const secretClassified = [
      data.stdout_classification,
      data.stderr_classification,
    ].includes("secret_detected");
    if (data.secret_match_count > 0 !== secretClassified) {
      errors.push("secret count and output classification disagree");
    }
    if (
      ["unit", "integration", "end_to_end"].includes(data.kind) &&
      data.status === "passed" &&
      data.passed < 1
    ) {
      errors.push("a passing test-kind receipt must report an executed test");
    }
    if (
      data.status === "assertion_failed" &&
      (data.failed < 1 || data.failed_names.length !== data.failed)
    ) {
      errors.push("assertion failure count and names must agree");
    }
    for (const file of data.affected_files ?? []) {
      if (
        file.preimage_sha256 !== null &&
        file.preimage_sha256 === file.postimage_sha256
      ) {
        errors.push("affected file preimage and postimage must differ");
      }
    }
  } else if (recordType === "logical-commit-plan") {
    errors.push(...logicalCommitPlanSemanticErrors(data));
  } else if (recordType === "local-review-packet") {
    errors.push(...logicalCommitPlanSemanticErrors(data.commit_plan ?? {}));
    errors.push(...localReviewPacketSemanticErrors(data));
  } else if (recordType === "pinned-commit-signer") {
    if (
      (data.kind === "open_pgp" && data.source !== "external_keyring") ||
      (data.kind === "hardware_backed" && data.source !== "platform_broker")
    ) {
      errors.push("signer kind and external inspection source disagree");
    }
  } else if (recordType === "candidate-tree-plan") {
    if (!isStrictlySortedBy(data.files ?? [], (file) => file.path)) {
      errors.push("candidate files must be strictly path ordered");
    }
    const operationIds = (data.files ?? []).map((file) => file.operation_id);
    if (new Set(operationIds).size !== operationIds.length) {
      errors.push("candidate operation identities must be unique");
    }
  } else if (recordType === "candidate-tree-receipt") {
    if (!isStrictlySortedBy(data.blobs ?? [], (blob) => blob.operation_id)) {
      errors.push("candidate blobs must be strictly operation ordered");
    }
    if (data.before_manifest_sha256 === data.after_manifest_sha256) {
      errors.push(
        "candidate object construction must record a distinct post-effect manifest",
      );
    }
  } else if (recordType === "local-commit-plan") {
    if (sha256String(data.message ?? "") !== data.message_sha256) {
      errors.push("commit message digest must match the exact message");
    }
    if ((data.task_branch ?? "").includes("..")) {
      errors.push("task branch cannot contain a ref traversal sequence");
    }
  } else if (recordType === "manual-commit-approval-receipt") {
    if (
      data.expires_at_epoch_ms <= data.approved_at_epoch_ms ||
      data.expires_at_epoch_ms - data.approved_at_epoch_ms > 600000
    ) {
      errors.push(
        "manual commit approval must have a positive bounded lifetime",
      );
    }
  } else if (recordType === "local-commit-receipt") {
    if (data.before_manifest_sha256 === data.after_manifest_sha256) {
      errors.push(
        "successful local commit must record a distinct post-effect manifest",
      );
    }
  }
  return errors;
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
    for (const riskId of duplicateValues(
      (data.risks ?? []).map((risk) => risk.risk_id),
    )) {
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
    : (validator.errors ?? []).map(
        (error) => `${error.instancePath || "/"} ${error.message}`,
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
      : (validator.errors ?? []).map(
          (error) => `${error.instancePath || "/"} ${error.message}`,
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
    data.model?.decoding?.maximum_context_tokens >
    data.budget?.maximum_context_tokens
  ) {
    errors.push("model context exceeds the global context budget");
  }
  if (
    data.model?.decoding?.maximum_output_tokens >
    data.budget?.maximum_output_tokens
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
    : (validator.errors ?? []).map(
        (error) => `${error.instancePath || "/"} ${error.message}`,
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
    const source =
      recordType === CONFIGURATION_BUNDLE_TYPE ? bundle : bundle[recordType];
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
      (recordType) => `schemas/configuration/${recordType}.schema.json`,
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
    rejected_mutation_count: mutationResults.filter((item) => item.rejected)
      .length,
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
    ...validateTestingRecord("fixture-provenance-ledger", ledger, validators),
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
  const configurationBoundResultResults = validateConfigurationResultFixtures();
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
    `Validated ${planningResults.length} planning and ` +
      `${testingResults.length} testing and ` +
      `${configurationResults.length} configuration schema fixture(s), plus ` +
      `${configurationProfileResults.length} profile record(s) and ` +
      `${configurationBoundResultResults.length} configuration-bound result(s) and ` +
      `${configurationReviewResults.length} review record(s) and ` +
      `${runtimeResults.length} runtime record(s).`,
  );
  return 0;
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  process.exitCode = main();
}
