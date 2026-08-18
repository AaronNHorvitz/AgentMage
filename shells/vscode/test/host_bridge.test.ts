import assert from "node:assert/strict";
import { createHash, createHmac } from "node:crypto";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createServer, type Socket } from "node:net";
import test from "node:test";

import {
  AuthenticatedLinuxHostBridge,
  type LinuxHostLaunchCredentials,
} from "../src/host_bridge.js";
import { LOCAL_HANDOFF_NOTICE } from "../src/handoff.js";

const DOMAIN = Buffer.from("agentmage-linux-ipc-auth-v1\0", "utf8");

void test("authenticated bridge sends the exact handshake and bounded request frame", async () => {
  const fixture = await socketFixture({
    kind: "read_preview",
    schema_version: 1,
    request_id: "request-0001",
    preview_id: "preview-00112233445566778899aabbccddeeff",
    components: ["src", "lib.ts"],
    byte_len: 7,
    content_sha256: "a".repeat(64),
    expires_at_epoch_ms: 1_786_320_060_000,
    confirmation_sha256: "b".repeat(64),
  });
  try {
    const bridge = new AuthenticatedLinuxHostBridge(fixture.credentials);
    assert.ok(fixture.credentials.launchSecret.every((byte) => byte === 0));
    const response = await bridge.previewRead({
      kind: "preview_read",
      schema_version: 1,
      request_id: "request-0001",
      workspace_id: "workspace-0001",
      workspace_root: "/tmp/workspace",
      components: ["src", "lib.ts"],
    });
    assert.equal(response.kind, "read_preview");
    assert.deepEqual(fixture.observedRequest, {
      kind: "preview_read",
      schema_version: 1,
      request_id: "request-0001",
      workspace_id: "workspace-0001",
      workspace_root: "/tmp/workspace",
      components: ["src", "lib.ts"],
    });
    bridge.dispose();
  } finally {
    await fixture.close();
  }
});

void test("unknown host response fields fail closed without exposing parser detail", async () => {
  const fixture = await socketFixture({
    kind: "denied",
    schema_version: 1,
    request_id: "request-0001",
    code: "host.read.denied",
    unexpected: "must fail",
  });
  try {
    const bridge = new AuthenticatedLinuxHostBridge(fixture.credentials);
    const response = await bridge.previewRead({
      kind: "preview_read",
      schema_version: 1,
      request_id: "request-0001",
      workspace_id: "workspace-0001",
      workspace_root: "/tmp/workspace",
      components: ["src", "lib.ts"],
    });
    assert.deepEqual(response, {
      kind: "denied",
      schema_version: 1,
      request_id: "request-0001",
      code: "host.connection.failed",
    });
    bridge.dispose();
  } finally {
    await fixture.close();
  }
});

void test("authenticated bridge validates the closed diagnostic export preview", async () => {
  const fixture = await socketFixture({
    kind: "diagnostic_export_preview",
    schema_version: 1,
    request_id: "request-export-0001",
    preview_id: "diagnostic-export-0001",
    destination_sha256: "a".repeat(64),
    payload_sha256: "b".repeat(64),
    payload_bytes: 512,
    included_fields: ["component", "state"],
    redactions: ["credentials", "prompts"],
    sensitivity: "content-free-local-diagnostic",
    retention: "user-managed-local-file",
    expires_at_epoch_ms: 1_786_320_060_000,
    confirmation_sha256: "c".repeat(64),
  });
  try {
    const bridge = new AuthenticatedLinuxHostBridge(fixture.credentials);
    const response = await bridge.previewDiagnosticExport({
      kind: "preview_diagnostic_export",
      schema_version: 1,
      request_id: "request-export-0001",
      destination: "/var/home/user/private/doctor.json",
    });
    assert.equal(response.kind, "diagnostic_export_preview");
    assert.equal(fixture.observedRequest.kind, "preview_diagnostic_export");
    bridge.dispose();
  } finally {
    await fixture.close();
  }
});

