import { createHmac } from "node:crypto";
import { createConnection, type Socket } from "node:net";

import {
  HOST_PROTOCOL_VERSION,
  type HostBridge,
  type HostReadResponse,
  type HostResponse,
  type ReceiptSummary,
} from "./provider.js";
import {
  parseModelPickerSnapshot,
  parseModelSelectionRevalidation,
} from "./model_discovery.js";
import {
  parseHandoffReview,
  parseLocalHandoffReceipt,
  parseRenderedHandoff,
} from "./handoff.js";
import { parseRuntimeHostResponse } from "./runtime_transport.js";
import { parseEngineeringHostResponse } from "./verified_chat_protocol.js";

const LINUX_IPC_PROTOCOL_VERSION = 1;
const AUTHENTICATION_DOMAIN = Buffer.from(
  "agentmage-linux-ipc-auth-v1\0",
  "utf8",
);
const HANDSHAKE_BYTES = 68;
const MAX_REQUEST_BYTES = 64 * 1024;
const MAX_RESPONSE_BYTES = 4 * 1024 * 1024;

/** One-use launch material delivered directly by the verified package bootstrap. */
export interface LinuxHostLaunchCredentials {
  readonly endpoint: string;
  readonly challenge: Uint8Array;
  readonly launchSecret: Uint8Array;
  readonly peer: {
    readonly uid: number;
    readonly pid: number;
    readonly startTimeTicks: bigint;
    readonly executableSha256: Uint8Array;
  };
}

/** Authenticated Unix-socket client with serialized, length-bounded exchanges. */
export class AuthenticatedLinuxHostBridge implements HostBridge {
  private socket: Socket | undefined;
  private reader: SocketReader | undefined;
  private connectPromise: Promise<void> | undefined;
  private queue: Promise<void> = Promise.resolve();
  private readonly endpoint: string;
  private readonly challenge: Uint8Array;
  private readonly peer: LinuxHostLaunchCredentials["peer"];
  private readonly secret: Uint8Array;

  constructor(credentials: LinuxHostLaunchCredentials) {
    try {
      validateCredentials(credentials);
    } catch (error) {
      credentials.launchSecret.fill(0);
      throw error;
    }
    this.endpoint = credentials.endpoint;
    this.challenge = Uint8Array.from(credentials.challenge);
    this.peer = {
      uid: credentials.peer.uid,
      pid: credentials.peer.pid,
      startTimeTicks: credentials.peer.startTimeTicks,
      executableSha256: Uint8Array.from(credentials.peer.executableSha256),
    };
    this.secret = Uint8Array.from(credentials.launchSecret);
    credentials.launchSecret.fill(0);
  }

  engineering(
    request: Parameters<HostBridge["engineering"]>[0],
  ): Promise<HostResponse> {
    return this.safeExchange(request);
  }

  previewHandoff(
    request: Parameters<HostBridge["previewHandoff"]>[0],
  ): Promise<HostResponse> {
    return this.safeExchange(request);
  }

  renderHandoff(
    request: Parameters<HostBridge["renderHandoff"]>[0],
  ): Promise<HostResponse> {
    return this.safeExchange(request);
  }

  cancelHandoff(
    request: Parameters<HostBridge["cancelHandoff"]>[0],
  ): Promise<HostResponse> {
    return this.safeExchange(request);
  }

  denyHandoffAction(
    request: Parameters<HostBridge["denyHandoffAction"]>[0],
  ): Promise<HostResponse> {
    return this.safeExchange(request);
  }

  discoverModels(
    request: Parameters<HostBridge["discoverModels"]>[0],
  ): Promise<HostResponse> {
    return this.safeExchange(request);
  }

  revalidateModel(
    request: Parameters<HostBridge["revalidateModel"]>[0],
  ): Promise<HostResponse> {
    return this.safeExchange(request);
  }

  doctor(request: Parameters<HostBridge["doctor"]>[0]): Promise<HostResponse> {
    return this.safeExchange(request);
  }

