import { createHash, randomUUID } from "node:crypto";

export const ENGINEERING_CHUNK_BYTES = 12 * 1024;
export const MAX_VERIFIED_SOURCE_BYTES = 64 * 1024 * 1024;

export type EngineeringMode = "ask" | "plan" | "agent" | "team";

export interface EngineeringHostRequest {
  readonly kind: "engineering";
  readonly schema_version: 1;
  readonly request_id: string;
  readonly request: Readonly<Record<string, unknown>>;
}

export interface EngineeringHostResponse {
  readonly kind: "engineering";
  readonly schema_version: 1;
  readonly request_id: string;
  readonly response: Readonly<Record<string, unknown>>;
}

export type EngineeringExchange = (
  request: EngineeringHostRequest,
) => Promise<
  EngineeringHostResponse | { readonly kind: "denied"; readonly code: string }
>;

export interface CapturedSource {
  readonly artifactId: string;
  readonly sourceSha256: string;
  readonly byteLength: number;
}

/** Captures exact UTF-8 bytes through ordered, digest-bound host RPC frames. */
export async function captureExactText(
  exchange: EngineeringExchange,
  sessionId: string,
  sourceKind: "paste" | "editor_selection",
  displayName: string,
  mediaType: string,
  text: string,
  now: () => number = Date.now,
): Promise<CapturedSource> {
  const bytes = Buffer.from(text, "utf8");
  return captureExactBytes(
    exchange,
    sessionId,
    sourceKind,
    displayName,
    mediaType,
    bytes,
    now,
  );
}

/** Captures exact explicitly selected file bytes through the same host authority. */
export async function captureExactBytes(
  exchange: EngineeringExchange,
  sessionId: string,
  sourceKind: "paste" | "editor_selection" | "file",
  displayName: string,
  mediaType: string,
  bytes: Uint8Array,
  now: () => number = Date.now,
): Promise<CapturedSource> {
  if (bytes.length === 0 || bytes.length > MAX_VERIFIED_SOURCE_BYTES) {
    throw new VerifiedChatProtocolError("verified-chat.source.size-invalid");
  }
  const uploadId = `upload-${randomUUID()}`;
  const sourceSha256 = sha256(bytes);
  await requireResult(
    exchange(
      request("begin_artifact", {
        upload_id: uploadId,
        session_id: sessionId,
        source_kind: sourceKind,
        display_name: displayName,
        media_type: mediaType,
        total_bytes: bytes.length,
        expected_sha256: sourceSha256,
      }),
    ),
    "artifact_upload_started",
  );
  try {
    for (let offset = 0, sequence = 0; offset < bytes.length; sequence += 1) {
      const end = Math.min(offset + ENGINEERING_CHUNK_BYTES, bytes.length);
      const chunk = bytes.subarray(offset, end);
      await requireResult(
        exchange(
          request("upload_artifact_chunk", {
            chunk: {
              schema_version: 1,
              upload_id: uploadId,
              session_id: sessionId,
              sequence,
              offset,
              total_bytes: bytes.length,
              bytes: [...chunk],
              chunk_sha256: sha256(chunk),
              final_chunk: end === bytes.length,
            },
          }),
        ),
        "artifact_chunk_accepted",
      );
      offset = end;
    }
    const completed = await requireResult(
      exchange(
        request("commit_artifact", {
          upload_id: uploadId,
          completed_at_epoch_ms: now(),
        }),
      ),
      "artifact_captured",
    );
    const capture = record(completed.capture);
    const artifactId = stringField(capture, "artifact_id");
    if (
      stringField(capture, "source_sha256") !== sourceSha256 ||
      numberField(capture, "byte_length") !== bytes.length
    ) {
      throw new VerifiedChatProtocolError("verified-chat.capture.mismatch");
    }
    return { artifactId, sourceSha256, byteLength: bytes.length };
  } catch (error) {
    await exchange(request("cancel_artifact", { upload_id: uploadId })).catch(
      () => undefined,
    );
    throw error;
  }
}

/** Creates one exact closed host request with a fresh correlation identity. */
export function request(
  operation: string,
  fields: Readonly<Record<string, unknown>> = {},
): EngineeringHostRequest {
  return {
    kind: "engineering",
    schema_version: 1,
    request_id: `request-${randomUUID()}`,
    request: { operation, ...fields },
  };
}

/** Parses only the closed Engineering host response shape. */
export function parseEngineeringHostResponse(
  value: unknown,
): EngineeringHostResponse {
  const candidate = record(value);
  exactKeys(candidate, ["kind", "request_id", "response", "schema_version"]);
  if (
    candidate.kind !== "engineering" ||
    candidate.schema_version !== 1 ||
    !validIdentifier(candidate.request_id)
  ) {
    throw new VerifiedChatProtocolError("verified-chat.response.invalid");
  }
  const response = record(candidate.response);
  const result = response.result;
  if (
    typeof result !== "string" ||
    !new Set([
      "session",
      "sessions",
      "artifact_upload_started",
      "artifact_chunk_accepted",
      "artifact_captured",
      "artifact_upload_cancelled",
      "artifact_range",
      "artifact_ingested",
      "verified_turn_completed",
      "plan_approved",
      "runtime_bound",
      "team_campaign_completed",
      "events",
      "lifecycle_event",
    ]).has(result)
  ) {
    throw new VerifiedChatProtocolError("verified-chat.response.invalid");
  }
  return candidate as unknown as EngineeringHostResponse;
}

export class VerifiedChatProtocolError extends Error {
  constructor(code: string) {
    super(code);
  }
}

async function requireResult(
  pending: ReturnType<EngineeringExchange>,
  expected: string,
): Promise<Record<string, unknown>> {
  const value = await pending;
  if (value.kind === "denied") {
    throw new VerifiedChatProtocolError(value.code);
  }
  const response = record(value.response);
  if (response.result !== expected) {
    throw new VerifiedChatProtocolError("verified-chat.response.mismatch");
  }
  return response;
}

function sha256(bytes: Uint8Array): string {
  return createHash("sha256").update(bytes).digest("hex");
}

function record(value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new VerifiedChatProtocolError("verified-chat.response.invalid");
  }
  return value as Record<string, unknown>;
}

function stringField(value: Record<string, unknown>, key: string): string {
  const field = value[key];
  if (typeof field !== "string" || field.length === 0) {
    throw new VerifiedChatProtocolError("verified-chat.response.invalid");
  }
  return field;
}

function numberField(value: Record<string, unknown>, key: string): number {
  const field = value[key];
  if (typeof field !== "number" || !Number.isSafeInteger(field) || field < 0) {
    throw new VerifiedChatProtocolError("verified-chat.response.invalid");
  }
  return field;
}

function validIdentifier(value: unknown): value is string {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    value.length <= 128 &&
    /^[A-Za-z0-9._:-]+$/u.test(value)
  );
}

function exactKeys(
  value: Record<string, unknown>,
  expected: readonly string[],
): void {
  const actual = Object.keys(value).sort();
  const wanted = [...expected].sort();
  if (
    actual.length !== wanted.length ||
    actual.some((item, index) => item !== wanted[index])
  ) {
    throw new VerifiedChatProtocolError("verified-chat.response.invalid");
  }
}