void test("authenticated doctor transport rejects every prohibited source field", async () => {
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
  ];
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
  ];
  for (const [index, field] of fields.entries()) {
    const canary = `AM-S15-CANARY-${index.toString().padStart(2, "0")}-${field}`;
    const fixture = await socketFixture({
      kind: "doctor_completed",
      schema_version: 1,
      request_id: "request-doctor-0001",
      report: {
        schema_version: 2,
        report_kind: "agentmage.local-doctor.v1",
        overall_state: "unavailable",
        items: components.map((component) => ({
          component,
          state: "unavailable",
          reason_code: "diagnostic.fixture.unavailable",
          remediation_code: "diagnostic.remediation.none",
          identity_sha256: null,
        })),
        report_sha256: "d".repeat(64),
        [field]: canary,
      },
    });
    try {
      const bridge = new AuthenticatedLinuxHostBridge(fixture.credentials);
      const response = await bridge.doctor({
        kind: "doctor",
        schema_version: 1,
        request_id: "request-doctor-0001",
      });
      assert.deepEqual(response, {
        kind: "denied",
        schema_version: 1,
        request_id: "request-doctor-0001",
        code: "host.connection.failed",
      });
      assert.doesNotMatch(JSON.stringify(response), new RegExp(canary));
      bridge.dispose();
    } finally {
      await fixture.close();
    }
  }
});

void test("authenticated bridge verifies model snapshot digests before display", async () => {
  const unsigned = {
    schema_version: 1,
    catalog_sha256: "a".repeat(64),
    catalog_signature_verified: true,
    observed_at_ms: 10,
    entries: [],
  };
  const fixture = await socketFixture({
    kind: "models_discovered",
    schema_version: 1,
    request_id: "request-models-0001",
    snapshot: {
      ...unsigned,
      snapshot_sha256: createHash("sha256")
        .update(JSON.stringify(unsigned), "utf8")
        .digest("hex"),
    },
  });
  try {
    const bridge = new AuthenticatedLinuxHostBridge(fixture.credentials);
    const response = await bridge.discoverModels({
      kind: "discover_models",
      schema_version: 1,
      request_id: "request-models-0001",
    });
    assert.equal(response.kind, "models_discovered");
    assert.deepEqual(fixture.observedRequest, {
      kind: "discover_models",
      schema_version: 1,
      request_id: "request-models-0001",
    });
    bridge.dispose();
  } finally {
    await fixture.close();
  }
});

void test("tampered authenticated model snapshot fails closed", async () => {
  const fixture = await socketFixture({
    kind: "models_discovered",
    schema_version: 1,
    request_id: "request-models-0001",
    snapshot: {
      schema_version: 1,
      catalog_sha256: "a".repeat(64),
      catalog_signature_verified: true,
      observed_at_ms: 10,
      entries: [],
      snapshot_sha256: "b".repeat(64),
    },
  });
  try {
    const bridge = new AuthenticatedLinuxHostBridge(fixture.credentials);
    const response = await bridge.discoverModels({
      kind: "discover_models",
      schema_version: 1,
      request_id: "request-models-0001",
    });
    assert.deepEqual(response, {
      kind: "denied",
      schema_version: 1,
      request_id: "request-models-0001",
      code: "host.connection.failed",
    });
    bridge.dispose();
  } finally {
    await fixture.close();
  }
});

void test("authenticated bridge transports one exact local handoff review", async () => {
  const fixture = await socketFixture({
    kind: "handoff_preview",
    schema_version: 1,
    request_id: "request-handoff-0001",
    review: handoffReview(),
  });
  try {
    const bridge = new AuthenticatedLinuxHostBridge(fixture.credentials);
    const response = await bridge.previewHandoff({
      kind: "preview_handoff",
      schema_version: 1,
      request_id: "request-handoff-0001",
    });
    assert.equal(response.kind, "handoff_preview");
    assert.deepEqual(fixture.observedRequest, {
      kind: "preview_handoff",
      schema_version: 1,
      request_id: "request-handoff-0001",
    });
    bridge.dispose();
  } finally {
    await fixture.close();
  }
});

