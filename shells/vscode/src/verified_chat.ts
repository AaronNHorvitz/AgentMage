import { createHash, randomBytes, randomUUID } from "node:crypto";

import * as vscode from "vscode";

import { selectableModelInformation } from "./model_discovery.js";
import type {
  HostBridge,
  HostResponse,
  SecureReadController,
  SelectedRuntimeProfile,
} from "./provider.js";
import {
  captureExactBytes,
  captureExactText,
  request,
  type EngineeringHostResponse,
  type EngineeringMode,
} from "./verified_chat_protocol.js";
import {
  VerifiedChatCommandChannel,
  VerifiedChatChannelError,
  VerifiedChatRunGate,
  type VerifiedChatWebviewMessage,
} from "./verified_chat_channel.js";
import {
  BoundedProjectionDelivery,
  VerifiedChatEventProjection,
  VerifiedChatProjectionError,
  parseVerifiedChatSessionSnapshot,
} from "./verified_chat_projection.js";

const VIEW_TYPE = "agentmage.verifiedChat";
const VIEW_ID = "agentmage.sessions";

/** Dedicated, reconnectable AgentMage Verified Chat editor and Activity Bar surface. */
export class VerifiedChatSurface implements vscode.WebviewViewProvider {
  private panel: vscode.WebviewPanel | undefined;
  private view: vscode.WebviewView | undefined;
  private sessionId: string | undefined;
  private mode: EngineeringMode = "ask";
  private latestPlan:
    | {
        readonly artifactId: string;
        readonly sourceSha256: string;
        readonly text: string;
      }
    | undefined;
  private approvedPlan:
    | {
        readonly sourceSessionId: string;
        readonly artifactId: string;
        readonly planSha256: string;
        readonly approvalId: string;
        readonly approvalSha256: string;
        readonly text: string;
      }
    | undefined;
  private profiles: readonly SelectedRuntimeProfile[] = [];
  private selectedProfileId: string | undefined;
  private attachedArtifactIds: string[] = [];
  private workspaceState: vscode.Memento | undefined;
  private commandChannel: VerifiedChatCommandChannel | undefined;
  private delivery:
    BoundedProjectionDelivery<Readonly<Record<string, unknown>>> | undefined;
  private eventProjection: VerifiedChatEventProjection | undefined;
  private messageTail: Promise<void> = Promise.resolve();
  private readonly runGate = new VerifiedChatRunGate();

  constructor(
    private readonly extensionUri: vscode.Uri,
    private readonly bridge: HostBridge,
    private readonly controller: SecureReadController,
  ) {}

  register(context: vscode.ExtensionContext): void {
    this.workspaceState = context.workspaceState;
    const restoredSession = context.workspaceState.get<unknown>(
      "agentmage.verifiedChat.sessionId",
    );
    if (validIdentifier(restoredSession)) this.sessionId = restoredSession;
    context.subscriptions.push(
      vscode.window.registerWebviewViewProvider(VIEW_ID, this, {
        webviewOptions: { retainContextWhenHidden: false },
      }),
      vscode.commands.registerCommand("agentmage.openVerifiedChat", () =>
        this.open(),
      ),
      vscode.commands.registerCommand("agentmage.newVerifiedSession", () =>
        this.createSession("ask"),
      ),
    );
  }

  resolveWebviewView(view: vscode.WebviewView): void {
    this.view = view;
    view.webview.options = { enableScripts: true };
    view.webview.html = sidebarHtml();
    view.webview.onDidReceiveMessage((value: unknown) => {
      if (isRecord(value) && value.type === "open") {
        this.open();
      }
    });
    view.onDidDispose(() => {
      if (this.view === view) this.view = undefined;
    });
    void this.refreshSessions();
  }

  open(): void {
    if (this.panel !== undefined) {
      this.panel.reveal(vscode.ViewColumn.Active, false);
      return;
    }
    const panel = vscode.window.createWebviewPanel(
      VIEW_TYPE,
      "AgentMage Verified Chat",
      vscode.ViewColumn.Active,
      { enableScripts: true, retainContextWhenHidden: false },
    );
    this.panel = panel;
    const channelId = `webview-${randomUUID()}`;
    this.commandChannel = new VerifiedChatCommandChannel(channelId);
    this.delivery = new BoundedProjectionDelivery(async (message) =>
      panel.webview.postMessage(message),
    );
    panel.webview.html = editorHtml(panel.webview, channelId);
    panel.webview.onDidReceiveMessage((value: unknown) => {
      this.messageTail = this.messageTail
        .then(() => this.handleMessage(value))
        .catch(async (error: unknown) => {
          if (error instanceof VerifiedChatProjectionError) {
            await vscode.window.showErrorMessage(error.message);
          }
        });
    });
    panel.onDidDispose(() => {
      if (this.panel === panel) {
        this.runGate.cancelActive();
        this.delivery?.close();
        this.delivery = undefined;
        this.commandChannel = undefined;
        this.eventProjection = undefined;
        this.panel = undefined;
      }
    });
  }

