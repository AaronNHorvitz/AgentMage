import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import test from "node:test";

import {
  SecureReadController,
  parseReadCommand,
  type ApprovalUi,
  type CancellationSignal,
  type CancellationSubscription,
  type HostBridge,
  type HostReadResponse,
  type HostResponse,
  type LocalWorkspace,
  type ReadPreview,
  type RequestIdentitySource,
  type WorkspaceSource,
} from "../src/provider.js";
import { LOCAL_HANDOFF_NOTICE } from "../src/handoff.js";
import type {
  RuntimeApprovalResponseEnvelope,
  RuntimeEventCursorEnvelope,
  RuntimeRunRequestEnvelope,
} from "../src/runtime_transport.js";
import { parseRuntimeHostResponse } from "../src/runtime_transport.js";

class Identities implements RequestIdentitySource {
  private nextValue = 0;

  next(): string {
    this.nextValue += 1;
    return `request-${this.nextValue.toString().padStart(4, "0")}`;
  }
}

class Signal implements CancellationSignal {
  isCancellationRequested = false;
  private listener: (() => void) | undefined;

  onCancellationRequested(listener: () => void): CancellationSubscription {
    this.listener = listener;
    return { dispose: () => (this.listener = undefined) };
  }

  cancel(): void {
    this.isCancellationRequested = true;
    this.listener?.();
  }
}

class Workspace implements WorkspaceSource {
  selectedLocalWorkspace(): LocalWorkspace | undefined {
    return { id: "workspace-0001", name: "fixture", root: "/tmp/fixture" };
  }
}

class Approvals implements ApprovalUi {
  workspaceApproved = true;
  readApproved = true;
  readBarrier: Promise<void> | undefined;
  onReadConfirmation: (() => void) | undefined;
  diagnosticDestination: string | undefined = "/tmp/private/doctor.json";
  diagnosticApproved = true;
  handoffApproved = true;
  nonPublicAcknowledged = true;
  runtimeDisposition: "allow" | "deny" = "deny";
  runtimeApprovalCalls = 0;
  onRuntimeConfirmation: (() => void) | undefined;
  runtimeBarrier: Promise<void> | undefined;

  confirmWorkspace(): Promise<boolean> {
    return Promise.resolve(this.workspaceApproved);
  }

  async confirmRead(): Promise<boolean> {
    this.onReadConfirmation?.();
    await this.readBarrier;
    return this.readApproved;
  }

  selectDiagnosticDestination(): Promise<string | undefined> {
    return Promise.resolve(this.diagnosticDestination);
  }

  confirmDiagnosticExport(): Promise<boolean> {
    return Promise.resolve(this.diagnosticApproved);
  }

  confirmHandoff(): Promise<{
    readonly approved: boolean;
    readonly nonPublicAcknowledged: boolean;
  }> {
    return Promise.resolve({
      approved: this.handoffApproved,
      nonPublicAcknowledged: this.nonPublicAcknowledged,
    });
  }

  async confirmRuntimeApproval(): Promise<"allow" | "deny"> {
    this.runtimeApprovalCalls += 1;
    this.onRuntimeConfirmation?.();
    await this.runtimeBarrier;
    return this.runtimeDisposition;
  }
}

class Bridge implements HostBridge {
  previewCalls = 0;
  approvalCalls = 0;
  cancellationCalls = 0;
  previewBarrier: Promise<void> | undefined;
  disposed = false;
  diagnosticPreviewCalls = 0;
  diagnosticApprovalCalls = 0;
  diagnosticCancellationCalls = 0;
  discoveryResponse:
    Awaited<ReturnType<HostBridge["discoverModels"]>> | undefined;
  revalidationResponse:
    Awaited<ReturnType<HostBridge["revalidateModel"]>> | undefined;
  revalidationCalls = 0;
  handoffPreviewCalls = 0;
  handoffRenderCalls = 0;
  handoffCancellationCalls = 0;
  handoffDenialCalls = 0;
  runtimePreparedRequest: RuntimeRunRequestEnvelope | undefined;
  runtimeSteps: Array<(requestId: string) => HostResponse> = [];
  runtimePrepareCalls = 0;
  runtimeStartCalls = 0;
  runtimeAdvanceCalls = 0;
  runtimeCancellationCalls = 0;
  runtimeReleaseCalls = 0;
  lastRuntimeCursor: RuntimeEventCursorEnvelope | null = null;
  lastRuntimeApproval: RuntimeApprovalResponseEnvelope | null = null;
  lastRuntimeCancellationId: string | undefined;
  engineeringResponse:
    Awaited<ReturnType<HostBridge["engineering"]>> | undefined;
  engineeringCalls = 0;
  runtimeCallOrder: string[] = [];
  lastEngineeringSessionId: string | null | undefined;

  engineering(
    request: Parameters<HostBridge["engineering"]>[0],
  ): ReturnType<HostBridge["engineering"]> {
    this.engineeringCalls += 1;
    this.runtimeCallOrder.push("bind");
    return Promise.resolve(
      this.engineeringResponse ?? {
        kind: "denied",
        schema_version: 1,
        request_id: request.request_id,
        code: "engineering.not-used",
      },
    );
  }

  previewHandoff(
    request: Parameters<HostBridge["previewHandoff"]>[0],
  ): ReturnType<HostBridge["previewHandoff"]> {
    this.handoffPreviewCalls += 1;
    return Promise.resolve({
      kind: "handoff_preview",
      schema_version: 1,
      request_id: request.request_id,
      review: reviewedHandoff(),
    });
  }

  renderHandoff(
    request: Parameters<HostBridge["renderHandoff"]>[0],
  ): ReturnType<HostBridge["renderHandoff"]> {
    this.handoffRenderCalls += 1;
    const review = reviewedHandoff();
    return Promise.resolve({
      kind: "handoff_rendered",
      schema_version: 1,
      request_id: request.request_id,
      rendered: {
        schema_version: 2,
        packet_markdown: review.packet_markdown,
        manifest: review.manifest,
        receipt: handoffReceipt("rendered", review.manifest.packet_sha256),
      },
    });
  }

  cancelHandoff(
    request: Parameters<HostBridge["cancelHandoff"]>[0],
  ): ReturnType<HostBridge["cancelHandoff"]> {
    this.handoffCancellationCalls += 1;
    return Promise.resolve({
      kind: "handoff_receipt",
      schema_version: 1,
      request_id: request.request_id,
      receipt: handoffReceipt("cancelled", null),
    });
  }

