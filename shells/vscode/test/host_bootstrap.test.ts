import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { PassThrough } from "node:stream";
import test from "node:test";

import { launchWith } from "../src/host_bootstrap.js";

void test("verified binary bootstrap names this process and remains supervised", async () => {
  const child = new FakeChild();
  const frame = validFrame();
  const bridge = await launchWith(() => {
    queueMicrotask(() => child.stdout.write(frame));
    return child;
  }, 100);
  assert.equal(child.killed, false);
  bridge.dispose();
  assert.equal(child.killed, true);
});

void test("wrong process and trailing bytes fail closed and kill the child", async () => {
  for (const frame of [
    validFrame({ pid: process.pid + 1 }),
    Buffer.concat([validFrame(), Buffer.from([0])]),
  ]) {
    const child = new FakeChild();
    const bridge = await launchWith(() => {
      queueMicrotask(() => child.stdout.write(frame));
      return child;
    }, 100);
    assert.equal(child.killed, true);
    const response = await bridge.previewRead({
      kind: "preview_read",
      schema_version: 1,
      request_id: "request-0001",
      workspace_id: "workspace-0001",
      workspace_root: "/tmp/workspace",
      components: ["README.md"],
    });
    assert.equal(response.kind, "denied");
    assert.equal(
      response.kind === "denied" ? response.code : "",
      "host.connection.unavailable",
    );
  }
});

void test("malformed, oversized, silent, and throwing launchers remain unavailable", async () => {
  const cases: Array<
    () => Promise<{
      bridge: Awaited<ReturnType<typeof launchWith>>;
      child?: FakeChild;
    }>
  > = [
    async () => {
      const child = new FakeChild();
      const bridge = await launchWith(() => {
        queueMicrotask(() => child.stdout.write(Buffer.from("not-a-frame")));
        return child;
      }, 20);
      return { bridge, child };
    },
    async () => {
      const child = new FakeChild();
      const bridge = await launchWith(() => {
        queueMicrotask(() => child.stdout.write(Buffer.alloc(500)));
        return child;
      }, 20);
      return { bridge, child };
    },
    async () => {
      const child = new FakeChild();
      return { bridge: await launchWith(() => child, 5), child };
    },
    async () => ({
      bridge: await launchWith(() => {
        throw new Error("synthetic launch failure");
      }, 20),
    }),
  ];
  for (const run of cases) {
    const { bridge, child } = await run();
    assert.equal(child?.killed ?? true, true);
    const response = await bridge.cancelRead({
      kind: "cancel_read",
      schema_version: 1,
      request_id: "request-0001",
      preview_id: "preview-0001",
    });
    assert.equal(response.kind, "denied");
    assert.equal(
      response.kind === "denied" ? response.code : "",
      "host.connection.unavailable",
    );
  }
});

class FakeChild extends EventEmitter {
  readonly stdout = new PassThrough();
  readonly stderr = new PassThrough();
  killed = false;

  kill(): boolean {
    this.killed = true;
    return true;
  }
}

function validFrame(overrides: { readonly pid?: number } = {}): Buffer {
  const uid = process.getuid?.() ?? 0;
  const endpoint = Buffer.from(
    `/run/user/${uid.toString()}/agentmage/host-123-456.sock`,
    "utf8",
  );
  const frame = Buffer.alloc(8 + endpoint.length + 32 + 32 + 4 + 4 + 8 + 32);
  frame.write("AGMB", 0, "ascii");
  frame.writeUInt16BE(1, 4);
  frame.writeUInt16BE(endpoint.length, 6);
  endpoint.copy(frame, 8);
  let offset = 8 + endpoint.length;
  Buffer.alloc(32, 7).copy(frame, offset);
  offset += 32;
  Buffer.alloc(32, 9).copy(frame, offset);
  offset += 32;
  frame.writeUInt32BE(uid, offset);
  offset += 4;
  frame.writeInt32BE(overrides.pid ?? process.pid, offset);
  offset += 4;
  frame.writeBigUInt64BE(456n, offset);
  offset += 8;
  Buffer.alloc(32, 11).copy(frame, offset);
  return frame;
}
