import { createHash } from "node:crypto";

export const LOCAL_HANDOFF_NOTICE =
  "This packet remains local. AgentMage has not contacted Codex or any external service. External handling begins only if you manually transfer selected content.";

export type HandoffProhibitedAction =
  | "codex_invocation"
  | "tab_activation"
  | "chat_population"
  | "clipboard_write"
  | "uri_launch"
  | "local_runtime_delivery"
  | "raw_runtime_delivery"
  | "network_call"
  | "automatic_submission";

export interface HandoffPacketManifest {
  readonly schema_version: 2;
  readonly handoff_id: string;
  readonly draft_sha256: string;
  readonly entry_sha256: readonly string[];
  readonly packet_sha256: string;
  readonly packet_bytes: number;
  readonly destination: "manual_codex_interface";
  readonly acknowledgment_required: boolean;
  readonly delivered: false;
  readonly manifest_sha256: string;
}

export interface HandoffReview {
  readonly schema_version: 2;
  readonly preview_id: string;
  readonly packet_markdown: string;
  readonly manifest: HandoffPacketManifest;
  readonly local_only_notice: string;
  readonly expires_at_ms: number;
  readonly confirmation_sha256: string;
}

export interface LocalHandoffReceipt {
  readonly schema_version: 2;
  readonly attempt_id: string;
  readonly handoff_id: string | null;
  readonly outcome: "rendered" | "cancelled" | "denied";
  readonly result_code: string;
  readonly prohibited_action: HandoffProhibitedAction | null;
  readonly packet_sha256: string | null;
  readonly external_delivery_attempted: false;
  readonly receipt_sha256: string;
}

export interface RenderedHandoff {
  readonly schema_version: 2;
  readonly packet_markdown: string;
  readonly manifest: HandoffPacketManifest;
  readonly receipt: LocalHandoffReceipt;
}

const SHA256 = /^[0-9a-f]{64}$/;
const IDENTIFIER = /^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/;
const CODE = /^[a-z0-9][a-z0-9._-]{0,127}$/;
const ACTIONS: readonly HandoffProhibitedAction[] = [
  "codex_invocation",
  "tab_activation",
  "chat_population",
  "clipboard_write",
  "uri_launch",
  "local_runtime_delivery",
  "raw_runtime_delivery",
  "network_call",
  "automatic_submission",
];

export function parseHandoffReview(candidate: unknown): HandoffReview {
  const value = record(candidate);
  requireKeys(value, [
    "confirmation_sha256",
    "expires_at_ms",
    "local_only_notice",
    "manifest",
    "packet_markdown",
    "preview_id",
    "schema_version",
  ]);
  const manifest = parseManifest(value.manifest);
  if (
    value.schema_version !== 2 ||
    !identifier(value.preview_id) ||
    typeof value.packet_markdown !== "string" ||
    Buffer.byteLength(value.packet_markdown, "utf8") === 0 ||
    Buffer.byteLength(value.packet_markdown, "utf8") > 128 * 1024 ||
    value.local_only_notice !== LOCAL_HANDOFF_NOTICE ||
    !positiveInteger(value.expires_at_ms) ||
    !sha(value.confirmation_sha256) ||
    manifest.packet_bytes !==
      Buffer.byteLength(value.packet_markdown, "utf8") ||
    manifest.packet_sha256 !== digestText(value.packet_markdown)
  ) {
    throw new Error("handoff.review.invalid");
  }
  const review: HandoffReview = {
    schema_version: 2,
    preview_id: value.preview_id,
    packet_markdown: value.packet_markdown,
    manifest,
    local_only_notice: LOCAL_HANDOFF_NOTICE,
    expires_at_ms: value.expires_at_ms,
    confirmation_sha256: value.confirmation_sha256,
  };
  const { confirmation_sha256: expected, ...unsigned } = review;
  if (digest(unsigned) !== expected) {
    throw new Error("handoff.review.invalid");
  }
  return review;
}

export function parseRenderedHandoff(candidate: unknown): RenderedHandoff {
  const value = record(candidate);
  requireKeys(value, [
    "manifest",
    "packet_markdown",
    "receipt",
    "schema_version",
  ]);
  const manifest = parseManifest(value.manifest);
  const receipt = parseLocalHandoffReceipt(value.receipt);
  if (
    value.schema_version !== 2 ||
    typeof value.packet_markdown !== "string" ||
    Buffer.byteLength(value.packet_markdown, "utf8") !==
      manifest.packet_bytes ||
    digestText(value.packet_markdown) !== manifest.packet_sha256 ||
    receipt.outcome !== "rendered" ||
    receipt.packet_sha256 !== manifest.packet_sha256 ||
    receipt.handoff_id !== manifest.handoff_id ||
    receipt.prohibited_action !== null
  ) {
    throw new Error("handoff.rendered.invalid");
  }
  return {
    schema_version: 2,
    packet_markdown: value.packet_markdown,
    manifest,
    receipt,
  };
}

