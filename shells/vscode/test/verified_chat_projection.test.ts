import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import test from "node:test";

import {
  BoundedProjectionDelivery,
  VerifiedChatEventProjection,
  VerifiedChatProjectionError,
  parseVerifiedChatSessionSnapshot,
} from "../src/verified_chat_projection.js";

const ZERO = "0".repeat(64);

function seal<T extends Readonly<Record<string, unknown>>>(
  value: T,
  digestField: string,
): T {
  return {
    ...value,
    [digestField]: createHash("sha256")
      .update(JSON.stringify(value), "utf8")
      .digest("hex"),
  };
}

function event(
  sequence: number,
  previous: string,
  kind: Readonly<Record<string, unknown>>,
): Readonly<Record<string, unknown>> {
  return seal(
    {
      schema_version: 1,
      event_id: `event-${sequence}`,
      session_id: "session-1",
      task_id: null,
      sequence,
      correlation_id: "correlation-1",
      occurred_at_epoch_ms: 1000 + sequence,
      kind,
      previous_event_sha256: previous,
      event_sha256: ZERO,
    },
    "event_sha256",
  );
}

void test("durable events project in exact order to accessible content-free cards", () => {
  const projection = new VerifiedChatEventProjection("session-1");
  const cards = projection.accept([
    event(0, ZERO, { event: "session_created" }),
    event(
      1,
      event(0, ZERO, { event: "session_created" }).event_sha256 as string,
      {
        event: "tool_observed",
        tool_call_id: "tool-call-1",
      },
    ),
  ]);
  assert.deepEqual(
    cards.map((card) => [card.sequence, card.category]),
    [
      [0, "session"],
      [1, "tool"],
    ],
  );
  assert.equal(projection.cursor, 1);
  assert.equal(projection.terminal, undefined);
});

void test("terminal, tool, verification, artifact, and lifecycle cards preserve closed truth", () => {
  const projection = new VerifiedChatEventProjection("session-1");
  const kinds = [
    { event: "session_created" },
    { event: "artifact_captured", artifact_id: "artifact-1" },
    { event: "context_admitted", context_packet_id: "context-1" },
    { event: "model_route_selected", route_decision_id: "route-1" },
    { event: "tool_observed", tool_call_id: "tool-call-1" },
    { event: "verification_completed", terminal: "SUCCESS" },
    { event: "task_paused" },
    { event: "task_resumed" },
    { event: "terminal", state: "SUCCESS" },
  ];
  let previous = ZERO;
  const values = kinds.map((kind, sequence) => {
    const value = event(sequence, previous, kind);
    previous = value.event_sha256 as string;
    return value;
  });
  const cards = projection.accept(values);
  assert.deepEqual(
    cards.map((card) => card.category),
    [
      "session",
      "artifact",
      "context",
      "model",
      "tool",
      "verification",
      "lifecycle",
      "lifecycle",
      "terminal",
    ],
  );
  assert.equal(projection.terminal, "SUCCESS");
  assert.throws(
    () => projection.accept([values[0]]),
    VerifiedChatProjectionError,
  );
});

void test("wrong session replay reorder duplication drift unknown fields and false terminal fail closed", () => {
  const mutations: Readonly<Record<string, unknown>>[] = [];
  const valid = event(0, ZERO, { event: "session_created" });
  mutations.push({ ...valid, session_id: "session-spoofed" });
  mutations.push({ ...valid, sequence: 1 });
  mutations.push({ ...valid, previous_event_sha256: "b".repeat(64) });
  mutations.push({ ...valid, event_sha256: "a".repeat(64) });
  mutations.push({ ...valid, extra: true });
  mutations.push({ ...valid, kind: { event: "terminal", state: "complete" } });
  mutations.push({
    ...valid,
    kind: { event: "model_reasoning", text: "secret" },
  });
  for (const mutation of mutations) {
    assert.throws(
      () => new VerifiedChatEventProjection("session-1").accept([mutation]),
      VerifiedChatProjectionError,
    );
  }
  const projection = new VerifiedChatEventProjection("session-1");
  projection.accept([valid]);
  assert.throws(() => projection.accept([valid]), VerifiedChatProjectionError);
});

void test("replay pages apply atomically when a later event is invalid", () => {
  const projection = new VerifiedChatEventProjection("session-1");
  assert.throws(
    () =>
      projection.accept([
        event(0, ZERO, { event: "session_created" }),
        event(2, "a".repeat(64), { event: "task_paused" }),
      ]),
    VerifiedChatProjectionError,
  );
  assert.equal(projection.cursor, undefined);
  assert.equal(
    projection.accept([event(0, ZERO, { event: "session_created" })]).length,
    1,
  );
});

