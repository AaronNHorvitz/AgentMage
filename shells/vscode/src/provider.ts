/** Closed controller for the native AgentMage chat provider. */

import {
  parseModelPickerSnapshot,
  parseModelSelectionRevalidation,
  renderModelAcquisitionReview,
  renderModelManagementReport,
  type ModelPickerSnapshot,
} from "./model_discovery.js";
import {
  parseHandoffReview,
  parseLocalHandoffReceipt,
  parseRenderedHandoff,
  type HandoffProhibitedAction,
  type HandoffReview,
  type LocalHandoffReceipt,
  type RenderedHandoff,
} from "./handoff.js";
import {
  parseRuntimeHostResponse,
  parseRuntimeRunRequest,
  renderRuntimeEvent,
  renderRuntimeOutcome,
  runtimeApprovalResponse,
  RuntimeStreamVerifier,
  type RuntimeApprovalChallengeEnvelope,
  type RuntimeApprovalResponseEnvelope,
  type RuntimeEventCursorEnvelope,
  type RuntimeHostResponse,
  type RuntimeRunRequestEnvelope,
  type RuntimeStepResponse,
} from "./runtime_transport.js";

export const PROVIDER_VENDOR = "agentmage" as const;
export const HOST_PROTOCOL_VERSION = 1 as const;

const MAX_PROMPT_BYTES = 4_096;
const MAX_PATH_COMPONENTS = 256;
const MAX_COMPONENT_BYTES = 255;

export interface LocalWorkspace {
  readonly id: string;
  readonly name: string;
  readonly root: string;
}

export interface ReadPreview {
  readonly kind: "read_preview";
  readonly schema_version: 1;
  readonly request_id: string;
  readonly preview_id: string;
  readonly components: readonly string[];
  readonly byte_len: number;
  readonly content_sha256: string;
  readonly expires_at_epoch_ms: number;
  readonly confirmation_sha256: string;
}

export interface ReceiptSummary {
  readonly receipt_id: string;
  readonly sequence: number;
  readonly receipt_sha256: string;
  readonly outcome:
    "succeeded" | "denied" | "failed" | "cancelled" | "timed_out" | "uncertain";
}

export type DiagnosticComponent =
  | "package"
  | "platform"
  | "model"
  | "runtime"
  | "hardware_fit"
  | "offline_boundary"
  | "sandbox_helper"
  | "workspace_grant"
  | "capabilities"
  | "repository_map"
  | "encrypted_store"
  | "receipt_chain"
  | "recovery";

export type DiagnosticState =
  | "healthy"
  | "degraded"
  | "blocked"
  | "unavailable"
  | "quarantined"
  | "unsupported";

export interface DoctorReport {
  readonly schema_version: 2;
  readonly report_kind: "agentmage.local-doctor.v1";
  readonly overall_state: DiagnosticState;
  readonly items: readonly {
    readonly component: DiagnosticComponent;
    readonly state: DiagnosticState;
    readonly reason_code: string;
    readonly remediation_code: string;
    readonly identity_sha256: string | null;
  }[];
  readonly report_sha256: string;
}

export interface DiagnosticExportPreview {
  readonly kind: "diagnostic_export_preview";
  readonly schema_version: 1;
  readonly request_id: string;
  readonly preview_id: string;
  readonly destination_sha256: string;
  readonly payload_sha256: string;
  readonly payload_bytes: number;
  readonly included_fields: readonly string[];
  readonly redactions: readonly string[];
  readonly sensitivity: string;
  readonly retention: string;
  readonly expires_at_epoch_ms: number;
  readonly confirmation_sha256: string;
}

export type HostReadResponse =
  | ReadPreview
  | {
      readonly kind: "read_completed";
      readonly schema_version: 1;
      readonly request_id: string;
      readonly content: string;
      readonly file_uri: string;
      readonly receipt: ReceiptSummary;
    }
  | {
      readonly kind: "cancelled";
      readonly schema_version: 1;
      readonly request_id: string;
    }
  | {
      readonly kind: "denied";
      readonly schema_version: 1;
      readonly request_id: string;
      readonly code: string;
      readonly receipt?: ReceiptSummary;
    };

export type HostResponse =
  | HostReadResponse
  | RuntimeHostResponse
  | {
      readonly kind: "models_discovered";
      readonly schema_version: 1;
      readonly request_id: string;
      readonly snapshot: ModelPickerSnapshot;
    }
  | {
      readonly kind: "model_revalidated";
      readonly schema_version: 1;
      readonly request_id: string;
      readonly revalidation: ReturnType<typeof parseModelSelectionRevalidation>;
    }
  | {
      readonly kind: "doctor_completed";
      readonly schema_version: 1;
      readonly request_id: string;
      readonly report: DoctorReport;
    }
  | DiagnosticExportPreview
  | {
      readonly kind: "diagnostic_export_completed";
      readonly schema_version: 1;
      readonly request_id: string;
      readonly destination_sha256: string;
      readonly payload_sha256: string;
      readonly payload_bytes: number;
      readonly outcome: "succeeded";
    }
  | {
      readonly kind: "handoff_preview";
      readonly schema_version: 1;
      readonly request_id: string;
      readonly review: HandoffReview;
    }
  | {
      readonly kind: "handoff_rendered";
      readonly schema_version: 1;
      readonly request_id: string;
      readonly rendered: RenderedHandoff;
    }
  | {
      readonly kind: "handoff_receipt";
      readonly schema_version: 1;
      readonly request_id: string;
      readonly receipt: LocalHandoffReceipt;
    };

export interface HostBridge {
  previewHandoff(request: {
    readonly kind: "preview_handoff";
    readonly schema_version: 1;
    readonly request_id: string;
  }): Promise<HostResponse>;

  renderHandoff(request: {
    readonly kind: "render_handoff";
    readonly schema_version: 1;
    readonly request_id: string;
    readonly preview_id: string;
    readonly confirmation_sha256: string;
    readonly non_public_acknowledged: boolean;
  }): Promise<HostResponse>;

  cancelHandoff(request: {
    readonly kind: "cancel_handoff";
    readonly schema_version: 1;
    readonly request_id: string;
    readonly preview_id: string;
  }): Promise<HostResponse>;

  denyHandoffAction(request: {
    readonly kind: "deny_handoff_action";
    readonly schema_version: 1;
    readonly request_id: string;
    readonly handoff_id: string | null;
    readonly action: HandoffProhibitedAction;
  }): Promise<HostResponse>;

  discoverModels(request: {
    readonly kind: "discover_models";
    readonly schema_version: 1;
    readonly request_id: string;
  }): Promise<HostResponse>;

  revalidateModel(request: {
    readonly kind: "revalidate_model";
    readonly schema_version: 1;
    readonly request_id: string;
    readonly profile_id: string;
    readonly expected_entry_sha256: string;
  }): Promise<HostResponse>;

  doctor(request: {
    readonly kind: "doctor";
    readonly schema_version: 1;
    readonly request_id: string;
  }): Promise<HostResponse>;

