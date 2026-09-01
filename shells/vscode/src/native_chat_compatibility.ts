import { createHash } from "node:crypto";

export const NATIVE_CHAT_COMPATIBILITY_VERSION = 1;
export const MAX_NATIVE_PROVIDER_MESSAGES = 64;
export const MAX_NATIVE_PROVIDER_PARTS = 256;
export const MAX_NATIVE_PROVIDER_PROMPT_BYTES = 4096;
export const VERIFIED_CHAT_COMMAND = "agentmage.openVerifiedChat" as const;

export interface NativeProviderMessageInput {
  readonly role: "user" | "assistant" | "unknown";
  readonly name: string | undefined;
  readonly parts: readonly {
    readonly kind: "text" | "unknown";
    readonly text?: string;
  }[];
}

export interface NativeProviderOptionsInput {
  readonly toolCount: number;
  readonly toolMode: "auto" | "required" | "unknown";
  readonly modelOptionKeys: readonly string[];
}

export type NativeProviderProjection =
  | Readonly<{
      status: "supported";
      prompt: string;
      promptSha256: string;
      messageCount: number;
      partCount: number;
      inputUtf8Bytes: number;
      limitations: readonly string[];
    }>
  | Readonly<{
      status: "blocked";
      code: string;
      messageCount: number;
      partCount: number;
      unsupportedPartCount: number;
      verifiedChatCommand: typeof VERIFIED_CHAT_COMMAND;
    }>;

/**
 * Preserves the complete stable text-message history in one closed canonical prompt projection.
 * Unsupported native semantics stop before a model request and name the canonical transition.
 */
export function projectNativeProviderRequest(
  messages: readonly NativeProviderMessageInput[],
  options: NativeProviderOptionsInput,
): NativeProviderProjection {
  const partCount = messages.reduce(
    (total, message) => total + message.parts.length,
    0,
  );
  const unsupportedPartCount = messages.reduce(
    (total, message) =>
      total + message.parts.filter((part) => part.kind !== "text").length,
    0,
  );
  const blocked = (code: string): NativeProviderProjection => ({
    status: "blocked",
    code,
    messageCount: messages.length,
    partCount,
    unsupportedPartCount,
    verifiedChatCommand: VERIFIED_CHAT_COMMAND,
  });
  if (
    messages.length === 0 ||
    messages.length > MAX_NATIVE_PROVIDER_MESSAGES ||
    partCount === 0 ||
    partCount > MAX_NATIVE_PROVIDER_PARTS
  ) {
    return blocked("vscode.provider.message-bounds-unsupported");
  }
  if (
    options.toolCount > 0 ||
    options.toolMode === "required" ||
    options.toolMode === "unknown"
  ) {
    return blocked("vscode.provider.external-tools-unsupported");
  }
  if (options.modelOptionKeys.length > 0) {
    return blocked("vscode.provider.model-options-unsupported");
  }
  if (
    unsupportedPartCount > 0 ||
    messages.some(
      (message) =>
        message.role === "unknown" ||
        (message.name !== undefined && !validName(message.name)) ||
        message.parts.some(
          (part) =>
            part.kind === "text" &&
            (typeof part.text !== "string" || part.text.includes("\0")),
        ),
    )
  ) {
    return blocked("vscode.provider.part-unsupported");
  }
  if (messages.at(-1)?.role !== "user") {
    return blocked("vscode.provider.final-user-message-required");
  }
  const record = {
    schema_version: NATIVE_CHAT_COMPATIBILITY_VERSION,
    record_type: "agentmage-native-chat-text-projection",
    messages: messages.map((message) => ({
      role: message.role,
      name: message.name ?? null,
      parts: message.parts.map((part) => part.text as string),
    })),
  };
  const prompt = JSON.stringify(record);
  const inputUtf8Bytes = Buffer.byteLength(prompt, "utf8");
  if (inputUtf8Bytes > MAX_NATIVE_PROVIDER_PROMPT_BYTES) {
    return blocked("vscode.provider.context-limit-unsupported");
  }
  return {
    status: "supported",
    prompt,
    promptSha256: sha256(prompt),
    messageCount: messages.length,
    partCount,
    inputUtf8Bytes,
    limitations: [
      "native-chat.text-only",
      "native-chat.external-tools-unavailable",
      "native-chat.no-persistent-lifecycle",
      "native-chat.no-complete-inspector",
      "native-chat.token-usage-unavailable",
    ],
  };
}