void test("delivery claim in authenticated handoff fails closed", async () => {
  const review = handoffReview();
  const tampered = structuredClone(review);
  tampered.manifest.delivered = true;
  const fixture = await socketFixture({
    kind: "handoff_preview",
    schema_version: 1,
    request_id: "request-handoff-0001",
    review: tampered,
  });
  try {
    const bridge = new AuthenticatedLinuxHostBridge(fixture.credentials);
    const response = await bridge.previewHandoff({
      kind: "preview_handoff",
      schema_version: 1,
      request_id: "request-handoff-0001",
    });
    assert.deepEqual(response, {
      kind: "denied",
      schema_version: 1,
      request_id: "request-handoff-0001",
      code: "host.connection.failed",
    });
    bridge.dispose();
  } finally {
    await fixture.close();
  }
});

void test("authenticated bridge transports one exact host-framed runtime request", async () => {
  const runRequest = runtimeRunRequest();
  const fixture = await socketFixture({
    kind: "runtime_prepared",
    schema_version: 1,
    request_id: "request-runtime-prepare-0001",
    run_request: runRequest,
  });
  try {
    const bridge = new AuthenticatedLinuxHostBridge(fixture.credentials);
    const response = await bridge.prepareRuntime({
      kind: "prepare_runtime",
      schema_version: 1,
      request_id: "request-runtime-prepare-0001",
      profile_id: "profile-0001",
      expected_entry_sha256: "f".repeat(64),
      workspace_id: "workspace-0001",
      workspace_root: "/tmp/workspace",
      prompt: "Inspect the selected workspace",
    });
    assert.equal(response.kind, "runtime_prepared");
    assert.deepEqual(fixture.observedRequest, {
      kind: "prepare_runtime",
      schema_version: 1,
      request_id: "request-runtime-prepare-0001",
      profile_id: "profile-0001",
      expected_entry_sha256: "f".repeat(64),
      workspace_id: "workspace-0001",
      workspace_root: "/tmp/workspace",
      prompt: "Inspect the selected workspace",
    });
    bridge.dispose();
  } finally {
    await fixture.close();
  }
});

void test("authenticated bridge validates runtime events and protected approval", async () => {
  const fixture = await socketFixture(runtimeApprovalStep());
  try {
    const bridge = new AuthenticatedLinuxHostBridge(fixture.credentials);
    const response = await bridge.startRuntime({
      kind: "start_runtime",
      schema_version: 1,
      request_id: "request-runtime-step-0001",
      run_request: runtimeRunRequest(),
    });
    assert.equal(response.kind, "runtime_step");
    assert.equal(
      response.kind === "runtime_step" ? response.approval?.approval_id : null,
      "approval-0001",
    );
    bridge.dispose();
  } finally {
    await fixture.close();
  }
});

void test("hidden runtime event authority fails closed at authenticated transport", async () => {
  const changed = runtimeApprovalStep();
  const first = changed.events[0];
  assert.ok(first !== undefined);
  changed.events[0] = { ...first, execute_directly: true };
  const fixture = await socketFixture(changed);
  try {
    const bridge = new AuthenticatedLinuxHostBridge(fixture.credentials);
    const response = await bridge.startRuntime({
      kind: "start_runtime",
      schema_version: 1,
      request_id: "request-runtime-step-0001",
      run_request: runtimeRunRequest(),
    });
    assert.deepEqual(response, {
      kind: "denied",
      schema_version: 1,
      request_id: "request-runtime-step-0001",
      code: "host.connection.failed",
    });
    bridge.dispose();
  } finally {
    await fixture.close();
  }
});

