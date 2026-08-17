import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import test from "node:test";

import {
  parseRuntimeHostResponse,
  renderRuntimeOutcome,
  runtimeApprovalResponse,
  RuntimeStreamVerifier,
  RuntimeTransportFailure,
  type RuntimeRunRequestEnvelope,
} from "../src/runtime_transport.js";

const ZERO_SHA256 = "0".repeat(64);

void test("closed runtime request and host response reject unknown fields", () => {
  const prepared = preparedResponse();
  const parsed = parseRuntimeHostResponse(prepared);
  assert.equal(parsed.kind, "runtime_prepared");
  assert.equal(parsed.run_request.model_profile.profile_id, "profile-0001");

  assert.throws(
    () =>
      parseRuntimeHostResponse({
        ...prepared,
        model_endpoint: "https://example.invalid",
      }),
    RuntimeTransportFailure,
  );
  assert.throws(
    () =>
      parseRuntimeHostResponse({
        ...prepared,
        run_request: {
          ...prepared.run_request,
          policy_override: true,
        },
      }),
    RuntimeTransportFailure,
  );
});

void test("ordered terminal stream renders only digest-checked canonical output", () => {
  const request = runtimeRequest();
  const response = parseRuntimeHostResponse(terminalResponse());
  assert.equal(response.kind, "runtime_step");
  const verifier = new RuntimeStreamVerifier(request);
  assert.equal(verifier.accept(response).length, 4);
  assert.equal(verifier.cursor()?.sequence, 3);
  assert.ok(response.outcome !== null);
  const rendered = renderRuntimeOutcome(response.outcome);
  assert.match(rendered, /Verified local result/u);
  assert.doesNotMatch(rendered, /command:agentmage\.unsafe/u);
  assert.match(rendered, /blocked local link/u);
  assert.match(rendered, /Status: SUCCESS/u);

  const replay = parseRuntimeHostResponse({
    ...terminalResponse(),
    request_id: "request-step-replay-0001",
    events: [],
  });
  assert.equal(replay.kind, "runtime_step");
  assert.equal(verifier.accept(replay).length, 0);
});

void test("stream substitution reordering and payload mutation fail closed", () => {
  const request = runtimeRequest();
  const reordered = terminalResponse();
  const first = reordered.events[0];
  const second = reordered.events[1];
  assert.ok(first !== undefined && second !== undefined);
  reordered.events = [second, first];
  const parsed = parseRuntimeHostResponse(reordered);
  assert.equal(parsed.kind, "runtime_step");
  assert.throws(
    () => new RuntimeStreamVerifier(request).accept(parsed),
    RuntimeTransportFailure,
  );

  const substituted = terminalResponse();
  const originalSecond = substituted.events[1];
  assert.ok(originalSecond !== undefined);
  substituted.events[1] = {
    ...originalSecond,
    previous_event_sha256: "f".repeat(64),
  };
  const changed = parseRuntimeHostResponse(substituted);
  assert.equal(changed.kind, "runtime_step");
  assert.throws(
    () => new RuntimeStreamVerifier(request).accept(changed),
    RuntimeTransportFailure,
  );

  const terminal = parseRuntimeHostResponse(terminalResponse());
  assert.equal(terminal.kind, "runtime_step");
  assert.ok(terminal.outcome?.output?.storage === "inline");
  const mutableBytes = [...terminal.outcome.output.payload.bytes];
  mutableBytes[0] = 0;
  const mutatedOutcome = {
    ...terminal.outcome,
    output: {
      ...terminal.outcome.output,
      payload: { ...terminal.outcome.output.payload, bytes: mutableBytes },
    },
  };
  assert.throws(
    () => renderRuntimeOutcome(mutatedOutcome),
    RuntimeTransportFailure,
  );
});

