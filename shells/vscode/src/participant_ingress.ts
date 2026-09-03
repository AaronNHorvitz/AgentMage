import { createHash, randomUUID } from "node:crypto";

import {
  captureExactBytes,
  captureExactText,
  request,
  type CapturedSource,
  type EngineeringExchange,
} from "./verified_chat_protocol.js";

export const PARTICIPANT_ID = "agentmage.participant" as const;
export const MAX_PARTICIPANT_REFERENCES = 32;
export const MAX_PARTICIPANT_TOTAL_BYTES = 64 * 1024 * 1024;
export const PARTICIPANT_REFERENCE_TIMEOUT_MS = 30_000;
export const MAX_PARTICIPANT_STATUS_CHARACTERS = 256;
export const PARTICIPANT_ACCESSIBILITY_CONTRACT = Object.freeze({
  statusDelivery: "chat-progress-polite",
  cancellation: "request-token-and-standard-chat-cancel",
  focusBehavior: "preserve-chat-input",
  errorDetail: "bounded-reason-code-only",
});

export type ParticipantSourceState =
  | "queued"
  | "reading"
  | "extracting"
  | "partial"
  | "unsupported"
  | "omitted"
  | "stale"
  | "cancelled"
  | "failed"
  | "included";

export interface ParticipantReferenceInput {
  readonly referenceId: string;
  readonly kind: "text" | "uri" | "location" | "unknown";
  readonly text?: string;
  readonly uri?: string;
  readonly displayName: string;
  readonly mediaType: string;
  readonly range?: readonly [number, number];
}

export interface ParticipantRequestInput {
  readonly requestId: string;
  readonly prompt: string;
  readonly command: string | undefined;
  readonly references: readonly ParticipantReferenceInput[];
}

export interface ResolvedParticipantReference {
  readonly bytes: Uint8Array;
  readonly displayName: string;
  readonly mediaType: string;
  readonly revisionBefore: string;
  readonly revisionAfter: string;
}

export interface ParticipantReferenceResolver {
  resolve(
    reference: ParticipantReferenceInput,
  ): Promise<ResolvedParticipantReference>;
}

export interface ParticipantCancellation {
  readonly isCancellationRequested: boolean;
  onCancellationRequested?(listener: () => void): { dispose(): void };
}

export interface ParticipantSourceRecord {
  readonly referenceId: string;
  readonly descriptorSha256: string;
  readonly state: ParticipantSourceState;
  readonly reasonCode: string;
  readonly artifactId: string | null;
  readonly sourceSha256: string | null;
  readonly byteLength: number;
}

export interface ParticipantIngressResult {
  readonly sessionId: string;
  readonly prompt: CapturedSource;
  readonly sources: readonly ParticipantSourceRecord[];
  readonly sourceManifestSha256: string;
  readonly totalBytes: number;
  readonly outputText: string;
}

export type ParticipantStatusSink = (record: ParticipantSourceRecord) => void;

export class ParticipantIngressError extends Error {
  constructor(
    readonly code: string,
    readonly records: readonly ParticipantSourceRecord[] = [],
  ) {
    super(code);
  }
}

/**
 * Captures one stable participant request through the existing authenticated Engineering RPC.
 * The caller supplies only references present on this request; this module cannot enumerate a
 * workspace, parse content, select tools, or acquire model/effect authority.
 */