void test("durable approved Plan binding is reconstructed without execution authority", () => {
  const approval = seal(
    {
      schema_version: 1,
      approval_id: "approval-1",
      session_id: "session-1",
      plan_artifact_id: "artifact-plan-1",
      plan_sha256: "c".repeat(64),
      approved_by: "actor-user-1",
      approved_at_epoch_ms: 2000,
      approval_sha256: ZERO,
    },
    "approval_sha256",
  );
  const projection = new VerifiedChatEventProjection("session-1");
  projection.accept([
    event(0, ZERO, { event: "session_created" }),
    event(
      1,
      event(0, ZERO, { event: "session_created" }).event_sha256 as string,
      { event: "plan_approved", approval },
    ),
  ]);
  assert.deepEqual(projection.approvedPlan, {
    sourceSessionId: "session-1",
    artifactId: "artifact-plan-1",
    planSha256: "c".repeat(64),
    approvalId: "approval-1",
    approvalSha256: approval.approval_sha256,
  });
  const tampered = { ...approval, approved_by: "actor-spoofed" };
  assert.throws(
    () =>
      new VerifiedChatEventProjection("session-1").accept([
        event(0, ZERO, { event: "plan_approved", approval: tampered }),
      ]),
    VerifiedChatProjectionError,
  );
});

void test("bounded delivery preserves order and rejects slow-consumer pressure", async () => {
  const delivered: number[] = [];
  let release: (() => void) | undefined;
  const gate = new Promise<void>((resolve) => {
    release = resolve;
  });
  const delivery = new BoundedProjectionDelivery<{ readonly value: number }>(
    async (value) => {
      await gate;
      delivered.push(value.value);
      return true;
    },
    2,
    1024,
  );
  const first = delivery.enqueue({ value: 1 });
  const second = delivery.enqueue({ value: 2 });
  await assert.rejects(
    delivery.enqueue({ value: 3 }),
    /verified-chat.delivery.backpressure/u,
  );
  release?.();
  await Promise.all([first, second]);
  assert.deepEqual(delivered, [1, 2]);
});

void test("bounded delivery rejects oversized messages, sender refusal, and post-disposal use", async () => {
  const oversized = new BoundedProjectionDelivery<{ readonly text: string }>(
    () => Promise.resolve(true),
    2,
    16,
  );
  await assert.rejects(
    oversized.enqueue({ text: "x".repeat(32) }),
    /verified-chat.delivery.backpressure/u,
  );
  const refused = new BoundedProjectionDelivery<{ readonly value: number }>(
    () => Promise.resolve(false),
  );
  await assert.rejects(
    refused.enqueue({ value: 1 }),
    /verified-chat.delivery.rejected/u,
  );
  refused.close();
  await assert.rejects(
    refused.enqueue({ value: 2 }),
    /verified-chat.delivery.closed/u,
  );
});

void test("reload snapshot restores only closed session and artifact projections", () => {
  const snapshot = seal(
    {
      schema_version: 1,
      session_id: "session-1",
      title: "Verified Chat",
      mode: "plan",
      model_profile_id: null,
      endpoint_profile_id: null,
      active_task_id: null,
      artifacts: [
        {
          schema_version: 1,
          upload_id: "upload-1",
          artifact_id: "artifact-1",
          session_id: "session-1",
          source_kind: "paste",
          display_name: "Prompt",
          media_type: "text/plain",
          byte_length: 6,
          line_count: 1,
          source_sha256: "a".repeat(64),
          disposition: "captured_exactly",
          payload_deduplicated: false,
          completed_at_epoch_ms: 1000,
          warning_codes: [],
          receipt_sha256: "b".repeat(64),
        },
      ],
      last_event_sequence: 4,
      terminal: null,
      snapshot_sha256: ZERO,
    },
    "snapshot_sha256",
  );
  assert.deepEqual(parseVerifiedChatSessionSnapshot(snapshot), {
    sessionId: "session-1",
    mode: "plan",
    artifacts: [
      {
        artifactId: "artifact-1",
        sourceKind: "paste",
        displayName: "Prompt",
        mediaType: "text/plain",
        byteLength: 6,
        sourceSha256: "a".repeat(64),
        disposition: "captured_exactly",
        warningCodes: [],
      },
    ],
    terminal: null,
  });
  for (const mutation of [
    { ...snapshot, session_id: "../spoof" },
    { ...snapshot, mode: "unrestricted" },
    { ...snapshot, terminal: "COMPLETE" },
    { ...snapshot, extra: true },
    { ...snapshot, snapshot_sha256: "a".repeat(64) },
    {
      ...snapshot,
      artifacts: [{ ...snapshot.artifacts[0], session_id: "session-other" }],
    },
  ]) {
    assert.throws(
      () => parseVerifiedChatSessionSnapshot(mutation),
      VerifiedChatProjectionError,
    );
  }
});