void test("approval is bound to the exact last permission event and grant", () => {
  const request = runtimeRequest();
  const parsed = parseRuntimeHostResponse(approvalResponse());
  assert.equal(parsed.kind, "runtime_step");
  const verifier = new RuntimeStreamVerifier(request);
  verifier.accept(parsed);
  assert.ok(parsed.approval !== null);
  assert.deepEqual(runtimeApprovalResponse(parsed.approval, "allow"), {
    schema_version: 2,
    run_id: "run-0001",
    approval_id: "approval-0001",
    disposition: "allow",
    challenge_sha256: "8".repeat(64),
    grant_id: "grant-0001",
  });

  const changed = approvalResponse();
  assert.ok(changed.approval !== null);
  changed.approval = {
    ...changed.approval,
    preview_sha256: "f".repeat(64),
  };
  const substituted = parseRuntimeHostResponse(changed);
  assert.equal(substituted.kind, "runtime_step");
  assert.throws(
    () => new RuntimeStreamVerifier(request).accept(substituted),
    RuntimeTransportFailure,
  );
});

export function runtimeRequest(): RuntimeRunRequestEnvelope {
  return {
    schema_version: 2,
    run_id: "run-0001",
    session_id: "session-0001",
    mode: "ephemeral_read_only",
    task: {
      schema_version: 2,
      task_id: "task-0001",
      session_id: "session-0001",
      objective: "Inspect the selected workspace",
      acceptance_criteria: ["Report grounded findings"],
      constraints: ["Remain read only"],
      status: "ready",
    },
    work_packet: {},
    workspace_id: "workspace-0001",
    workspace_snapshot_sha256: "1".repeat(64),
    repository_snapshot_id: "repository-snapshot-0001",
    repository_snapshot_sha256: "2".repeat(64),
    model_profile: { profile_id: "profile-0001" },
    context_budget: {},
    tool_catalog_id: "tool-catalog-0001",
    tool_catalog_sha256: "3".repeat(64),
    visible_tools: [],
    policy_id: "policy-0001",
    policy_sha256: "4".repeat(64),
    limits: {},
    event_cursor: null,
    request_sha256: "5".repeat(64),
  };
}

export function preparedResponse(): {
  kind: "runtime_prepared";
  schema_version: 1;
  request_id: string;
  run_request: RuntimeRunRequestEnvelope;
} {
  return {
    kind: "runtime_prepared",
    schema_version: 1,
    request_id: "request-prepare-0001",
    run_request: runtimeRequest(),
  };
}

export function terminalResponse(): MutableRuntimeStep {
  const text = "Verified local result. [Unsafe](command:agentmage.unsafe)\n";
  const bytes = [...Buffer.from(text, "utf8")];
  const outputSha256 = createHash("sha256")
    .update(Uint8Array.from(bytes))
    .digest("hex");
  const events = [
    runtimeEvent(0, ZERO_SHA256, null, {
      event: "run_started",
      request_sha256: "5".repeat(64),
    }),
    runtimeEvent(1, "a".repeat(64), "event-0000", {
      event: "turn_started",
    }),
    runtimeEvent(2, "b".repeat(64), "event-0001", {
      event: "turn_completed",
      outcome_sha256: "6".repeat(64),
    }),
    runtimeEvent(3, "c".repeat(64), "event-0002", {
      event: "run_terminal",
      state: "SUCCESS",
      outcome_sha256: "7".repeat(64),
    }),
  ];
  return {
    kind: "runtime_step",
    schema_version: 1,
    request_id: "request-step-0001",
    run_id: "run-0001",
    request_sha256: "5".repeat(64),
    events,
    approval: null,
    outcome: {
      schema_version: 2,
      run_id: "run-0001",
      session_id: "session-0001",
      task_id: "task-0001",
      request_sha256: "5".repeat(64),
      state: "SUCCESS",
      turn_count: 1,
      model_call_count: 1,
      tool_call_count: 0,
      prior_event_id: "event-0002",
      prior_event_sha256: "c".repeat(64),
      evidence: [evidenceReference()],
      receipt_ids: [],
      unresolved_codes: [],
      output: {
        storage: "inline",
        payload: {
          schema: {
            schema_id: "runtime-output",
            schema_version: 2,
            schema_sha256: "9".repeat(64),
          },
          media_type: "text/markdown",
          bytes,
          sha256: outputSha256,
        },
      },
      outcome_sha256: "7".repeat(64),
    },
  };
}