  private async handleMessage(value: unknown): Promise<void> {
    let message: VerifiedChatWebviewMessage;
    try {
      message = this.commandChannel?.accept(
        value,
      ) as VerifiedChatWebviewMessage;
      if (message === undefined)
        throw new VerifiedChatChannelError("verified-chat.channel.closed");
    } catch (error) {
      await this.post({
        type: "status",
        state: "blocked",
        code:
          error instanceof Error
            ? error.message
            : "verified-chat.message.invalid",
      });
      return;
    }
    if (message.type === "ready") {
      await this.restoreSession();
      await this.post({
        type: "session",
        sessionId: this.sessionId ?? null,
        mode: this.mode,
      });
      await this.refreshModels();
      return;
    }
    if (!this.runGate.allows(message.type)) {
      await this.post({
        type: "status",
        state: "blocked",
        code: "verified-chat.agent.run-active",
      });
      return;
    }
    if (message.type === "selectModel") {
      const profile = this.profiles.find(
        (candidate) => candidate.profileId === message.text,
      );
      if (profile === undefined) {
        await this.post({
          type: "status",
          state: "blocked",
          code: "verified-chat.model.selection-invalid",
        });
        return;
      }
      this.selectedProfileId = profile.profileId;
      await this.post({ type: "modelSelected", profileId: profile.profileId });
      return;
    }
    if (message.type === "newSession") {
      await this.createSession(message.mode ?? "ask");
      return;
    }
    if (message.type === "startApprovedPlan") {
      const approved = this.approvedPlan;
      const targetMode = message.mode;
      if (
        approved === undefined ||
        (targetMode !== "agent" && targetMode !== "team")
      ) {
        await this.post({
          type: "status",
          state: "blocked",
          code: "verified-chat.plan-handoff.unavailable",
        });
        return;
      }
      const profile = this.profiles.find(
        (candidate) => candidate.profileId === this.selectedProfileId,
      );
      if (targetMode === "agent" && profile === undefined) {
        await this.post({
          type: "status",
          state: "blocked",
          code: "verified-chat.model.selection-required",
        });
        return;
      }
      const targetSessionId = `session-${randomUUID()}`;
      const response = await this.exchange(
        request("create_session_from_approved_plan", {
          source_session_id: approved.sourceSessionId,
          plan_artifact_id: approved.artifactId,
          plan_sha256: approved.planSha256,
          approval_id: approved.approvalId,
          approval_sha256: approved.approvalSha256,
          target_session_id: targetSessionId,
          title:
            targetMode === "agent"
              ? "Approved Plan Agent"
              : "Approved Plan Team",
          target_mode: targetMode,
          correlation_id: `correlation-${randomUUID()}`,
          occurred_at_epoch_ms: Date.now(),
        }),
      );
      if (
        response.kind === "denied" ||
        response.response.result !== "session"
      ) {
        await this.post({
          type: "status",
          state: "blocked",
          code:
            response.kind === "denied"
              ? response.code
              : "verified-chat.plan-handoff.failed",
        });
        return;
      }
      const targetSnapshot = parseVerifiedChatSessionSnapshot(
        response.response.snapshot,
      );
      if (
        targetSnapshot.sessionId !== targetSessionId ||
        targetSnapshot.mode !== targetMode
      ) {
        await this.post({
          type: "status",
          state: "blocked",
          code: "verified-chat.plan-handoff.binding-invalid",
        });
        return;
      }
      this.sessionId = targetSessionId;
      this.mode = targetMode;
      this.eventProjection = new VerifiedChatEventProjection(targetSessionId);
      this.latestPlan = undefined;
      this.approvedPlan = undefined;
      this.attachedArtifactIds = [];
      await this.post({
        type: "session",
        sessionId: targetSessionId,
        mode: targetMode,
      });
      await this.persistSession(targetSessionId);
      await this.replayEvents(targetSessionId);
      await this.refreshSessions();
      if (targetMode === "team") {
        await this.post({
          type: "status",
          state: "blocked",
          code: "verified-chat.team.runtime-unavailable",
        });
        return;
      }
      this.startAgentExecution(
        approved.text,
        targetSessionId,
        profile as SelectedRuntimeProfile,
      );
      return;
    }
    if (
      message.type === "pause" ||
      message.type === "resume" ||
      message.type === "cancel"
    ) {
      const sessionId = this.sessionId;
      if (sessionId === undefined) {
        await this.post({
          type: "status",
          state: "blocked",
          code: "verified-chat.session.unavailable",
        });
        return;
      }
      const operation =
        message.type === "pause"
          ? "pause_session"
          : message.type === "resume"
            ? "resume_session"
            : "cancel_session";
      if (message.type === "cancel") {
        this.runGate.cancel(sessionId);
      }
      const response = await this.exchange(
        request(operation, {
          session_id: sessionId,
          correlation_id: `correlation-${randomUUID()}`,
          occurred_at_epoch_ms: Date.now(),
        }),
      );
      if (
        response.kind === "denied" ||
        response.response.result !== "lifecycle_event"
      ) {
        await this.post({
          type: "status",
          state: "blocked",
          code:
            response.kind === "denied"
              ? response.code
              : "verified-chat.lifecycle.failed",
        });
        return;
      }
      await this.replayEvents(sessionId);
      return;
    }
    const requestedMode = "mode" in message ? message.mode : this.mode;
    if (this.sessionId === undefined || requestedMode !== this.mode) {
      await this.createSession(requestedMode);
    }
    const sessionId = this.sessionId;
    if (sessionId === undefined) return;
    if (message.type === "approvePlan") {
      const plan = this.latestPlan;
      if (this.mode !== "plan" || plan === undefined) {
        await this.post({
          type: "status",
          state: "blocked",
          code: "verified-chat.plan-approval.unavailable",
        });
        return;
      }
      const response = await this.exchange(
        request("approve_plan", {
          session_id: sessionId,
          plan_artifact_id: plan.artifactId,
          plan_sha256: plan.sourceSha256,
          approval_id: `approval-${randomUUID()}`,
          correlation_id: `correlation-${randomUUID()}`,
          occurred_at_epoch_ms: Date.now(),
        }),
      );
      if (
        response.kind === "denied" ||
        response.response.result !== "plan_approved"
      ) {
        await this.post({
          type: "status",
          state: "blocked",
          code:
            response.kind === "denied"
              ? response.code
              : "verified-chat.plan-approval.failed",
        });
        return;
      }
      const approval = asRecord(response.response.approval);
      if (
        approval === undefined ||
        typeof approval.approval_id !== "string" ||
        typeof approval.approval_sha256 !== "string" ||
        approval.session_id !== sessionId ||
        approval.plan_artifact_id !== plan.artifactId ||
        approval.plan_sha256 !== plan.sourceSha256
      ) {
        await this.post({
          type: "status",
          state: "blocked",
          code: "verified-chat.plan-approval.invalid",
        });
        return;
      }
      this.approvedPlan = {
        sourceSessionId: sessionId,
        artifactId: plan.artifactId,
        planSha256: plan.sourceSha256,
        approvalId: approval.approval_id,
        approvalSha256: approval.approval_sha256,
        text: plan.text,
      };
      this.latestPlan = undefined;
      await this.replayEvents(sessionId);
      await this.post({ type: "planState", approved: true });
      await this.post({
        type: "status",
        state: "complete",
        code: "verified-chat.plan.approved",
      });
      return;
    }
    if (message.type === "capturePaste") {
      await this.capture(sessionId, "paste", "Pasted text", message.text ?? "");
      return;
    }
    if (message.type === "attachFiles") {
      await this.attachFiles(sessionId);
      return;
    }
    if (message.type === "send" && (message.text ?? "").trim().length > 0) {
      const text = message.text ?? "";
      await this.post({ type: "message", role: "user", text });
      const captured = await this.capture(
        sessionId,
        "paste",
        "Verified Chat message",
        text,
      );
      if (captured === undefined) return;
      const response = await this.exchange(
        request("execute_verified_turn", {
          session_id: sessionId,
          prompt_artifact_id: captured.artifactId,
          context_artifact_ids: this.attachedArtifactIds,
          correlation_id: `correlation-${randomUUID()}`,
          occurred_at_epoch_ms: Date.now(),
        }),
      );
      if (response.kind === "denied") {
        await this.post({
          type: "status",
          state: "blocked",
          code: response.code,
        });
        return;
      }
      if (response.response.result !== "verified_turn_completed") {
        await this.post({
          type: "status",
          state: "blocked",
          code: "verified-chat.response.mismatch",
        });
        return;
      }
      await this.replayEvents(sessionId);
      const turn = asRecord(response.response.turn);
      if (turn === undefined || typeof turn.output_text !== "string") {
        await this.post({
          type: "status",
          state: "blocked",
          code: "verified-chat.response.invalid",
        });
        return;
      }
      const context = contextSummary(turn.context);
      if (context === undefined) {
        await this.post({
          type: "status",
          state: "blocked",
          code: "verified-chat.context-receipt.invalid",
        });
        return;
      }
      await this.post({ type: "contextReceipt", context });
      const planArtifact = asRecord(response.response.plan_artifact);
      if (this.mode === "plan") {
        if (
          planArtifact === undefined ||
          typeof planArtifact.artifact_id !== "string" ||
          typeof planArtifact.display_name !== "string" ||
          typeof planArtifact.source_sha256 !== "string" ||
          typeof planArtifact.byte_length !== "number"
        ) {
          await this.post({
            type: "status",
            state: "blocked",
            code: "verified-chat.plan-artifact.invalid",
          });
          return;
        }
        await this.post({
          type: "artifact",
          artifactId: planArtifact.artifact_id,
          displayName: planArtifact.display_name,
          sourceSha256: planArtifact.source_sha256,
          byteLength: planArtifact.byte_length,
        });
        this.latestPlan = {
          artifactId: planArtifact.artifact_id,
          sourceSha256: planArtifact.source_sha256,
          text: turn.output_text,
        };
        await this.post({ type: "planState", approved: false });
      } else if (response.response.plan_artifact !== null) {
        await this.post({
          type: "status",
          state: "blocked",
          code: "verified-chat.plan-artifact.unexpected",
        });
        return;
      }
      await this.streamMessage(turn.output_text);
    }
  }