export function parseLocalHandoffReceipt(
  candidate: unknown,
): LocalHandoffReceipt {
  const value = record(candidate);
  requireKeys(value, [
    "attempt_id",
    "external_delivery_attempted",
    "handoff_id",
    "outcome",
    "packet_sha256",
    "prohibited_action",
    "receipt_sha256",
    "result_code",
    "schema_version",
  ]);
  if (
    value.schema_version !== 2 ||
    !identifier(value.attempt_id) ||
    !(value.handoff_id === null || identifier(value.handoff_id)) ||
    !oneOf(value.outcome, ["rendered", "cancelled", "denied"] as const) ||
    !code(value.result_code) ||
    !(
      value.prohibited_action === null ||
      oneOf(value.prohibited_action, ACTIONS)
    ) ||
    !(value.packet_sha256 === null || sha(value.packet_sha256)) ||
    value.external_delivery_attempted !== false ||
    !sha(value.receipt_sha256)
  ) {
    throw new Error("handoff.receipt.invalid");
  }
  const receipt = value as unknown as LocalHandoffReceipt;
  const { receipt_sha256: expected, ...unsigned } = receipt;
  if (digest(unsigned) !== expected) {
    throw new Error("handoff.receipt.invalid");
  }
  return receipt;
}

function parseManifest(candidate: unknown): HandoffPacketManifest {
  const value = record(candidate);
  requireKeys(value, [
    "acknowledgment_required",
    "delivered",
    "destination",
    "draft_sha256",
    "entry_sha256",
    "handoff_id",
    "manifest_sha256",
    "packet_bytes",
    "packet_sha256",
    "schema_version",
  ]);
  if (
    value.schema_version !== 2 ||
    !identifier(value.handoff_id) ||
    !sha(value.draft_sha256) ||
    !Array.isArray(value.entry_sha256) ||
    value.entry_sha256.length === 0 ||
    value.entry_sha256.length > 32 ||
    !value.entry_sha256.every(sha) ||
    new Set(value.entry_sha256).size !== value.entry_sha256.length ||
    !sha(value.packet_sha256) ||
    !positiveInteger(value.packet_bytes) ||
    value.packet_bytes > 128 * 1024 ||
    value.destination !== "manual_codex_interface" ||
    typeof value.acknowledgment_required !== "boolean" ||
    value.delivered !== false ||
    !sha(value.manifest_sha256)
  ) {
    throw new Error("handoff.manifest.invalid");
  }
  const manifest = value as unknown as HandoffPacketManifest;
  const { manifest_sha256: expected, ...unsigned } = manifest;
  if (digest(unsigned) !== expected) {
    throw new Error("handoff.manifest.invalid");
  }
  return manifest;
}

function digest(value: unknown): string {
  return createHash("sha256")
    .update(JSON.stringify(value), "utf8")
    .digest("hex");
}

function digestText(value: string): string {
  return createHash("sha256").update(value, "utf8").digest("hex");
}

function record(value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error("handoff.shape.invalid");
  }
  return value as Record<string, unknown>;
}

function requireKeys(
  value: Record<string, unknown>,
  expected: readonly string[],
): void {
  const actual = Object.keys(value).sort();
  const ordered = [...expected].sort();
  if (
    actual.length !== ordered.length ||
    actual.some((item, index) => item !== ordered[index])
  ) {
    throw new Error("handoff.shape.invalid");
  }
}

function oneOf<const T extends string>(
  value: unknown,
  choices: readonly T[],
): value is T {
  return typeof value === "string" && choices.includes(value as T);
}

function identifier(value: unknown): value is string {
  return typeof value === "string" && IDENTIFIER.test(value);
}

function sha(value: unknown): value is string {
  return typeof value === "string" && SHA256.test(value);
}

function code(value: unknown): value is string {
  return typeof value === "string" && CODE.test(value);
}

function positiveInteger(value: unknown): value is number {
  return Number.isSafeInteger(value) && (value as number) > 0;
}