export function approvalResponse(): MutableRuntimeStep {
  const events = [
    runtimeEvent(0, ZERO_SHA256, null, {
      event: "run_started",
      request_sha256: "5".repeat(64),
    }),
    runtimeEvent(1, "a".repeat(64), "event-0000", {
      event: "permission_requested",
      approval_id: "approval-0001",
      operation: "workspace_read",
      preview_sha256: "6".repeat(64),
      expires_at_epoch_ms: Date.now() + 60_000,
    }),
  ];
  return {
    kind: "runtime_step",
    schema_version: 1,
    request_id: "request-step-0001",
    run_id: "run-0001",
    request_sha256: "5".repeat(64),
    events,
    approval: {
      schema_version: 2,
      run_id: "run-0001",
      task_id: "task-0001",
      turn_id: "turn-0001",
      operation_id: "operation-0001",
      tool_call_id: "tool-call-0001",
      approval_id: "approval-0001",
      proposed_grant_id: "grant-0001",
      operation: "workspace_read",
      preview_sha256: "6".repeat(64),
      expires_at_epoch_ms: events[1]?.kind.expires_at_epoch_ms as number,
      challenge_sha256: "8".repeat(64),
    },
    outcome: null,
  };
}

interface MutableRuntimeStep {
  kind: "runtime_step";
  schema_version: 1;
  request_id: string;
  run_id: string;
  request_sha256: string;
  events: MutableRuntimeEvent[];
  approval: null | Record<string, unknown>;
  outcome: null | Record<string, unknown>;
}

interface MutableRuntimeEvent {
  schema_version: 2;
  event_id: string;
  run_id: string;
  session_id: string;
  task_id: string;
  turn_id: string | null;
  operation_id: string | null;
  correlation_id: string;
  causation_event_id: string | null;
  sequence: number;
  occurred_at_epoch_ms: number;
  sensitivity: "internal";
  retention: { kind: "ephemeral"; expires_at_epoch_ms: null };
  persistence: "correctness";
  policy_id: string;
  payload_reference: null;
  kind: Record<string, unknown>;
  previous_event_sha256: string;
  event_sha256: string;
}

function runtimeEvent(
  sequence: number,
  previousEventSha256: string,
  causationEventId: string | null,
  kind: Record<string, unknown>,
): MutableRuntimeEvent {
  return {
    schema_version: 2,
    event_id: `event-${sequence.toString().padStart(4, "0")}`,
    run_id: "run-0001",
    session_id: "session-0001",
    task_id: "task-0001",
    turn_id:
      sequence === 0 || kind.event === "run_terminal" ? null : "turn-0001",
    operation_id:
      kind.event === "permission_requested" ||
      kind.event === "permission_decided"
        ? "operation-0001"
        : null,
    correlation_id: "correlation-0001",
    causation_event_id: causationEventId,
    sequence,
    occurred_at_epoch_ms: 1_000 + sequence,
    sensitivity: "internal",
    retention: { kind: "ephemeral", expires_at_epoch_ms: null },
    persistence: "correctness",
    policy_id: "policy-0001",
    payload_reference: null,
    kind,
    previous_event_sha256: previousEventSha256,
    event_sha256: String.fromCharCode("a".charCodeAt(0) + sequence).repeat(64),
  };
}

function evidenceReference(): Record<string, unknown> {
  return {
    schema_version: 2,
    evidence_id: "evidence-0001",
    kind: "validation",
    source_id: "fixture",
    object_id: "runtime output",
    fragment: null,
    content_sha256: "d".repeat(64),
    observed_revision: "fixture-revision",
  };
}