  previewDiagnosticExport(request: {
    readonly kind: "preview_diagnostic_export";
    readonly schema_version: 1;
    readonly request_id: string;
    readonly destination: string;
  }): Promise<HostResponse>;

  approveDiagnosticExport(request: {
    readonly kind: "approve_diagnostic_export";
    readonly schema_version: 1;
    readonly request_id: string;
    readonly preview_id: string;
    readonly confirmation_sha256: string;
  }): Promise<HostResponse>;

  cancelDiagnosticExport(request: {
    readonly kind: "cancel_diagnostic_export";
    readonly schema_version: 1;
    readonly request_id: string;
    readonly preview_id: string;
  }): Promise<HostResponse>;

  previewRead(request: {
    readonly kind: "preview_read";
    readonly schema_version: 1;
    readonly request_id: string;
    readonly workspace_id: string;
    readonly workspace_root: string;
    readonly components: readonly string[];
  }): Promise<HostReadResponse>;

  approveRead(request: {
    readonly kind: "approve_read";
    readonly schema_version: 1;
    readonly request_id: string;
    readonly preview_id: string;
    readonly confirmation_sha256: string;
  }): Promise<HostReadResponse>;

  cancelRead(request: {
    readonly kind: "cancel_read";
    readonly schema_version: 1;
    readonly request_id: string;
    readonly preview_id: string;
  }): Promise<HostReadResponse>;

  prepareRuntime(request: {
    readonly kind: "prepare_runtime";
    readonly schema_version: 1;
    readonly request_id: string;
    readonly profile_id: string;
    readonly expected_entry_sha256: string;
    readonly workspace_id: string;
    readonly workspace_root: string;
    readonly prompt: string;
  }): Promise<HostResponse>;

  startRuntime(request: {
    readonly kind: "start_runtime";
    readonly schema_version: 1;
    readonly request_id: string;
    readonly run_request: RuntimeRunRequestEnvelope;
  }): Promise<HostResponse>;

  advanceRuntime(request: {
    readonly kind: "advance_runtime";
    readonly schema_version: 1;
    readonly request_id: string;
    readonly run_id: string;
    readonly request_sha256: string;
    readonly after_event_cursor: RuntimeEventCursorEnvelope | null;
    readonly approval_response: RuntimeApprovalResponseEnvelope | null;
  }): Promise<HostResponse>;

  cancelRuntime(request: {
    readonly kind: "cancel_runtime";
    readonly schema_version: 1;
    readonly request_id: string;
    readonly run_id: string;
    readonly request_sha256: string;
    readonly cancellation_id: string;
    readonly after_event_cursor: RuntimeEventCursorEnvelope | null;
  }): Promise<HostResponse>;

  releaseRuntime(request: {
    readonly kind: "release_runtime";
    readonly schema_version: 1;
    readonly request_id: string;
    readonly run_id: string;
    readonly request_sha256: string;
  }): Promise<HostResponse>;

  dispose(): void;
}

export interface WorkspaceSource {
  selectedLocalWorkspace(): LocalWorkspace | undefined;
}

export interface ApprovalUi {
  confirmWorkspace(
    workspace: LocalWorkspace,
    components: readonly string[],
  ): Promise<boolean>;
  confirmRead(preview: ReadPreview): Promise<boolean>;
  selectDiagnosticDestination(): Promise<string | undefined>;
  confirmDiagnosticExport(
    preview: DiagnosticExportPreview,
    destination: string,
  ): Promise<boolean>;
  confirmHandoff(review: HandoffReview): Promise<{
    readonly approved: boolean;
    readonly nonPublicAcknowledged: boolean;
  }>;
  confirmRuntimeApproval(
    challenge: RuntimeApprovalChallengeEnvelope,
  ): Promise<"allow" | "deny">;
}

export interface SelectedRuntimeProfile {
  readonly profileId: string;
  readonly expectedEntrySha256: string;
  readonly manifestSha256: string;
  readonly artifactSha256: string;
  readonly runtimeAdapterId: string;
  readonly runtimeSha256: string;
  readonly maxContextTokens: number;
  readonly maxOutputTokens: number;
  readonly toolCalling: boolean;
  readonly visionInput: boolean;
}

interface PendingRuntimeRun {
  readonly requestSha256: string;
  readonly cancellationId: string;
  cursor: RuntimeEventCursorEnvelope | null;
  started: boolean;
  terminal: boolean;
  cancellationSent: boolean;
  requestCancellation: (() => void) | undefined;
  cancellationResponse: Promise<HostResponse> | undefined;
}

interface RuntimeCancellationExchange {
  readonly requestId: string;
  readonly response: Promise<HostResponse>;
}

export interface CancellationSubscription {
  dispose(): void;
}

export interface CancellationSignal {
  readonly isCancellationRequested: boolean;
  onCancellationRequested(listener: () => void): CancellationSubscription;
}

export interface RequestIdentitySource {
  next(): string;
}

export interface ControllerResult {
  /** Ordered semantic response parts for native Chat streaming. */
  readonly parts: readonly string[];
  /** Complete response retained for non-streaming tests and thin clients. */
  readonly text: string;
}

/** Inert bridge used until a verified package injects authenticated IPC. */
export class UnavailableHostBridge implements HostBridge {
  previewHandoff(
    request: Parameters<HostBridge["previewHandoff"]>[0],
  ): Promise<HostResponse> {
    return Promise.resolve(
      denied(request.request_id, "host.connection.unavailable"),
    );
  }

  renderHandoff(
    request: Parameters<HostBridge["renderHandoff"]>[0],
  ): Promise<HostResponse> {
    return Promise.resolve(
      denied(request.request_id, "host.connection.unavailable"),
    );
  }

  cancelHandoff(
    request: Parameters<HostBridge["cancelHandoff"]>[0],
  ): Promise<HostResponse> {
    return Promise.resolve(
      denied(request.request_id, "host.connection.unavailable"),
    );
  }

  denyHandoffAction(
    request: Parameters<HostBridge["denyHandoffAction"]>[0],
  ): Promise<HostResponse> {
    return Promise.resolve(
      denied(request.request_id, "host.connection.unavailable"),
    );
  }

  discoverModels(
    request: Parameters<HostBridge["discoverModels"]>[0],
  ): Promise<HostResponse> {
    return Promise.resolve(
      denied(request.request_id, "host.connection.unavailable"),
    );
  }

  revalidateModel(
    request: Parameters<HostBridge["revalidateModel"]>[0],
  ): Promise<HostResponse> {
    return Promise.resolve(
      denied(request.request_id, "host.connection.unavailable"),
    );
  }

  doctor(request: Parameters<HostBridge["doctor"]>[0]): Promise<HostResponse> {
    return Promise.resolve(
      denied(request.request_id, "host.connection.unavailable"),
    );
  }

  previewDiagnosticExport(
    request: Parameters<HostBridge["previewDiagnosticExport"]>[0],
  ): Promise<HostResponse> {
    return Promise.resolve(
      denied(request.request_id, "host.connection.unavailable"),
    );
  }

