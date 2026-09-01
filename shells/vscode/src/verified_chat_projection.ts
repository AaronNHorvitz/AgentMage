import { createHash } from "node:crypto";

const ZERO_SHA256 = "0".repeat(64);
const MAX_REPLAY_EVENTS = 2048;
const DEFAULT_MAX_PENDING_MESSAGES = 128;
const DEFAULT_MAX_PENDING_BYTES = 1024 * 1024;

export interface VerifiedChatEventCard {
  readonly type: "runtimeEvent";
  readonly category:
    | "session"
    | "artifact"
    | "context"
    | "model"
    | "tool"
    | "verification"
    | "approval"
    | "lifecycle"
    | "team"
    | "terminal";
  readonly sequence: number;
  readonly eventId: string;
  readonly label: string;
  readonly state?: string;
}

export type VerifiedChatSessionMode = "ask" | "plan" | "agent" | "team";

export interface VerifiedChatRestoredSession {
  readonly sessionId: string;
  readonly mode: VerifiedChatSessionMode;
  readonly artifacts: readonly VerifiedChatRestoredArtifact[];
  readonly terminal: string | null;
}

export interface VerifiedChatRestoredArtifact {
  readonly artifactId: string;
  readonly sourceKind: string;
  readonly displayName: string;
  readonly mediaType: string;
  readonly byteLength: number;
  readonly sourceSha256: string;
  readonly disposition: unknown;
  readonly warningCodes: readonly unknown[];
}

export interface VerifiedChatApprovedPlanProjection {
  readonly sourceSessionId: string;
  readonly artifactId: string;
  readonly planSha256: string;
  readonly approvalId: string;
  readonly approvalSha256: string;
}

export class VerifiedChatProjectionError extends Error {
  constructor(code: string) {
    super(code);
  }
}

/** Parses one closed Rust-owned session snapshot for reload reconstruction. */
export function parseVerifiedChatSessionSnapshot(
  value: unknown,
): VerifiedChatRestoredSession {
  const snapshot = exactRecord(value, [
    "active_task_id",
    "artifacts",
    "endpoint_profile_id",
    "last_event_sequence",
    "mode",
    "model_profile_id",
    "schema_version",
    "session_id",
    "snapshot_sha256",
    "terminal",
    "title",
  ]);
  if (
    snapshot.schema_version !== 1 ||
    !validIdentifier(snapshot.session_id) ||
    typeof snapshot.title !== "string" ||
    snapshot.title.length === 0 ||
    snapshot.title.length > 256 ||
    !new Set(["ask", "plan", "agent", "team"]).has(snapshot.mode as string) ||
    (snapshot.model_profile_id !== null &&
      !validIdentifier(snapshot.model_profile_id)) ||
    (snapshot.endpoint_profile_id !== null &&
      !validIdentifier(snapshot.endpoint_profile_id)) ||
    (snapshot.active_task_id !== null &&
      !validIdentifier(snapshot.active_task_id)) ||
    (snapshot.last_event_sequence !== null &&
      (!Number.isSafeInteger(snapshot.last_event_sequence) ||
        (snapshot.last_event_sequence as number) < 0)) ||
    !validSha256(snapshot.snapshot_sha256) ||
    !Array.isArray(snapshot.artifacts) ||
    (snapshot.terminal !== null && !validTerminal(snapshot.terminal))
  ) {
    throw new VerifiedChatProjectionError("verified-chat.session.invalid");
  }
  verifySealedRecord(snapshot, "snapshot_sha256");
  const artifacts = snapshot.artifacts.map((value) => {
    const artifact = exactRecord(value, [
      "artifact_id",
      "byte_length",
      "completed_at_epoch_ms",
      "display_name",
      "disposition",
      "line_count",
      "media_type",
      "payload_deduplicated",
      "receipt_sha256",
      "schema_version",
      "session_id",
      "source_kind",
      "source_sha256",
      "upload_id",
      "warning_codes",
    ]);
    if (
      artifact.schema_version !== 1 ||
      !validIdentifier(artifact.artifact_id) ||
      artifact.session_id !== snapshot.session_id ||
      !validIdentifier(artifact.upload_id) ||
      typeof artifact.display_name !== "string" ||
      artifact.display_name.length === 0 ||
      artifact.display_name.length > 256 ||
      typeof artifact.media_type !== "string" ||
      artifact.media_type.length === 0 ||
      artifact.media_type.length > 128 ||
      !nonnegativeInteger(artifact.byte_length) ||
      (artifact.line_count !== null &&
        !nonnegativeInteger(artifact.line_count)) ||
      typeof artifact.payload_deduplicated !== "boolean" ||
      !positiveInteger(artifact.completed_at_epoch_ms) ||
      !new Set(["paste", "file", "reference", "generated"]).has(
        artifact.source_kind as string,
      ) ||
      !new Set([
        "captured_exactly",
        "captured_with_warnings",
        "duplicate_payload",
      ]).has(artifact.disposition as string) ||
      !validSha256(artifact.source_sha256) ||
      !validSha256(artifact.receipt_sha256) ||
      !Array.isArray(artifact.warning_codes) ||
      artifact.warning_codes.some(
        (warning) => typeof warning !== "string" || !validIdentifier(warning),
      )
    ) {
      throw new VerifiedChatProjectionError(
        "verified-chat.session.artifact-invalid",
      );
    }
    return {
      artifactId: artifact.artifact_id,
      sourceKind: artifact.source_kind as string,
      displayName: artifact.display_name,
      mediaType: artifact.media_type,
      byteLength: artifact.byte_length,
      sourceSha256: artifact.source_sha256,
      disposition: artifact.disposition,
      warningCodes: artifact.warning_codes,
    };
  });
  return {
    sessionId: snapshot.session_id,
    mode: snapshot.mode as VerifiedChatSessionMode,
    artifacts,
    terminal: snapshot.terminal,
  };
}