  previewDiagnosticExport(
    request: Parameters<HostBridge["previewDiagnosticExport"]>[0],
  ): Promise<HostResponse> {
    return this.safeExchange(request);
  }

  approveDiagnosticExport(
    request: Parameters<HostBridge["approveDiagnosticExport"]>[0],
  ): Promise<HostResponse> {
    return this.safeExchange(request);
  }

  cancelDiagnosticExport(
    request: Parameters<HostBridge["cancelDiagnosticExport"]>[0],
  ): Promise<HostResponse> {
    return this.safeExchange(request);
  }

  previewRead(
    request: Parameters<HostBridge["previewRead"]>[0],
  ): Promise<HostReadResponse> {
    return this.safeReadExchange(request);
  }

  approveRead(
    request: Parameters<HostBridge["approveRead"]>[0],
  ): Promise<HostReadResponse> {
    return this.safeReadExchange(request);
  }

  cancelRead(
    request: Parameters<HostBridge["cancelRead"]>[0],
  ): Promise<HostReadResponse> {
    return this.safeReadExchange(request);
  }

  prepareRuntime(
    request: Parameters<HostBridge["prepareRuntime"]>[0],
  ): Promise<HostResponse> {
    return this.safeExchange(request);
  }

  startRuntime(
    request: Parameters<HostBridge["startRuntime"]>[0],
  ): Promise<HostResponse> {
    return this.safeExchange(request);
  }

  advanceRuntime(
    request: Parameters<HostBridge["advanceRuntime"]>[0],
  ): Promise<HostResponse> {
    return this.safeExchange(request);
  }

  cancelRuntime(
    request: Parameters<HostBridge["cancelRuntime"]>[0],
  ): Promise<HostResponse> {
    return this.safeExchange(request);
  }

  releaseRuntime(
    request: Parameters<HostBridge["releaseRuntime"]>[0],
  ): Promise<HostResponse> {
    return this.safeExchange(request);
  }

  /** Closes the local channel and erases retained one-use secret bytes. */
  dispose(): void {
    this.socket?.destroy();
    this.socket = undefined;
    this.reader = undefined;
    this.challenge.fill(0);
    this.secret.fill(0);
  }

  private safeExchange(
    request: object & { readonly request_id: string },
  ): Promise<HostResponse> {
    return this.serialize(() => this.exchange(request)).catch(() => ({
      kind: "denied",
      schema_version: HOST_PROTOCOL_VERSION,
      request_id: request.request_id,
      code: "host.connection.failed",
    }));
  }

  private safeReadExchange(
    request: object & { readonly request_id: string },
  ): Promise<HostReadResponse> {
    return this.safeExchange(request).then((response) =>
      response.kind === "doctor_completed" ||
      response.kind === "models_discovered" ||
      response.kind === "model_revalidated" ||
      response.kind === "diagnostic_export_preview" ||
      response.kind === "diagnostic_export_completed" ||
      response.kind === "handoff_preview" ||
      response.kind === "handoff_rendered" ||
      response.kind === "handoff_receipt" ||
      response.kind === "engineering" ||
      response.kind === "runtime_prepared" ||
      response.kind === "runtime_step"
        ? {
            kind: "denied",
            schema_version: HOST_PROTOCOL_VERSION,
            request_id: request.request_id,
            code: "host.protocol.response_mismatch",
          }
        : response,
    );
  }

  private serialize(
    operation: () => Promise<HostResponse>,
  ): Promise<HostResponse> {
    const result = this.queue.then(operation, operation);
    this.queue = result.then(
      () => undefined,
      () => undefined,
    );
    return result;
  }