  denyHandoffAction(
    request: Parameters<HostBridge["denyHandoffAction"]>[0],
  ): ReturnType<HostBridge["denyHandoffAction"]> {
    this.handoffDenialCalls += 1;
    const unsigned = {
      schema_version: 2 as const,
      attempt_id: "attempt-denial-0001",
      handoff_id: request.handoff_id,
      outcome: "denied" as const,
      result_code: "handoff.local.action-denied",
      prohibited_action: request.action,
      packet_sha256: null,
      external_delivery_attempted: false as const,
    };
    return Promise.resolve({
      kind: "handoff_receipt",
      schema_version: 1,
      request_id: request.request_id,
      receipt: { ...unsigned, receipt_sha256: jsonDigest(unsigned) },
    });
  }

  discoverModels(
    request: Parameters<HostBridge["discoverModels"]>[0],
  ): ReturnType<HostBridge["discoverModels"]> {
    return Promise.resolve(
      this.discoveryResponse ?? {
        kind: "denied",
        schema_version: 1,
        request_id: request.request_id,
        code: "host.model-discovery.unavailable",
      },
    );
  }

  revalidateModel(
    request: Parameters<HostBridge["revalidateModel"]>[0],
  ): ReturnType<HostBridge["revalidateModel"]> {
    this.revalidationCalls += 1;
    return Promise.resolve(
      this.revalidationResponse ?? {
        kind: "denied",
        schema_version: 1,
        request_id: request.request_id,
        code: "host.model-discovery.unavailable",
      },
    );
  }