  private async createSession(mode: EngineeringMode): Promise<void> {
    const sessionId = `session-${randomUUID()}`;
    const response = await this.exchange(
      request("create_session", {
        session_id: sessionId,
        title: "Verified Chat",
        mode,
        correlation_id: `correlation-${randomUUID()}`,
        occurred_at_epoch_ms: Date.now(),
      }),
    );
    if (response.kind === "denied" || response.response.result !== "session") {
      await this.post({
        type: "status",
        state: "blocked",
        code:
          response.kind === "denied"
            ? response.code
            : "verified-chat.session.invalid",
      });
      return;
    }
    const snapshot = parseVerifiedChatSessionSnapshot(
      response.response.snapshot,
    );
    if (snapshot.sessionId !== sessionId || snapshot.mode !== mode) {
      await this.post({
        type: "status",
        state: "blocked",
        code: "verified-chat.session.binding-invalid",
      });
      return;
    }
    this.sessionId = sessionId;
    this.mode = mode;
    this.eventProjection = new VerifiedChatEventProjection(sessionId);
    this.latestPlan = undefined;
    this.approvedPlan = undefined;
    this.attachedArtifactIds = [];
    await this.post({ type: "session", sessionId, mode });
    await this.persistSession(sessionId);
    await this.replayEvents(sessionId);
    await this.refreshSessions();
  }