  private async exchange(request: object): Promise<HostResponse> {
    await this.connect();
    const socket = this.socket;
    const reader = this.reader;
    if (socket === undefined || reader === undefined) {
      throw new HostBridgeFailure();
    }
    const body = Buffer.from(JSON.stringify(request), "utf8");
    if (body.length === 0 || body.length > MAX_REQUEST_BYTES) {
      throw new HostBridgeFailure();
    }
    const header = Buffer.alloc(4);
    header.writeUInt32BE(body.length);
    await write(socket, Buffer.concat([header, body]));
    const responseLength = (await reader.read(4)).readUInt32BE(0);
    if (responseLength === 0 || responseLength > MAX_RESPONSE_BYTES) {
      throw new HostBridgeFailure();
    }
    const response = JSON.parse(
      (await reader.read(responseLength)).toString("utf8"),
    ) as unknown;
    return parseResponse(response);
  }

  private connect(): Promise<void> {
    this.connectPromise ??= new Promise<void>((resolve, reject) => {
      const socket = createConnection({ path: this.endpoint });
      const rejectBounded = (): void => reject(new HostBridgeFailure());
      socket.once("error", rejectBounded);
      socket.once("connect", () => {
        socket.off("error", rejectBounded);
        socket.on("error", () => socket.destroy());
        this.socket = socket;
        this.reader = new SocketReader(socket);
        write(
          socket,
          authenticationFrame(this.challenge, this.peer, this.secret),
        ).then(resolve, rejectBounded);
      });
    });
    return this.connectPromise;
  }
}

class HostBridgeFailure extends Error {
  constructor() {
    super("host.bridge.failed");
  }
}

class SocketReader {
  private retained = Buffer.alloc(0);
  private readonly waiting: Array<{
    readonly bytes: number;
    readonly resolve: (value: Buffer) => void;
    readonly reject: () => void;
  }> = [];
  private closed = false;

  constructor(socket: Socket) {
    socket.on("data", (chunk: Buffer) => {
      if (this.retained.length + chunk.length > MAX_RESPONSE_BYTES + 4) {
        socket.destroy();
        return;
      }
      this.retained = Buffer.concat([this.retained, chunk]);
      this.drain();
    });
    const close = (): void => {
      this.closed = true;
      for (const pending of this.waiting.splice(0)) {
        pending.reject();
      }
    };
    socket.once("close", close);
    socket.once("end", close);
  }

  read(bytes: number): Promise<Buffer> {
    if (bytes <= 0 || bytes > MAX_RESPONSE_BYTES || this.closed) {
      return Promise.reject(new HostBridgeFailure());
    }
    return new Promise<Buffer>((resolve, reject) => {
      this.waiting.push({
        bytes,
        resolve,
        reject: () => reject(new HostBridgeFailure()),
      });
      this.drain();
    });
  }

  private drain(): void {
    while (this.waiting.length > 0) {
      const pending = this.waiting[0];
      if (pending === undefined || this.retained.length < pending.bytes) {
        return;
      }
      this.waiting.shift();
      const value = this.retained.subarray(0, pending.bytes);
      this.retained = this.retained.subarray(pending.bytes);
      pending.resolve(value);
    }
  }
}

function authenticationFrame(
  challenge: Uint8Array,
  peer: LinuxHostLaunchCredentials["peer"],
  launchSecret: Uint8Array,
): Buffer {
  const identity = Buffer.alloc(4 + 4 + 8 + 32);
  identity.writeUInt32BE(peer.uid, 0);
  identity.writeInt32BE(peer.pid, 4);
  identity.writeBigUInt64BE(peer.startTimeTicks, 8);
  Buffer.from(peer.executableSha256).copy(identity, 16);

  const version = Buffer.alloc(4);
  version.writeUInt32BE(LINUX_IPC_PROTOCOL_VERSION);
  const key = Buffer.from(launchSecret);
  const response = createHmac("sha256", key)
    .update(AUTHENTICATION_DOMAIN)
    .update(version)
    .update(challenge)
    .update(identity)
    .digest();
  key.fill(0);
  const frame = Buffer.concat([version, Buffer.from(challenge), response]);
  if (frame.length !== HANDSHAKE_BYTES) {
    throw new HostBridgeFailure();
  }
  return frame;
}

