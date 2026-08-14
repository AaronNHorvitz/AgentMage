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
  type LocalWorkspace,
  type ReadPreview,
  type RequestIdentitySource,
  type WorkspaceSource,
} from "../src/provider.js";
import { LOCAL_HANDOFF_NOTICE } from "../src/handoff.js";

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

  dispose(): void {
    this.disposed = true;
  }
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
