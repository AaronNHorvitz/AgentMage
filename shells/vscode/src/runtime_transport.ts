import { createHash } from "node:crypto";

export const RUNTIME_CONTRACT_VERSION = 2 as const;

const ZERO_SHA256 = "0".repeat(64);
const MAX_RUNTIME_OUTPUT_BYTES = 4 * 1024 * 1024;

const RUNTIME_REQUEST_KEYS = [
  "context_budget",
  "event_cursor",
  "limits",
  "mode",
  "model_profile",
  "policy_id",
  "policy_sha256",
  "repository_snapshot_id",
  "repository_snapshot_sha256",
  "request_sha256",
  "run_id",
  "schema_version",
  "session_id",
  "task",
  "tool_catalog_id",
  "tool_catalog_sha256",
  "visible_tools",
  "work_packet",
  "workspace_id",
  "workspace_snapshot_sha256",
] as const;

const RUNTIME_EVENT_KEYS = [
  "causation_event_id",
  "correlation_id",
  "event_id",
  "event_sha256",
  "kind",
  "occurred_at_epoch_ms",
  "operation_id",
  "payload_reference",
  "persistence",
  "policy_id",
  "previous_event_sha256",
  "retention",
  "run_id",
  "schema_version",
  "sensitivity",
  "sequence",
  "session_id",
  "task_id",
  "turn_id",
] as const;

const RUNTIME_OUTCOME_KEYS = [
  "answer_evidence",
  "evidence",
  "model_call_count",
  "outcome_sha256",
  "output",
  "prior_event_id",
  "prior_event_sha256",
  "receipt_ids",
  "request_sha256",
  "run_id",
  "schema_version",
  "session_id",
  "state",
  "task_id",
  "tool_call_count",
  "turn_count",
  "unresolved_codes",
] as const;

const OPERATIONS = new Set([
  "workspace_read",
  "workspace_write",
  "workspace_delete",
  "command_execute",
  "network_access",
  "git_clone",
  "git_fetch",
  "git_worktree_create",
  "git_worktree_remove",
  "git_branch_fast_forward",
  "git_commit",
  "git_push",
  "publish",
  "send",
  "upload",
  "deploy",
  "database_read",
  "database_write",
  "credential_access",
  "model_inference",
  "draft_create",
  "administration",
]);

const TERMINAL_STATES = new Set([
  "SUCCESS",
  "NO_OP",
  "BLOCKED",
  "DECLINED",
  "STALLED",
  "EXHAUSTED",
  "UNCERTAIN",
  "CANCELLED",
  "FAILED",
]);

export interface RuntimeTaskEnvelope {
  readonly schema_version: 2;
  readonly task_id: string;
  readonly session_id: string;
  readonly objective: string;
  readonly acceptance_criteria: readonly string[];
  readonly constraints: readonly string[];
  readonly status: "ready" | "running";
}

export interface RuntimeModelProfileEnvelope {
  readonly profile_id: string;
  readonly [key: string]: unknown;
}

/** Exact host-framed request forwarded unchanged by the display-only shell. */
export interface RuntimeRunRequestEnvelope {
  readonly schema_version: 2;
  readonly run_id: string;
  readonly session_id: string;
  readonly mode: "ephemeral_read_only" | "controlled_write";
  readonly task: RuntimeTaskEnvelope;
  readonly work_packet: unknown;
  readonly workspace_id: string;
  readonly workspace_snapshot_sha256: string;
  readonly repository_snapshot_id: string;
  readonly repository_snapshot_sha256: string;
  readonly model_profile: RuntimeModelProfileEnvelope;
  readonly context_budget: unknown;
  readonly tool_catalog_id: string;
  readonly tool_catalog_sha256: string;
  readonly visible_tools: readonly unknown[];
  readonly policy_id: string;
  readonly policy_sha256: string;
  readonly limits: unknown;
  readonly event_cursor: null;
  readonly request_sha256: string;
}

export interface RuntimeEventCursorEnvelope {
  readonly run_id: string;
  readonly event_id: string;
  readonly sequence: number;
  readonly event_sha256: string;
}

export interface RuntimeEventKindEnvelope {
  readonly event: string;
  readonly [key: string]: unknown;
}

export interface RuntimeEventEnvelope {
  readonly schema_version: 2;
  readonly event_id: string;
  readonly run_id: string;
  readonly session_id: string;
  readonly task_id: string;
  readonly turn_id: string | null;
  readonly operation_id: string | null;
  readonly correlation_id: string;
  readonly causation_event_id: string | null;
  readonly sequence: number;
  readonly occurred_at_epoch_ms: number;
  readonly sensitivity: "public" | "internal" | "private" | "restricted";
  readonly retention: Readonly<Record<string, unknown>>;
  readonly persistence: "correctness" | "progress" | "metric";
  readonly policy_id: string;
  readonly payload_reference: Readonly<Record<string, unknown>> | null;
  readonly kind: RuntimeEventKindEnvelope;
  readonly previous_event_sha256: string;
  readonly event_sha256: string;
}

export interface RuntimeApprovalChallengeEnvelope {
  readonly schema_version: 2;
  readonly run_id: string;
  readonly task_id: string;
  readonly turn_id: string;
  readonly operation_id: string;
  readonly tool_call_id: string;
  readonly approval_id: string;
  readonly proposed_grant_id: string;
  readonly operation: string;
  readonly preview_sha256: string;
  readonly expires_at_epoch_ms: number;
  readonly challenge_sha256: string;
}

export interface RuntimeApprovalResponseEnvelope {
  readonly schema_version: 2;
  readonly run_id: string;
  readonly approval_id: string;
  readonly disposition: "allow" | "deny";
  readonly challenge_sha256: string;
  readonly grant_id: string | null;
}