/**
 * Verifies and projects one Rust-owned durable Engineering event stream.
 *
 * The projection consumes content-free identifiers and states only. It cannot acknowledge an
 * approval, select a policy, dispatch a tool, retry work, or establish completion.
 */
export class VerifiedChatEventProjection {
  private nextSequence = 0;
  private previousEventSha256 = ZERO_SHA256;
  private terminalState: string | undefined;
  private approvedPlanState: VerifiedChatApprovedPlanProjection | undefined;
  private readonly eventIds = new Set<string>();

  constructor(readonly sessionId: string) {
    if (!validIdentifier(sessionId)) {
      throw new VerifiedChatProjectionError(
        "verified-chat.projection.session-invalid",
      );
    }
  }

  get cursor(): number | undefined {
    return this.nextSequence === 0 ? undefined : this.nextSequence - 1;
  }

  get terminal(): string | undefined {
    return this.terminalState;
  }

  get approvedPlan(): VerifiedChatApprovedPlanProjection | undefined {
    return this.approvedPlanState;
  }

  /** Accepts one exact ordered replay page atomically. */
  accept(events: readonly unknown[]): readonly VerifiedChatEventCard[] {
    if (events.length > MAX_REPLAY_EVENTS || this.terminalState !== undefined) {
      if (events.length === 0) return [];
      throw new VerifiedChatProjectionError(
        "verified-chat.projection.replay-invalid",
      );
    }
    let nextSequence = this.nextSequence;
    let previousEventSha256 = this.previousEventSha256;
    let terminalState: string | undefined = this.terminalState;
    let approvedPlanState = this.approvedPlanState;
    const newIds = new Set<string>();
    const cards: VerifiedChatEventCard[] = [];
    for (const value of events) {
      const event = exactRecord(value, [
        "correlation_id",
        "event_id",
        "event_sha256",
        "kind",
        "occurred_at_epoch_ms",
        "previous_event_sha256",
        "schema_version",
        "sequence",
        "session_id",
        "task_id",
      ]);
      const eventId = identifier(event.event_id);
      const eventSha256 = sha256(event.event_sha256);
      if (
        event.schema_version !== 1 ||
        event.session_id !== this.sessionId ||
        event.sequence !== nextSequence ||
        event.previous_event_sha256 !== previousEventSha256 ||
        !validIdentifier(event.correlation_id) ||
        !Number.isSafeInteger(event.occurred_at_epoch_ms) ||
        (event.occurred_at_epoch_ms as number) <= 0 ||
        (event.task_id !== null && !validIdentifier(event.task_id)) ||
        this.eventIds.has(eventId) ||
        newIds.has(eventId)
      ) {
        throw new VerifiedChatProjectionError(
          "verified-chat.projection.event-invalid",
        );
      }
      verifySealedRecord(event, "event_sha256");
      const kind = exactRecord(event.kind, undefined);
      const approvedPlan = projectApprovedPlan(kind, this.sessionId);
      if (approvedPlan !== undefined) {
        if (approvedPlanState !== undefined) {
          throw new VerifiedChatProjectionError(
            "verified-chat.projection.approval-duplicate",
          );
        }
        approvedPlanState = approvedPlan;
      }
      const projected = projectKind(kind, nextSequence, eventId);
      if (projected.category === "terminal") {
        if (terminalState !== undefined) {
          throw new VerifiedChatProjectionError(
            "verified-chat.projection.terminal-duplicate",
          );
        }
        terminalState = projected.state;
      }
      cards.push(projected);
      newIds.add(eventId);
      nextSequence += 1;
      previousEventSha256 = eventSha256;
    }
    this.nextSequence = nextSequence;
    this.previousEventSha256 = previousEventSha256;
    this.terminalState = terminalState;
    this.approvedPlanState = approvedPlanState;
    for (const id of newIds) this.eventIds.add(id);
    return cards;
  }
}