export async function runParticipantIngress(
  exchange: EngineeringExchange,
  resolver: ParticipantReferenceResolver,
  input: ParticipantRequestInput,
  cancellation: ParticipantCancellation,
  onStatus: ParticipantStatusSink = () => undefined,
  now: () => number = Date.now,
  beforeSubmit: () => Promise<void> = () => Promise.resolve(),
): Promise<ParticipantIngressResult> {
  validateRequest(input);
  const sessionId = `session-${randomUUID()}`;
  await requireEngineeringResult(
    exchange(
      request("create_session", {
        session_id: sessionId,
        title: "AgentMage participant request",
        mode: "ask",
        correlation_id: `correlation-${randomUUID()}`,
        occurred_at_epoch_ms: now(),
      }),
    ),
    "session",
  );
  if (cancellation.isCancellationRequested) {
    await cancelSession(exchange, sessionId, now);
    throw new ParticipantIngressError("vscode.participant.cancelled");
  }

  const prompt = await captureExactText(
    exchange,
    sessionId,
    "paste",
    "Participant prompt",
    "text/plain; charset=utf-8",
    effectivePrompt(input),
    now,
    () => cancellation.isCancellationRequested,
  );
  const records: ParticipantSourceRecord[] = [];
  const artifactIds: string[] = [];
  let totalBytes = prompt.byteLength;

  for (const reference of input.references) {
    let record = sourceRecord(reference, "queued", "participant.source.queued");
    records.push(record);
    onStatus(record);
    if (cancellation.isCancellationRequested) {
      record = sourceRecord(
        reference,
        "cancelled",
        "participant.source.cancelled",
      );
      records[records.length - 1] = record;
      onStatus(record);
      await cancelSession(exchange, sessionId, now);
      throw new ParticipantIngressError(
        "vscode.participant.cancelled",
        records,
      );
    }
    if (reference.kind === "unknown") {
      record = sourceRecord(
        reference,
        "unsupported",
        "participant.source.kind-unsupported",
      );
      records[records.length - 1] = record;
      onStatus(record);
      continue;
    }
    try {
      let captured: CapturedSource;
      if (reference.kind === "text") {
        const text = reference.text ?? "";
        if (text.length === 0) {
          throw new ParticipantIngressError("participant.source.text-empty");
        }
        record = sourceRecord(
          reference,
          "reading",
          "participant.source.reading",
        );
        records[records.length - 1] = record;
        onStatus(record);
        const bytes = Buffer.from(text, "utf8");
        totalBytes = checkedTotal(totalBytes, bytes.length);
        captured = await captureExactText(
          exchange,
          sessionId,
          "paste",
          reference.displayName,
          reference.mediaType,
          text,
          now,
          () => cancellation.isCancellationRequested,
        );
      } else {
        record = sourceRecord(
          reference,
          "reading",
          "participant.source.reading",
        );
        records[records.length - 1] = record;
        onStatus(record);
        const resolved = await resolveBounded(
          resolver,
          reference,
          cancellation,
        );
        if (resolved.revisionBefore !== resolved.revisionAfter) {
          record = sourceRecord(
            reference,
            "stale",
            "participant.source.changed-during-read",
          );
          records[records.length - 1] = record;
          onStatus(record);
          continue;
        }
        totalBytes = checkedTotal(totalBytes, resolved.bytes.length);
        captured = await captureExactBytes(
          exchange,
          sessionId,
          "file",
          resolved.displayName,
          resolved.mediaType,
          resolved.bytes,
          now,
          () => cancellation.isCancellationRequested,
        );
      }
      artifactIds.push(captured.artifactId);
      record = sourceRecord(
        reference,
        "included",
        "participant.source.included",
        captured,
      );
      records[records.length - 1] = record;
      onStatus(record);
    } catch (error) {
      if (
        error instanceof ParticipantIngressError &&
        error.code.includes("budget")
      ) {
        record = sourceRecord(reference, "omitted", error.code);
      } else {
        record = sourceRecord(reference, "failed", errorCode(error));
      }
      records[records.length - 1] = record;
      onStatus(record);
    }
  }

  if (cancellation.isCancellationRequested) {
    await cancelSession(exchange, sessionId, now);
    throw new ParticipantIngressError("vscode.participant.cancelled", records);
  }
  if (records.some((record) => record.state !== "included")) {
    await cancelSession(exchange, sessionId, now);
    throw new ParticipantIngressError(
      "vscode.participant.references-unresolved",
      records,
    );
  }
  const sourceManifestSha256 = manifestDigest(input, prompt, records);
  try {
    await beforeSubmit();
  } catch (error) {
    await cancelSession(exchange, sessionId, now);
    throw error;
  }
  if (cancellation.isCancellationRequested) {
    await cancelSession(exchange, sessionId, now);
    throw new ParticipantIngressError("vscode.participant.cancelled", records);
  }
  const completed = await requireEngineeringResult(
    exchange(
      request("execute_verified_turn", {
        session_id: sessionId,
        prompt_artifact_id: prompt.artifactId,
        context_artifact_ids: artifactIds,
        correlation_id: `correlation-${randomUUID()}`,
        occurred_at_epoch_ms: now(),
      }),
    ),
    "verified_turn_completed",
  );
  const turn = recordValue(completed.turn);
  if (typeof turn.output_text !== "string" || turn.output_text.length === 0) {
    throw new ParticipantIngressError(
      "vscode.participant.response-invalid",
      records,
    );
  }
  return {
    sessionId,
    prompt,
    sources: records,
    sourceManifestSha256,
    totalBytes,
    outputText: turn.output_text,
  };
}