interface RuntimeInlineOutput {
  readonly storage: "inline";
  readonly payload: {
    readonly schema: Readonly<Record<string, unknown>>;
    readonly media_type: string;
    readonly bytes: readonly number[];
    readonly sha256: string;
  };
}

interface RuntimeArtifactOutput {
  readonly storage: "artifact";
  readonly reference: {
    readonly artifact_id: string;
    readonly sha256: string;
    readonly byte_size: number;
    readonly media_type: string;
  };
}

export type RuntimeOutputEnvelope = RuntimeInlineOutput | RuntimeArtifactOutput;

export interface RuntimeAnswerEvidenceEnvelope {
  readonly schema_version: 2;
  readonly task_id: string;
  readonly model_run_id: string;
  readonly response_sha256: string;
  readonly output_sha256: string;
  readonly output_byte_size: number;
  readonly output_media_type: string;
  readonly rendered_claim_ids: readonly string[];
  readonly assignments: readonly Readonly<Record<string, unknown>>[];
  readonly answer_evidence_sha256: string;
}

export interface RuntimeArtifactReferenceEnvelope {
  readonly schema_version: 2;
  readonly artifact_id: string;
  readonly manifest_sha256: string;
  readonly payload_sha256: string;
  readonly byte_size: number;
  readonly media_type: string;
}

export interface RuntimeOutcomeEnvelope {
  readonly schema_version: 2;
  readonly run_id: string;
  readonly session_id: string;
  readonly task_id: string;
  readonly request_sha256: string;
  readonly state: string;
  readonly turn_count: number;
  readonly model_call_count: number;
  readonly tool_call_count: number;
  readonly prior_event_id: string;
  readonly prior_event_sha256: string;
  readonly evidence: readonly Readonly<Record<string, unknown>>[];
  readonly receipt_ids: readonly string[];
  readonly unresolved_codes: readonly string[];
  readonly output: RuntimeOutputEnvelope | null;
  readonly answer_evidence: RuntimeAnswerEvidenceEnvelope | null;
  readonly outcome_sha256: string;
}

export interface RuntimePreparedResponse {
  readonly kind: "runtime_prepared";
  readonly schema_version: 1;
  readonly request_id: string;
  readonly run_request: RuntimeRunRequestEnvelope;
}

export interface RuntimeStepResponse {
  readonly kind: "runtime_step";
  readonly schema_version: 1;
  readonly request_id: string;
  readonly run_id: string;
  readonly request_sha256: string;
  readonly events: readonly RuntimeEventEnvelope[];
  readonly artifacts: readonly RuntimeArtifactReferenceEnvelope[];
  readonly approval: RuntimeApprovalChallengeEnvelope | null;
  readonly outcome: RuntimeOutcomeEnvelope | null;
}

export type RuntimeHostResponse = RuntimePreparedResponse | RuntimeStepResponse;

export class RuntimeTransportFailure extends Error {
  constructor() {
    super("vscode.runtime.response_invalid");
  }
}

export function parseRuntimeHostResponse(
  candidate: unknown,
): RuntimeHostResponse {
  const record = requiredRecord(candidate);
  if (record.schema_version !== 1 || !validIdentifier(record.request_id)) {
    throw new RuntimeTransportFailure();
  }
  if (record.kind === "runtime_prepared") {
    requireKeys(record, [
      "kind",
      "request_id",
      "run_request",
      "schema_version",
    ]);
    return {
      kind: "runtime_prepared",
      schema_version: 1,
      request_id: record.request_id,
      run_request: parseRuntimeRunRequest(record.run_request),
    };
  }
  if (record.kind === "runtime_step") {
    requireKeys(record, [
      "approval",
      "artifacts",
      "events",
      "kind",
      "outcome",
      "request_id",
      "request_sha256",
      "run_id",
      "schema_version",
    ]);
    if (
      !validIdentifier(record.run_id) ||
      !validSha256(record.request_sha256) ||
      !Array.isArray(record.events) ||
      record.events.length > 65_536 ||
      !Array.isArray(record.artifacts) ||
      record.artifacts.length > 1_024
    ) {
      throw new RuntimeTransportFailure();
    }
    const approval =
      record.approval === null
        ? null
        : parseRuntimeApprovalChallenge(record.approval);
    const outcome =
      record.outcome === null ? null : parseRuntimeOutcome(record.outcome);
    if ((approval === null) === (outcome === null)) {
      throw new RuntimeTransportFailure();
    }
    const events = record.events.map(parseRuntimeEvent);
    const artifacts = record.artifacts.map(parseRuntimeArtifactReference);
    if (
      events.some((event) => event.run_id !== record.run_id) ||
      (approval !== null && approval.run_id !== record.run_id) ||
      (outcome !== null &&
        (outcome.run_id !== record.run_id ||
          outcome.request_sha256 !== record.request_sha256))
    ) {
      throw new RuntimeTransportFailure();
    }
    return {
      kind: "runtime_step",
      schema_version: 1,
      request_id: record.request_id,
      run_id: record.run_id,
      request_sha256: record.request_sha256,
      events,
      artifacts,
      approval,
      outcome,
    };
  }
  throw new RuntimeTransportFailure();
}

function parseRuntimeArtifactReference(
  candidate: unknown,
): RuntimeArtifactReferenceEnvelope {
  const record = requiredRecord(candidate);
  requireKeys(record, [
    "artifact_id",
    "byte_size",
    "manifest_sha256",
    "media_type",
    "payload_sha256",
    "schema_version",
  ]);
  if (
    record.schema_version !== RUNTIME_CONTRACT_VERSION ||
    !validIdentifier(record.artifact_id) ||
    !validSha256(record.manifest_sha256) ||
    !validSha256(record.payload_sha256) ||
    !positiveSafeInteger(record.byte_size) ||
    !validMediaType(record.media_type)
  ) {
    throw new RuntimeTransportFailure();
  }
  return record as unknown as RuntimeArtifactReferenceEnvelope;
}