  private async restoreSession(): Promise<void> {
    const sessionId = this.sessionId;
    if (sessionId === undefined) return;
    const response = await this.exchange(
      request("open_session", { session_id: sessionId }),
    );
    if (response.kind === "denied" || response.response.result !== "session") {
      this.sessionId = undefined;
      this.eventProjection = undefined;
      await this.persistSession(undefined);
      await this.post({
        type: "status",
        state: "blocked",
        code:
          response.kind === "denied"
            ? response.code
            : "verified-chat.session.restore-invalid",
      });
      return;
    }
    try {
      const snapshot = parseVerifiedChatSessionSnapshot(
        response.response.snapshot,
      );
      if (snapshot.sessionId !== sessionId) {
        throw new VerifiedChatProjectionError(
          "verified-chat.session.restore-mismatch",
        );
      }
      this.mode = snapshot.mode;
      this.latestPlan = undefined;
      this.approvedPlan = undefined;
      this.attachedArtifactIds = [];
      this.eventProjection = new VerifiedChatEventProjection(sessionId);
      await this.post({
        type: "sessionRestored",
        sessionId,
        mode: snapshot.mode,
        artifacts: snapshot.artifacts,
        terminal: snapshot.terminal,
      });
      await this.replayEvents(sessionId);
      await this.recoverPlanState(snapshot);
    } catch (error) {
      this.sessionId = undefined;
      this.eventProjection = undefined;
      await this.persistSession(undefined);
      await this.post({
        type: "status",
        state: "blocked",
        code:
          error instanceof Error
            ? error.message
            : "verified-chat.session.restore-invalid",
      });
    }
  }

  private async replayEvents(sessionId: string): Promise<void> {
    const projection = this.eventProjection;
    if (projection === undefined || projection.sessionId !== sessionId) {
      throw new VerifiedChatProjectionError(
        "verified-chat.projection.session-mismatch",
      );
    }
    const response = await this.exchange(
      request("replay_events", {
        session_id: sessionId,
        after_sequence: projection.cursor ?? null,
      }),
    );
    if (
      response.kind === "denied" ||
      response.response.result !== "events" ||
      !Array.isArray(response.response.events)
    ) {
      throw new VerifiedChatProjectionError(
        response.kind === "denied"
          ? response.code
          : "verified-chat.events.invalid",
      );
    }
    const cards = projection.accept(response.response.events);
    for (const card of cards) await this.post({ ...card });
  }

  private async recoverPlanState(
    snapshot: ReturnType<typeof parseVerifiedChatSessionSnapshot>,
  ): Promise<void> {
    const approved = this.eventProjection?.approvedPlan;
    const planArtifact =
      approved === undefined
        ? [...snapshot.artifacts]
            .reverse()
            .find(
              (artifact) =>
                snapshot.mode === "plan" &&
                artifact.sourceKind === "generated" &&
                artifact.displayName === "Verified Chat plan draft" &&
                artifact.mediaType === "text/markdown",
            )
        : snapshot.artifacts.find(
            (artifact) =>
              artifact.artifactId === approved.artifactId &&
              artifact.sourceSha256 === approved.planSha256 &&
              artifact.sourceKind === "generated" &&
              artifact.displayName === "Verified Chat plan draft" &&
              artifact.mediaType === "text/markdown",
          );
    if (planArtifact === undefined) {
      if (approved !== undefined) {
        throw new VerifiedChatProjectionError(
          "verified-chat.plan.restore-artifact-missing",
        );
      }
      return;
    }
    const text = await this.readArtifactText(
      snapshot.sessionId,
      planArtifact.artifactId,
      planArtifact.sourceSha256,
      planArtifact.byteLength,
    );
    if (approved === undefined) {
      this.latestPlan = {
        artifactId: planArtifact.artifactId,
        sourceSha256: planArtifact.sourceSha256,
        text,
      };
      await this.post({ type: "planState", approved: false });
      return;
    }
    this.approvedPlan = { ...approved, text };
    await this.post({ type: "planState", approved: true });
  }