function projectApprovedPlan(
  kind: Readonly<Record<string, unknown>>,
  sessionId: string,
): VerifiedChatApprovedPlanProjection | undefined {
  if (kind.event !== "plan_approved") return undefined;
  const approval = exactRecord(kind.approval, [
    "approval_id",
    "approval_sha256",
    "approved_at_epoch_ms",
    "approved_by",
    "plan_artifact_id",
    "plan_sha256",
    "schema_version",
    "session_id",
  ]);
  if (
    approval.schema_version !== 1 ||
    approval.session_id !== sessionId ||
    !validIdentifier(approval.approval_id) ||
    !validIdentifier(approval.approved_by) ||
    !validIdentifier(approval.plan_artifact_id) ||
    !validSha256(approval.plan_sha256) ||
    !validSha256(approval.approval_sha256) ||
    !positiveInteger(approval.approved_at_epoch_ms)
  ) {
    invalidKind();
  }
  verifySealedRecord(approval, "approval_sha256");
  return {
    sourceSessionId: sessionId,
    artifactId: approval.plan_artifact_id,
    planSha256: approval.plan_sha256,
    approvalId: approval.approval_id,
    approvalSha256: approval.approval_sha256,
  };
}

interface PendingDelivery<T> {
  readonly value: T;
  readonly bytes: number;
  readonly resolve: () => void;
  readonly reject: (error: Error) => void;
}

/** Ordered count-and-byte-bounded delivery to one disposable webview. */
export class BoundedProjectionDelivery<
  T extends Readonly<Record<string, unknown>>,