  approveDiagnosticExport(
    request: Parameters<HostBridge["approveDiagnosticExport"]>[0],
  ): Promise<HostResponse> {
    return Promise.resolve(
      denied(request.request_id, "host.connection.unavailable"),
    );
  }

  cancelDiagnosticExport(
    request: Parameters<HostBridge["cancelDiagnosticExport"]>[0],
  ): Promise<HostResponse> {
    return Promise.resolve(
      denied(request.request_id, "host.connection.unavailable"),
    );
  }

  previewRead(
    request: Parameters<HostBridge["previewRead"]>[0],
  ): Promise<HostReadResponse> {
    return Promise.resolve(
      denied(request.request_id, "host.connection.unavailable"),
    );
  }

  approveRead(
    request: Parameters<HostBridge["approveRead"]>[0],
  ): Promise<HostReadResponse> {
    return Promise.resolve(
      denied(request.request_id, "host.connection.unavailable"),
    );
  }

  cancelRead(
    request: Parameters<HostBridge["cancelRead"]>[0],
  ): Promise<HostReadResponse> {
    return Promise.resolve(
      denied(request.request_id, "host.connection.unavailable"),
    );
  }

  prepareRuntime(
    request: Parameters<HostBridge["prepareRuntime"]>[0],
  ): Promise<HostResponse> {
    return Promise.resolve(
      denied(request.request_id, "host.connection.unavailable"),
    );
  }

  startRuntime(
    request: Parameters<HostBridge["startRuntime"]>[0],
  ): Promise<HostResponse> {
    return Promise.resolve(
      denied(request.request_id, "host.connection.unavailable"),
    );
  }

  advanceRuntime(
    request: Parameters<HostBridge["advanceRuntime"]>[0],
  ): Promise<HostResponse> {
    return Promise.resolve(
      denied(request.request_id, "host.connection.unavailable"),
    );
  }

  cancelRuntime(
    request: Parameters<HostBridge["cancelRuntime"]>[0],
  ): Promise<HostResponse> {
    return Promise.resolve(
      denied(request.request_id, "host.connection.unavailable"),
    );
  }

  releaseRuntime(
    request: Parameters<HostBridge["releaseRuntime"]>[0],
  ): Promise<HostResponse> {
    return Promise.resolve(
      denied(request.request_id, "host.connection.unavailable"),
    );
  }

  dispose(): void {}
}

/** Monotonic non-authoritative request correlation for one extension process. */
export class SessionRequestIdentitySource implements RequestIdentitySource {
  private sequence = 0;

  next(): string {
    this.sequence += 1;
    return `vscode-request-${this.sequence.toString(16).padStart(8, "0")}`;
  }
}

/** Coordinates one exact user-directed read without filesystem or process access. */
export class SecureReadController {
  private disposed = false;
  private readonly pendingPreviews = new Set<string>();
  private readonly pendingDiagnosticExports = new Set<string>();
  private readonly pendingHandoffs = new Set<string>();
  private readonly pendingRuntimeRuns = new Map<string, PendingRuntimeRun>();

  constructor(
    private readonly host: HostBridge,
    private readonly workspaces: WorkspaceSource,
    private readonly approvals: ApprovalUi,
    private readonly identities: RequestIdentitySource,
  ) {}

  /** Returns only a current digest-verified model snapshot from the host. */
  async discoverModels(
    cancellation: CancellationSignal,
  ): Promise<ModelPickerSnapshot | undefined> {
    if (this.disposed || cancellation.isCancellationRequested) {
      return undefined;
    }
    const requestId = this.identities.next();
    const response = await this.host.discoverModels({
      kind: "discover_models",
      schema_version: HOST_PROTOCOL_VERSION,
      request_id: requestId,
    });
    if (
      this.disposed ||
      cancellation.isCancellationRequested ||
      response.kind !== "models_discovered" ||
      response.schema_version !== HOST_PROTOCOL_VERSION ||
      response.request_id !== requestId
    ) {
      return undefined;
    }
    try {
      return parseModelPickerSnapshot(response.snapshot);
    } catch {
      return undefined;
    }
  }

  /** Revalidates one exact displayed entry and never selects an alternative. */
  async revalidateSelectedModel(
    profileId: string,
    expectedEntrySha256: string,
    cancellation: CancellationSignal,
  ): Promise<ControllerResult | undefined> {
    if (this.disposed || cancellation.isCancellationRequested) {
      return cancelledResult();
    }
    const requestId = this.identities.next();
    const response = await this.host.revalidateModel({
      kind: "revalidate_model",
      schema_version: HOST_PROTOCOL_VERSION,
      request_id: requestId,
      profile_id: profileId,
      expected_entry_sha256: expectedEntrySha256,
    });
    if (
      this.disposed ||
      cancellation.isCancellationRequested ||
      response.kind !== "model_revalidated" ||
      response.schema_version !== HOST_PROTOCOL_VERSION ||
      response.request_id !== requestId
    ) {
      return result(
        "# Request Stopped\n\nAgentMage could not revalidate the selected local model. No model or tool was started.\n\n- Status: unavailable\n- Code: `vscode.model.revalidation-unavailable`",
      );
    }
    let revalidation: ReturnType<typeof parseModelSelectionRevalidation>;
    try {
      revalidation = parseModelSelectionRevalidation(response.revalidation);
    } catch {
      return result(
        "# Request Stopped\n\nAgentMage rejected an invalid model revalidation response. No model or tool was started.\n\n- Status: denied\n- Code: `vscode.model.revalidation-invalid`",
      );
    }
    if (
      revalidation.profile_id !== profileId ||
      revalidation.expected_entry_sha256 !== expectedEntrySha256 ||
      !revalidation.admitted
    ) {
      return result(
        `# Request Stopped\n\nThe selected local model is no longer available in its displayed state. AgentMage did not substitute another model.\n\n- Exact profile: \`${profileId}\`\n- Status: denied\n- Code: \`${revalidation.result_code}\``,
      );
    }
    return undefined;
  }