function validateCredentials(credentials: LinuxHostLaunchCredentials): void {
  if (
    credentials.endpoint.length === 0 ||
    credentials.endpoint.length > 4_096 ||
    !credentials.endpoint.startsWith("/") ||
    credentials.endpoint.includes("\0") ||
    credentials.challenge.length !== 32 ||
    credentials.launchSecret.length !== 32 ||
    credentials.peer.executableSha256.length !== 32 ||
    !Number.isInteger(credentials.peer.uid) ||
    credentials.peer.uid < 0 ||
    credentials.peer.uid > 0xffff_ffff ||
    !Number.isInteger(credentials.peer.pid) ||
    credentials.peer.pid <= 0 ||
    credentials.peer.pid > 0x7fff_ffff ||
    credentials.peer.startTimeTicks <= 0n ||
    credentials.peer.startTimeTicks > 0xffff_ffff_ffff_ffffn
  ) {
    throw new HostBridgeFailure();
  }
}

function parseResponse(candidate: unknown): HostResponse {
  if (
    !isRecord(candidate) ||
    candidate.schema_version !== HOST_PROTOCOL_VERSION
  ) {
    throw new HostBridgeFailure();
  }
  switch (candidate.kind) {
    case "engineering":
      try {
        return parseEngineeringHostResponse(candidate);
      } catch {
        throw new HostBridgeFailure();
      }
    case "runtime_prepared":
    case "runtime_step":
      try {
        return parseRuntimeHostResponse(candidate);
      } catch {
        throw new HostBridgeFailure();
      }
    case "handoff_preview":
      requireKeys(candidate, [
        "kind",
        "request_id",
        "review",
        "schema_version",
      ]);
      if (!validIdentifier(candidate.request_id)) {
        throw new HostBridgeFailure();
      }
      try {
        return {
          kind: "handoff_preview",
          schema_version: HOST_PROTOCOL_VERSION,
          request_id: candidate.request_id,
          review: parseHandoffReview(candidate.review),
        };
      } catch {
        throw new HostBridgeFailure();
      }
    case "handoff_rendered":
      requireKeys(candidate, [
        "kind",
        "rendered",
        "request_id",
        "schema_version",
      ]);
      if (!validIdentifier(candidate.request_id)) {
        throw new HostBridgeFailure();
      }
      try {
        return {
          kind: "handoff_rendered",
          schema_version: HOST_PROTOCOL_VERSION,
          request_id: candidate.request_id,
          rendered: parseRenderedHandoff(candidate.rendered),
        };
      } catch {
        throw new HostBridgeFailure();
      }
    case "handoff_receipt":
      requireKeys(candidate, [
        "kind",
        "receipt",
        "request_id",
        "schema_version",
      ]);
      if (!validIdentifier(candidate.request_id)) {
        throw new HostBridgeFailure();
      }
      try {
        return {
          kind: "handoff_receipt",
          schema_version: HOST_PROTOCOL_VERSION,
          request_id: candidate.request_id,
          receipt: parseLocalHandoffReceipt(candidate.receipt),
        };
      } catch {
        throw new HostBridgeFailure();
      }
    case "models_discovered":
      requireKeys(candidate, [
        "kind",
        "request_id",
        "schema_version",
        "snapshot",
      ]);
      if (!validIdentifier(candidate.request_id)) {
        throw new HostBridgeFailure();
      }
      try {
        return {
          kind: "models_discovered",
          schema_version: HOST_PROTOCOL_VERSION,
          request_id: candidate.request_id,
          snapshot: parseModelPickerSnapshot(candidate.snapshot),
        };
      } catch {
        throw new HostBridgeFailure();
      }
    case "model_revalidated":
      requireKeys(candidate, [
        "kind",
        "request_id",
        "revalidation",
        "schema_version",
      ]);
      if (!validIdentifier(candidate.request_id)) {
        throw new HostBridgeFailure();
      }
      try {
        return {
          kind: "model_revalidated",
          schema_version: HOST_PROTOCOL_VERSION,
          request_id: candidate.request_id,
          revalidation: parseModelSelectionRevalidation(candidate.revalidation),
        };
      } catch {
        throw new HostBridgeFailure();
      }
    case "doctor_completed":
      requireKeys(candidate, [
        "kind",
        "report",
        "request_id",
        "schema_version",
      ]);
      if (
        !validIdentifier(candidate.request_id) ||
        !validDoctorReport(candidate.report)
      ) {
        throw new HostBridgeFailure();
      }
      return candidate as unknown as HostResponse;
    case "diagnostic_export_preview":
      requireKeys(candidate, [
        "confirmation_sha256",
        "destination_sha256",
        "expires_at_epoch_ms",
        "included_fields",
        "kind",
        "payload_bytes",
        "payload_sha256",
        "preview_id",
        "redactions",
        "request_id",
        "retention",
        "schema_version",
        "sensitivity",
      ]);
      if (
        !validIdentifier(candidate.request_id) ||
        !validIdentifier(candidate.preview_id) ||
        !validSha256(candidate.destination_sha256) ||
        !validSha256(candidate.payload_sha256) ||
        !Number.isSafeInteger(candidate.payload_bytes) ||
        !Array.isArray(candidate.included_fields) ||
        !candidate.included_fields.every(validCode) ||
        !Array.isArray(candidate.redactions) ||
        !candidate.redactions.every(validCode) ||
        !validCode(candidate.sensitivity) ||
        !validCode(candidate.retention) ||
        !Number.isSafeInteger(candidate.expires_at_epoch_ms) ||
        !validSha256(candidate.confirmation_sha256)
      ) {
        throw new HostBridgeFailure();
      }
      return candidate as unknown as HostResponse;
    case "diagnostic_export_completed":
      requireKeys(candidate, [
        "destination_sha256",
        "kind",
        "outcome",
        "payload_bytes",
        "payload_sha256",
        "request_id",
        "schema_version",
      ]);
      if (
        !validIdentifier(candidate.request_id) ||
        !validSha256(candidate.destination_sha256) ||
        !validSha256(candidate.payload_sha256) ||
        !Number.isSafeInteger(candidate.payload_bytes) ||
        candidate.outcome !== "succeeded"
      ) {
        throw new HostBridgeFailure();
      }
      return candidate as unknown as HostResponse;
    case "read_preview":
      requireKeys(candidate, [
        "byte_len",
        "components",
        "confirmation_sha256",
        "content_sha256",
        "expires_at_epoch_ms",
        "kind",
        "preview_id",
        "request_id",
        "schema_version",
      ]);
      if (
        !validIdentifier(candidate.request_id) ||
        !validIdentifier(candidate.preview_id) ||
        !Array.isArray(candidate.components) ||
        !candidate.components.every((value) => typeof value === "string") ||
        !Number.isSafeInteger(candidate.byte_len) ||
        !Number.isSafeInteger(candidate.expires_at_epoch_ms) ||
        !validSha256(candidate.content_sha256) ||
        !validSha256(candidate.confirmation_sha256)
      ) {
        throw new HostBridgeFailure();
      }
      return candidate as unknown as HostReadResponse;
    case "read_completed":
      requireKeys(candidate, [
        "content",
        "file_uri",
        "kind",
        "receipt",
        "request_id",
        "schema_version",
      ]);
      if (
        !validIdentifier(candidate.request_id) ||
        typeof candidate.content !== "string" ||
        typeof candidate.file_uri !== "string" ||
        !validReceipt(candidate.receipt)
      ) {
        throw new HostBridgeFailure();
      }
      return candidate as unknown as HostReadResponse;
    case "cancelled":
      requireKeys(candidate, ["kind", "request_id", "schema_version"]);
      if (!validIdentifier(candidate.request_id)) {
        throw new HostBridgeFailure();
      }
      return candidate as unknown as HostReadResponse;
    case "denied": {
      const keys = ["code", "kind", "request_id", "schema_version"];
      if (candidate.receipt !== undefined) {
        keys.push("receipt");
      }
      requireKeys(candidate, keys);
      if (
        !validIdentifier(candidate.request_id) ||
        typeof candidate.code !== "string" ||
        (candidate.receipt !== undefined && !validReceipt(candidate.receipt))
      ) {
        throw new HostBridgeFailure();
      }
      return candidate as unknown as HostReadResponse;
    }
    default:
      throw new HostBridgeFailure();
  }
}