export function parseRuntimeRunRequest(
  candidate: unknown,
): RuntimeRunRequestEnvelope {
  const record = requiredRecord(candidate);
  requireKeys(record, RUNTIME_REQUEST_KEYS);
  const task = parseRuntimeTask(record.task);
  const modelProfile = requiredRecord(record.model_profile);
  if (
    record.schema_version !== RUNTIME_CONTRACT_VERSION ||
    !validIdentifier(record.run_id) ||
    !validIdentifier(record.session_id) ||
    !["ephemeral_read_only", "controlled_write"].includes(
      String(record.mode),
    ) ||
    task.session_id !== record.session_id ||
    !isRecord(record.work_packet) ||
    !validIdentifier(record.workspace_id) ||
    !validSha256(record.workspace_snapshot_sha256) ||
    !validIdentifier(record.repository_snapshot_id) ||
    !validSha256(record.repository_snapshot_sha256) ||
    !validIdentifier(modelProfile.profile_id) ||
    !isRecord(record.context_budget) ||
    !validIdentifier(record.tool_catalog_id) ||
    !validSha256(record.tool_catalog_sha256) ||
    !Array.isArray(record.visible_tools) ||
    !validIdentifier(record.policy_id) ||
    !validSha256(record.policy_sha256) ||
    !isRecord(record.limits) ||
    record.event_cursor !== null ||
    !validSha256(record.request_sha256)
  ) {
    throw new RuntimeTransportFailure();
  }
  return record as unknown as RuntimeRunRequestEnvelope;
}

export function runtimeApprovalResponse(
  challenge: RuntimeApprovalChallengeEnvelope,
  disposition: "allow" | "deny",
): RuntimeApprovalResponseEnvelope {
  return {
    schema_version: RUNTIME_CONTRACT_VERSION,
    run_id: challenge.run_id,
    approval_id: challenge.approval_id,
    disposition,
    challenge_sha256: challenge.challenge_sha256,
    grant_id: disposition === "allow" ? challenge.proposed_grant_id : null,
  };
}

export function runtimeCursor(
  event: RuntimeEventEnvelope,
): RuntimeEventCursorEnvelope {
  return {
    run_id: event.run_id,
    event_id: event.event_id,
    sequence: event.sequence,
    event_sha256: event.event_sha256,
  };
}

/** Stateful presentation-side verifier for one exact ordered runtime stream. */
export class RuntimeStreamVerifier {
  private readonly eventIds = new Set<string>();
  private readonly eventDigests = new Map<string, string>();
  private readonly artifactEvents = new Map<
    string,
    {
      readonly manifestSha256: string;
      readonly payloadSha256: string;
      readonly byteSize: number;
      readonly mediaType: string;
    }
  >();
  private nextSequence = 0;
  private previousEventSha256 = ZERO_SHA256;
  private correlationId: string | undefined;
  private lastOccurredAt = 0;
  private lastEvent: RuntimeEventEnvelope | undefined;
  private terminalEvent: RuntimeEventEnvelope | undefined;

  constructor(private readonly request: RuntimeRunRequestEnvelope) {}

  accept(step: RuntimeStepResponse): readonly RuntimeEventEnvelope[] {
    if (
      step.run_id !== this.request.run_id ||
      step.request_sha256 !== this.request.request_sha256
    ) {
      throw new RuntimeTransportFailure();
    }
    for (const event of step.events) {
      this.acceptEvent(event);
    }
    const artifactIds = new Set<string>();
    if (step.artifacts.length !== this.artifactEvents.size) {
      throw new RuntimeTransportFailure();
    }
    for (const reference of step.artifacts) {
      const eventReference = this.artifactEvents.get(reference.artifact_id);
      if (
        artifactIds.has(reference.artifact_id) ||
        eventReference === undefined ||
        eventReference.manifestSha256 !== reference.manifest_sha256 ||
        eventReference.payloadSha256 !== reference.payload_sha256 ||
        eventReference.byteSize !== reference.byte_size ||
        eventReference.mediaType !== reference.media_type
      ) {
        throw new RuntimeTransportFailure();
      }
      artifactIds.add(reference.artifact_id);
    }
    if (step.approval !== null) {
      const last = this.lastEvent;
      if (
        this.terminalEvent !== undefined ||
        last?.kind.event !== "permission_requested" ||
        step.approval.run_id !== this.request.run_id ||
        step.approval.task_id !== this.request.task.task_id ||
        step.approval.approval_id !== last.kind.approval_id ||
        step.approval.turn_id !== last.turn_id ||
        step.approval.operation_id !== last.operation_id ||
        step.approval.operation !== last.kind.operation ||
        step.approval.preview_sha256 !== last.kind.preview_sha256 ||
        step.approval.expires_at_epoch_ms !== last.kind.expires_at_epoch_ms
      ) {
        throw new RuntimeTransportFailure();
      }
    }
    if (step.outcome !== null) {
      const terminal = this.terminalEvent;
      const artifactOutput =
        step.outcome.output?.storage === "artifact"
          ? step.outcome.output.reference
          : undefined;
      const retainedOutput =
        artifactOutput === undefined
          ? undefined
          : step.artifacts.find(
              (reference) =>
                reference.artifact_id === artifactOutput.artifact_id,
            );
      if (
        terminal === undefined ||
        step.outcome.run_id !== this.request.run_id ||
        step.outcome.session_id !== this.request.session_id ||
        step.outcome.task_id !== this.request.task.task_id ||
        step.outcome.request_sha256 !== this.request.request_sha256 ||
        step.outcome.state !== terminal.kind.state ||
        step.outcome.outcome_sha256 !== terminal.kind.outcome_sha256 ||
        terminal.causation_event_id !== step.outcome.prior_event_id ||
        terminal.previous_event_sha256 !== step.outcome.prior_event_sha256 ||
        this.eventDigests.get(step.outcome.prior_event_id) !==
          step.outcome.prior_event_sha256 ||
        (artifactOutput !== undefined &&
          (retainedOutput === undefined ||
            retainedOutput.payload_sha256 !== artifactOutput.sha256 ||
            retainedOutput.byte_size !== artifactOutput.byte_size ||
            retainedOutput.media_type !== artifactOutput.media_type))
      ) {
        throw new RuntimeTransportFailure();
      }
    }
    return step.events;
  }