  async respond(
    prompt: string,
    cancellation: CancellationSignal,
    runtimeProfile?: SelectedRuntimeProfile,
    onPart?: (part: string) => void,
  ): Promise<ControllerResult> {
    if (this.disposed) {
      return deniedResult(
        "vscode.host.deactivated",
        "The local host is no longer active.",
      );
    }
    if (prompt === "models") {
      const snapshot = await this.discoverModels(cancellation);
      return snapshot === undefined
        ? result(
            "# Local Model Profiles\n\nModel discovery is unavailable. No profile is selectable.\n\n- Status: unavailable\n- Code: `vscode.model.discovery-unavailable`",
          )
        : result(renderModelManagementReport(snapshot));
    }
    const modelReviewProfile = parseModelReviewCommand(prompt);
    if (modelReviewProfile !== undefined) {
      const snapshot = await this.discoverModels(cancellation);
      const review = snapshot === undefined
        ? undefined
        : renderModelAcquisitionReview(snapshot, modelReviewProfile);
      return review === undefined
        ? result(
            `# Model Acquisition Review\n\nThe exact requested profile is unavailable. AgentMage did not select or substitute another model.\n\n- Exact profile: \`${modelReviewProfile}\`\n- Status: unavailable\n- Code: \`vscode.model.review-unavailable\``,
          )
        : result(review);
    }
    if (prompt === "doctor") {
      if (cancellation.isCancellationRequested) {
        return cancelledResult();
      }
      const requestId = this.identities.next();
      const response = await this.host.doctor({
        kind: "doctor",
        schema_version: HOST_PROTOCOL_VERSION,
        request_id: requestId,
      });
      if (cancellation.isCancellationRequested) {
        return cancelledResult();
      }
      return renderDoctor(response, requestId);
    }
    if (prompt === "export diagnostics") {
      return this.exportDiagnostics(cancellation);
    }
    if (prompt === "handoff") {
      return this.renderLocalHandoff(cancellation);
    }
    const components = parseReadCommand(prompt);
    if (components === undefined) {
      if (runtimeProfile !== undefined) {
        return this.runNativeChatRuntime(
          prompt,
          runtimeProfile,
          cancellation,
          onPart,
        );
      }
      return deniedResult(
        "vscode.read.command_invalid",
        "The request does not match an available bounded command.",
      );
    }
    const workspace = this.workspaces.selectedLocalWorkspace();
    if (workspace === undefined) {
      return deniedResult(
        "vscode.workspace.unavailable",
        "Select exactly one local file workspace before retrying.",
      );
    }
    if (cancellation.isCancellationRequested) {
      return cancelledResult();
    }
    if (!(await this.approvals.confirmWorkspace(workspace, components))) {
      return deniedResult(
        "vscode.workspace.not_approved",
        "The workspace read was not approved. No file was opened.",
      );
    }

    const previewRequestId = this.identities.next();
    let previewId: string | undefined;
    let cancellationSent = false;
    const subscription = cancellation.onCancellationRequested(() => {
      const selectedPreview = previewId;
      if (selectedPreview !== undefined && !cancellationSent) {
        cancellationSent = true;
        void this.host.cancelRead({
          kind: "cancel_read",
          schema_version: HOST_PROTOCOL_VERSION,
          request_id: this.identities.next(),
          preview_id: selectedPreview,
        });
      }
    });
    try {
      const previewResponse = await this.host.previewRead({
        kind: "preview_read",
        schema_version: HOST_PROTOCOL_VERSION,
        request_id: previewRequestId,
        workspace_id: workspace.id,
        workspace_root: workspace.root,
        components,
      });
      if (previewResponse.kind !== "read_preview") {
        return renderTerminal(previewResponse, previewRequestId);
      }
      if (!validPreview(previewResponse, previewRequestId, components)) {
        return deniedResult(
          "vscode.host.preview_invalid",
          "The host returned an invalid read preview. No operation was approved.",
        );
      }
      previewId = previewResponse.preview_id;
      this.pendingPreviews.add(previewId);
      if (this.disposed || cancellation.isCancellationRequested) {
        await this.cancel(previewId);
        cancellationSent = true;
        return cancelledResult();
      }
      if (!(await this.approvals.confirmRead(previewResponse))) {
        await this.cancel(previewId);
        cancellationSent = true;
        return deniedResult(
          "vscode.read.not_approved",
          "The exact read preview was not approved. No read was started.",
        );
      }
      if (this.disposed || cancellation.isCancellationRequested) {
        await this.cancel(previewId);
        cancellationSent = true;
        return cancelledResult();
      }
      this.pendingPreviews.delete(previewId);
      const approvalRequestId = this.identities.next();
      const completion = await this.host.approveRead({
        kind: "approve_read",
        schema_version: HOST_PROTOCOL_VERSION,
        request_id: approvalRequestId,
        preview_id: previewId,
        confirmation_sha256: previewResponse.confirmation_sha256,
      });
      return renderTerminal(completion, approvalRequestId);
    } finally {
      if (previewId !== undefined) {
        this.pendingPreviews.delete(previewId);
      }
      subscription.dispose();
    }
  }

