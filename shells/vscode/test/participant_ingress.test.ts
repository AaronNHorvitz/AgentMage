import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import test from "node:test";

import {
  ParticipantIngressError,
  accountProviderParts,
  runParticipantIngress,
  type ParticipantReferenceInput,
  type ParticipantReferenceResolver,
} from "../src/participant_ingress.js";
import type {
  EngineeringExchange,
  EngineeringHostRequest,
} from "../src/verified_chat_protocol.js";

class ExchangeFixture {
  readonly operations: string[] = [];
  readonly chunks = new Map<string, number[]>();
  private artifact = 0;

  exchange: EngineeringExchange = async (envelope) => {
    await Promise.resolve();
    const operation = String(envelope.request.operation);
    this.operations.push(operation);
    if (operation === "create_session")
      return response(envelope, "session", {});
    if (operation === "begin_artifact") {
      const upload = String(envelope.request.upload_id);
      this.chunks.set(upload, []);
      return response(envelope, "artifact_upload_started", {});
    }
    if (operation === "upload_artifact_chunk") {
      const chunk = envelope.request.chunk as {
        upload_id: string;
        sequence: number;
        bytes: number[];
      };
      const retained = this.chunks.get(chunk.upload_id);
      assert.ok(retained);
      assert.equal(chunk.sequence, retained.length);
      retained.push(...chunk.bytes);
      return response(envelope, "artifact_chunk_accepted", {});
    }
    if (operation === "commit_artifact") {
      const upload = String(envelope.request.upload_id);
      const bytes = this.chunks.get(upload);
      assert.ok(bytes);
      this.artifact += 1;
      return response(envelope, "artifact_captured", {
        capture: {
          artifact_id: `artifact-${this.artifact.toString().padStart(4, "0")}`,
          source_sha256: sha256(Uint8Array.from(bytes)),
          byte_length: bytes.length,
        },
      });
    }
    if (operation === "execute_verified_turn") {
      return response(envelope, "verified_turn_completed", {
        turn: { output_text: "Verified fixture answer." },
      });
    }
    if (operation === "cancel_artifact") {
      return response(envelope, "artifact_upload_cancelled", {});
    }
    return { kind: "denied", code: "fixture.operation-unsupported" };
  };
}

class Resolver implements ParticipantReferenceResolver {
  calls: string[] = [];
  stale = new Set<string>();

  resolve(reference: ParticipantReferenceInput) {
    this.calls.push(reference.referenceId);
    const bytes = Buffer.from(`resolved:${reference.referenceId}`, "utf8");
    return Promise.resolve({
      bytes,
      displayName: reference.displayName,
      mediaType: reference.mediaType,
      revisionBefore: "revision-1",
      revisionAfter: this.stale.has(reference.referenceId)
        ? "revision-2"
        : "revision-1",
    });
  }
}

void test("999 and 1001 character prompts plus supplied references are completely accounted", async () => {
  for (const length of [999, 1001]) {
    const fixture = new ExchangeFixture();
    const resolver = new Resolver();
    const statuses: string[] = [];
    const result = await runParticipantIngress(
      fixture.exchange,
      resolver,
      {
        requestId: `participant-${length.toString()}`,
        prompt: "p".repeat(length),
        command: "ask",
        references: [
          reference("text-1", "text", { text: "explicit text" }),
          reference("file-1", "uri", { uri: "file:///fixture/a.txt" }),
          reference("virtual-1", "uri", { uri: "untitled:fixture" }),
          reference("unknown-1", "unknown", {}),
        ],
      },
      { isCancellationRequested: false },
      (record) => statuses.push(`${record.referenceId}:${record.state}`),
      () => 1_000,
    );
    assert.equal(result.prompt.byteLength, length + "/ask\n".length);
    assert.deepEqual(
      result.sources.map((item) => item.state),
      ["included", "included", "included", "unsupported"],
    );
    assert.deepEqual(resolver.calls, ["file-1", "virtual-1"]);
    assert.match(result.sourceManifestSha256, /^[a-f0-9]{64}$/u);
    assert.ok(result.totalBytes > result.prompt.byteLength);
    assert.equal(result.outputText, "Verified fixture answer.");
    assert.ok(statuses.includes("unknown-1:unsupported"));
    assert.equal(fixture.operations.at(-1), "execute_verified_turn");
  }
});