  private async readArtifactText(
    sessionId: string,
    artifactId: string,
    sourceSha256: string,
    byteLength: number,
  ): Promise<string> {
    if (!Number.isSafeInteger(byteLength) || byteLength <= 0) {
      throw new VerifiedChatProjectionError(
        "verified-chat.plan.restore-length-invalid",
      );
    }
    const pageBytes = 1024 * 1024;
    const bytes = new Uint8Array(byteLength);
    for (let offset = 0; offset < byteLength; offset += pageBytes) {
      const length = Math.min(pageBytes, byteLength - offset);
      const response = await this.exchange(
        request("read_artifact_range", {
          session_id: sessionId,
          artifact_id: artifactId,
          offset,
          length,
        }),
      );
      if (
        response.kind === "denied" ||
        response.response.result !== "artifact_range"
      ) {
        throw new VerifiedChatProjectionError(
          response.kind === "denied"
            ? response.code
            : "verified-chat.plan.restore-read-invalid",
        );
      }
      const range = exactRecord(response.response.range, [
        "artifact_id",
        "bytes",
        "requested_length",
        "requested_offset",
        "returned_offset",
        "returned_sha256",
        "schema_version",
        "source_sha256",
        "truncated",
      ]);
      if (
        range === undefined ||
        range.schema_version !== 1 ||
        range.artifact_id !== artifactId ||
        range.source_sha256 !== sourceSha256 ||
        range.requested_offset !== offset ||
        range.requested_length !== length ||
        range.returned_offset !== offset ||
        range.truncated !== offset + length < byteLength ||
        !Array.isArray(range.bytes) ||
        range.bytes.length !== length ||
        range.bytes.some(
          (byte) =>
            typeof byte !== "number" ||
            !Number.isInteger(byte) ||
            byte < 0 ||
            byte > 255,
        )
      ) {
        throw new VerifiedChatProjectionError(
          "verified-chat.plan.restore-range-invalid",
        );
      }
      const page = Uint8Array.from(range.bytes as number[]);
      const pageSha256 = createHash("sha256").update(page).digest("hex");
      if (range.returned_sha256 !== pageSha256) {
        throw new VerifiedChatProjectionError(
          "verified-chat.plan.restore-range-tampered",
        );
      }
      bytes.set(page, offset);
    }
    if (createHash("sha256").update(bytes).digest("hex") !== sourceSha256) {
      throw new VerifiedChatProjectionError(
        "verified-chat.plan.restore-source-tampered",
      );
    }
    try {
      return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
    } catch {
      throw new VerifiedChatProjectionError(
        "verified-chat.plan.restore-encoding-invalid",
      );
    }
  }

  private async persistSession(sessionId: string | undefined): Promise<void> {
    await this.workspaceState?.update(
      "agentmage.verifiedChat.sessionId",
      sessionId,
    );
  }

  private async streamMessage(text: string): Promise<void> {
    const streamId = `stream-${randomUUID()}`;
    const chunkCharacters = 2048;
    if (text.length === 0) {
      await this.post({
        type: "messagePart",
        role: "assistant",
        streamId,
        text: "",
        final: true,
      });
      return;
    }
    for (let offset = 0; offset < text.length; offset += chunkCharacters) {
      const end = Math.min(offset + chunkCharacters, text.length);
      await this.post({
        type: "messagePart",
        role: "assistant",
        streamId,
        text: text.slice(offset, end),
        final: end === text.length,
      });
    }
  }

  private async capture(
    sessionId: string,
    sourceKind: "paste" | "editor_selection",
    displayName: string,
    text: string,
  ): Promise<
    | {
        readonly artifactId: string;
        readonly sourceSha256: string;
        readonly byteLength: number;
      }
    | undefined
  > {
    try {
      const captured = await captureExactText(
        (hostRequest) => this.exchange(hostRequest),
        sessionId,
        sourceKind,
        displayName,
        "text/plain",
        text,
      );
      await this.post({ type: "artifact", ...captured, displayName });
      await this.replayEvents(sessionId);
      return captured;
    } catch (error) {
      await this.post({
        type: "status",
        state: "blocked",
        code:
          error instanceof Error
            ? error.message
            : "verified-chat.capture.failed",
      });
      return undefined;
    }
  }

  private async attachFiles(sessionId: string): Promise<void> {
    const selected = await vscode.window.showOpenDialog({
      canSelectFiles: true,
      canSelectFolders: false,
      canSelectMany: true,
      title: "Attach files to AgentMage Verified Chat",
      openLabel: "Attach",
    });
    if (selected === undefined) return;
    if (selected.length > 16) {
      await this.post({
        type: "status",
        state: "blocked",
        code: "verified-chat.attachments.too-many",
      });
      return;
    }
    for (const uri of selected) {
      const displayName = uri.path.split("/").pop() ?? "Selected file";
      try {
        const bytes = await vscode.workspace.fs.readFile(uri);
        const mediaType = mediaTypeFor(displayName);
        const captured = await captureExactBytes(
          (hostRequest) => this.exchange(hostRequest),
          sessionId,
          "file",
          displayName,
          mediaType,
          bytes,
        );
        await this.replayEvents(sessionId);
        if (mediaType.startsWith("text/") || mediaType === "application/json") {
          this.attachedArtifactIds.push(captured.artifactId);
        } else {
          await this.post({
            type: "status",
            state: "blocked",
            code: "verified-chat.attachment.captured-retrieval-only",
          });
        }
        await this.post({
          type: "attachment",
          ...captured,
          displayName,
        });
      } catch (error) {
        await this.post({
          type: "status",
          state: "blocked",
          code:
            error instanceof Error
              ? error.message
              : "verified-chat.attachment.failed",
        });
      }
    }
  }