  cursor(): RuntimeEventCursorEnvelope | null {
    return this.lastEvent === undefined ? null : runtimeCursor(this.lastEvent);
  }

  private acceptEvent(event: RuntimeEventEnvelope): void {
    if (
      this.terminalEvent !== undefined ||
      event.run_id !== this.request.run_id ||
      event.session_id !== this.request.session_id ||
      event.task_id !== this.request.task.task_id ||
      event.policy_id !== this.request.policy_id ||
      (this.correlationId !== undefined &&
        event.correlation_id !== this.correlationId) ||
      event.sequence !== this.nextSequence ||
      event.occurred_at_epoch_ms < this.lastOccurredAt ||
      event.previous_event_sha256 !== this.previousEventSha256 ||
      this.eventIds.has(event.event_id) ||
      (event.sequence === 0
        ? event.kind.event !== "run_started" ||
          event.kind.request_sha256 !== this.request.request_sha256 ||
          event.turn_id !== null ||
          event.operation_id !== null ||
          event.causation_event_id !== null
        : event.causation_event_id === null ||
          !this.eventIds.has(event.causation_event_id))
    ) {
      throw new RuntimeTransportFailure();
    }
    this.eventIds.add(event.event_id);
    this.eventDigests.set(event.event_id, event.event_sha256);
    this.nextSequence += 1;
    this.previousEventSha256 = event.event_sha256;
    this.correlationId ??= event.correlation_id;
    this.lastOccurredAt = event.occurred_at_epoch_ms;
    this.lastEvent = event;
    if (event.kind.event === "artifact_created") {
      const artifactId = event.kind.artifact_id as string;
      const manifestSha256 = event.kind.manifest_sha256 as string;
      const payload = event.payload_reference;
      if (
        this.artifactEvents.has(artifactId) ||
        payload === null ||
        payload.artifact_id !== artifactId
      ) {
        throw new RuntimeTransportFailure();
      }
      this.artifactEvents.set(artifactId, {
        manifestSha256,
        payloadSha256: payload.sha256 as string,
        byteSize: payload.byte_size as number,
        mediaType: payload.media_type as string,
      });
    }
    if (event.kind.event === "run_terminal") {
      this.terminalEvent = event;
    }
  }
}

export function renderRuntimeEvent(event: RuntimeEventEnvelope): string {
  const labels: Readonly<Record<string, string>> = {
    run_started: "Run started",
    turn_started: "Working",
    turn_completed: "Step completed",
    model_requested: "Local model started",
    model_completed: "Local model completed",
    model_failed: "Local model failed",
    tool_requested: "Tool request prepared",
    tool_started: "Approved tool started",
    tool_completed: "Tool completed",
    tool_failed: "Tool failed",
    permission_requested: "Approval required",
    permission_decided: "Approval decision recorded",
    file_observed: "Workspace evidence observed",
    file_modified: "Workspace change verified",
    artifact_created: "Result artifact created",
    checkpoint_committed: "Checkpoint committed",
    cancellation_requested: "Cancellation requested",
    cancellation_observed: "Cancellation completed",
    progress: "Progress updated",
    metric: "Local metric recorded",
    run_terminal: "Run completed",
  };
  const label = labels[event.kind.event];
  if (label === undefined) {
    throw new RuntimeTransportFailure();
  }
  return `- ${event.sequence.toString()}: ${label}\n`;
}