  private async runNativeChatRuntime(
    prompt: string,
    profile: SelectedRuntimeProfile,
    cancellation: CancellationSignal,
    onPart: ((part: string) => void) | undefined,
  ): Promise<ControllerResult> {
    if (
      !validIdentifier(profile.profileId) ||
      !validSha256(profile.expectedEntrySha256) ||
      prompt.trim().length === 0 ||
      Buffer.byteLength(prompt, "utf8") > MAX_PROMPT_BYTES ||
      prompt.includes("\0")
    ) {
      return deniedResult(
        "vscode.runtime.request_invalid",
        "The selected model or request did not match the closed runtime boundary.",
      );
    }
    const workspace = this.workspaces.selectedLocalWorkspace();
    if (workspace === undefined) {
      return deniedResult(
        "vscode.workspace.unavailable",
        "Select exactly one local file workspace before retrying.",
      );
    }
    if (this.disposed || cancellation.isCancellationRequested) {
      return cancelledResult();
    }

    const prepareRequestId = this.identities.next();
    const preparedResponse = await this.host.prepareRuntime({
      kind: "prepare_runtime",
      schema_version: HOST_PROTOCOL_VERSION,
      request_id: prepareRequestId,
      profile_id: profile.profileId,
      expected_entry_sha256: profile.expectedEntrySha256,
      workspace_id: workspace.id,
      workspace_root: workspace.root,
      prompt,
    });
    let request: RuntimeRunRequestEnvelope;
    try {
      const parsed = parseRuntimeHostResponse(preparedResponse);
      if (
        parsed.kind !== "runtime_prepared" ||
        parsed.request_id !== prepareRequestId
      ) {
        throw new Error("runtime prepare mismatch");
      }
      request = parseRuntimeRunRequest(parsed.run_request);
    } catch {
      return runtimeHostDenied(preparedResponse, prepareRequestId);
    }
    if (
      request.model_profile.profile_id !== profile.profileId ||
      !runtimeRequestMatchesSelection(request, profile) ||
      request.workspace_id !== workspace.id ||
      request.task.objective !== prompt ||
      request.task.session_id !== request.session_id ||
      !["ephemeral_read_only", "controlled_write"].includes(request.mode)
    ) {
      await this.releaseRuntime(request.run_id, request.request_sha256);
      return deniedResult(
        "vscode.runtime.request_substituted",
        "The host-framed runtime request did not match the selected model, workspace, or prompt.",
      );
    }

    const active: PendingRuntimeRun = {
      requestSha256: request.request_sha256,
      cancellationId: `runtime-cancel-${this.identities.next()}`,
      cursor: null,
      started: false,
      terminal: false,
      cancellationSent: false,
      requestCancellation: undefined,
      cancellationResponse: undefined,
    };
    this.pendingRuntimeRuns.set(request.run_id, active);
    const cancellationState: {
      exchange: RuntimeCancellationExchange | undefined;
    } = { exchange: undefined };
    const pendingCancellation = (): RuntimeCancellationExchange | undefined =>
      cancellationState.exchange;
    const requestCancellation = (): void => {
      if (!active.started || active.terminal || active.cancellationSent) {
        return;
      }
      active.cancellationSent = true;
      const requestId = this.identities.next();
      cancellationState.exchange = {
        requestId,
        response: this.host.cancelRuntime({
          kind: "cancel_runtime",
          schema_version: HOST_PROTOCOL_VERSION,
          request_id: requestId,
          run_id: request.run_id,
          request_sha256: request.request_sha256,
          cancellation_id: active.cancellationId,
          after_event_cursor: active.cursor,
        }),
      };
      active.cancellationResponse = cancellationState.exchange.response;
    };
    active.requestCancellation = requestCancellation;
    const subscription =
      cancellation.onCancellationRequested(requestCancellation);
    const verifier = new RuntimeStreamVerifier(request);
    const parts: string[] = [];
    const emit = (part: string): void => {
      parts.push(part);
      onPart?.(part);
    };
    let progressStarted = false;
    const acceptStep = (
      response: HostResponse,
      expectedRequestId: string,
      expectedCancellationId?: string,
    ): RuntimeStepResponse => {
      const parsed = parseRuntimeHostResponse(response);
      if (
        parsed.kind !== "runtime_step" ||
        parsed.request_id !== expectedRequestId
      ) {
        throw new Error("runtime step mismatch");
      }
      const events = verifier.accept(parsed);
      if (
        expectedCancellationId !== undefined &&
        parsed.outcome?.state === "CANCELLED" &&
        (!events.some(
          (event) =>
            event.kind.event === "cancellation_requested" &&
            event.kind.cancellation_id === expectedCancellationId,
        ) ||
          !events.some(
            (event) =>
              event.kind.event === "cancellation_observed" &&
              event.kind.cancellation_id === expectedCancellationId,
          ))
      ) {
        throw new Error("runtime cancellation identity mismatch");
      }
      active.cursor = verifier.cursor();
      if (!progressStarted) {
        progressStarted = true;
        emit(renderRuntimeSessionBoundary(request, profile));
      }
      for (const event of events) {
        emit(renderRuntimeEvent(event));
      }
      if (parsed.outcome !== null) {
        active.terminal = true;
      }
      return parsed;
    };

    try {
      if (this.disposed || cancellation.isCancellationRequested) {
        return cancelledResult();
      }
      active.started = true;
      const startRequestId = this.identities.next();
      const startResponse = await this.host.startRuntime({
        kind: "start_runtime",
        schema_version: HOST_PROTOCOL_VERSION,
        request_id: startRequestId,
        run_request: request,
      });
      if (cancellation.isCancellationRequested) {
        requestCancellation();
      }
      const cancellationAfterStart = pendingCancellation();
      let step =
        cancellationAfterStart === undefined
          ? acceptStep(startResponse, startRequestId)
          : acceptStep(
              await cancellationAfterStart.response,
              cancellationAfterStart.requestId,
              active.cancellationId,
            );

      while (step.outcome === null) {
        const challenge = step.approval;
        if (challenge === null) {
          throw new Error("runtime boundary missing challenge");
        }
        let disposition: "allow" | "deny" | undefined;
        if (!cancellation.isCancellationRequested && !this.disposed) {
          disposition = await this.approvals.confirmRuntimeApproval(challenge);
        }
        if (
          disposition === undefined ||
          cancellation.isCancellationRequested ||
          this.disposed ||
          Date.now() >= challenge.expires_at_epoch_ms
        ) {
          requestCancellation();
        }

        const cancellationAfterDecision = pendingCancellation();
        if (cancellationAfterDecision !== undefined) {
          step = acceptStep(
            await cancellationAfterDecision.response,
            cancellationAfterDecision.requestId,
            active.cancellationId,
          );
          continue;
        }
        const advanceRequestId = this.identities.next();
        const advanceResponse = await this.host.advanceRuntime({
          kind: "advance_runtime",
          schema_version: HOST_PROTOCOL_VERSION,
          request_id: advanceRequestId,
          run_id: request.run_id,
          request_sha256: request.request_sha256,
          after_event_cursor: active.cursor,
          approval_response: runtimeApprovalResponse(
            challenge,
            disposition ?? "deny",
          ),
        });
        if (cancellation.isCancellationRequested) {
          requestCancellation();
        }
        const cancellationAfterAdvance = pendingCancellation();
        step =
          cancellationAfterAdvance === undefined
            ? acceptStep(advanceResponse, advanceRequestId)
            : acceptStep(
                await cancellationAfterAdvance.response,
                cancellationAfterAdvance.requestId,
                active.cancellationId,
              );
      }

      emit(renderRuntimeOutcome(step.outcome));
      return result(...parts);
    } catch {
      const stopped = deniedResult(
        "vscode.runtime.response_invalid",
        "The shared runtime returned an invalid or unavailable boundary. AgentMage stopped the request.",
      );
      for (const part of stopped.parts) {
        emit(part);
      }
      return result(...parts);
    } finally {
      subscription.dispose();
      if (active.started && !active.terminal && !active.cancellationSent) {
        requestCancellation();
      }
      const pending = pendingCancellation();
      if (pending !== undefined) {
        await pending.response.catch(() => undefined);
      }
      await this.releaseRuntime(request.run_id, request.request_sha256);
      this.pendingRuntimeRuns.delete(request.run_id);
    }
  }

  private async releaseRuntime(
    runId: string,
    requestSha256: string,
  ): Promise<void> {
    await this.host
      .releaseRuntime({
        kind: "release_runtime",
        schema_version: HOST_PROTOCOL_VERSION,
        request_id: this.identities.next(),
        run_id: runId,
        request_sha256: requestSha256,
      })
      .catch(() => undefined);
  }

  private async exportDiagnostics(
    cancellation: CancellationSignal,
  ): Promise<ControllerResult> {
    const destination = await this.approvals.selectDiagnosticDestination();
    if (destination === undefined) {
      return result(
        "# Diagnostic Export Cancelled\n\nAgentMage cancelled the diagnostic export. No destination was written.\n\n- Status: cancelled",
      );
    }
    if (this.disposed || cancellation.isCancellationRequested) {
      return cancelledResult();
    }
    const previewRequestId = this.identities.next();
    const response = await this.host.previewDiagnosticExport({
      kind: "preview_diagnostic_export",
      schema_version: HOST_PROTOCOL_VERSION,
      request_id: previewRequestId,
      destination,
    });
    if (
      response.kind !== "diagnostic_export_preview" ||
      !validDiagnosticExportPreview(response, previewRequestId)
    ) {
      return renderExportTerminal(response, previewRequestId);
    }
    const preview = response;
    this.pendingDiagnosticExports.add(preview.preview_id);
    if (this.disposed || cancellation.isCancellationRequested) {
      await this.cancelDiagnosticExport(preview.preview_id);
      return cancelledResult();
    }
    const approved = await this.approvals.confirmDiagnosticExport(
      preview,
      destination,
    );
    if (!approved || this.disposed || cancellation.isCancellationRequested) {
      await this.cancelDiagnosticExport(preview.preview_id);
      return approved
        ? cancelledResult()
        : result(
            "# Diagnostic Export Cancelled\n\nAgentMage cancelled the diagnostic export. No destination was written.\n\n- Status: cancelled",
          );
    }
    this.pendingDiagnosticExports.delete(preview.preview_id);
    const approvalRequestId = this.identities.next();
    const completed = await this.host.approveDiagnosticExport({
      kind: "approve_diagnostic_export",
      schema_version: HOST_PROTOCOL_VERSION,
      request_id: approvalRequestId,
      preview_id: preview.preview_id,
      confirmation_sha256: preview.confirmation_sha256,
    });
    return renderExportTerminal(completed, approvalRequestId);
  }