> {
  private readonly pending: PendingDelivery<T>[] = [];
  private pendingBytes = 0;
  private inFlightMessages = 0;
  private inFlightBytes = 0;
  private draining = false;
  private closed = false;

  constructor(
    private readonly send: (value: T) => Promise<boolean>,
    private readonly maxPendingMessages = DEFAULT_MAX_PENDING_MESSAGES,
    private readonly maxPendingBytes = DEFAULT_MAX_PENDING_BYTES,
  ) {
    if (maxPendingMessages <= 0 || maxPendingBytes <= 0) {
      throw new VerifiedChatProjectionError(
        "verified-chat.delivery.limits-invalid",
      );
    }
  }

  enqueue(value: T): Promise<void> {
    if (this.closed) {
      return Promise.reject(
        new VerifiedChatProjectionError("verified-chat.delivery.closed"),
      );
    }
    const bytes = Buffer.byteLength(JSON.stringify(value), "utf8");
    if (
      bytes === 0 ||
      bytes > this.maxPendingBytes ||
      this.pending.length + this.inFlightMessages >= this.maxPendingMessages ||
      this.pendingBytes + this.inFlightBytes + bytes > this.maxPendingBytes
    ) {
      return Promise.reject(
        new VerifiedChatProjectionError("verified-chat.delivery.backpressure"),
      );
    }
    return new Promise<void>((resolve, reject) => {
      this.pending.push({ value, bytes, resolve, reject });
      this.pendingBytes += bytes;
      void this.drain();
    });
  }

  close(): void {
    this.closed = true;
    const error = new VerifiedChatProjectionError(
      "verified-chat.delivery.closed",
    );
    for (const item of this.pending.splice(0)) item.reject(error);
    this.pendingBytes = 0;
  }

  private async drain(): Promise<void> {
    if (this.draining) return;
    this.draining = true;
    try {
      while (!this.closed && this.pending.length > 0) {
        const item = this.pending.shift();
        if (item === undefined) break;
        this.pendingBytes -= item.bytes;
        this.inFlightMessages = 1;
        this.inFlightBytes = item.bytes;
        try {
          if (!(await this.send(item.value))) {
            throw new VerifiedChatProjectionError(
              "verified-chat.delivery.rejected",
            );
          }
          item.resolve();
        } catch (error) {
          item.reject(
            error instanceof Error
              ? error
              : new VerifiedChatProjectionError(
                  "verified-chat.delivery.rejected",
                ),
          );
        }
        this.inFlightMessages = 0;
        this.inFlightBytes = 0;
      }
    } finally {
      this.inFlightMessages = 0;
      this.inFlightBytes = 0;
      this.draining = false;
    }
  }
}

function projectKind(
  kind: Readonly<Record<string, unknown>>,
  sequence: number,
  eventId: string,
): VerifiedChatEventCard {
  const event = kind.event;
  if (typeof event !== "string") {
    throw new VerifiedChatProjectionError(
      "verified-chat.projection.kind-invalid",
    );
  }
  const card = (
    category: VerifiedChatEventCard["category"],
    label: string,
    state?: string,
  ): VerifiedChatEventCard => ({
    type: "runtimeEvent",
    category,
    sequence,
    eventId,
    label,
    ...(state === undefined ? {} : { state }),
  });
  switch (event) {
    case "session_created":
      exactKeys(kind, ["event"]);
      return card("session", "Session created");
    case "session_created_from_plan":
      exactKeys(kind, ["event", "handoff"]);
      return card("session", "Approved plan session created");
    case "artifact_transfer_started":
      exactKeys(kind, ["event", "upload_id"]);
      return card(
        "artifact",
        `Artifact transfer ${identifier(kind.upload_id)} started`,
      );
    case "artifact_transfer_progress":
      exactKeys(kind, ["bytes", "event", "upload_id"]);
      if (!nonnegativeInteger(kind.bytes)) invalidKind();
      return card(
        "artifact",
        `Artifact transfer ${identifier(kind.upload_id)}: ${kind.bytes} bytes`,
      );
    case "artifact_captured":
      exactKeys(kind, ["artifact_id", "event"]);
      return card(
        "artifact",
        `Artifact ${identifier(kind.artifact_id)} captured`,
      );
    case "context_admitted":
      exactKeys(kind, ["context_packet_id", "event"]);
      return card(
        "context",
        `Context ${identifier(kind.context_packet_id)} admitted`,
      );
    case "model_route_selected":
      exactKeys(kind, ["event", "route_decision_id"]);
      return card(
        "model",
        `Route ${identifier(kind.route_decision_id)} selected`,
      );
    case "tool_observed":
      exactKeys(kind, ["event", "tool_call_id"]);
      return card("tool", `Tool ${identifier(kind.tool_call_id)} observed`);
    case "verification_completed": {
      exactKeys(kind, ["event", "terminal"]);
      const state = terminal(kind.terminal);
      return card("verification", `Verification: ${state}`, state);
    }
    case "plan_approved":
      exactKeys(kind, ["approval", "event"]);
      return card("approval", "Plan approval recorded");
    case "runtime_bound":
      exactKeys(kind, ["binding", "event"]);
      return card("approval", "Approved plan bound to runtime");
    case "task_paused":
      exactKeys(kind, ["event"]);
      return card("lifecycle", "Task paused");
    case "task_resumed":
      exactKeys(kind, ["event"]);
      return card("lifecycle", "Task resumed");
    case "worker_updated":
      exactKeys(kind, ["event", "lease_id", "state"]);
      return card(
        "team",
        `Worker ${identifier(kind.lease_id)}: ${closedState(kind.state)}`,
      );
    case "integration_updated":
      exactKeys(kind, ["event", "integration_id", "state"]);
      return card(
        "team",
        `Integration ${identifier(kind.integration_id)}: ${closedState(kind.state)}`,
      );
    case "campaign_updated":
      exactKeys(kind, [
        "campaign_artifact_id",
        "campaign_id",
        "campaign_sha256",
        "event",
        "state",
      ]);
      sha256(kind.campaign_sha256);
      identifier(kind.campaign_artifact_id);
      return card(
        "team",
        `Campaign ${identifier(kind.campaign_id)}: ${closedState(kind.state)}`,
      );
    case "terminal": {
      exactKeys(kind, ["event", "state"]);
      const state = terminal(kind.state);
      return card("terminal", `Terminal: ${state}`, state);
    }
    default:
      return invalidKind();
  }
}