export interface NativeProviderRouteDisclosure {
  readonly schema_version: 1;
  readonly record_type: "agentmage-native-chat-route-disclosure";
  readonly profile_id: string;
  readonly manifest_sha256: string;
  readonly runtime_adapter_id: string;
  readonly runtime_sha256: string;
  readonly route_state: "requested_profile";
  readonly endpoint_class: "strict_local";
  readonly fallback: false;
  readonly request_sha256: string;
  readonly limitations: readonly string[];
  readonly verified_chat_command: typeof VERIFIED_CHAT_COMMAND;
}

export function nativeProviderRouteDisclosure(
  profile: {
    readonly profileId: string;
    readonly manifestSha256: string;
    readonly runtimeAdapterId: string;
    readonly runtimeSha256: string;
  },
  projection: Extract<NativeProviderProjection, { status: "supported" }>,
): NativeProviderRouteDisclosure {
  return {
    schema_version: 1,
    record_type: "agentmage-native-chat-route-disclosure",
    profile_id: profile.profileId,
    manifest_sha256: profile.manifestSha256,
    runtime_adapter_id: profile.runtimeAdapterId,
    runtime_sha256: profile.runtimeSha256,
    route_state: "requested_profile",
    endpoint_class: "strict_local",
    fallback: false,
    request_sha256: projection.promptSha256,
    limitations: projection.limitations,
    verified_chat_command: VERIFIED_CHAT_COMMAND,
  };
}

export interface NativeProviderUsageDisclosure {
  readonly schema_version: 1;
  readonly record_type: "agentmage-native-chat-usage-disclosure";
  readonly input_messages: number;
  readonly input_parts: number;
  readonly input_utf8_bytes: number;
  readonly output_parts: number;
  readonly output_utf8_bytes: number;
  readonly exact_input_tokens: null;
  readonly exact_output_tokens: null;
  readonly token_usage_state: "unavailable";
  readonly reason_code: "vscode.provider.exact-token-usage-unavailable";
}

export function nativeProviderUsageDisclosure(
  projection: Extract<NativeProviderProjection, { status: "supported" }>,
  outputParts: readonly string[],
): NativeProviderUsageDisclosure {
  return {
    schema_version: 1,
    record_type: "agentmage-native-chat-usage-disclosure",
    input_messages: projection.messageCount,
    input_parts: projection.partCount,
    input_utf8_bytes: projection.inputUtf8Bytes,
    output_parts: outputParts.length,
    output_utf8_bytes: outputParts.reduce(
      (total, part) => total + Buffer.byteLength(part, "utf8"),
      0,
    ),
    exact_input_tokens: null,
    exact_output_tokens: null,
    token_usage_state: "unavailable",
    reason_code: "vscode.provider.exact-token-usage-unavailable",
  };
}

export function renderNativeProviderBlock(
  projection: Extract<NativeProviderProjection, { status: "blocked" }>,
): string {
  return `# Request stopped\n\nNative Chat cannot preserve every requested semantic guarantee, so AgentMage started no model or tool work. Open **AgentMage: Open Verified Chat** and retry there.\n\n- Messages: ${projection.messageCount.toString()}\n- Parts: ${projection.partCount.toString()}\n- Unsupported parts: ${projection.unsupportedPartCount.toString()}\n- Transition command: \`${projection.verifiedChatCommand}\`\n- Code: \`${projection.code}\``;
}

function validName(value: string): boolean {
  if (value.length === 0 || value.length > 128) return false;
  for (const character of value) {
    const code = character.codePointAt(0);
    if (code === undefined || code < 0x20 || code === 0x7f) return false;
  }
  return true;
}

function sha256(value: string): string {
  return createHash("sha256").update(value, "utf8").digest("hex");
}