  private async renderLocalHandoff(
    cancellation: CancellationSignal,
  ): Promise<ControllerResult> {
    if (cancellation.isCancellationRequested) {
      return cancelledResult();
    }
    const previewRequestId = this.identities.next();
    const response = await this.host.previewHandoff({
      kind: "preview_handoff",
      schema_version: HOST_PROTOCOL_VERSION,
      request_id: previewRequestId,
    });
    if (
      response.kind !== "handoff_preview" ||
      response.schema_version !== HOST_PROTOCOL_VERSION ||
      response.request_id !== previewRequestId
    ) {
      return renderHandoffTerminal(response, previewRequestId);
    }
    let review: HandoffReview;
    try {
      review = parseHandoffReview(response.review);
    } catch {
      return deniedResult(
        "vscode.handoff.preview_invalid",
        "The local host returned an invalid handoff review. Nothing was rendered or delivered.",
      );
    }
    if (review.expires_at_ms <= Date.now()) {
      return deniedResult(
        "vscode.handoff.preview_expired",
        "The local handoff review expired. Nothing was rendered or delivered.",
      );
    }
    this.pendingHandoffs.add(review.preview_id);
    if (this.disposed || cancellation.isCancellationRequested) {
      await this.cancelLocalHandoff(review.preview_id);
      return cancelledResult();
    }
    const decision = await this.approvals.confirmHandoff(review);
    if (
      !decision.approved ||
      (review.manifest.acknowledgment_required &&
        !decision.nonPublicAcknowledged) ||
      this.disposed ||
      cancellation.isCancellationRequested
    ) {
      await this.cancelLocalHandoff(review.preview_id);
      return decision.approved && !this.disposed
        ? deniedResult(
            "vscode.handoff.acknowledgment_required",
            "The reviewed packet requires explicit acknowledgement. Nothing was rendered or delivered.",
          )
        : result(
            "# Local Handoff Cancelled\n\nThe reviewed packet was discarded locally. Nothing was rendered or delivered.\n\n- Status: cancelled",
          );
    }
    this.pendingHandoffs.delete(review.preview_id);
    const renderRequestId = this.identities.next();
    const renderedResponse = await this.host.renderHandoff({
      kind: "render_handoff",
      schema_version: HOST_PROTOCOL_VERSION,
      request_id: renderRequestId,
      preview_id: review.preview_id,
      confirmation_sha256: review.confirmation_sha256,
      non_public_acknowledged: decision.nonPublicAcknowledged,
    });
    if (
      renderedResponse.kind !== "handoff_rendered" ||
      renderedResponse.schema_version !== HOST_PROTOCOL_VERSION ||
      renderedResponse.request_id !== renderRequestId
    ) {
      return renderHandoffTerminal(renderedResponse, renderRequestId);
    }
    let rendered: RenderedHandoff;
    try {
      rendered = parseRenderedHandoff(renderedResponse.rendered);
    } catch {
      return deniedResult(
        "vscode.handoff.render_invalid",
        "The local host returned an invalid rendered packet. Nothing was delivered.",
      );
    }
    if (
      rendered.packet_markdown !== review.packet_markdown ||
      rendered.manifest.manifest_sha256 !== review.manifest.manifest_sha256
    ) {
      return deniedResult(
        "vscode.handoff.render_changed",
        "The rendered packet did not match the reviewed bytes. Nothing was delivered.",
      );
    }
    return result(
      rendered.packet_markdown,
      `\n\n---\n\n${review.local_only_notice}\n\n- Status: rendered locally\n- Packet: \`${rendered.manifest.packet_sha256}\`\n- Receipt: \`${rendered.receipt.receipt_sha256}\``,
    );
  }

  private async cancelLocalHandoff(previewId: string): Promise<void> {
    if (!this.pendingHandoffs.delete(previewId)) {
      return;
    }
    await this.host.cancelHandoff({
      kind: "cancel_handoff",
      schema_version: HOST_PROTOCOL_VERSION,
      request_id: this.identities.next(),
      preview_id: previewId,
    });
  }

  private async cancelDiagnosticExport(previewId: string): Promise<void> {
    if (!this.pendingDiagnosticExports.delete(previewId)) {
      return;
    }
    await this.host.cancelDiagnosticExport({
      kind: "cancel_diagnostic_export",
      schema_version: HOST_PROTOCOL_VERSION,
      request_id: this.identities.next(),
      preview_id: previewId,
    });
  }

  /** Cancels every unconsumed preview before closing retained bridge state. */
  async dispose(): Promise<void> {
    if (this.disposed) {
      return;
    }
    this.disposed = true;
    const previews = [...this.pendingPreviews];
    this.pendingPreviews.clear();
    const diagnosticExports = [...this.pendingDiagnosticExports];
    this.pendingDiagnosticExports.clear();
    const handoffs = [...this.pendingHandoffs];
    this.pendingHandoffs.clear();
    const runtimeRuns = [...this.pendingRuntimeRuns.entries()];
    for (const [, active] of runtimeRuns) {
      active.requestCancellation?.();
    }
    await Promise.allSettled([
      ...previews.map((previewId) =>
        this.host.cancelRead({
          kind: "cancel_read",
          schema_version: HOST_PROTOCOL_VERSION,
          request_id: this.identities.next(),
          preview_id: previewId,
        }),
      ),
      ...diagnosticExports.map((previewId) =>
        this.host.cancelDiagnosticExport({
          kind: "cancel_diagnostic_export",
          schema_version: HOST_PROTOCOL_VERSION,
          request_id: this.identities.next(),
          preview_id: previewId,
        }),
      ),
      ...handoffs.map((previewId) =>
        this.host.cancelHandoff({
          kind: "cancel_handoff",
          schema_version: HOST_PROTOCOL_VERSION,
          request_id: this.identities.next(),
          preview_id: previewId,
        }),
      ),
      ...runtimeRuns.map(async ([runId, active]) => {
        await active.cancellationResponse?.catch(() => undefined);
        await this.releaseRuntime(runId, active.requestSha256);
      }),
    ]);
    this.pendingRuntimeRuns.clear();
    this.host.dispose();
  }

  private async cancel(previewId: string): Promise<void> {
    if (!this.pendingPreviews.delete(previewId)) {
      return;
    }
    await this.host.cancelRead({
      kind: "cancel_read",
      schema_version: HOST_PROTOCOL_VERSION,
      request_id: this.identities.next(),
      preview_id: previewId,
    });
  }
}