void test("stale reference remains visible and is never submitted as context", async () => {
  const fixture = new ExchangeFixture();
  const resolver = new Resolver();
  resolver.stale.add("stale-1");
  const result = await runParticipantIngress(
    fixture.exchange,
    resolver,
    {
      requestId: "participant-stale",
      prompt: "inspect",
      command: undefined,
      references: [
        reference("stale-1", "location", { uri: "file:///fixture/stale" }),
      ],
    },
    { isCancellationRequested: false },
  );
  assert.equal(result.sources[0]?.state, "stale");
  assert.equal(result.sources[0]?.artifactId, null);
  assert.equal(
    fixture.operations.filter((operation) => operation === "commit_artifact")
      .length,
    1,
    "only the prompt is committed",
  );
});

void test("cancellation and duplicate identities stop without a model turn", async () => {
  const fixture = new ExchangeFixture();
  await assert.rejects(
    runParticipantIngress(
      fixture.exchange,
      new Resolver(),
      {
        requestId: "participant-cancelled",
        prompt: "inspect",
        command: undefined,
        references: [],
      },
      { isCancellationRequested: true },
    ),
    (error: unknown) =>
      error instanceof ParticipantIngressError &&
      error.code === "vscode.participant.cancelled",
  );
  assert.ok(!fixture.operations.includes("execute_verified_turn"));

  await assert.rejects(
    runParticipantIngress(
      fixture.exchange,
      new Resolver(),
      {
        requestId: "participant-duplicate",
        prompt: "inspect",
        command: undefined,
        references: [
          reference("same", "text", { text: "one" }),
          reference("same", "text", { text: "two" }),
        ],
      },
      { isCancellationRequested: false },
    ),
    ParticipantIngressError,
  );
});

void test("stale model revalidation immediately before submission starts no turn", async () => {
  const fixture = new ExchangeFixture();
  let revalidations = 0;
  await assert.rejects(
    runParticipantIngress(
      fixture.exchange,
      new Resolver(),
      {
        requestId: "participant-model-drift",
        prompt: "inspect",
        command: undefined,
        references: [],
      },
      { isCancellationRequested: false },
      () => undefined,
      () => 1_000,
      () => {
        revalidations += 1;
        return Promise.reject(
          new ParticipantIngressError(
            "vscode.participant.model-stale-before-submit",
          ),
        );
      },
    ),
    (error: unknown) =>
      error instanceof ParticipantIngressError &&
      error.code === "vscode.participant.model-stale-before-submit",
  );
  assert.equal(revalidations, 1);
  assert.ok(!fixture.operations.includes("execute_verified_turn"));
});

void test("provider compatibility reports every non-text part instead of filtering", () => {
  assert.deepEqual(
    accountProviderParts([
      { kind: "text", text: "alpha" },
      { kind: "unknown" },
      { kind: "text", text: "omega" },
      { kind: "unknown" },
    ]),
    {
      text: "alphaomega",
      totalParts: 4,
      unsupportedParts: 2,
      reasonCode: "vscode.provider.part-unsupported",
    },
  );
  assert.deepEqual(accountProviderParts([]), {
    text: "",
    totalParts: 0,
    unsupportedParts: 0,
    reasonCode: null,
  });
});

void test("label-only proxy and MCP-without-resource fixtures cannot claim source bytes", () => {
  const labelOnly = accountProviderParts([
    { kind: "text", text: "Attachment: quarterly report" },
    { kind: "unknown" },
  ]);
  assert.equal(labelOnly.unsupportedParts, 1);
  assert.equal(labelOnly.reasonCode, "vscode.provider.part-unsupported");

  const omittedProxyBody = accountProviderParts([]);
  assert.equal(omittedProxyBody.text, "");
  assert.equal(omittedProxyBody.totalParts, 0);

  const mcpWithoutResource = accountProviderParts([{ kind: "unknown" }]);
  assert.equal(mcpWithoutResource.text, "");
  assert.equal(mcpWithoutResource.unsupportedParts, 1);
  assert.equal(
    mcpWithoutResource.reasonCode,
    "vscode.provider.part-unsupported",
  );
});

function reference(
  referenceId: string,
  kind: ParticipantReferenceInput["kind"],
  optional: { readonly text?: string; readonly uri?: string },
): ParticipantReferenceInput {
  return {
    referenceId,
    kind,
    displayName: `Reference ${referenceId}`,
    mediaType: "text/plain; charset=utf-8",
    ...optional,
  };
}

function response(
  envelope: EngineeringHostRequest,
  result: string,
  fields: Readonly<Record<string, unknown>>,
) {
  return {
    kind: "engineering" as const,
    schema_version: 1 as const,
    request_id: envelope.request_id,
    response: { result, ...fields },
  };
}

function sha256(bytes: Uint8Array): string {
  return createHash("sha256").update(bytes).digest("hex");
}
