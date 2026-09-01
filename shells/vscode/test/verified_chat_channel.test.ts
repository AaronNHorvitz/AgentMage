import assert from "node:assert/strict";
import test from "node:test";

import {
  VerifiedChatCommandChannel,
  VerifiedChatChannelError,
  VerifiedChatRunGate,
} from "../src/verified_chat_channel.js";

void test("one disposable channel admits each exact command once in order", () => {
  const channel = new VerifiedChatCommandChannel("webview-channel-1");
  assert.equal(
    channel.accept({
      type: "ready",
      channel: "webview-channel-1",
      sequence: 0,
    }).type,
    "ready",
  );
  assert.deepEqual(
    channel.accept({
      type: "send",
      channel: "webview-channel-1",
      sequence: 1,
      mode: "ask",
      text: "Inspect the fixture",
    }),
    {
      type: "send",
      channel: "webview-channel-1",
      sequence: 1,
      mode: "ask",
      text: "Inspect the fixture",
    },
  );
});

void test("replay spoof reorder extra fields unknown modes and oversized text fail closed", () => {
  const invalid = [
    { type: "ready", channel: "wrong", sequence: 0 },
    { type: "ready", channel: "webview-channel-1", sequence: 1 },
    {
      type: "ready",
      channel: "webview-channel-1",
      sequence: 0,
      grant: "self-issued",
    },
    {
      type: "send",
      channel: "webview-channel-1",
      sequence: 0,
      mode: "unrestricted",
      text: "x",
    },
    {
      type: "send",
      channel: "webview-channel-1",
      sequence: 0,
      mode: "ask",
      text: "x".repeat(64 * 1024 * 1024 + 1),
    },
  ];
  for (const value of invalid) {
    assert.throws(
      () => new VerifiedChatCommandChannel("webview-channel-1").accept(value),
      VerifiedChatChannelError,
    );
  }
});

void test("invalid input does not consume the next sequence", () => {
  const channel = new VerifiedChatCommandChannel("webview-channel-1");
  assert.throws(
    () =>
      channel.accept({
        type: "ready",
        channel: "wrong",
        sequence: 0,
      }),
    VerifiedChatChannelError,
  );
  assert.equal(
    channel.accept({
      type: "ready",
      channel: "webview-channel-1",
      sequence: 0,
    }).type,
    "ready",
  );
  assert.throws(
    () =>
      channel.accept({
        type: "ready",
        channel: "webview-channel-1",
        sequence: 0,
      }),
    VerifiedChatChannelError,
  );
});

void test("active run gate admits only cancellation and closes the cancellation race", () => {
  const gate = new VerifiedChatRunGate();
  let cancellations = 0;
  gate.begin("view-run-1", "session-1", () => {
    cancellations += 1;
  });
  assert.equal(gate.isActive, true);
  assert.equal(gate.allows("send"), false);
  assert.equal(gate.allows("pause"), false);
  assert.equal(gate.allows("cancel"), true);
  assert.equal(gate.cancel("session-other"), false);
  assert.equal(gate.cancel("session-1"), true);
  assert.equal(cancellations, 1);
  assert.throws(
    () => gate.begin("view-run-2", "session-1", () => undefined),
    VerifiedChatChannelError,
  );
  gate.finish("view-run-stale");
  assert.equal(gate.isActive, true);
  gate.finish("view-run-1");
  assert.equal(gate.isActive, false);
  assert.equal(gate.allows("send"), true);
});