  private async exchange(
    hostRequest: Parameters<HostBridge["engineering"]>[0],
  ): Promise<
    EngineeringHostResponse | { readonly kind: "denied"; readonly code: string }
  > {
    const response: HostResponse = await this.bridge.engineering(hostRequest);
    if (response.kind === "engineering") return response;
    if (response.kind === "denied") return response;
    return { kind: "denied", code: "verified-chat.response.mismatch" };
  }

  /** Starts one cancellable presentation of the existing controlled runtime without blocking IPC. */
  private startAgentExecution(
    plan: string,
    sessionId: string,
    profile: SelectedRuntimeProfile,
  ): void {
    const cancellation = new vscode.CancellationTokenSource();
    const runId = `view-run-${randomUUID()}`;
    this.runGate.begin(runId, sessionId, () => cancellation.cancel());
    const streamId = `stream-${randomUUID()}`;
    void (async () => {
      let streamTail = Promise.resolve();
      try {
        const result = await this.controller.runApprovedAgentPlan(
          plan,
          sessionId,
          profile,
          cancellation.token,
          (part) => {
            streamTail = streamTail.then(() =>
              this.post({
                type: "messagePart",
                role: "assistant",
                streamId,
                text: part,
                final: false,
              }),
            );
          },
        );
        await streamTail;
        await this.post({
          type: "messagePart",
          role: "assistant",
          streamId,
          text: "",
          final: true,
        });
        if (result.parts.length === 0) {
          await this.post({
            type: "status",
            state: "blocked",
            code: "verified-chat.agent.runtime-empty",
          });
        }
      } catch (error) {
        cancellation.cancel();
        await this.post({
          type: "status",
          state: "blocked",
          code:
            error instanceof Error
              ? error.message
              : "verified-chat.agent.runtime-failed",
        }).catch(() => undefined);
      } finally {
        this.runGate.finish(runId);
        cancellation.dispose();
      }
    })();
  }

  private async refreshSessions(): Promise<void> {
    const response = await this.exchange(request("list_sessions"));
    const sessions =
      response.kind === "engineering" && response.response.result === "sessions"
        ? response.response.sessions
        : [];
    await this.view?.webview.postMessage({ type: "sessions", sessions });
  }

  private async refreshModels(): Promise<void> {
    const cancellation = new vscode.CancellationTokenSource();
    const snapshot = await this.controller.discoverModels(cancellation.token);
    cancellation.dispose();
    const models =
      snapshot === undefined ? [] : selectableModelInformation(snapshot);
    this.profiles = models.map((model) => ({
      profileId: model.id,
      expectedEntrySha256: model.entrySha256,
      manifestSha256: model.version,
      artifactSha256: model.artifactSha256,
      runtimeAdapterId: model.runtimeAdapterId,
      runtimeSha256: model.runtimeSha256,
      maxContextTokens: model.maxInputTokens,
      maxOutputTokens: model.maxOutputTokens,
      toolCalling: model.capabilities.toolCalling,
      visionInput: model.visionInput,
    }));
    if (
      !this.profiles.some(
        (profile) => profile.profileId === this.selectedProfileId,
      )
    ) {
      this.selectedProfileId = this.profiles[0]?.profileId;
    }
    await this.post({
      type: "models",
      models: models.map((model) => ({ id: model.id, name: model.name })),
      selectedProfileId: this.selectedProfileId ?? null,
    });
  }

  private async post(
    message: Readonly<Record<string, unknown>>,
  ): Promise<void> {
    const delivery = this.delivery;
    if (delivery === undefined) return;
    await delivery.enqueue(message);
  }
}

function validIdentifier(value: unknown): value is string {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    value.length <= 128 &&
    /^[A-Za-z0-9._:-]+$/u.test(value)
  );
}

function contextSummary(
  value: unknown,
): Readonly<Record<string, unknown>> | undefined {
  const context = asRecord(value);
  if (
    context === undefined ||
    typeof context.context_packet_id !== "string" ||
    typeof context.model_profile_id !== "string" ||
    typeof context.endpoint_profile_id !== "string" ||
    typeof context.route_decision_id !== "string" ||
    typeof context.inline_bytes !== "number" ||
    typeof context.estimated_tokens !== "number" ||
    typeof context.token_limit !== "number" ||
    typeof context.context_sha256 !== "string" ||
    !Array.isArray(context.artifacts)
  ) {
    return undefined;
  }
  return {
    contextPacketId: context.context_packet_id,
    modelProfileId: context.model_profile_id,
    endpointProfileId: context.endpoint_profile_id,
    routeDecisionId: context.route_decision_id,
    inlineBytes: context.inline_bytes,
    estimatedTokens: context.estimated_tokens,
    tokenLimit: context.token_limit,
    contextSha256: context.context_sha256,
    artifacts: context.artifacts,
  };
}