function runtimeRequestMatchesSelection(
  request: RuntimeRunRequestEnvelope,
  profile: SelectedRuntimeProfile,
): boolean {
  const model = request.model_profile;
  const artifact = nestedRecord(model.artifact);
  const runtime = nestedRecord(model.runtime);
  const context = nestedRecord(model.context);
  return (
    model.manifest_sha256 === profile.manifestSha256 &&
    artifact?.sha256 === profile.artifactSha256 &&
    runtime?.adapter_id === profile.runtimeAdapterId &&
    runtime.runtime_sha256 === profile.runtimeSha256 &&
    context?.max_context_tokens === profile.maxContextTokens
  );
}

function nestedRecord(value: unknown): Record<string, unknown> | undefined {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? value as Record<string, unknown>
    : undefined;
}

function renderRuntimeSessionBoundary(
  request: RuntimeRunRequestEnvelope,
  profile: SelectedRuntimeProfile,
): string {
  return [
    "# AgentMage Runtime",
    "",
    "## Session Boundary",
    "",
    `- Model: \`${profile.profileId}\``,
    `- Manifest: \`${profile.manifestSha256}\``,
    `- Artifact: \`${profile.artifactSha256}\``,
    `- Runtime: \`${profile.runtimeAdapterId}\` (${profile.runtimeSha256})`,
    `- Context: ${profile.maxContextTokens.toLocaleString("en-US")} input tokens; ${profile.maxOutputTokens.toLocaleString("en-US")} output tokens`,
    `- Tool limit: ${profile.toolCalling ? `${request.visible_tools.length.toString()} exact visible tools` : "tool calling unavailable"}`,
    `- Vision limit: ${profile.visionInput ? "declared by the exact profile" : "image input unavailable"}`,
    `- Resource status: bounded by the host-framed run limits (request ${request.request_sha256})`,
    "",
    "## Progress",
    "",
  ].join("\n");
}

function parseModelReviewCommand(prompt: string): string | undefined {
  const match = /^review model ([A-Za-z0-9][A-Za-z0-9._:-]{0,127})$/.exec(prompt);
  return match?.[1];
}

function renderHandoffTerminal(
  response: HostResponse,
  expectedRequestId: string,
): ControllerResult {
  if (
    response.schema_version !== HOST_PROTOCOL_VERSION ||
    response.request_id !== expectedRequestId
  ) {
    return deniedResult(
      "vscode.host.response_invalid",
      "The local host response did not match this handoff request.",
    );
  }
  if (response.kind === "denied") {
    return deniedResult(
      validCode(response.code) ? response.code : "vscode.host.response_invalid",
      "The local host refused the handoff request. Nothing was delivered.",
    );
  }
  if (response.kind === "handoff_receipt") {
    try {
      const receipt = parseLocalHandoffReceipt(response.receipt);
      return result(
        `# Local Handoff ${receipt.outcome === "cancelled" ? "Cancelled" : "Denied"}\n\nNothing was delivered.\n\n- Status: ${receipt.outcome}\n- Code: \`${receipt.result_code}\`\n- Receipt: \`${receipt.receipt_sha256}\``,
      );
    } catch {
      // The generic invalid response below intentionally exposes no parser detail.
    }
  }
  return deniedResult(
    "vscode.host.response_invalid",
    "The local host returned an invalid handoff response. Nothing was delivered.",
  );
}

function runtimeHostDenied(
  response: HostResponse,
  expectedRequestId: string,
): ControllerResult {
  if (
    response.kind === "denied" &&
    response.schema_version === HOST_PROTOCOL_VERSION &&
    response.request_id === expectedRequestId
  ) {
    return deniedResult(
      validCode(response.code)
        ? response.code
        : "vscode.runtime.response_invalid",
      "The local host refused the runtime request. No substitute model or route was used.",
    );
  }
  return deniedResult(
    "vscode.runtime.response_invalid",
    "The local host returned an invalid runtime response. No substitute model or route was used.",
  );
}

function validDiagnosticExportPreview(
  preview: DiagnosticExportPreview,
  requestId: string,
): boolean {
  return (
    preview.schema_version === HOST_PROTOCOL_VERSION &&
    preview.request_id === requestId &&
    validIdentifier(preview.preview_id) &&
    validSha256(preview.destination_sha256) &&
    validSha256(preview.payload_sha256) &&
    Number.isSafeInteger(preview.payload_bytes) &&
    preview.payload_bytes > 0 &&
    preview.included_fields.length > 0 &&
    preview.included_fields.every(validCode) &&
    preview.redactions.length > 0 &&
    preview.redactions.every(validCode) &&
    validCode(preview.sensitivity) &&
    validCode(preview.retention) &&
    Number.isSafeInteger(preview.expires_at_epoch_ms) &&
    validSha256(preview.confirmation_sha256)
  );
}

function renderExportTerminal(
  response: HostResponse,
  expectedRequestId: string,
): ControllerResult {
  if (
    response.schema_version !== HOST_PROTOCOL_VERSION ||
    response.request_id !== expectedRequestId
  ) {
    return deniedResult(
      "vscode.host.response_invalid",
      "The host response did not match this request.",
    );
  }
  if (response.kind === "denied") {
    return deniedResult(
      validCode(response.code) ? response.code : "vscode.host.response_invalid",
      "The local host refused the diagnostic export.",
    );
  }
  if (
    response.kind !== "diagnostic_export_completed" ||
    response.outcome !== "succeeded" ||
    !validSha256(response.destination_sha256) ||
    !validSha256(response.payload_sha256) ||
    !Number.isSafeInteger(response.payload_bytes) ||
    response.payload_bytes <= 0
  ) {
    return deniedResult(
      "vscode.host.response_invalid",
      "The diagnostic export completion was invalid.",
    );
  }
  return result(
    "# Diagnostic Export Complete\n\n",
    `- Status: succeeded\n- Bytes: ${response.payload_bytes.toString()}\n- Payload: \`${response.payload_sha256}\``,
  );
}

function renderDoctor(
  response: HostResponse,
  expectedRequestId: string,
): ControllerResult {
  if (
    response.schema_version !== HOST_PROTOCOL_VERSION ||
    response.request_id !== expectedRequestId
  ) {
    return deniedResult(
      "vscode.host.response_invalid",
      "The host response did not match this status request.",
    );
  }
  if (response.kind === "denied") {
    return deniedResult(
      validCode(response.code) ? response.code : "vscode.host.response_invalid",
      "The local host refused the status request.",
    );
  }
  if (
    response.kind !== "doctor_completed" ||
    !validDoctorReport(response.report)
  ) {
    return deniedResult(
      "vscode.host.response_invalid",
      "The local status report was invalid.",
    );
  }
  const lines = [
    "# AgentMage Local Status",
    "",
    `Overall status: **${stateLabel(response.report.overall_state)}**`,
    "",
    "## Components",
    "",
    ...response.report.items.map(
      (item) =>
        `- **${componentLabel(item.component)}:** ${stateLabel(item.state)} (${item.reason_code}; ${item.remediation_code})`,
    ),
    "",
    "## Evidence",
    "",
    `- Report: \`${response.report.report_sha256}\``,
  ];
  return result(lines.join("\n"));
}