export function renderRuntimeOutcome(outcome: RuntimeOutcomeEnvelope): string {
  validateRuntimeAnswerEvidence(
    outcome.answer_evidence,
    outcome.output,
    outcome.task_id,
    outcome.evidence,
    outcome.state,
  );
  const lines = [
    "\n## Result\n",
    `\n- Status: ${outcome.state}`,
    `- Turns: ${outcome.turn_count.toString()}`,
    `- Model calls: ${outcome.model_call_count.toString()}`,
    `- Tool calls: ${outcome.tool_call_count.toString()}`,
    `- Evidence records: ${outcome.evidence.length.toString()}`,
    `- Receipts: ${outcome.receipt_ids.length.toString()}`,
    ...(outcome.answer_evidence === null
      ? []
      : [
          `- Evidence states: inferred (${outcome.answer_evidence.assignments.length.toString()})`,
        ]),
    `- Outcome: \`${outcome.outcome_sha256}\``,
  ];
  const output = outcome.output;
  if (output === null) {
    return `${lines.join("\n")}\n`;
  }
  if (output.storage === "artifact") {
    lines.splice(
      1,
      0,
      `\nOutput retained as verified local artifact \`${output.reference.artifact_id}\` (${output.reference.media_type}, ${output.reference.byte_size.toString()} bytes, \`${output.reference.sha256}\`).\n`,
    );
    return `${lines.join("\n")}\n`;
  }
  const bytes = Uint8Array.from(output.payload.bytes);
  const digest = createHash("sha256").update(bytes).digest("hex");
  if (digest !== output.payload.sha256) {
    throw new RuntimeTransportFailure();
  }
  let text: string;
  try {
    text = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch {
    throw new RuntimeTransportFailure();
  }
  if (output.payload.media_type === "text/markdown") {
    lines.splice(1, 0, `\n${sanitizeRuntimeMarkdown(text)}\n`);
  } else {
    const indented = text
      .split("\n")
      .map((line) => `    ${line}`)
      .join("\n");
    lines.splice(1, 0, `\n${indented}\n`);
  }
  return `${lines.join("\n")}\n`;
}

function parseRuntimeTask(candidate: unknown): RuntimeTaskEnvelope {
  const record = requiredRecord(candidate);
  requireKeys(record, [
    "acceptance_criteria",
    "constraints",
    "objective",
    "schema_version",
    "session_id",
    "status",
    "task_id",
  ]);
  if (
    record.schema_version !== RUNTIME_CONTRACT_VERSION ||
    !validIdentifier(record.task_id) ||
    !validIdentifier(record.session_id) ||
    !validText(record.objective, 4_096) ||
    !validTextArray(record.acceptance_criteria, 128, false) ||
    !validTextArray(record.constraints, 128, true) ||
    (record.status !== "ready" && record.status !== "running")
  ) {
    throw new RuntimeTransportFailure();
  }
  return record as unknown as RuntimeTaskEnvelope;
}

function parseRuntimeEvent(candidate: unknown): RuntimeEventEnvelope {
  const record = requiredRecord(candidate);
  requireKeys(record, RUNTIME_EVENT_KEYS);
  const kind = parseRuntimeEventKind(record.kind);
  const retention = requiredRecord(record.retention);
  requireKeys(retention, ["expires_at_epoch_ms", "kind"]);
  if (
    !["ephemeral", "session", "until_expiration", "user_hold"].includes(
      String(retention.kind),
    ) ||
    (retention.kind === "until_expiration"
      ? !positiveSafeInteger(retention.expires_at_epoch_ms)
      : retention.expires_at_epoch_ms !== null)
  ) {
    throw new RuntimeTransportFailure();
  }
  const payloadReference = parsePayloadReference(record.payload_reference);
  if (
    record.schema_version !== RUNTIME_CONTRACT_VERSION ||
    !validIdentifier(record.event_id) ||
    !validIdentifier(record.run_id) ||
    !validIdentifier(record.session_id) ||
    !validIdentifier(record.task_id) ||
    !optionalIdentifier(record.turn_id) ||
    !optionalIdentifier(record.operation_id) ||
    !validIdentifier(record.correlation_id) ||
    !optionalIdentifier(record.causation_event_id) ||
    !nonnegativeSafeInteger(record.sequence) ||
    !positiveSafeInteger(record.occurred_at_epoch_ms) ||
    !["public", "internal", "private", "restricted"].includes(
      String(record.sensitivity),
    ) ||
    !["correctness", "progress", "metric"].includes(
      String(record.persistence),
    ) ||
    !validIdentifier(record.policy_id) ||
    !validSha256(record.previous_event_sha256) ||
    !validSha256(record.event_sha256)
  ) {
    throw new RuntimeTransportFailure();
  }
  return {
    ...(record as unknown as Omit<
      RuntimeEventEnvelope,
      "kind" | "payload_reference"
    >),
    kind,
    payload_reference: payloadReference,
  };
}

function parseRuntimeEventKind(candidate: unknown): RuntimeEventKindEnvelope {
  const record = requiredRecord(candidate);
  const event = record.event;
  if (typeof event !== "string") {
    throw new RuntimeTransportFailure();
  }
  switch (event) {
    case "run_started":
      exactKind(
        record,
        ["event", "request_sha256"],
        [record.request_sha256],
        validSha256,
      );
      break;
    case "turn_started":
      requireKeys(record, ["event"]);
      break;
    case "turn_completed":
      exactKind(
        record,
        ["event", "outcome_sha256"],
        [record.outcome_sha256],
        validSha256,
      );
      break;
    case "model_requested":
      requireKeys(record, ["event", "model_run_id", "request_sha256"]);
      requireValid(
        validIdentifier(record.model_run_id) &&
          validSha256(record.request_sha256),
      );
      break;
    case "model_completed":
      requireKeys(record, ["event", "model_run_id", "result_sha256"]);
      requireValid(
        validIdentifier(record.model_run_id) &&
          validSha256(record.result_sha256),
      );
      break;
    case "model_failed":
      requireKeys(record, ["event", "failure_code", "model_run_id"]);
      requireValid(
        validIdentifier(record.model_run_id) && validCode(record.failure_code),
      );
      break;
    case "tool_requested":
      requireKeys(record, ["arguments_sha256", "event", "tool_call_id"]);
      requireValid(
        validIdentifier(record.tool_call_id) &&
          validSha256(record.arguments_sha256),
      );
      break;
    case "tool_started":
      requireKeys(record, ["authority_sha256", "event", "tool_call_id"]);
      requireValid(
        validIdentifier(record.tool_call_id) &&
          validSha256(record.authority_sha256),
      );
      break;
    case "tool_completed":
      requireKeys(record, [
        "event",
        "receipt_id",
        "result_sha256",
        "tool_call_id",
      ]);
      requireValid(
        validIdentifier(record.tool_call_id) &&
          validIdentifier(record.receipt_id) &&
          validSha256(record.result_sha256),
      );
      break;
    case "tool_failed":
      requireKeys(record, [
        "event",
        "failure_code",
        "receipt_id",
        "tool_call_id",
      ]);
      requireValid(
        validIdentifier(record.tool_call_id) &&
          optionalIdentifier(record.receipt_id) &&
          validCode(record.failure_code),
      );
      break;
    case "permission_requested":
      requireKeys(record, [
        "approval_id",
        "event",
        "expires_at_epoch_ms",
        "operation",
        "preview_sha256",
      ]);
      requireValid(
        validIdentifier(record.approval_id) &&
          validOperation(record.operation) &&
          validSha256(record.preview_sha256) &&
          positiveSafeInteger(record.expires_at_epoch_ms),
      );
      break;
    case "permission_decided":
      requireKeys(record, [
        "approval_id",
        "decision_sha256",
        "disposition",
        "event",
        "grant_id",
      ]);
      requireValid(
        validIdentifier(record.approval_id) &&
          ["ALLOW", "ASK", "DENY"].includes(String(record.disposition)) &&
          optionalIdentifier(record.grant_id) &&
          validSha256(record.decision_sha256),
      );
      break;
    case "file_observed":
      requireKeys(record, [
        "event",
        "object_identity_sha256",
        "observation_sha256",
      ]);
      requireValid(
        validSha256(record.object_identity_sha256) &&
          validSha256(record.observation_sha256),
      );
      break;
    case "file_modified":
      requireKeys(record, [
        "event",
        "object_identity_sha256",
        "postcondition_sha256",
        "receipt_id",
      ]);
      requireValid(
        validSha256(record.object_identity_sha256) &&
          validSha256(record.postcondition_sha256) &&
          validIdentifier(record.receipt_id),
      );
      break;
    case "artifact_created":
      requireKeys(record, ["artifact_id", "event", "manifest_sha256"]);
      requireValid(
        validIdentifier(record.artifact_id) &&
          validSha256(record.manifest_sha256),
      );
      break;
    case "checkpoint_committed":
      requireKeys(record, ["checkpoint_id", "checkpoint_sha256", "event"]);
      requireValid(
        validIdentifier(record.checkpoint_id) &&
          validSha256(record.checkpoint_sha256),
      );
      break;
    case "cancellation_requested":
    case "cancellation_observed":
      requireKeys(record, ["cancellation_id", "event"]);
      requireValid(validIdentifier(record.cancellation_id));
      break;
    case "progress":
      requireKeys(record, ["code", "event"]);
      requireValid(validCode(record.code));
      break;
    case "metric":
      requireKeys(record, ["event", "name", "value"]);
      requireValid(
        validCode(record.name) && Number.isSafeInteger(record.value),
      );
      break;
    case "run_terminal":
      requireKeys(record, ["event", "outcome_sha256", "state"]);
      requireValid(
        TERMINAL_STATES.has(String(record.state)) &&
          validSha256(record.outcome_sha256),
      );
      break;
    default:
      throw new RuntimeTransportFailure();
  }
  return record as unknown as RuntimeEventKindEnvelope;
}

function parseRuntimeApprovalChallenge(
  candidate: unknown,
): RuntimeApprovalChallengeEnvelope {
  const record = requiredRecord(candidate);
  requireKeys(record, [
    "approval_id",
    "challenge_sha256",
    "expires_at_epoch_ms",
    "operation",
    "operation_id",
    "preview_sha256",
    "proposed_grant_id",
    "run_id",
    "schema_version",
    "task_id",
    "tool_call_id",
    "turn_id",
  ]);
  if (
    record.schema_version !== RUNTIME_CONTRACT_VERSION ||
    !validIdentifier(record.run_id) ||
    !validIdentifier(record.task_id) ||
    !validIdentifier(record.turn_id) ||
    !validIdentifier(record.operation_id) ||
    !validIdentifier(record.tool_call_id) ||
    !validIdentifier(record.approval_id) ||
    !validIdentifier(record.proposed_grant_id) ||
    !validOperation(record.operation) ||
    !validSha256(record.preview_sha256) ||
    !positiveSafeInteger(record.expires_at_epoch_ms) ||
    !validSha256(record.challenge_sha256)
  ) {
    throw new RuntimeTransportFailure();
  }
  return record as unknown as RuntimeApprovalChallengeEnvelope;
}

function parseRuntimeOutcome(candidate: unknown): RuntimeOutcomeEnvelope {
  const record = requiredRecord(candidate);
  requireKeys(record, RUNTIME_OUTCOME_KEYS);
  if (
    record.schema_version !== RUNTIME_CONTRACT_VERSION ||
    !validIdentifier(record.run_id) ||
    !validIdentifier(record.session_id) ||
    !validIdentifier(record.task_id) ||
    !validSha256(record.request_sha256) ||
    !TERMINAL_STATES.has(String(record.state)) ||
    !nonnegativeSafeInteger(record.turn_count) ||
    !nonnegativeSafeInteger(record.model_call_count) ||
    !nonnegativeSafeInteger(record.tool_call_count) ||
    !validIdentifier(record.prior_event_id) ||
    !validSha256(record.prior_event_sha256) ||
    !Array.isArray(record.evidence) ||
    record.evidence.length > 4_096 ||
    !record.evidence.every(validEvidenceReference) ||
    !Array.isArray(record.receipt_ids) ||
    !record.receipt_ids.every(validIdentifier) ||
    !Array.isArray(record.unresolved_codes) ||
    !record.unresolved_codes.every(validCode) ||
    !validSha256(record.outcome_sha256)
  ) {
    throw new RuntimeTransportFailure();
  }
  const output = parseRuntimeOutput(record.output);
  const answerEvidence = parseRuntimeAnswerEvidence(
    record.answer_evidence,
    output,
    String(record.task_id),
    record.evidence as readonly Readonly<Record<string, unknown>>[],
    String(record.state),
  );
  return {
    ...(record as unknown as Omit<
      RuntimeOutcomeEnvelope,
      "answer_evidence" | "output"
    >),
    output,
    answer_evidence: answerEvidence,
  };
}

function parseRuntimeAnswerEvidence(
  candidate: unknown,
  output: RuntimeOutputEnvelope | null,
  taskId: string,
  outcomeEvidence: readonly Readonly<Record<string, unknown>>[],
  state: string,
): RuntimeAnswerEvidenceEnvelope | null {
  if (candidate === null) {
    validateRuntimeAnswerEvidence(null, output, taskId, outcomeEvidence, state);
    return null;
  }
  const record = requiredRecord(candidate);
  requireKeys(record, [
    "answer_evidence_sha256",
    "assignments",
    "model_run_id",
    "output_byte_size",
    "output_media_type",
    "output_sha256",
    "rendered_claim_ids",
    "response_sha256",
    "schema_version",
    "task_id",
  ]);
  if (
    record.schema_version !== RUNTIME_CONTRACT_VERSION ||
    !validIdentifier(record.task_id) ||
    !validIdentifier(record.model_run_id) ||
    !validSha256(record.response_sha256) ||
    !validSha256(record.output_sha256) ||
    !positiveSafeInteger(record.output_byte_size) ||
    !validMediaType(record.output_media_type) ||
    !Array.isArray(record.rendered_claim_ids) ||
    !record.rendered_claim_ids.every(validIdentifier) ||
    !Array.isArray(record.assignments) ||
    !validSha256(record.answer_evidence_sha256)
  ) {
    throw new RuntimeTransportFailure();
  }
  const answer = record as unknown as RuntimeAnswerEvidenceEnvelope;
  validateRuntimeAnswerEvidence(answer, output, taskId, outcomeEvidence, state);
  return answer;
}

function validateRuntimeAnswerEvidence(
  answer: RuntimeAnswerEvidenceEnvelope | null,
  output: RuntimeOutputEnvelope | null,
  taskId: string,
  outcomeEvidence: readonly Readonly<Record<string, unknown>>[],
  state: string,
): void {
  const successfulOutput =
    (state === "SUCCESS" || state === "NO_OP") && output !== null;
  if (answer === null) {
    requireValid(!successfulOutput);
    return;
  }
  if (!successfulOutput || output === null || answer.task_id !== taskId) {
    throw new RuntimeTransportFailure();
  }
  const outputIdentity =
    output.storage === "inline"
      ? {
          sha256: output.payload.sha256,
          byteSize: output.payload.bytes.length,
          mediaType: output.payload.media_type,
        }
      : {
          sha256: output.reference.sha256,
          byteSize: output.reference.byte_size,
          mediaType: output.reference.media_type,
        };
  requireValid(
    answer.output_sha256 === outputIdentity.sha256 &&
      answer.output_byte_size === outputIdentity.byteSize &&
      answer.output_media_type === outputIdentity.mediaType &&
      answer.rendered_claim_ids.length === 1 &&
      answer.rendered_claim_ids[0] === "runtime.answer.content" &&
      answer.assignments.length === 1,
  );
  const assignment = requiredRecord(answer.assignments[0]);
  requireKeys(assignment, [
    "assignment_id",
    "claim",
    "evidence_state",
    "schema_version",
  ]);
  const claim = requiredRecord(assignment.claim);
  requireKeys(claim, [
    "claim_id",
    "expected_revision",
    "kind",
    "prerequisite_claim_ids",
    "schema_version",
    "statement",
    "subject_id",
    "task_id",
  ]);
  const evidenceState = requiredRecord(assignment.evidence_state);
  requireKeys(evidenceState, ["provenance", "state"]);
  const provenance = requiredRecord(evidenceState.provenance);
  requireKeys(provenance, ["citations", "runtime"]);
  const runtime = requiredRecord(provenance.runtime);
  requireKeys(runtime, ["manifest", "model_run_id", "response_sha256"]);
  const manifest = requiredRecord(runtime.manifest);
  requireKeys(manifest, [
    "artifact_sha256",
    "codec_sha256",
    "manifest_sha256",
    "profile_id",
    "runtime",
    "template_sha256",
    "tokenizer_sha256",
  ]);
  const modelRuntime = requiredRecord(manifest.runtime);
  requireKeys(modelRuntime, [
    "adapter_id",
    "architecture",
    "contract_version",
    "kind",
    "platform",
    "runtime_build",
    "runtime_sha256",
  ]);
  requireValid(
    assignment.schema_version === RUNTIME_CONTRACT_VERSION &&
      assignment.assignment_id === "runtime.answer.assignment" &&
      claim.schema_version === RUNTIME_CONTRACT_VERSION &&
      claim.claim_id === "runtime.answer.content" &&
      claim.task_id === taskId &&
      claim.kind === "read" &&
      claim.statement === "Rendered model answer content" &&
      claim.subject_id === "runtime.answer" &&
      claim.expected_revision === answer.output_sha256 &&
      Array.isArray(claim.prerequisite_claim_ids) &&
      claim.prerequisite_claim_ids.length === 0 &&
      evidenceState.state === "inferred" &&
      Array.isArray(provenance.citations) &&
      provenance.citations.length === outcomeEvidence.length &&
      provenance.citations.every(validEvidenceReference) &&
      JSON.stringify(provenance.citations) ===
        JSON.stringify(outcomeEvidence) &&
      runtime.model_run_id === answer.model_run_id &&
      runtime.response_sha256 === answer.response_sha256 &&
      validIdentifier(manifest.profile_id) &&
      validSha256(manifest.manifest_sha256) &&
      validSha256(manifest.artifact_sha256) &&
      validSha256(manifest.tokenizer_sha256) &&
      validSha256(manifest.template_sha256) &&
      validSha256(manifest.codec_sha256) &&
      validIdentifier(modelRuntime.adapter_id) &&
      typeof modelRuntime.kind === "string" &&
      positiveSafeInteger(modelRuntime.contract_version) &&
      validIdentifier(modelRuntime.runtime_build) &&
      validSha256(modelRuntime.runtime_sha256) &&
      typeof modelRuntime.platform === "string" &&
      typeof modelRuntime.architecture === "string",
  );
  const preimage = {
    ...answer,
    answer_evidence_sha256: "0".repeat(64),
  };
  requireValid(
    createHash("sha256").update(JSON.stringify(preimage)).digest("hex") ===
      answer.answer_evidence_sha256,
  );
}

function parseRuntimeOutput(candidate: unknown): RuntimeOutputEnvelope | null {
  if (candidate === null) {
    return null;
  }
  const record = requiredRecord(candidate);
  if (record.storage === "inline") {
    requireKeys(record, ["payload", "storage"]);
    const payload = requiredRecord(record.payload);
    requireKeys(payload, ["bytes", "media_type", "schema", "sha256"]);
    if (
      !isRecord(payload.schema) ||
      !["text/markdown", "text/plain", "application/json"].includes(
        String(payload.media_type),
      ) ||
      !Array.isArray(payload.bytes) ||
      payload.bytes.length === 0 ||
      payload.bytes.length > MAX_RUNTIME_OUTPUT_BYTES ||
      !payload.bytes.every(
        (byte) => Number.isSafeInteger(byte) && byte >= 0 && byte <= 255,
      ) ||
      !validSha256(payload.sha256)
    ) {
      throw new RuntimeTransportFailure();
    }
    return record as unknown as RuntimeInlineOutput;
  }
  if (record.storage === "artifact") {
    requireKeys(record, ["reference", "storage"]);
    const reference = requiredRecord(record.reference);
    requireKeys(reference, [
      "artifact_id",
      "byte_size",
      "media_type",
      "sha256",
    ]);
    if (
      !validIdentifier(reference.artifact_id) ||
      !validSha256(reference.sha256) ||
      !positiveSafeInteger(reference.byte_size) ||
      !validMediaType(reference.media_type)
    ) {
      throw new RuntimeTransportFailure();
    }
    return record as unknown as RuntimeArtifactOutput;
  }
  throw new RuntimeTransportFailure();
}

function parsePayloadReference(
  candidate: unknown,
): Readonly<Record<string, unknown>> | null {
  if (candidate === null) {
    return null;
  }
  const record = requiredRecord(candidate);
  requireKeys(record, ["artifact_id", "byte_size", "media_type", "sha256"]);
  if (
    !validIdentifier(record.artifact_id) ||
    !validSha256(record.sha256) ||
    !positiveSafeInteger(record.byte_size) ||
    !validMediaType(record.media_type)
  ) {
    throw new RuntimeTransportFailure();
  }
  return record;
}

function validEvidenceReference(candidate: unknown): boolean {
  if (!isRecord(candidate)) {
    return false;
  }
  try {
    requireKeys(candidate, [
      "content_sha256",
      "evidence_id",
      "fragment",
      "kind",
      "object_id",
      "observed_revision",
      "schema_version",
      "source_id",
    ]);
  } catch {
    return false;
  }
  return (
    candidate.schema_version === RUNTIME_CONTRACT_VERSION &&
    validIdentifier(candidate.evidence_id) &&
    typeof candidate.kind === "string" &&
    validText(candidate.source_id, 4_096) &&
    validText(candidate.object_id, 4_096) &&
    (candidate.fragment === null || validText(candidate.fragment, 4_096)) &&
    validSha256(candidate.content_sha256) &&
    (candidate.observed_revision === null ||
      validText(candidate.observed_revision, 4_096))
  );
}

function sanitizeRuntimeMarkdown(value: string): string {
  return value
    .replace(
      /\]\(\s*(?:command|vscode|file|data|javascript):[^)]*\)/giu,
      "](`blocked local link`)",
    )
    .replace(
      /\b(?:command|vscode|file|data|javascript):/giu,
      "blocked-local-link:",
    );
}