function mediaTypeFor(displayName: string): string {
  const extension = displayName.toLowerCase().split(".").pop();
  const known: Readonly<Record<string, string>> = {
    csv: "text/csv",
    json: "application/json",
    log: "text/plain",
    md: "text/markdown",
    pdf: "application/pdf",
    txt: "text/plain",
  };
  return known[extension ?? ""] ?? "application/octet-stream";
}

function sidebarHtml(): string {
  const nonce = randomBytes(18).toString("base64");
  return `<!doctype html><html><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'nonce-${nonce}'; script-src 'nonce-${nonce}'"><style nonce="${nonce}">body{padding:12px;color:var(--vscode-foreground);font-family:var(--vscode-font-family)}button{width:100%;height:32px;color:var(--vscode-button-foreground);background:var(--vscode-button-background);border:0}#sessions{margin-top:12px;font-size:12px;color:var(--vscode-descriptionForeground)}</style></head><body><button id="open">Open Verified Chat</button><div id="sessions"></div><script nonce="${nonce}">const vscode=acquireVsCodeApi();document.getElementById('open').addEventListener('click',()=>vscode.postMessage({type:'open'}));window.addEventListener('message',event=>{if(event.data?.type==='sessions'){document.getElementById('sessions').textContent=Array.isArray(event.data.sessions)?event.data.sessions.length+' sessions':'';}});</script></body></html>`;
}