const DIAGNOSTIC_COMPONENTS: readonly DiagnosticComponent[] = [
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
];

const DIAGNOSTIC_STATES: readonly DiagnosticState[] = [
  "healthy",
  "degraded",
  "blocked",
  "unavailable",
  "quarantined",
  "unsupported",
];

function validDoctorReport(report: DoctorReport): boolean {
  return (
    report.schema_version === 2 &&
    report.report_kind === "agentmage.local-doctor.v1" &&
    DIAGNOSTIC_STATES.includes(report.overall_state) &&
    validSha256(report.report_sha256) &&
    report.items.length === DIAGNOSTIC_COMPONENTS.length &&
    report.items.every(
      (item, index) =>
        item.component === DIAGNOSTIC_COMPONENTS[index] &&
        DIAGNOSTIC_STATES.includes(item.state) &&
        validCode(item.reason_code) &&
        validCode(item.remediation_code) &&
        (item.identity_sha256 === null || validSha256(item.identity_sha256)),
    )
  );
}

function componentLabel(component: DiagnosticComponent): string {
  return component
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function stateLabel(state: DiagnosticState): string {
  return state.charAt(0).toUpperCase() + state.slice(1);
}

/** Parses only `read <relative-path>` into already separated path components. */
export function parseReadCommand(
  prompt: string,
): readonly string[] | undefined {
  if (Buffer.byteLength(prompt, "utf8") > MAX_PROMPT_BYTES) {
    return undefined;
  }
  const match = /^read ([^\r\n]+)$/.exec(prompt);
  if (match === null) {
    return undefined;
  }
  const candidate = match[1];
  if (
    candidate === undefined ||
    candidate.startsWith("/") ||
    candidate.startsWith("~") ||
    candidate.includes("\\") ||
    candidate.includes(":") ||
    /%2f|%5c/i.test(candidate)
  ) {
    return undefined;
  }
  const components = candidate.split("/");
  if (
    components.length === 0 ||
    components.length > MAX_PATH_COMPONENTS ||
    components.some(
      (component) =>
        component.length === 0 ||
        component === "." ||
        component === ".." ||
        component.includes("*") ||
        Buffer.byteLength(component, "utf8") > MAX_COMPONENT_BYTES ||
        hasControlCharacter(component),
    )
  ) {
    return undefined;
  }
  return components;
}

function validPreview(
  preview: ReadPreview,
  requestId: string,
  expectedComponents: readonly string[],
): boolean {
  return (
    preview.schema_version === HOST_PROTOCOL_VERSION &&
    preview.request_id === requestId &&
    validIdentifier(preview.preview_id) &&
    sameComponents(preview.components, expectedComponents) &&
    Number.isSafeInteger(preview.byte_len) &&
    preview.byte_len >= 0 &&
    Number.isSafeInteger(preview.expires_at_epoch_ms) &&
    validSha256(preview.content_sha256) &&
    validSha256(preview.confirmation_sha256)
  );
}

function renderTerminal(
  response: HostReadResponse,
  expectedRequestId: string,
): ControllerResult {
  if (
    response.schema_version !== HOST_PROTOCOL_VERSION ||
    response.request_id !== expectedRequestId
  ) {
    return deniedResult(
      "vscode.host.response_invalid",
      "The host response did not match this read request.",
    );
  }
  switch (response.kind) {
    case "read_completed": {
      if (!validReceipt(response.receipt) || !validFileUri(response.file_uri)) {
        return deniedResult(
          "vscode.host.response_invalid",
          "The read result contained an invalid receipt or display link.",
        );
      }
      const content = response.content
        .split("\n")
        .map((line) => `    ${line}`)
        .join("\n");
      return result(
        "# Read Complete\n\n## Content\n\n",
        `${content}\n\n`,
        `## Evidence\n\n- File: [Open validated local file](${response.file_uri})\n- Outcome: succeeded\n- Receipt: \`${response.receipt.receipt_sha256}\``,
      );
    }
    case "denied":
      return deniedResult(
        validCode(response.code)
          ? response.code
          : "vscode.host.response_invalid",
        "The local host refused the read request.",
      );
    case "cancelled":
      return cancelledResult();
    case "read_preview":
      return deniedResult(
        "vscode.host.response_invalid",
        "A preview cannot be accepted as a terminal read result.",
      );
  }
}

function validReceipt(receipt: ReceiptSummary): boolean {
  return (
    validIdentifier(receipt.receipt_id) &&
    Number.isSafeInteger(receipt.sequence) &&
    receipt.sequence > 0 &&
    validSha256(receipt.receipt_sha256) &&
    receipt.outcome === "succeeded"
  );
}

function validIdentifier(value: string): boolean {
  return /^[A-Za-z0-9._:-]{1,128}$/.test(value);
}

function validSha256(value: string): boolean {
  return /^[0-9a-f]{64}$/.test(value);
}

function validCode(value: string): boolean {
  return /^[a-z0-9._-]{1,128}$/.test(value);
}

function validFileUri(value: string): boolean {
  if (
    Buffer.byteLength(value, "ascii") > 8 * 1_024 ||
    !value.startsWith("file:///") ||
    value.length === "file:///".length ||
    !/^[\x20-\x7e]+$/u.test(value) ||
    /[?#]/u.test(value)
  ) {
    return false;
  }
  for (let index = "file://".length; index < value.length; index += 1) {
    const character = value[index];
    if (character === "%") {
      if (!/^[0-9A-Fa-f]{2}$/u.test(value.slice(index + 1, index + 3))) {
        return false;
      }
      index += 2;
      continue;
    }
    if (character === undefined || !/[A-Za-z0-9/._~-]/u.test(character)) {
      return false;
    }
  }
  return true;
}

function sameComponents(
  left: readonly string[],
  right: readonly string[],
): boolean {
  return (
    left.length === right.length &&
    left.every((value, index) => value === right[index])
  );
}

function hasControlCharacter(value: string): boolean {
  for (const character of value) {
    const code = character.codePointAt(0);
    if (code !== undefined && (code < 0x20 || code === 0x7f)) {
      return true;
    }
  }
  return false;
}

function denied(requestId: string, code: string): HostReadResponse {
  return {
    kind: "denied",
    schema_version: HOST_PROTOCOL_VERSION,
    request_id: requestId,
    code,
  };
}

function result(...parts: readonly string[]): ControllerResult {
  return { parts, text: parts.join("") };
}

function deniedResult(code: string, guidance: string): ControllerResult {
  const renderedCode = validCode(code) ? code : "vscode.host.response_invalid";
  return result(
    `# Request Denied\n\n${guidance}\n\n- Status: denied\n- Code: \`${renderedCode}\``,
  );
}

function cancelledResult(): ControllerResult {
  return result(
    "# Request Cancelled\n\n- Status: cancelled\n- Code: `vscode.read.cancelled`",
  );
}