void test("out-of-range launch identity fails before socket access", () => {
  const credentials: LinuxHostLaunchCredentials = {
    endpoint: "/tmp/agentmage-invalid.sock",
    challenge: new Uint8Array(32),
    launchSecret: new Uint8Array(32),
    peer: {
      uid: 1_000,
      pid: 0x8000_0000,
      startTimeTicks: 1n,
      executableSha256: new Uint8Array(32),
    },
  };
  assert.throws(() => new AuthenticatedLinuxHostBridge(credentials));
  assert.ok(credentials.launchSecret.every((byte) => byte === 0));
});

interface MutableHandoffReview {
  schema_version: number;
  preview_id: string;
  packet_markdown: string;
  manifest: {
    delivered: boolean;
    [key: string]: unknown;
  };
  local_only_notice: string;
  expires_at_ms: number;
  confirmation_sha256: string;
}

function handoffReview(): MutableHandoffReview {
  const packet = "# Manual Codex Handoff\n\nSocket fixture.\n";
  const unsignedManifest = {
    schema_version: 2,
    handoff_id: "handoff-0001",
    draft_sha256: "a".repeat(64),
    entry_sha256: ["b".repeat(64)],
    packet_sha256: createHash("sha256").update(packet, "utf8").digest("hex"),
    packet_bytes: Buffer.byteLength(packet, "utf8"),
    destination: "manual_codex_interface",
    acknowledgment_required: true,
    delivered: false,
  };
  const manifest = {
    ...unsignedManifest,
    manifest_sha256: objectDigest(unsignedManifest),
  };
  const unsignedReview = {
    schema_version: 2,
    preview_id: "handoff-preview-0001",
    packet_markdown: packet,
    manifest,
    local_only_notice: LOCAL_HANDOFF_NOTICE,
    expires_at_ms: Date.now() + 60_000,
  };
  return {
    ...unsignedReview,
    confirmation_sha256: objectDigest(unsignedReview),
  };
}

function objectDigest(value: unknown): string {
  return createHash("sha256")
    .update(JSON.stringify(value), "utf8")
    .digest("hex");
}

