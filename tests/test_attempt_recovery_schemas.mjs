import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import Ajv2020 from "ajv/dist/2020.js";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const ajv = new Ajv2020({ allErrors: true, strict: true, formats: { "date-time": true } });
const compile = (name) => ajv.compile(JSON.parse(fs.readFileSync(path.join(ROOT, `schemas/runtime/${name}.schema.json`), "utf8")));
const checkpointValidator = compile("attempt-recovery-checkpoint");
const decisionValidator = compile("attempt-recovery-decision");
const sha = "a".repeat(64);

function checkpoint() {
  return {
    schema_version: 2, attempt_checkpoint_id: "checkpoint-1", workflow_checkpoint_id: "workflow-checkpoint-1",
    workflow_id: "workflow-1", execution_id: "execution-1", step_execution_id: "step-1", attempt_id: "attempt-1",
    source_sha256: sha, plan_sha256: sha, model_sha256: sha, context_sha256: sha, tool_catalog_sha256: sha,
    policy_sha256: sha, grants_sha256: sha, approvals_sha256: sha, preflights_sha256: sha,
    attempts_sha256: sha, receipts_sha256: sha, artifacts_sha256: sha, verifier_sha256: sha,
    budget_policy_sha256: sha,
    budget_state: { parser_repairs: 1, model_repairs: 0, step_attempts: 1, error_class_counts: Array(14).fill(0), workflow_work: 2, replans: 0 },
    repeated_state_sha256: sha, repeated_state_count: 1,
    event_cursor: { run_id: "run-1", event_id: "event-1", sequence: 1, event_sha256: sha },
    environment_sha256: sha, effect_state: "uncertain", authority_consumed: true,
    created_at: "2026-08-31T12:00:00Z", checkpoint_sha256: sha,
  };
}

function decision() {
  return { action: "reconcile_effect", reason_code: "attempt_recovery.effect_not_replayable", drift: [],
    exhausted_budgets: [], diagnosis: null, prior_effect_replay_allowed: false,
    fresh_identity_and_authority_required: false };
}

test("attempt recovery schemas accept closed content-free records", () => {
  assert.equal(checkpointValidator(checkpoint()), true, JSON.stringify(checkpointValidator.errors));
  assert.equal(decisionValidator(decision()), true, JSON.stringify(decisionValidator.errors));
});

test("attempt recovery schemas reject replay, hidden content, missing budgets, and widening", () => {
  const mutations = [
    [checkpointValidator, { ...checkpoint(), tool_arguments: { command: "hidden" } }],
    [checkpointValidator, { ...checkpoint(), budget_state: { ...checkpoint().budget_state, error_class_counts: [0] } }],
    [checkpointValidator, { ...checkpoint(), effect_state: "assume_not_applied" }],
    [decisionValidator, { ...decision(), prior_effect_replay_allowed: true }],
    [decisionValidator, { ...decision(), action: "resume_old_call" }],
    [decisionValidator, { ...decision(), model_explanation: "trust me" }],
  ];
  for (const [validator, value] of mutations) assert.equal(validator(value), false);
});
