import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import test from "node:test";

import {
  captureExactText,
  ENGINEERING_CHUNK_BYTES,
  parseEngineeringHostResponse,
  type EngineeringHostRequest,
} from "../src/verified_chat_protocol.js";

void test("50,000-character paste reaches the host in exact ordered chunks", async () => {
  const text = `${"0123456789".repeat(5_000)}\nterminal`;
  const retained: number[] = [];
  let expectedSequence = 0;
  const exchange = (request: EngineeringHostRequest) => {
    const operation = request.request.operation;
    if (operation === "begin_artifact") {
      return Promise.resolve(
        response(request.request_id, {
          result: "artifact_upload_started",
          upload_id: request.request.upload_id,
        }),
      );
    }
    if (operation === "upload_artifact_chunk") {
      const chunk = request.request.chunk as {
        readonly sequence: number;
        readonly bytes: readonly number[];
      };
      assert.equal(chunk.sequence, expectedSequence);
      expectedSequence += 1;
      assert.ok(chunk.bytes.length <= ENGINEERING_CHUNK_BYTES);
      retained.push(...chunk.bytes);
      return Promise.resolve(
        response(request.request_id, {
          result: "artifact_chunk_accepted",
          upload_id: "upload-fixture",
          sequence: chunk.sequence,
        }),
      );
    }
    if (operation === "commit_artifact") {
      const bytes = Buffer.from(retained);
      return Promise.resolve(
        response(request.request_id, {
          result: "artifact_captured",
          capture: {
            artifact_id: "verified-artifact-fixture",
            source_sha256: createHash("sha256").update(bytes).digest("hex"),
            byte_length: bytes.length,
          },
        }),
      );
    }
    return Promise.resolve({ kind: "denied" as const, code: "unexpected" });
  };

  const captured = await captureExactText(
    exchange,
    "session-fixture",
    "paste",
    "Pasted text",
    "text/plain",
    text,
    () => 1,
  );
  assert.deepEqual(Buffer.from(retained), Buffer.from(text, "utf8"));
  assert.equal(captured.byteLength, Buffer.byteLength(text));
  assert.ok(expectedSequence > 1);
});

void test("unknown Engineering response fields fail closed", () => {
  assert.throws(() =>
    parseEngineeringHostResponse({
      kind: "engineering",
      schema_version: 1,
      request_id: "request-1",
      response: { result: "sessions", sessions: [], authority: "invented" },
      extra: true,
    }),
  );
});

function response(requestId: string, body: Readonly<Record<string, unknown>>) {
  return {
    kind: "engineering" as const,
    schema_version: 1 as const,
    request_id: requestId,
    response: body,
  };
}