function exactKind(
  record: Record<string, unknown>,
  keys: readonly string[],
  values: readonly unknown[],
  validator: (value: unknown) => boolean,
): void {
  requireKeys(record, keys);
  requireValid(values.every(validator));
}

function requireValid(valid: boolean): void {
  if (!valid) {
    throw new RuntimeTransportFailure();
  }
}

function requiredRecord(candidate: unknown): Record<string, unknown> {
  if (!isRecord(candidate)) {
    throw new RuntimeTransportFailure();
  }
  return candidate;
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
    throw new RuntimeTransportFailure();
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
    typeof candidate === "string" &&
    /^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/u.test(candidate)
  );
}

function optionalIdentifier(candidate: unknown): candidate is string | null {
  return candidate === null || validIdentifier(candidate);
}

function validSha256(candidate: unknown): candidate is string {
  return typeof candidate === "string" && /^[0-9a-f]{64}$/u.test(candidate);
}

function validCode(candidate: unknown): candidate is string {
  return (
    typeof candidate === "string" &&
    /^[a-z0-9][a-z0-9_.-]{0,127}$/u.test(candidate)
  );
}

function validOperation(candidate: unknown): candidate is string {
  return typeof candidate === "string" && OPERATIONS.has(candidate);
}

function validMediaType(candidate: unknown): candidate is string {
  return (
    typeof candidate === "string" &&
    /^[A-Za-z0-9][A-Za-z0-9/+.-]{0,127}$/u.test(candidate)
  );
}

function validText(candidate: unknown, maxBytes: number): candidate is string {
  return (
    typeof candidate === "string" &&
    candidate.trim().length > 0 &&
    Buffer.byteLength(candidate, "utf8") <= maxBytes &&
    !candidate.includes("\0")
  );
}

function validTextArray(
  candidate: unknown,
  maxItems: number,
  allowEmpty: boolean,
): candidate is readonly string[] {
  return (
    Array.isArray(candidate) &&
    candidate.length <= maxItems &&
    (allowEmpty || candidate.length > 0) &&
    candidate.every((value) => validText(value, 4_096))
  );
}

function positiveSafeInteger(candidate: unknown): candidate is number {
  return Number.isSafeInteger(candidate) && Number(candidate) > 0;
}

function nonnegativeSafeInteger(candidate: unknown): candidate is number {
  return Number.isSafeInteger(candidate) && Number(candidate) >= 0;
}
