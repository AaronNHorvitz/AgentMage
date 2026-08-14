/** Closed controller for the Phase 9 secure-read chat provider. */

export const PROVIDER_VENDOR = "agentmage" as const;
export const PROVIDER_MODEL_ID = "secure-local-read" as const;
export const PROVIDER_FAMILY = "agentmage-secure-read" as const;
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
    };

export interface HostBridge {
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
  readonly text: string;
}

/** Inert bridge used until a verified package injects authenticated IPC. */
export class UnavailableHostBridge implements HostBridge {
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

  constructor(
    private readonly host: HostBridge,
    private readonly workspaces: WorkspaceSource,
    private readonly approvals: ApprovalUi,
    private readonly identities: RequestIdentitySource,
  ) {}

  async respond(
    prompt: string,
    cancellation: CancellationSignal,
  ): Promise<ControllerResult> {
    if (this.disposed) {
      return result("AgentMage denied the request: `vscode.host.deactivated`.");
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
    const components = parseReadCommand(prompt);
    if (components === undefined) {
      return result(
        "AgentMage denied the request: `vscode.read.command_invalid`.",
      );
    }
    const workspace = this.workspaces.selectedLocalWorkspace();
    if (workspace === undefined) {
      return result(
        "AgentMage denied the request: `vscode.workspace.unavailable`.",
      );
    }
    if (cancellation.isCancellationRequested) {
      return cancelledResult();
    }
    if (!(await this.approvals.confirmWorkspace(workspace, components))) {
      return result(
        "AgentMage denied the request: `vscode.workspace.not_approved`.",
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
        return result(
          "AgentMage denied the request: `vscode.host.preview_invalid`.",
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
        return result(
          "AgentMage denied the request: `vscode.read.not_approved`.",
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

  private async exportDiagnostics(
    cancellation: CancellationSignal,
  ): Promise<ControllerResult> {
    const destination = await this.approvals.selectDiagnosticDestination();
    if (destination === undefined) {
      return result("AgentMage cancelled the diagnostic export.");
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
        : result("AgentMage cancelled the diagnostic export.");
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
    ]);
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
    return result(
      "AgentMage denied the request: `vscode.host.response_invalid`.",
    );
  }
  if (response.kind === "denied") {
    return result(
      `AgentMage denied the request: \`${validCode(response.code) ? response.code : "vscode.host.response_invalid"}\`.`,
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
    return result(
      "AgentMage denied the request: `vscode.host.response_invalid`.",
    );
  }
  return result(
    `AgentMage wrote the reviewed local diagnostic export (${response.payload_bytes.toString()} bytes). Payload: \`${response.payload_sha256}\`.`,
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
    return result(
      "AgentMage denied the request: `vscode.host.response_invalid`.",
    );
  }
  if (response.kind === "denied") {
    return result(
      `AgentMage denied the request: \`${validCode(response.code) ? response.code : "vscode.host.response_invalid"}\`.`,
    );
  }
  if (
    response.kind !== "doctor_completed" ||
    !validDoctorReport(response.report)
  ) {
    return result(
      "AgentMage denied the request: `vscode.host.response_invalid`.",
    );
  }
  const lines = [
    `AgentMage local status: **${stateLabel(response.report.overall_state)}**`,
    "",
    ...response.report.items.map(
      (item) =>
        `- **${componentLabel(item.component)}:** ${stateLabel(item.state)} (${item.reason_code}; ${item.remediation_code})`,
    ),
    "",
    `Report: \`${response.report.report_sha256}\``,
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
    return result(
      "AgentMage denied the request: `vscode.host.response_invalid`.",
    );
  }
  switch (response.kind) {
    case "read_completed": {
      if (!validReceipt(response.receipt) || !validFileUri(response.file_uri)) {
        return result(
          "AgentMage denied the request: `vscode.host.response_invalid`.",
        );
      }
      const content = response.content
        .split("\n")
        .map((line) => `    ${line}`)
        .join("\n");
      return result(
        `${content}\n\n[Open file](${response.file_uri})\n\nReceipt: \`${response.receipt.receipt_sha256}\``,
      );
    }
    case "denied":
      return result(
        `AgentMage denied the request: \`${validCode(response.code) ? response.code : "vscode.host.response_invalid"}\`.`,
      );
    case "cancelled":
      return cancelledResult();
    case "read_preview":
      return result(
        "AgentMage denied the request: `vscode.host.response_invalid`.",
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
  return value.startsWith("file:///") && !/[\r\n<>]/u.test(value);
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

function result(text: string): ControllerResult {
  return { text };
}

function cancelledResult(): ControllerResult {
  return result("AgentMage cancelled the request: `vscode.read.cancelled`.");
}