const DIAGNOSTIC_COMPONENTS = [
  "package",
  "platform",
  "model",
  "runtime",
  "hardware_fit",
  "offline_boundary",
  "sandbox_helper",
  "workspace_grant",
  "capabilities",
  "repository_map",
  "encrypted_store",
  "receipt_chain",
  "recovery",
] as const;

const DIAGNOSTIC_STATES = [
  "healthy",
  "degraded",
  "blocked",
  "unavailable",
  "quarantined",
  "unsupported",
] as const;

function validDoctorReport(candidate: unknown): boolean {
  if (!isRecord(candidate)) {
    return false;
  }
  try {
    requireKeys(candidate, [
      "items",
      "overall_state",
      "report_kind",
      "report_sha256",
      "schema_version",
    ]);
  } catch {
    return false;
  }
  if (
    candidate.schema_version !== 2 ||
    candidate.report_kind !== "agentmage.local-doctor.v1" ||
    !DIAGNOSTIC_STATES.includes(candidate.overall_state as never) ||
    !validSha256(candidate.report_sha256) ||
    !Array.isArray(candidate.items) ||
    candidate.items.length !== DIAGNOSTIC_COMPONENTS.length
  ) {
    return false;
  }
  return candidate.items.every((item, index) => {
    if (!isRecord(item)) {
      return false;
    }
    try {
      requireKeys(item, [
        "component",
        "identity_sha256",
        "reason_code",
        "remediation_code",
        "state",
      ]);
    } catch {
      return false;
    }
    return (
      item.component === DIAGNOSTIC_COMPONENTS[index] &&
      DIAGNOSTIC_STATES.includes(item.state as never) &&
      validCode(item.reason_code) &&
      validCode(item.remediation_code) &&
      (item.identity_sha256 === null || validSha256(item.identity_sha256))
    );
  });
}