  doctor(
    request: Parameters<HostBridge["doctor"]>[0],
  ): ReturnType<HostBridge["doctor"]> {
    const components = [
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
    return Promise.resolve({
      kind: "doctor_completed",
      schema_version: 1,
      request_id: request.request_id,
      report: {
        schema_version: 2,
        report_kind: "agentmage.local-doctor.v1",
        overall_state: "unavailable",
        items: components.map((component) => ({
          component,
          state: component === "package" ? "healthy" : "unavailable",
          reason_code: "diagnostic.fixture.observed",
          remediation_code: "diagnostic.remediation.none",
          identity_sha256: null,
        })),
        report_sha256: "d".repeat(64),
      },
    });
  }

  previewDiagnosticExport(
    request: Parameters<HostBridge["previewDiagnosticExport"]>[0],
  ): ReturnType<HostBridge["previewDiagnosticExport"]> {
    this.diagnosticPreviewCalls += 1;
    return Promise.resolve({
      kind: "diagnostic_export_preview",
      schema_version: 1,
      request_id: request.request_id,
      preview_id: "diagnostic-export-0001",
      destination_sha256: "a".repeat(64),
      payload_sha256: "b".repeat(64),
      payload_bytes: 1024,
      included_fields: ["component", "state"],
      redactions: ["credentials", "prompts"],
      sensitivity: "content-free-local-diagnostic",
      retention: "user-managed-local-file",
      expires_at_epoch_ms: 1_786_320_060_000,
      confirmation_sha256: "c".repeat(64),
    });
  }

  approveDiagnosticExport(
    request: Parameters<HostBridge["approveDiagnosticExport"]>[0],
  ): ReturnType<HostBridge["approveDiagnosticExport"]> {
    this.diagnosticApprovalCalls += 1;
    return Promise.resolve({
      kind: "diagnostic_export_completed",
      schema_version: 1,
      request_id: request.request_id,
      destination_sha256: "a".repeat(64),
      payload_sha256: "b".repeat(64),
      payload_bytes: 1024,
      outcome: "succeeded",
    });
  }

  cancelDiagnosticExport(
    request: Parameters<HostBridge["cancelDiagnosticExport"]>[0],
  ): ReturnType<HostBridge["cancelDiagnosticExport"]> {
    this.diagnosticCancellationCalls += 1;
    return Promise.resolve({
      kind: "cancelled",
      schema_version: 1,
      request_id: request.request_id,
    });
  }

  async previewRead(
    request: Parameters<HostBridge["previewRead"]>[0],
  ): Promise<HostReadResponse> {
    this.previewCalls += 1;
    await this.previewBarrier;
    return preview(request.request_id, request.components);
  }

  approveRead(
    request: Parameters<HostBridge["approveRead"]>[0],
  ): Promise<HostReadResponse> {
    this.approvalCalls += 1;
    return Promise.resolve({
      kind: "read_completed",
      schema_version: 1,
      request_id: request.request_id,
      content: "const value = 1;",
      file_uri: "file:///tmp/fixture/src/lib.ts",
      receipt: {
        receipt_id: "receipt-0001",
        sequence: 1,
        receipt_sha256: "b".repeat(64),
        outcome: "succeeded",
      },
    });
  }

  cancelRead(
    request: Parameters<HostBridge["cancelRead"]>[0],
  ): Promise<HostReadResponse> {
    this.cancellationCalls += 1;
    return Promise.resolve({
      kind: "cancelled",
      schema_version: 1,
      request_id: request.request_id,
    });
  }

  prepareRuntime(
    request: Parameters<HostBridge["prepareRuntime"]>[0],
  ): ReturnType<HostBridge["prepareRuntime"]> {
    this.runtimePrepareCalls += 1;
    this.runtimeCallOrder.push("prepare");
    this.lastEngineeringSessionId = request.engineering_session_id;
    return Promise.resolve(
      this.runtimePreparedRequest === undefined
        ? runtimeDenied(request.request_id)
        : {
            kind: "runtime_prepared",
            schema_version: 1,
            request_id: request.request_id,
            run_request: this.runtimePreparedRequest,
          },
    );
  }

  startRuntime(
    request: Parameters<HostBridge["startRuntime"]>[0],
  ): ReturnType<HostBridge["startRuntime"]> {
    this.runtimeStartCalls += 1;
    this.runtimeCallOrder.push("start");
    return Promise.resolve(this.nextRuntimeStep(request.request_id));
  }

  advanceRuntime(
    request: Parameters<HostBridge["advanceRuntime"]>[0],
  ): ReturnType<HostBridge["advanceRuntime"]> {
    this.runtimeAdvanceCalls += 1;
    this.lastRuntimeCursor = request.after_event_cursor;
    this.lastRuntimeApproval = request.approval_response;
    return Promise.resolve(this.nextRuntimeStep(request.request_id));
  }

  cancelRuntime(
    request: Parameters<HostBridge["cancelRuntime"]>[0],
  ): ReturnType<HostBridge["cancelRuntime"]> {
    this.runtimeCancellationCalls += 1;
    this.lastRuntimeCursor = request.after_event_cursor;
    this.lastRuntimeCancellationId = request.cancellation_id;
    return Promise.resolve(this.nextRuntimeStep(request.request_id));
  }

  releaseRuntime(
    request: Parameters<HostBridge["releaseRuntime"]>[0],
  ): ReturnType<HostBridge["releaseRuntime"]> {
    this.runtimeReleaseCalls += 1;
    return Promise.resolve({
      kind: "cancelled",
      schema_version: 1,
      request_id: request.request_id,
    });
  }

  private nextRuntimeStep(requestId: string): HostResponse {
    return this.runtimeSteps.shift()?.(requestId) ?? runtimeDenied(requestId);
  }

  dispose(): void {
    this.disposed = true;
  }
}

function runtimeDenied(requestId: string): HostReadResponse {
  return {
    kind: "denied",
    schema_version: 1,
    request_id: requestId,
    code: "host.runtime.run_unavailable",
  };
}

function emptyModelSnapshot() {
  const unsigned = {
    schema_version: 1 as const,
    catalog_sha256: "a".repeat(64),
    catalog_signature_verified: true as const,
    observed_at_ms: 10,
    entries: [],
  };
  return {
    ...unsigned,
    snapshot_sha256: createHash("sha256")
      .update(JSON.stringify(unsigned), "utf8")
      .digest("hex"),
  };
}

function preview(
  requestId: string,
  components: readonly string[],
): ReadPreview {
  return {
    kind: "read_preview",
    schema_version: 1,
    request_id: requestId,
    preview_id: "preview-00112233445566778899aabbccddeeff",
    components,
    byte_len: 16,
    content_sha256: "a".repeat(64),
    expires_at_epoch_ms: 1_786_320_060_000,
    confirmation_sha256: "c".repeat(64),
  };
}

function reviewedHandoff() {
  const packetMarkdown = "# Manual Codex Handoff\n\nExact local fixture.\n";
  const unsignedManifest = {
    schema_version: 2 as const,
    handoff_id: "handoff-0001",
    draft_sha256: "a".repeat(64),
    entry_sha256: ["b".repeat(64)],
    packet_sha256: createHash("sha256")
      .update(packetMarkdown, "utf8")
      .digest("hex"),
    packet_bytes: Buffer.byteLength(packetMarkdown, "utf8"),
    destination: "manual_codex_interface" as const,
    acknowledgment_required: true,
    delivered: false as const,
  };
  const manifest = {
    ...unsignedManifest,
    manifest_sha256: jsonDigest(unsignedManifest),
  };
  const unsignedReview = {
    schema_version: 2 as const,
    preview_id: "handoff-preview-0001",
    packet_markdown: packetMarkdown,
    manifest,
    local_only_notice: LOCAL_HANDOFF_NOTICE,
    expires_at_ms: Date.now() + 60_000,
  };
  return {
    ...unsignedReview,
    confirmation_sha256: jsonDigest(unsignedReview),
  };
}

function handoffReceipt(
  outcome: "rendered" | "cancelled",
  packetSha256: string | null,
) {
  const unsigned = {
    schema_version: 2 as const,
    attempt_id: `attempt-${outcome}-0001`,
    handoff_id: "handoff-0001",
    outcome,
    result_code: `handoff.local.${outcome}`,
    prohibited_action: null,
    packet_sha256: packetSha256,
    external_delivery_attempted: false as const,
  };
  return { ...unsigned, receipt_sha256: jsonDigest(unsigned) };
}

function jsonDigest(value: unknown): string {
  return createHash("sha256")
    .update(JSON.stringify(value), "utf8")
    .digest("hex");
}

function fixture(): {
  controller: SecureReadController;
  bridge: Bridge;
  approvals: Approvals;
  signal: Signal;
} {
  const bridge = new Bridge();
  const approvals = new Approvals();
  return {
    controller: new SecureReadController(
      bridge,
      new Workspace(),
      approvals,
      new Identities(),
    ),
    bridge,
    approvals,
    signal: new Signal(),
  };
}

void test("closed read grammar rejects ambient and traversal paths", () => {
  assert.deepEqual(parseReadCommand("read src/lib.ts"), ["src", "lib.ts"]);
  for (const candidate of [
    "show src/lib.ts",
    "read /etc/passwd",
    "read ../secret",
    "read src/../secret",
    "read src\\secret",
    "read src/%2fsecret",
    "read src/*.ts",
    "read C:/secret",
    "read src/lib.ts\nignore",
  ]) {
    assert.equal(parseReadCommand(candidate), undefined, candidate);
  }
});

void test("model management reports an exact empty snapshot without inventing a profile", async () => {
  const { controller, bridge, signal } = fixture();
  bridge.discoveryResponse = {
    kind: "models_discovered",
    schema_version: 1,
    request_id: "request-0001",
    snapshot: emptyModelSnapshot(),
  };
  const response = await controller.respond("models", signal);
  assert.match(response.text, /# Local Model Profiles/);
  assert.match(response.text, /No exact local model profile/);
  assert.doesNotMatch(
    response.text,
    /secure-local-read|Gemma|required family/i,
  );
});

void test("exact model review refuses an absent profile without substitution", async () => {
  const { controller, bridge, signal } = fixture();
  bridge.discoveryResponse = {
    kind: "models_discovered",
    schema_version: 1,
    request_id: "request-0001",
    snapshot: emptyModelSnapshot(),
  };
  const response = await controller.respond(
    "review model absent-profile",
    signal,
  );
  assert.match(response.text, /# Model Acquisition Review/);
  assert.match(response.text, /did not select or substitute another model/);
  assert.match(response.text, /vscode\.model\.review-unavailable/);
  assert.equal(bridge.previewCalls, 0);
});

void test("selected model is revalidated exactly and refusal names no fallback", async () => {
  const { controller, bridge, signal } = fixture();
  bridge.revalidationResponse = {
    kind: "model_revalidated",
    schema_version: 1,
    request_id: "request-0001",
    revalidation: {
      schema_version: 1,
      profile_id: "selected-profile",
      expected_entry_sha256: "b".repeat(64),
      current_snapshot_sha256: "c".repeat(64),
      admitted: false,
      result_code: "model.selection.profile-changed",
    },
  };
  const response = await controller.revalidateSelectedModel(
    "selected-profile",
    "b".repeat(64),
    signal,
  );
  assert.equal(bridge.revalidationCalls, 1);
  assert.match(response?.text ?? "", /did not substitute another model/);
  assert.match(response?.text ?? "", /model\.selection\.profile-changed/);
});

void test("doctor renders every typed state without workspace approval", async () => {
  const { controller, bridge, approvals, signal } = fixture();
  approvals.workspaceApproved = false;
  const response = await controller.respond("doctor", signal);
  assert.equal(bridge.previewCalls, 0);
  assert.match(response.text, /# AgentMage Local Status/);
  assert.match(response.text, /Overall status: \*\*Unavailable\*\*/);
  assert.match(response.text, /\*\*Package:\*\* Healthy/);
  assert.match(response.text, /\*\*Recovery:\*\* Unavailable/);
  assert.match(response.text, new RegExp("d{64}"));
});

void test("doctor rejects prohibited-source fields without canary disclosure", async () => {
  const fields = [
    "prompt",
    "file_content",
    "credential",
    "private_key",
    "environment_value",
    "absolute_path",
    "hostname",
    "username",
    "device_identifier",
  ] as const;
  for (const [index, field] of fields.entries()) {
    const { controller, bridge, signal } = fixture();
    const original = bridge.doctor.bind(bridge);
    const canary = `AM-S15-CANARY-${index.toString().padStart(2, "0")}-${field}`;
    bridge.doctor = async (request) => {
      const response = await original(request);
      assert.equal(response.kind, "doctor_completed");
      return {
        ...response,
        report: { ...response.report, [field]: canary },
      };
    };
    const response = await controller.respond("doctor", signal);
    assert.match(response.text, /vscode\.host\.response_invalid/);
    assert.doesNotMatch(response.text, new RegExp(canary));
    assert.doesNotMatch(JSON.stringify(response), new RegExp(canary));
  }
});

void test("diagnostic export requires destination preview and one approval", async () => {
  const { controller, bridge, signal } = fixture();
  const response = await controller.respond("export diagnostics", signal);
  assert.equal(bridge.diagnosticPreviewCalls, 1);
  assert.equal(bridge.diagnosticApprovalCalls, 1);
  assert.equal(bridge.diagnosticCancellationCalls, 0);
  assert.match(response.text, /# Diagnostic Export Complete/);
  assert.match(response.text, /- Status: succeeded/);
  assert.match(response.text, new RegExp("b{64}"));
});

void test("cancelled and malformed diagnostic previews never approve", async () => {
  const first = fixture();
  first.approvals.diagnosticApproved = false;
  const cancelled = await first.controller.respond(
    "export diagnostics",
    first.signal,
  );
  assert.match(cancelled.text, /cancelled the diagnostic export/);
  assert.equal(first.bridge.diagnosticApprovalCalls, 0);
  assert.equal(first.bridge.diagnosticCancellationCalls, 1);

  const second = fixture();
  second.bridge.previewDiagnosticExport = (request) =>
    Promise.resolve({
      kind: "diagnostic_export_preview",
      schema_version: 1,
      request_id: request.request_id,
      preview_id: "diagnostic-export-0001",
      destination_sha256: "a".repeat(64),
      payload_sha256: "b".repeat(64),
      payload_bytes: 0,
      included_fields: ["component"],
      redactions: ["credentials"],
      sensitivity: "content-free-local-diagnostic",
      retention: "user-managed-local-file",
      expires_at_epoch_ms: 1_786_320_060_000,
      confirmation_sha256: "c".repeat(64),
    });
  const malformed = await second.controller.respond(
    "export diagnostics",
    second.signal,
  );
  assert.match(malformed.text, /host\.response_invalid/);
  assert.equal(second.bridge.diagnosticApprovalCalls, 0);
});

void test("reviewed handoff renders the exact local packet without delivery", async () => {
  const { controller, bridge, signal } = fixture();
  const response = await controller.respond("handoff", signal);
  assert.equal(bridge.handoffPreviewCalls, 1);
  assert.equal(bridge.handoffRenderCalls, 1);
  assert.equal(bridge.handoffCancellationCalls, 0);
  assert.equal(bridge.handoffDenialCalls, 0);
  assert.match(response.text, /# Manual Codex Handoff/);
  assert.match(response.text, /Exact local fixture/);
  assert.match(response.text, /AgentMage has not contacted Codex/);
  assert.match(response.text, /Status: rendered locally/);
});

void test("handoff cancellation and missing acknowledgement never render", async () => {
  const cancelled = fixture();
  cancelled.approvals.handoffApproved = false;
  const cancelledResult = await cancelled.controller.respond(
    "handoff",
    cancelled.signal,
  );
  assert.equal(cancelled.bridge.handoffRenderCalls, 0);
  assert.equal(cancelled.bridge.handoffCancellationCalls, 1);
  assert.match(cancelledResult.text, /Handoff Cancelled/);

  const unacknowledged = fixture();
  unacknowledged.approvals.nonPublicAcknowledged = false;
  const deniedResult = await unacknowledged.controller.respond(
    "handoff",
    unacknowledged.signal,
  );
  assert.equal(unacknowledged.bridge.handoffRenderCalls, 0);
  assert.equal(unacknowledged.bridge.handoffCancellationCalls, 1);
  assert.match(deniedResult.text, /handoff\.acknowledgment_required/);
});

void test("handoff byte mutation fails closed before unreviewed display", async () => {
  const { controller, bridge, signal } = fixture();
  bridge.renderHandoff = (request) => {
    bridge.handoffRenderCalls += 1;
    const review = reviewedHandoff();
    return Promise.resolve({
      kind: "handoff_rendered",
      schema_version: 1,
      request_id: request.request_id,
      rendered: {
        schema_version: 2,
        packet_markdown: `${review.packet_markdown}unreviewed`,
        manifest: review.manifest,
        receipt: handoffReceipt("rendered", review.manifest.packet_sha256),
      },
    });
  };
  const response = await controller.respond("handoff", signal);
  assert.match(response.text, /handoff\.render_invalid/);
  assert.doesNotMatch(response.text, /unreviewed/);
});

void test("one approved read renders bounded content citation and receipt", async () => {
  const { controller, bridge, signal } = fixture();
  const response = await controller.respond("read src/lib.ts", signal);
  assert.equal(bridge.previewCalls, 1);
  assert.equal(bridge.approvalCalls, 1);
  assert.equal(bridge.cancellationCalls, 0);
  assert.ok(response.text.includes("    const value = 1;"));
  assert.match(
    response.text,
    /\[Open validated local file\]\(file:\/\/\/tmp\/fixture\/src\/lib\.ts\)/,
  );
  assert.ok(response.text.includes(`Receipt: \`${"b".repeat(64)}\``));
  assert.equal(response.parts.length, 3);
  assert.match(response.parts[0] ?? "", /# Read Complete/);
  assert.match(response.parts[2] ?? "", /## Evidence/);
});

void test("unsafe display links never become clickable native Chat output", async () => {
  for (const fileUri of [
    "file:///tmp/fixture/src/lib.ts?command=run",
    "file:///tmp/fixture/src/lib.ts#L1",
    "file:///tmp/fixture/src/%2",
    "file:///tmp/fixture/src/[label](command:run)",
    "file:///tmp/fixture/src/é.ts",
    `file:///${"a".repeat(8 * 1_024)}`,
  ]) {
    const current = fixture();
    current.bridge.approveRead = (request) =>
      Promise.resolve({
        kind: "read_completed",
        schema_version: 1,
        request_id: request.request_id,
        content: "untrusted content",
        file_uri: fileUri,
        receipt: {
          receipt_id: "receipt-0001",
          sequence: 1,
          receipt_sha256: "b".repeat(64),
          outcome: "succeeded",
        },
      });
    const response = await current.controller.respond(
      "read src/lib.ts",
      current.signal,
    );
    assert.match(response.text, /vscode\.host\.response_invalid/);
    assert.doesNotMatch(response.text, /\]\(file:/);
  }
});

void test("workspace and operation denials start no approved worker", async () => {
  const first = fixture();
  first.approvals.workspaceApproved = false;
  const workspaceDenied = await first.controller.respond(
    "read src/lib.ts",
    first.signal,
  );
  assert.equal(first.bridge.previewCalls, 0);
  assert.equal(first.bridge.approvalCalls, 0);
  assert.match(workspaceDenied.text, /workspace\.not_approved/);

  const second = fixture();
  second.approvals.readApproved = false;
  const readDenied = await second.controller.respond(
    "read src/lib.ts",
    second.signal,
  );
  assert.equal(second.bridge.previewCalls, 1);
  assert.equal(second.bridge.approvalCalls, 0);
  assert.equal(second.bridge.cancellationCalls, 1);
  assert.match(readDenied.text, /read\.not_approved/);
});

void test("cancellation after preview cancels the exact pending operation", async () => {
  const { controller, bridge, signal } = fixture();
  let releasePreview: (() => void) | undefined;
  bridge.previewBarrier = new Promise<void>((resolve) => {
    releasePreview = resolve;
  });
  const response = controller.respond("read src/lib.ts", signal);
  signal.cancel();
  releasePreview?.();
  const completed = await response;
  assert.equal(bridge.approvalCalls, 0);
  assert.equal(bridge.cancellationCalls, 1);
  assert.match(completed.text, /read\.cancelled/);
});

void test("mutated preview identity is denied before approval", async () => {
  const { controller, bridge, signal } = fixture();
  bridge.previewRead = (request) =>
    Promise.resolve(preview("different-request", request.components));
  const response = await controller.respond("read src/lib.ts", signal);
  assert.equal(bridge.approvalCalls, 0);
  assert.match(response.text, /host\.preview_invalid/);
});

void test("mismatched terminal request identity is never rendered", async () => {
  const { controller, bridge, signal } = fixture();
  bridge.approveRead = () => {
    bridge.approvalCalls += 1;
    return Promise.resolve({
      kind: "read_completed",
      schema_version: 1,
      request_id: "request-from-another-operation",
      content: "must not render",
      file_uri: "file:///tmp/fixture/src/lib.ts",
      receipt: {
        receipt_id: "receipt-0001",
        sequence: 1,
        receipt_sha256: "b".repeat(64),
        outcome: "succeeded",
      },
    });
  };
  const result = await controller.respond("read src/lib.ts", signal);
  assert.match(result.text, /vscode\.host\.response_invalid/);
  assert.doesNotMatch(result.text, /must not render/);
});

void test("deactivation cancels an unconsumed preview and closes the controller", async () => {
  const { controller, bridge, approvals, signal } = fixture();
  let enterConfirmation: (() => void) | undefined;
  const confirmationEntered = new Promise<void>((resolve) => {
    enterConfirmation = resolve;
  });
  let releaseConfirmation: (() => void) | undefined;
  approvals.readBarrier = new Promise<void>((resolve) => {
    releaseConfirmation = resolve;
  });
  approvals.onReadConfirmation = enterConfirmation;
  const response = controller.respond("read src/lib.ts", signal);
  await confirmationEntered;
  const deactivation = controller.dispose();
  releaseConfirmation?.();
  await deactivation;
  const completed = await response;
  assert.equal(bridge.cancellationCalls, 1);
  assert.equal(bridge.approvalCalls, 0);
  assert.equal(bridge.disposed, true);
  assert.match(completed.text, /read\.cancelled/);
  const afterDeactivation = await controller.respond("read src/lib.ts", signal);
  assert.match(afterDeactivation.text, /host\.deactivated/);
});

void test("native Chat renders one complete shared-runtime stream and outcome", async () => {
  const { controller, bridge, signal } = fixture();
  const prompt = "Inspect the selected workspace";
  bridge.runtimePreparedRequest = runtimeRequest(prompt);
  bridge.runtimeSteps.push((requestId) => completedRuntimeStep(requestId));
  const streamed: string[] = [];

  const response = await controller.respond(
    prompt,
    signal,
    runtimeProfile(),
    (part) => streamed.push(part),
  );

  assert.deepEqual(streamed, response.parts);
  assert.match(response.text, /## Session Boundary/u);
  assert.match(response.text, /Model: `profile-0001`/u);
  assert.match(response.text, /Manifest: `6{64}`/u);
  assert.match(response.text, /Artifact: `7{64}`/u);
  assert.match(response.text, /Runtime: `runtime-adapter-0001` \(8{64}\)/u);
  assert.match(response.text, /8,192 input tokens; 256 output tokens/u);
  assert.match(response.text, /Tool limit: tool calling unavailable/u);
  assert.match(response.text, /Vision limit: image input unavailable/u);
  assert.match(response.text, /Resource status: bounded/u);
  assert.match(response.text, /Verified local result/u);
  assert.match(response.text, /Status: SUCCESS/u);
  assert.equal(bridge.runtimePrepareCalls, 1);
  assert.equal(bridge.runtimeStartCalls, 1);
  assert.equal(bridge.runtimeAdvanceCalls, 0);
  assert.equal(bridge.runtimeCancellationCalls, 0);
  assert.equal(bridge.runtimeReleaseCalls, 1);
  assert.equal(bridge.previewCalls, 0);
});

void test("native Chat accepts an exact host-framed controlled-write run", async () => {
  const { controller, bridge, signal } = fixture();
  const prompt = "Apply one already policy-bound coding change";
  bridge.runtimePreparedRequest = runtimeRequest(prompt, "controlled_write");
  bridge.runtimeSteps.push((requestId) => completedRuntimeStep(requestId));

  const response = await controller.respond(prompt, signal, runtimeProfile());

  assert.match(response.text, /Status: SUCCESS/u);
  assert.equal(bridge.runtimeStartCalls, 1);
  assert.equal(bridge.runtimeReleaseCalls, 1);
  assert.equal(bridge.previewCalls, 0);
});

void test("Verified Agent binds the exact approved Plan before controlled execution", async () => {
  const { controller, bridge, signal } = fixture();
  const plan = "Apply one exact approved Plan";
  const request = runtimeRequest(plan, "controlled_write");
  bridge.runtimePreparedRequest = request;
  bridge.runtimeSteps.push((requestId) => completedRuntimeStep(requestId));
  bridge.revalidationResponse = {
    kind: "model_revalidated",
    schema_version: 1,
    request_id: "request-0001",
    revalidation: {
      schema_version: 1,
      profile_id: "profile-0001",
      expected_entry_sha256: "f".repeat(64),
      current_snapshot_sha256: "b".repeat(64),
      admitted: true,
      result_code: "model.selection.admitted",
    },
  };
  bridge.engineeringResponse = {
    kind: "engineering",
    schema_version: 1,
    request_id: "request-binding-fixture",
    response: {
      result: "runtime_bound",
      binding: {
        session_id: request.session_id,
        run_id: request.run_id,
        request_sha256: request.request_sha256,
      },
    },
  };

  const response = await controller.runApprovedAgentPlan(
    plan,
    request.session_id,
    runtimeProfile(),
    signal,
  );

  assert.match(response.text, /Status: SUCCESS/u);
  assert.deepEqual(bridge.runtimeCallOrder, ["prepare", "bind", "start"]);
  assert.equal(bridge.lastEngineeringSessionId, request.session_id);
  assert.equal(bridge.engineeringCalls, 1);
  assert.equal(bridge.runtimeReleaseCalls, 1);
});

void test("Verified Agent never starts when approved Plan binding is unavailable", async () => {
  const { controller, bridge, signal } = fixture();
  const plan = "Apply one exact approved Plan";
  const request = runtimeRequest(plan, "controlled_write");
  bridge.runtimePreparedRequest = request;
  bridge.revalidationResponse = {
    kind: "model_revalidated",
    schema_version: 1,
    request_id: "request-0001",
    revalidation: {
      schema_version: 1,
      profile_id: "profile-0001",
      expected_entry_sha256: "f".repeat(64),
      current_snapshot_sha256: "b".repeat(64),
      admitted: true,
      result_code: "model.selection.admitted",
    },
  };

  const response = await controller.runApprovedAgentPlan(
    plan,
    request.session_id,
    runtimeProfile(),
    signal,
  );

  assert.match(response.text, /engineering\.not-used/u);
  assert.deepEqual(bridge.runtimeCallOrder, ["prepare", "bind"]);
  assert.equal(bridge.runtimeStartCalls, 0);
  assert.equal(bridge.runtimeReleaseCalls, 1);
});

void test("native Chat rejects a substituted profile workspace or prompt before start", async () => {
  const { controller, bridge, signal } = fixture();
  const prompt = "Inspect the selected workspace";
  bridge.runtimePreparedRequest = {
    ...runtimeRequest(prompt),
    model_profile: { profile_id: "profile-substituted" },
  };

  const response = await controller.respond(prompt, signal, runtimeProfile());

  assert.match(response.text, /runtime\.request_substituted/u);
  assert.equal(bridge.runtimeStartCalls, 0);
  assert.equal(bridge.runtimeCancellationCalls, 0);
  assert.equal(bridge.runtimeReleaseCalls, 1);
});

void test("native Chat rejects every selected session identity mutation before start", async () => {
  const mutations: readonly ((profile: Record<string, unknown>) => void)[] = [
    (profile) => {
      profile.manifest_sha256 = "9".repeat(64);
    },
    (profile) => {
      (profile.artifact as Record<string, unknown>).sha256 = "9".repeat(64);
    },
    (profile) => {
      (profile.runtime as Record<string, unknown>).adapter_id =
        "substituted-adapter";
    },
    (profile) => {
      (profile.runtime as Record<string, unknown>).runtime_sha256 = "9".repeat(
        64,
      );
    },
    (profile) => {
      (profile.context as Record<string, unknown>).max_context_tokens = 4096;
    },
  ];
  for (const mutate of mutations) {
    const { controller, bridge, signal } = fixture();
    const request = runtimeRequest("Inspect the selected workspace");
    mutate(request.model_profile);
    bridge.runtimePreparedRequest = request;
    const response = await controller.respond(
      "Inspect the selected workspace",
      signal,
      runtimeProfile(),
    );
    assert.match(response.text, /runtime\.request_substituted/u);
    assert.equal(bridge.runtimeStartCalls, 0);
    assert.equal(bridge.runtimeReleaseCalls, 1);
  }
});

void test("native Chat relays one exact protected denial before terminal output", async () => {
  const { controller, bridge, approvals, signal } = fixture();
  const prompt = "Review the selected repository";
  bridge.runtimePreparedRequest = runtimeRequest(prompt);
  bridge.runtimeSteps.push(
    (requestId) => approvalRuntimeStep(requestId),
    (requestId) => declinedRuntimeStep(requestId),
  );
  approvals.runtimeDisposition = "deny";

  const response = await controller.respond(prompt, signal, runtimeProfile());

  assert.match(response.text, /Approval required/u);
  assert.match(response.text, /Status: DECLINED/u);
  assert.equal(approvals.runtimeApprovalCalls, 1);
  assert.equal(bridge.runtimeAdvanceCalls, 1);
  assert.deepEqual(bridge.lastRuntimeCursor, {
    run_id: "run-0001",
    event_id: "event-0001",
    sequence: 1,
    event_sha256: "b".repeat(64),
  });
  assert.deepEqual(bridge.lastRuntimeApproval, {
    schema_version: 2,
    run_id: "run-0001",
    approval_id: "approval-0001",
    disposition: "deny",
    challenge_sha256: "8".repeat(64),
    grant_id: null,
  });
  assert.equal(bridge.runtimeReleaseCalls, 1);
});

void test("native Chat cancellation uses the exact accepted cursor and starts no approval", async () => {
  const { controller, bridge, approvals, signal } = fixture();
  const prompt = "Review the selected repository";
  bridge.runtimePreparedRequest = runtimeRequest(prompt);
  bridge.runtimeSteps.push(
    (requestId) => approvalRuntimeStep(requestId),
    (requestId) =>
      cancelledRuntimeStep(
        requestId,
        bridge.lastRuntimeCancellationId ?? "missing-cancellation",
      ),
  );
  approvals.onRuntimeConfirmation = () => signal.cancel();

  const response = await controller.respond(prompt, signal, runtimeProfile());

  assert.match(response.text, /Cancellation completed/u);
  assert.match(response.text, /Status: CANCELLED/u);
  assert.equal(bridge.runtimeAdvanceCalls, 0);
  assert.equal(bridge.runtimeCancellationCalls, 1);
  assert.equal(bridge.lastRuntimeCursor?.sequence, 1);
  assert.match(bridge.lastRuntimeCancellationId ?? "", /^runtime-cancel-/u);
  assert.equal(bridge.runtimeReleaseCalls, 1);
});

void test("deactivation cancels and releases an active native Chat run before IPC closes", async () => {
  const { controller, bridge, approvals, signal } = fixture();
  const prompt = "Review the selected repository";
  bridge.runtimePreparedRequest = runtimeRequest(prompt);
  bridge.runtimeSteps.push(
    (requestId) => approvalRuntimeStep(requestId),
    (requestId) =>
      cancelledRuntimeStep(
        requestId,
        bridge.lastRuntimeCancellationId ?? "missing-cancellation",
      ),
  );
  let approvalEntered: (() => void) | undefined;
  const entered = new Promise<void>((resolve) => {
    approvalEntered = resolve;
  });
  let releaseApproval: (() => void) | undefined;
  approvals.runtimeBarrier = new Promise<void>((resolve) => {
    releaseApproval = resolve;
  });
  approvals.onRuntimeConfirmation = approvalEntered;

  const response = controller.respond(prompt, signal, runtimeProfile());
  await entered;
  const deactivation = controller.dispose();
  releaseApproval?.();
  const [completed] = await Promise.all([response, deactivation]);

  assert.match(completed.text, /Status: CANCELLED/u);
  assert.equal(bridge.runtimeCancellationCalls, 1);
  assert.ok(bridge.runtimeReleaseCalls >= 1);
  assert.equal(bridge.disposed, true);
});

function runtimeProfile(): {
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
} {
  return {
    profileId: "profile-0001",
    expectedEntrySha256: "f".repeat(64),
    manifestSha256: "6".repeat(64),
    artifactSha256: "7".repeat(64),
    runtimeAdapterId: "runtime-adapter-0001",
    runtimeSha256: "8".repeat(64),
    maxContextTokens: 8192,
    maxOutputTokens: 256,
    toolCalling: false,
    visionInput: false,
  };
}

function runtimeRequest(
  prompt: string,
  mode: RuntimeRunRequestEnvelope["mode"] = "ephemeral_read_only",
): RuntimeRunRequestEnvelope {
  return {
    schema_version: 2,
    run_id: "run-0001",
    session_id: "session-0001",
    mode,
    task: {
      schema_version: 2,
      task_id: "task-0001",
      session_id: "session-0001",
      objective: prompt,
      acceptance_criteria: ["Report grounded findings"],
      constraints: ["Remain read only"],
      status: "ready",
    },
    work_packet: {},
    workspace_id: "workspace-0001",
    workspace_snapshot_sha256: "1".repeat(64),
    repository_snapshot_id: "repository-snapshot-0001",
    repository_snapshot_sha256: "2".repeat(64),
    model_profile: {
      profile_id: "profile-0001",
      manifest_sha256: "6".repeat(64),
      artifact: { sha256: "7".repeat(64) },
      runtime: {
        adapter_id: "runtime-adapter-0001",
        runtime_sha256: "8".repeat(64),
      },
      context: { max_context_tokens: 8192 },
    },
    context_budget: {},
    tool_catalog_id: "tool-catalog-0001",
    tool_catalog_sha256: "3".repeat(64),
    visible_tools: [],
    policy_id: "policy-0001",
    policy_sha256: "4".repeat(64),
    limits: {},
    event_cursor: null,
    request_sha256: "5".repeat(64),
  };
}

function completedRuntimeStep(requestId: string): HostResponse {
  const text = "Verified local result.\n";
  const bytes = [...Buffer.from(text, "utf8")];
  const outputSha256 = createHash("sha256")
    .update(Uint8Array.from(bytes))
    .digest("hex");
  const evidence = runtimeEvidence();
  return runtimeStep(requestId, completedEvents(), null, {
    ...runtimeOutcome("SUCCESS", "event-0002", "c".repeat(64)),
    evidence: [evidence],
    output: {
      storage: "inline",
      payload: {
        schema: {
          schema_id: "runtime-output",
          schema_version: 2,
          schema_sha256: "9".repeat(64),
        },
        media_type: "text/markdown",
        bytes,
        sha256: outputSha256,
      },
    },
    answer_evidence: runtimeAnswerEvidence(
      outputSha256,
      bytes.length,
      "text/markdown",
      evidence,
    ),
  });
}

function approvalRuntimeStep(requestId: string): HostResponse {
  const expiresAt = Date.now() + 60_000;
  return runtimeStep(
    requestId,
    [
      runtimeEvent(0, "0".repeat(64), null, {
        event: "run_started",
        request_sha256: "5".repeat(64),
      }),
      runtimeEvent(1, "a".repeat(64), "event-0000", {
        event: "permission_requested",
        approval_id: "approval-0001",
        operation: "workspace_read",
        preview_sha256: "6".repeat(64),
        expires_at_epoch_ms: expiresAt,
      }),
    ],
    {
      schema_version: 2,
      run_id: "run-0001",
      task_id: "task-0001",
      turn_id: "turn-0001",
      operation_id: "operation-0001",
      tool_call_id: "tool-call-0001",
      approval_id: "approval-0001",
      proposed_grant_id: "grant-0001",
      operation: "workspace_read",
      preview_sha256: "6".repeat(64),
      expires_at_epoch_ms: expiresAt,
      challenge_sha256: "8".repeat(64),
    },
    null,
  );
}

function declinedRuntimeStep(requestId: string): HostResponse {
  return runtimeStep(
    requestId,
    [
      runtimeEvent(2, "b".repeat(64), "event-0001", {
        event: "permission_decided",
        approval_id: "approval-0001",
        disposition: "DENY",
        grant_id: null,
        decision_sha256: "8".repeat(64),
      }),
      runtimeEvent(3, "c".repeat(64), "event-0002", {
        event: "turn_completed",
        outcome_sha256: "9".repeat(64),
      }),
      runtimeEvent(4, "d".repeat(64), "event-0003", {
        event: "run_terminal",
        state: "DECLINED",
        outcome_sha256: "e".repeat(64),
      }),
    ],
    null,
    runtimeOutcome("DECLINED", "event-0003", "d".repeat(64)),
  );
}

function cancelledRuntimeStep(
  requestId: string,
  cancellationId: string,
): HostResponse {
  return runtimeStep(
    requestId,
    [
      runtimeEvent(2, "b".repeat(64), "event-0001", {
        event: "cancellation_requested",
        cancellation_id: cancellationId,
      }),
      runtimeEvent(3, "c".repeat(64), "event-0002", {
        event: "cancellation_observed",
        cancellation_id: cancellationId,
      }),
      runtimeEvent(4, "d".repeat(64), "event-0003", {
        event: "run_terminal",
        state: "CANCELLED",
        outcome_sha256: "e".repeat(64),
      }),
    ],
    null,
    runtimeOutcome("CANCELLED", "event-0003", "d".repeat(64)),
  );
}

function runtimeStep(
  requestId: string,
  events: readonly Record<string, unknown>[],
  approval: Record<string, unknown> | null,
  outcome: Record<string, unknown> | null,
): HostResponse {
  return parseRuntimeHostResponse({
    kind: "runtime_step",
    schema_version: 1,
    request_id: requestId,
    run_id: "run-0001",
    request_sha256: "5".repeat(64),
    events,
    artifacts: [],
    approval,
    outcome,
  });
}

function completedEvents(): readonly Record<string, unknown>[] {
  return [
    runtimeEvent(0, "0".repeat(64), null, {
      event: "run_started",
      request_sha256: "5".repeat(64),
    }),
    runtimeEvent(1, "a".repeat(64), "event-0000", {
      event: "turn_started",
    }),
    runtimeEvent(2, "b".repeat(64), "event-0001", {
      event: "turn_completed",
      outcome_sha256: "9".repeat(64),
    }),
    runtimeEvent(3, "c".repeat(64), "event-0002", {
      event: "run_terminal",
      state: "SUCCESS",
      outcome_sha256: "e".repeat(64),
    }),
  ];
}

function runtimeEvent(
  sequence: number,
  previousEventSha256: string,
  causationEventId: string | null,
  kind: Record<string, unknown>,
): Record<string, unknown> {
  return {
    schema_version: 2,
    event_id: `event-${sequence.toString().padStart(4, "0")}`,
    run_id: "run-0001",
    session_id: "session-0001",
    task_id: "task-0001",
    turn_id:
      sequence === 0 || kind.event === "run_terminal" ? null : "turn-0001",
    operation_id:
      kind.event === "permission_requested" ||
      kind.event === "permission_decided"
        ? "operation-0001"
        : null,
    correlation_id: "correlation-0001",
    causation_event_id: causationEventId,
    sequence,
    occurred_at_epoch_ms: 1_000 + sequence,
    sensitivity: "internal",
    retention: { kind: "ephemeral", expires_at_epoch_ms: null },
    persistence: "correctness",
    policy_id: "policy-0001",
    payload_reference: null,
    kind,
    previous_event_sha256: previousEventSha256,
    event_sha256: String.fromCharCode("a".charCodeAt(0) + sequence).repeat(64),
  };
}

function runtimeOutcome(
  state: "SUCCESS" | "DECLINED" | "CANCELLED",
  priorEventId: string,
  priorEventSha256: string,
): Record<string, unknown> {
  return {
    schema_version: 2,
    run_id: "run-0001",
    session_id: "session-0001",
    task_id: "task-0001",
    request_sha256: "5".repeat(64),
    state,
    turn_count: 1,
    model_call_count: 1,
    tool_call_count: 0,
    prior_event_id: priorEventId,
    prior_event_sha256: priorEventSha256,
    evidence: [],
    receipt_ids: [],
    unresolved_codes: [],
    output: null,
    answer_evidence: null,
    outcome_sha256: "e".repeat(64),
  };
}

function runtimeEvidence(): Record<string, unknown> {
  return {
    schema_version: 2,
    evidence_id: "evidence-0001",
    kind: "validation",
    source_id: "fixture",
    object_id: "runtime output",
    fragment: null,
    content_sha256: "d".repeat(64),
    observed_revision: "fixture-revision",
  };
}

function runtimeAnswerEvidence(
  outputSha256: string,
  outputByteSize: number,
  outputMediaType: string,
  evidence: Record<string, unknown>,
): Record<string, unknown> {
  const answer: Record<string, unknown> = {
    schema_version: 2,
    task_id: "task-0001",
    model_run_id: "model-run-0001",
    response_sha256: "f".repeat(64),
    output_sha256: outputSha256,
    output_byte_size: outputByteSize,
    output_media_type: outputMediaType,
    rendered_claim_ids: ["runtime.answer.content"],
    assignments: [
      {
        schema_version: 2,
        assignment_id: "runtime.answer.assignment",
        claim: {
          schema_version: 2,
          claim_id: "runtime.answer.content",
          task_id: "task-0001",
          kind: "read",
          statement: "Rendered model answer content",
          subject_id: "runtime.answer",
          expected_revision: outputSha256,
          prerequisite_claim_ids: [],
        },
        evidence_state: {
          state: "inferred",
          provenance: {
            citations: [evidence],
            runtime: {
              model_run_id: "model-run-0001",
              manifest: {
                profile_id: "profile-0001",
                manifest_sha256: "a".repeat(64),
                artifact_sha256: "b".repeat(64),
                tokenizer_sha256: "c".repeat(64),
                template_sha256: "d".repeat(64),
                codec_sha256: "e".repeat(64),
                runtime: {
                  adapter_id: "adapter-0001",
                  kind: "deterministic_fake",
                  contract_version: 1,
                  runtime_build: "runtime-1",
                  runtime_sha256: "6".repeat(64),
                  platform: "deterministic_fake",
                  architecture: "x86_64",
                },
              },
              response_sha256: "f".repeat(64),
            },
          },
        },
      },
    ],
    answer_evidence_sha256: "0".repeat(64),
  };
  answer.answer_evidence_sha256 = createHash("sha256")
    .update(JSON.stringify(answer))
    .digest("hex");
  return answer;
}