export interface ProviderPartAccounting {
  readonly text: string;
  readonly totalParts: number;
  readonly unsupportedParts: number;
  readonly reasonCode: string | null;
}

/** Accounts for every provider part; unknown values are never silently filtered. */
export function accountProviderParts(
  parts: readonly {
    readonly kind: "text" | "unknown";
    readonly text?: string;
  }[],
): ProviderPartAccounting {
  const text: string[] = [];
  let unsupportedParts = 0;
  for (const part of parts) {
    if (part.kind === "text" && typeof part.text === "string") {
      text.push(part.text);
    } else {
      unsupportedParts += 1;
    }
  }
  return {
    text: text.join(""),
    totalParts: parts.length,
    unsupportedParts,
    reasonCode:
      unsupportedParts === 0 ? null : "vscode.provider.part-unsupported",
  };
}

export function renderParticipantSource(
  record: ParticipantSourceRecord,
): string {
  return boundedStatus(
    `Source ${record.referenceId}: ${record.state} (${record.reasonCode}); ${record.byteLength.toString()} bytes.`,
  );
}

/** Returns a bounded, content-free terminal status suitable for Chat's announced Markdown stream. */
export function renderParticipantError(error: unknown): string {
  const code =
    error instanceof ParticipantIngressError
      ? error.code
      : "vscode.participant.failed";
  const sourceCount =
    error instanceof ParticipantIngressError ? error.records.length : 0;
  return boundedStatus(
    `AgentMage stopped safely (${code}); ${sourceCount.toString()} source record(s) require attention.`,
  );
}

function effectivePrompt(input: ParticipantRequestInput): string {
  return input.command === undefined
    ? input.prompt
    : `/${input.command}\n${input.prompt}`;
}

function validateRequest(input: ParticipantRequestInput): void {
  if (
    !validIdentifier(input.requestId) ||
    input.prompt.includes("\0") ||
    Buffer.byteLength(effectivePrompt(input), "utf8") === 0 ||
    Buffer.byteLength(effectivePrompt(input), "utf8") >
      MAX_PARTICIPANT_TOTAL_BYTES ||
    input.references.length > MAX_PARTICIPANT_REFERENCES ||
    input.references.some((item) => !validIdentifier(item.referenceId)) ||
    new Set(input.references.map((item) => item.referenceId)).size !==
      input.references.length
  ) {
    throw new ParticipantIngressError("vscode.participant.request-invalid");
  }
}

function boundedStatus(value: string): string {
  return value.length <= MAX_PARTICIPANT_STATUS_CHARACTERS
    ? value
    : `${value.slice(0, MAX_PARTICIPANT_STATUS_CHARACTERS - 1)}…`;
}

function checkedTotal(current: number, next: number): number {
  const total = current + next;
  if (
    !Number.isSafeInteger(total) ||
    next <= 0 ||
    total > MAX_PARTICIPANT_TOTAL_BYTES
  ) {
    throw new ParticipantIngressError("participant.source.budget-exhausted");
  }
  return total;
}