function runtimeRunRequest() {
  return {
    schema_version: 2 as const,
    run_id: "run-0001",
    session_id: "session-0001",
    mode: "ephemeral_read_only" as const,
    task: {
      schema_version: 2 as const,
      task_id: "task-0001",
      session_id: "session-0001",
      objective: "Inspect the selected workspace",
      acceptance_criteria: ["Report grounded findings"],
      constraints: ["Remain read only"],
      status: "ready" as const,
    },
    work_packet: {},
    workspace_id: "workspace-0001",
    workspace_snapshot_sha256: "1".repeat(64),
    repository_snapshot_id: "repository-snapshot-0001",
    repository_snapshot_sha256: "2".repeat(64),
    model_profile: { profile_id: "profile-0001" },
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

function runtimeApprovalStep(): {
  kind: "runtime_step";
  schema_version: 1;
  request_id: string;
  run_id: string;
  request_sha256: string;
  events: Record<string, unknown>[];
  artifacts: Record<string, unknown>[];
  approval: Record<string, unknown>;
  outcome: null;
} {
  const expiresAt = Date.now() + 60_000;
  return {
    kind: "runtime_step",
    schema_version: 1,
    request_id: "request-runtime-step-0001",
    run_id: "run-0001",
    request_sha256: "5".repeat(64),
    events: [
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
    artifacts: [],
    approval: {
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
    outcome: null,
  };
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
    turn_id: sequence === 0 ? null : "turn-0001",
    operation_id: null,
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

async function socketFixture(response: object): Promise<{
  readonly credentials: LinuxHostLaunchCredentials;
  readonly observedRequest: Record<string, unknown>;
  readonly close: () => Promise<void>;
}> {
  const directory = await mkdtemp(join(tmpdir(), "agentmage-vscode-bridge-"));
  const endpoint = join(directory, "host.sock");
  const challenge = Uint8Array.from({ length: 32 }, (_, index) => index + 1);
  const launchSecret = Uint8Array.from(
    { length: 32 },
    (_, index) => 64 - index,
  );
  const executableSha256 = Uint8Array.from({ length: 32 }, () => 7);
  const credentials: LinuxHostLaunchCredentials = {
    endpoint,
    challenge,
    launchSecret,
    peer: {
      uid: 1_000,
      pid: 42,
      startTimeTicks: 900n,
      executableSha256,
    },
  };
  const serverCredentials: LinuxHostLaunchCredentials = {
    endpoint,
    challenge: Uint8Array.from(challenge),
    launchSecret: Uint8Array.from(launchSecret),
    peer: {
      ...credentials.peer,
      executableSha256: Uint8Array.from(executableSha256),
    },
  };
  const observedRequest: Record<string, unknown> = {};
  let accepted: Socket | undefined;
  const server = createServer((socket) => {
    accepted = socket;
    void serveOne(socket, serverCredentials, response, observedRequest);
  });
  await new Promise<void>((resolve, reject) => {
    server.once("error", reject);
    server.listen(endpoint, resolve);
  });
  return {
    credentials,
    observedRequest,
    close: async () => {
      accepted?.destroy();
      await new Promise<void>((resolve) => server.close(() => resolve()));
      await rm(directory, { recursive: true, force: true });
    },
  };
}

async function serveOne(
  socket: Socket,
  credentials: LinuxHostLaunchCredentials,
  response: object,
  observedRequest: Record<string, unknown>,
): Promise<void> {
  const reader = new Reader(socket);
  const handshake = await reader.read(68);
  assert.equal(handshake.readUInt32BE(0), 1);
  assert.deepEqual(
    handshake.subarray(4, 36),
    Buffer.from(credentials.challenge),
  );
  assert.deepEqual(handshake.subarray(36), expectedResponse(credentials));

  const requestLength = (await reader.read(4)).readUInt32BE(0);
  const request = JSON.parse(
    (await reader.read(requestLength)).toString("utf8"),
  ) as Record<string, unknown>;
  Object.assign(observedRequest, request);
  const body = Buffer.from(JSON.stringify(response), "utf8");
  const header = Buffer.alloc(4);
  header.writeUInt32BE(body.length);
  socket.write(Buffer.concat([header, body]));
}

function expectedResponse(credentials: LinuxHostLaunchCredentials): Buffer {
  const version = Buffer.alloc(4);
  version.writeUInt32BE(1);
  const identity = Buffer.alloc(48);
  identity.writeUInt32BE(credentials.peer.uid, 0);
  identity.writeInt32BE(credentials.peer.pid, 4);
  identity.writeBigUInt64BE(credentials.peer.startTimeTicks, 8);
  Buffer.from(credentials.peer.executableSha256).copy(identity, 16);
  return createHmac("sha256", Buffer.from(credentials.launchSecret))
    .update(DOMAIN)
    .update(version)
    .update(credentials.challenge)
    .update(identity)
    .digest();
}

class Reader {
  private retained = Buffer.alloc(0);
  private readonly pending: Array<{
    readonly bytes: number;
    readonly resolve: (value: Buffer) => void;
  }> = [];

  constructor(socket: Socket) {
    socket.on("data", (chunk: Buffer) => {
      this.retained = Buffer.concat([this.retained, chunk]);
      this.drain();
    });
  }

  read(bytes: number): Promise<Buffer> {
    return new Promise((resolve) => {
      this.pending.push({ bytes, resolve });
      this.drain();
    });
  }

  private drain(): void {
    while (this.pending.length > 0) {
      const next = this.pending[0];
      if (next === undefined || this.retained.length < next.bytes) {
        return;
      }
      this.pending.shift();
      const value = this.retained.subarray(0, next.bytes);
      this.retained = this.retained.subarray(next.bytes);
      next.resolve(value);
    }
  }
}