function validReceipt(candidate: unknown): candidate is ReceiptSummary {
  if (!isRecord(candidate)) {
    return false;
  }
  try {
    requireKeys(candidate, [
      "outcome",
      "receipt_id",
      "receipt_sha256",
      "sequence",
    ]);
  } catch {
    return false;
  }
  return (
    validIdentifier(candidate.receipt_id) &&
    Number.isSafeInteger(candidate.sequence) &&
    validSha256(candidate.receipt_sha256) &&
    typeof candidate.outcome === "string"
  );
}

function requireKeys(
  candidate: Record<string, unknown>,
  expected: readonly string[],
): void {
  const actual = Object.keys(candidate).sort();
  const ordered = [...expected].sort();
  if (
    actual.length !== ordered.length ||
    actual.some((value, index) => value !== ordered[index])
  ) {
    throw new HostBridgeFailure();
  }
}

function isRecord(candidate: unknown): candidate is Record<string, unknown> {
  return (
    typeof candidate === "object" &&
    candidate !== null &&
    !Array.isArray(candidate)
  );
}

function validIdentifier(candidate: unknown): candidate is string {
  return (
    typeof candidate === "string" && /^[A-Za-z0-9._:-]{1,128}$/.test(candidate)
  );
}

function validSha256(candidate: unknown): candidate is string {
  return typeof candidate === "string" && /^[0-9a-f]{64}$/.test(candidate);
}

function validCode(candidate: unknown): candidate is string {
  return (
    typeof candidate === "string" && /^[a-z0-9._-]{1,128}$/.test(candidate)
  );
}

function write(socket: Socket, bytes: Buffer): Promise<void> {
  return new Promise<void>((resolve, reject) => {
    socket.write(bytes, (error) =>
      error === null || error === undefined
        ? resolve()
        : reject(new HostBridgeFailure()),
    );
  });
}