function sourceRecord(
  reference: ParticipantReferenceInput,
  state: ParticipantSourceState,
  reasonCode: string,
  captured?: CapturedSource,
): ParticipantSourceRecord {
  return {
    referenceId: reference.referenceId,
    descriptorSha256: sha256(
      Buffer.from(
        canonicalJson({
          kind: reference.kind,
          media_type: reference.mediaType,
          range: reference.range ?? null,
          reference_id: reference.referenceId,
        }),
        "utf8",
      ),
    ),
    state,
    reasonCode,
    artifactId: captured?.artifactId ?? null,
    sourceSha256: captured?.sourceSha256 ?? null,
    byteLength: captured?.byteLength ?? 0,
  };
}

function manifestDigest(
  input: ParticipantRequestInput,
  prompt: CapturedSource,
  records: readonly ParticipantSourceRecord[],
): string {
  return sha256(
    Buffer.from(
      canonicalJson({
        command: input.command ?? null,
        prompt: {
          artifact_id: prompt.artifactId,
          byte_length: prompt.byteLength,
          source_sha256: prompt.sourceSha256,
        },
        request_id: input.requestId,
        sources: records,
      }),
      "utf8",
    ),
  );
}

async function cancelSession(
  exchange: EngineeringExchange,
  sessionId: string,
  now: () => number,
): Promise<void> {
  await exchange(
    request("cancel_session", {
      session_id: sessionId,
      correlation_id: `correlation-${randomUUID()}`,
      occurred_at_epoch_ms: now(),
    }),
  ).catch(() => undefined);
}

async function resolveBounded(
  resolver: ParticipantReferenceResolver,
  reference: ParticipantReferenceInput,
  cancellation: ParticipantCancellation,
): Promise<ResolvedParticipantReference> {
  let timeout: ReturnType<typeof setTimeout> | undefined;
  let subscription: { dispose(): void } | undefined;
  const stopped = new Promise<never>((_resolve, reject) => {
    timeout = setTimeout(
      () =>
        reject(
          new ParticipantIngressError("participant.source.resolve-timeout"),
        ),
      PARTICIPANT_REFERENCE_TIMEOUT_MS,
    );
    subscription = cancellation.onCancellationRequested?.(() =>
      reject(new ParticipantIngressError("participant.source.cancelled")),
    );
  });
  try {
    return await Promise.race([resolver.resolve(reference), stopped]);
  } finally {
    if (timeout !== undefined) clearTimeout(timeout);
    subscription?.dispose();
  }
}

async function requireEngineeringResult(
  pending: ReturnType<EngineeringExchange>,
  expected: string,
): Promise<Record<string, unknown>> {
  const value = await pending;
  if (value.kind === "denied") {
    throw new ParticipantIngressError(value.code);
  }
  if (value.response.result !== expected) {
    throw new ParticipantIngressError("vscode.participant.response-mismatch");
  }
  return value.response;
}

function recordValue(value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new ParticipantIngressError("vscode.participant.response-invalid");
  }
  return value as Record<string, unknown>;
}

function errorCode(error: unknown): string {
  if (error instanceof ParticipantIngressError) return error.code;
  if (error instanceof Error && /^[a-z0-9.-]+$/u.test(error.message))
    return error.message;
  return "participant.source.unavailable";
}

function sha256(bytes: Uint8Array): string {
  return createHash("sha256").update(bytes).digest("hex");
}

function canonicalJson(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (typeof value === "object" && value !== null) {
    return `{${Object.entries(value)
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([key, item]) => `${JSON.stringify(key)}:${canonicalJson(item)}`)
      .join(",")}}`;
  }
  return JSON.stringify(value);
}

function validIdentifier(value: string): boolean {
  return (
    value.length > 0 && value.length <= 128 && /^[A-Za-z0-9._:-]+$/u.test(value)
  );
}