function terminal(value: unknown): string {
  const state = closedState(value);
  if (!validTerminal(state)) {
    invalidKind();
  }
  return state;
}

function closedState(value: unknown): string {
  if (typeof value !== "string" || !/^[A-Za-z0-9_:-]{1,64}$/u.test(value)) {
    invalidKind();
  }
  return value;
}

function invalidKind(): never {
  throw new VerifiedChatProjectionError(
    "verified-chat.projection.kind-invalid",
  );
}

function exactRecord(
  value: unknown,
  keys: readonly string[] | undefined,
): Readonly<Record<string, unknown>> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new VerifiedChatProjectionError(
      "verified-chat.projection.shape-invalid",
    );
  }
  const record = value as Readonly<Record<string, unknown>>;
  if (keys !== undefined) exactKeys(record, keys);
  return record;
}

function exactKeys(
  value: Readonly<Record<string, unknown>>,
  expected: readonly string[],
): void {
  const actual = Object.keys(value).sort();
  const wanted = [...expected].sort();
  if (
    actual.length !== wanted.length ||
    actual.some((item, index) => item !== wanted[index])
  ) {
    throw new VerifiedChatProjectionError(
      "verified-chat.projection.shape-invalid",
    );
  }
}

function identifier(value: unknown): string {
  if (!validIdentifier(value)) {
    throw new VerifiedChatProjectionError(
      "verified-chat.projection.identifier-invalid",
    );
  }
  return value;
}

function validIdentifier(value: unknown): value is string {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    value.length <= 128 &&
    /^[A-Za-z0-9._:-]+$/u.test(value)
  );
}

function sha256(value: unknown): string {
  if (!validSha256(value)) {
    throw new VerifiedChatProjectionError(
      "verified-chat.projection.digest-invalid",
    );
  }
  return value;
}

function validSha256(value: unknown): value is string {
  return typeof value === "string" && /^[0-9a-f]{64}$/u.test(value);
}

function validTerminal(value: unknown): value is string {
  return (
    typeof value === "string" &&
    new Set([
      "SUCCESS",
      "NO_OP",
      "BLOCKED",
      "DECLINED",
      "STALLED",
      "EXHAUSTED",
      "UNCERTAIN",
      "CANCELLED",
      "FAILED",
    ]).has(value)
  );
}

function verifySealedRecord(
  value: Readonly<Record<string, unknown>>,
  digestField: string,
): void {
  const expected = sha256(value[digestField]);
  const candidate = { ...value, [digestField]: ZERO_SHA256 };
  const actual = createHash("sha256")
    .update(JSON.stringify(candidate), "utf8")
    .digest("hex");
  if (actual !== expected) {
    throw new VerifiedChatProjectionError(
      "verified-chat.projection.digest-mismatch",
    );
  }
}

function nonnegativeInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

function positiveInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value > 0;
}