function editorHtml(webview: vscode.Webview, channelId: string): string {
  const nonce = randomBytes(18).toString("base64");
  const cspSource = webview.cspSource;
  return `<!doctype html>
<html><head><meta charset="utf-8">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src ${cspSource}; style-src 'nonce-${nonce}'; script-src 'nonce-${nonce}'">
<meta name="viewport" content="width=device-width,initial-scale=1">
<style nonce="${nonce}">
*{box-sizing:border-box}body{margin:0;color:var(--vscode-foreground);background:var(--vscode-editor-background);font-family:var(--vscode-font-family);height:100vh;display:grid;grid-template-rows:42px 1fr auto}.bar{display:flex;gap:8px;align-items:center;padding:6px 12px;border-bottom:1px solid var(--vscode-panel-border);overflow-x:auto}select,textarea,button{font:inherit;color:inherit}select,textarea{background:var(--vscode-input-background);border:1px solid var(--vscode-input-border);color:var(--vscode-input-foreground)}button{height:30px;border:0;background:var(--vscode-button-background);color:var(--vscode-button-foreground);padding:0 12px;white-space:nowrap}button:disabled{opacity:.55}.workspace{min-height:0;display:grid;grid-template-columns:minmax(0,1fr) 320px}.messages{overflow:auto;padding:16px;display:flex;flex-direction:column;gap:10px}.message{white-space:pre-wrap;line-height:1.45;max-width:900px}.user{border-left:3px solid var(--vscode-charts-blue);padding-left:10px}.status{color:var(--vscode-descriptionForeground);font-size:12px}.artifacts{color:var(--vscode-charts-green);font-size:12px}.event-card{border:1px solid var(--vscode-panel-border);border-left:3px solid var(--vscode-charts-purple);padding:7px 9px;font-size:12px}.event-card.tool{border-left-color:var(--vscode-charts-orange)}.event-card.verification,.event-card.terminal{border-left-color:var(--vscode-testing-iconPassed)}.event-card.approval{border-left-color:var(--vscode-charts-yellow)}.inspector{overflow:auto;border-left:1px solid var(--vscode-panel-border);padding:12px}.inspector h2{font-size:13px;margin:0 0 10px}.inspector pre{white-space:pre-wrap;overflow-wrap:anywhere;font:12px var(--vscode-editor-font-family);color:var(--vscode-descriptionForeground)}.composer{padding:10px 12px;border-top:1px solid var(--vscode-panel-border);display:grid;grid-template-columns:1fr auto;gap:8px}.compose-tools{grid-column:1/-1;display:flex;align-items:center;gap:8px;min-height:30px}.attachments{font-size:12px;color:var(--vscode-descriptionForeground);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}textarea{resize:vertical;min-height:72px;max-height:240px;padding:8px;letter-spacing:0}@media(max-width:800px){.workspace{grid-template-columns:1fr}.inspector{border-left:0;border-top:1px solid var(--vscode-panel-border);max-height:180px}}
</style></head><body>
<header class="bar"><strong>AgentMage</strong><select id="mode" aria-label="Mode"><option value="ask">Ask</option><option value="plan">Plan</option><option value="agent">Agent</option><option value="team">Team</option></select><select id="model" aria-label="Model"><option value="">No qualified model</option></select><button id="approve" disabled>Approve Plan</button><button id="startAgent" disabled>Start Agent</button><button id="startTeam" disabled>Start Team</button><button id="pause">Pause</button><button id="resume">Resume</button><button id="cancel">Cancel</button><button id="new">New</button><span id="session" class="status" role="status"></span></header>
<div class="workspace"><main id="messages" class="messages" role="log" aria-live="polite" aria-relevant="additions text"></main><aside class="inspector" aria-label="What the Model Saw"><h2>What the Model Saw</h2><pre id="context">No model context recorded</pre></aside></div>
<footer class="composer"><div class="compose-tools"><button id="attach" title="Attach files">Attach</button><span id="attachments" class="attachments"></span></div><textarea id="composer" aria-label="Message"></textarea><button id="send">Send</button></footer>
<script nonce="${nonce}">
const vscode=acquireVsCodeApi(),composer=document.getElementById('composer'),messages=document.getElementById('messages'),mode=document.getElementById('mode'),model=document.getElementById('model'),approve=document.getElementById('approve'),startAgent=document.getElementById('startAgent'),startTeam=document.getElementById('startTeam'),attachments=document.getElementById('attachments'),context=document.getElementById('context'),channel=${JSON.stringify(channelId)};let outboundSequence=0,planApproved=false,attachmentNames=[];const streams=new Map();function post(type,extra={}){vscode.postMessage({type,channel,sequence:outboundSequence++,...extra});}function add(text,kind='status'){const item=document.createElement('div');item.className='message '+kind;item.textContent=String(text??'');messages.appendChild(item);messages.scrollTop=messages.scrollHeight;return item;}function addEvent(data){const item=document.createElement('div');item.className='event-card '+String(data.category??'');item.setAttribute('role','status');item.setAttribute('aria-label',String(data.category??'Runtime')+' event '+String(data.sequence??''));item.textContent=String(data.label??'Runtime event');messages.appendChild(item);messages.scrollTop=messages.scrollHeight;}function stream(data){let item=streams.get(data.streamId);if(!item){item=add('',data.role==='user'?'user':'assistant');streams.set(data.streamId,item);}item.textContent+=String(data.text??'');if(data.final)streams.delete(data.streamId);messages.scrollTop=messages.scrollHeight;}function actions(){approve.disabled=planApproved;startAgent.disabled=!planApproved||!model.value;startTeam.disabled=!planApproved;}function resetSession(data){document.getElementById('session').textContent=data.sessionId??'';mode.value=data.mode??'ask';planApproved=false;attachmentNames=[];attachments.textContent='';context.textContent='No model context recorded';actions();}composer.addEventListener('paste',event=>{const text=event.clipboardData?.getData('text/plain');if(typeof text!=='string')return;event.preventDefault();const start=composer.selectionStart,end=composer.selectionEnd;composer.setRangeText(text,start,end,'end');post('capturePaste',{text});});model.addEventListener('change',()=>post('selectModel',{text:model.value}));document.getElementById('new').addEventListener('click',()=>post('newSession',{mode:mode.value}));document.getElementById('attach').addEventListener('click',()=>post('attachFiles',{mode:mode.value}));document.getElementById('pause').addEventListener('click',()=>post('pause'));document.getElementById('resume').addEventListener('click',()=>post('resume'));document.getElementById('cancel').addEventListener('click',()=>post('cancel'));approve.addEventListener('click',()=>post('approvePlan'));startAgent.addEventListener('click',()=>post('startApprovedPlan',{mode:'agent'}));startTeam.addEventListener('click',()=>post('startApprovedPlan',{mode:'team'}));document.getElementById('send').addEventListener('click',()=>{const text=composer.value;if(!text.trim())return;post('send',{text,mode:mode.value});composer.value='';});window.addEventListener('message',event=>{const data=event.data;if(data?.type==='session'){resetSession(data);}else if(data?.type==='sessionRestored'){resetSession(data);for(const artifact of data.artifacts??[]){add(artifact.displayName+' · '+artifact.byteLength+' bytes · '+artifact.sourceSha256.slice(0,12),'artifacts');}if(data.terminal)add('Restored terminal state: '+data.terminal,'status');}else if(data?.type==='models'){model.replaceChildren();for(const entry of data.models??[]){const option=document.createElement('option');option.value=entry.id;option.textContent=entry.name;model.appendChild(option);}if(model.options.length===0){const option=document.createElement('option');option.value='';option.textContent='No qualified model';model.appendChild(option);}model.value=data.selectedProfileId??'';actions();}else if(data?.type==='modelSelected'){model.value=data.profileId??'';actions();}else if(data?.type==='message'){add(data.text,data.role==='user'?'user':'assistant');}else if(data?.type==='messagePart'){stream(data);}else if(data?.type==='runtimeEvent'){addEvent(data);}else if(data?.type==='artifact'){add(data.displayName+' · '+data.byteLength+' bytes · '+data.sourceSha256.slice(0,12),'artifacts');}else if(data?.type==='attachment'){attachmentNames.push(data.displayName);attachments.textContent=attachmentNames.join(', ');add(data.displayName+' · '+data.byteLength+' bytes · '+data.sourceSha256.slice(0,12),'artifacts');}else if(data?.type==='contextReceipt'){context.textContent=JSON.stringify(data.context,null,2);}else if(data?.type==='planState'){planApproved=Boolean(data.approved);actions();}else if(data?.type==='status'){add(data.code,'status');}});actions();post('ready');
</script></body></html>`;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function asRecord(value: unknown): Record<string, unknown> | undefined {
  return isRecord(value) ? value : undefined;
}

function exactRecord(
  value: unknown,
  expectedKeys: readonly string[],
): Record<string, unknown> | undefined {
  const record = asRecord(value);
  if (record === undefined) return undefined;
  const actual = Object.keys(record).sort();
  const expected = [...expectedKeys].sort();
  return actual.length === expected.length &&
    actual.every((key, index) => key === expected[index])
    ? record
    : undefined;
}
