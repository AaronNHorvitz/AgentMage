import assert from "node:assert/strict";
import test from "node:test";

import {
  nativeProviderRouteDisclosure,
  nativeProviderUsageDisclosure,
  projectNativeProviderRequest,
  renderNativeProviderBlock,
  type NativeProviderMessageInput,
} from "../src/native_chat_compatibility.js";

const OPTIONS = {
  toolCount: 0,
  toolMode: "auto" as const,
  modelOptionKeys: [],
};

void test("provider preserves every supported role text part and message boundary", () => {
  const projection = projectNativeProviderRequest(
    [
      {
        role: "user",
        name: "operator",
        parts: [{ kind: "text", text: "first" }],
      },
      {
        role: "assistant",
        name: undefined,
        parts: [
          { kind: "text", text: "second-a" },
          { kind: "text", text: "second-b" },
        ],
      },
      {
        role: "user",
        name: undefined,
        parts: [{ kind: "text", text: "third" }],
      },
    ],
    OPTIONS,
  );
  assert.equal(projection.status, "supported");
  if (projection.status !== "supported") return;
  assert.equal(projection.messageCount, 3);
  assert.equal(projection.partCount, 4);
  const prompt = JSON.parse(projection.prompt) as {
    messages: { role: string; name: string | null; parts: string[] }[];
  };
  assert.deepEqual(prompt.messages, [
    { role: "user", name: "operator", parts: ["first"] },
    {
      role: "assistant",
      name: null,
      parts: ["second-a", "second-b"],
    },
    { role: "user", name: null, parts: ["third"] },
  ]);
  assert.match(projection.promptSha256, /^[0-9a-f]{64}$/u);
});

void test("tools options unknown parts invalid roles and oversize route visibly to Verified Chat", () => {
  const baseMessage: NativeProviderMessageInput = {
    role: "user",
    name: undefined,
    parts: [{ kind: "text", text: "inspect" }],
  };
  const base = [baseMessage];
  const blocked = [
    projectNativeProviderRequest(base, { ...OPTIONS, toolCount: 1 }),
    projectNativeProviderRequest(base, {
      ...OPTIONS,
      modelOptionKeys: ["temperature"],
    }),
    projectNativeProviderRequest(
      [{ ...baseMessage, parts: [{ kind: "unknown" as const }] }],
      OPTIONS,
    ),
    projectNativeProviderRequest(
      [{ ...baseMessage, role: "unknown" as const }],
      OPTIONS,
    ),
    projectNativeProviderRequest(
      [
        {
          ...baseMessage,
          parts: [{ kind: "text" as const, text: "x".repeat(4096) }],
        },
      ],
      OPTIONS,
    ),
  ];
  for (const projection of blocked) {
    assert.equal(projection.status, "blocked");
    if (projection.status !== "blocked") continue;
    assert.equal(projection.verifiedChatCommand, "agentmage.openVerifiedChat");
    assert.match(renderNativeProviderBlock(projection), /Open Verified Chat/u);
  }
});

void test("route and usage disclose exact available facts and unavailable token accounting", () => {
  const projection = projectNativeProviderRequest(
    [
      {
        role: "user",
        name: undefined,
        parts: [{ kind: "text", text: "inspect" }],
      },
    ],
    OPTIONS,
  );
  assert.equal(projection.status, "supported");
  if (projection.status !== "supported") return;
  assert.deepEqual(
    nativeProviderRouteDisclosure(
      {
        profileId: "profile-1",
        manifestSha256: "a".repeat(64),
        runtimeAdapterId: "runtime-1",
        runtimeSha256: "b".repeat(64),
      },
      projection,
    ),
    {
      schema_version: 1,
      record_type: "agentmage-native-chat-route-disclosure",
      profile_id: "profile-1",
      manifest_sha256: "a".repeat(64),
      runtime_adapter_id: "runtime-1",
      runtime_sha256: "b".repeat(64),
      route_state: "requested_profile",
      endpoint_class: "strict_local",
      fallback: false,
      request_sha256: projection.promptSha256,
      limitations: projection.limitations,
      verified_chat_command: "agentmage.openVerifiedChat",
    },
  );
  assert.deepEqual(nativeProviderUsageDisclosure(projection, ["one", "two"]), {
    schema_version: 1,
    record_type: "agentmage-native-chat-usage-disclosure",
    input_messages: 1,
    input_parts: 1,
    input_utf8_bytes: projection.inputUtf8Bytes,
    output_parts: 2,
    output_utf8_bytes: 6,
    exact_input_tokens: null,
    exact_output_tokens: null,
    token_usage_state: "unavailable",
    reason_code: "vscode.provider.exact-token-usage-unavailable",
  });
});
