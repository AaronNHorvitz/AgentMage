import { randomBytes, randomUUID } from "node:crypto";

import * as vscode from "vscode";

import { selectableModelInformation } from "./model_discovery.js";
import type {
  HostBridge,
  HostResponse,
  SecureReadController,
  SelectedRuntimeProfile,
} from "./provider.js";
import {
  captureExactText,
  request,
  type EngineeringHostResponse,
  type EngineeringMode,
} from "./verified_chat_protocol.js";

const VIEW_TYPE = "agentmage.verifiedChat";
const VIEW_ID = "agentmage.sessions";

interface WebviewMessage {
  readonly type:
    | "ready"
    | "newSession"
    | "capturePaste"
    | "send"
    | "approvePlan"
    | "startApprovedPlan"
    | "selectModel";
  readonly text?: string;
  readonly mode?: EngineeringMode;
}

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

  constructor(
    private readonly extensionUri: vscode.Uri,
    private readonly bridge: HostBridge,
    private readonly controller: SecureReadController,
  ) {}

  register(context: vscode.ExtensionContext): void {
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
    panel.webview.html = editorHtml(panel.webview);
    panel.webview.onDidReceiveMessage((value: unknown) => {
      void this.handleMessage(value);
    });
    panel.onDidDispose(() => {
      if (this.panel === panel) this.panel = undefined;
    });
  }

  private async handleMessage(value: unknown): Promise<void> {
    const message = parseWebviewMessage(value);
    if (message === undefined) {
      await this.post({
        type: "status",
        state: "blocked",
        code: "verified-chat.message.invalid",
      });
      return;
    }
    if (message.type === "ready") {
      await this.post({
        type: "session",
        sessionId: this.sessionId ?? null,
        mode: this.mode,
      });
      await this.refreshModels();
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
      this.sessionId = targetSessionId;
      this.mode = targetMode;
      this.latestPlan = undefined;
      this.approvedPlan = undefined;
      await this.post({
        type: "session",
        sessionId: targetSessionId,
        mode: targetMode,
      });
      await this.refreshSessions();
      if (targetMode === "team") {
        await this.post({
          type: "status",
          state: "blocked",
          code: "verified-chat.team.runtime-unavailable",
        });
        return;
      }
      const cancellation = new vscode.CancellationTokenSource();
      const result = await this.controller.runApprovedAgentPlan(
        approved.text,
        targetSessionId,
        profile as SelectedRuntimeProfile,
        cancellation.token,
        (part) => {
          void this.post({ type: "message", role: "assistant", text: part });
        },
      );
      cancellation.dispose();
      if (result.parts.length === 0) {
        await this.post({
          type: "status",
          state: "blocked",
          code: "verified-chat.agent.runtime-empty",
        });
      }
      return;
    }
    const requestedMode = message.mode ?? this.mode;
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
      const turn = asRecord(response.response.turn);
      if (turn === undefined || typeof turn.output_text !== "string") {
        await this.post({
          type: "status",
          state: "blocked",
          code: "verified-chat.response.invalid",
        });
        return;
      }
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
      await this.post({
        type: "message",
        role: "assistant",
        text: turn.output_text,
      });
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
    if (response.kind === "denied") {
      await this.post({
        type: "status",
        state: "blocked",
        code: response.code,
      });
      return;
    }
    this.sessionId = sessionId;
    this.mode = mode;
    this.latestPlan = undefined;
    this.approvedPlan = undefined;
    await this.post({ type: "session", sessionId, mode });
    await this.refreshSessions();
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
    await this.panel?.webview.postMessage(message);
  }
}

function parseWebviewMessage(value: unknown): WebviewMessage | undefined {
  if (!isRecord(value) || typeof value.type !== "string") return undefined;
  const allowed = new Set([
    "ready",
    "newSession",
    "capturePaste",
    "send",
    "approvePlan",
    "startApprovedPlan",
    "selectModel",
  ]);
  if (!allowed.has(value.type)) return undefined;
  if (
    value.text !== undefined &&
    (typeof value.text !== "string" || value.text.length > 64 * 1024 * 1024)
  ) {
    return undefined;
  }
  if (
    value.mode !== undefined &&
    (typeof value.mode !== "string" ||
      !new Set(["ask", "plan", "agent", "team"]).has(value.mode))
  ) {
    return undefined;
  }
  return value as unknown as WebviewMessage;
}

function sidebarHtml(): string {
  const nonce = randomBytes(18).toString("base64");
  return `<!doctype html><html><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'nonce-${nonce}'; script-src 'nonce-${nonce}'"><style nonce="${nonce}">body{padding:12px;color:var(--vscode-foreground);font-family:var(--vscode-font-family)}button{width:100%;height:32px;color:var(--vscode-button-foreground);background:var(--vscode-button-background);border:0}#sessions{margin-top:12px;font-size:12px;color:var(--vscode-descriptionForeground)}</style></head><body><button id="open">Open Verified Chat</button><div id="sessions"></div><script nonce="${nonce}">const vscode=acquireVsCodeApi();document.getElementById('open').addEventListener('click',()=>vscode.postMessage({type:'open'}));window.addEventListener('message',event=>{if(event.data?.type==='sessions'){document.getElementById('sessions').textContent=Array.isArray(event.data.sessions)?event.data.sessions.length+' sessions':'';}});</script></body></html>`;
}

function editorHtml(webview: vscode.Webview): string {
  const nonce = randomBytes(18).toString("base64");
  const cspSource = webview.cspSource;
  return `<!doctype html><html><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src ${cspSource}; style-src 'nonce-${nonce}'; script-src 'nonce-${nonce}'"><meta name="viewport" content="width=device-width,initial-scale=1"><style nonce="${nonce}">*{box-sizing:border-box}body{margin:0;color:var(--vscode-foreground);background:var(--vscode-editor-background);font-family:var(--vscode-font-family);height:100vh;display:grid;grid-template-rows:42px 1fr auto}.bar{display:flex;gap:8px;align-items:center;padding:6px 12px;border-bottom:1px solid var(--vscode-panel-border)}select,textarea,button{font:inherit;color:inherit}select,textarea{background:var(--vscode-input-background);border:1px solid var(--vscode-input-border);color:var(--vscode-input-foreground)}button{height:30px;border:0;background:var(--vscode-button-background);color:var(--vscode-button-foreground);padding:0 12px}button:disabled{opacity:.55}.messages{overflow:auto;padding:16px;display:flex;flex-direction:column;gap:10px}.message{white-space:pre-wrap;line-height:1.45;max-width:900px}.user{border-left:3px solid var(--vscode-charts-blue);padding-left:10px}.status{color:var(--vscode-descriptionForeground);font-size:12px}.artifacts{color:var(--vscode-charts-green);font-size:12px}.composer{padding:10px 12px;border-top:1px solid var(--vscode-panel-border);display:grid;grid-template-columns:1fr auto;gap:8px}textarea{resize:vertical;min-height:72px;max-height:240px;padding:8px;letter-spacing:0}</style></head><body><header class="bar"><strong>AgentMage</strong><select id="mode" aria-label="Mode"><option value="ask">Ask</option><option value="plan">Plan</option><option value="agent">Agent</option><option value="team">Team</option></select><select id="model" aria-label="Model"><option value="">No qualified model</option></select><button id="approve" disabled>Approve Plan</button><button id="startAgent" disabled>Start Agent</button><button id="startTeam" disabled>Start Team</button><button id="new">New</button><span id="session" class="status"></span></header><main id="messages" class="messages" aria-live="polite"></main><footer class="composer"><textarea id="composer" aria-label="Message"></textarea><button id="send">Send</button></footer><script nonce="${nonce}">const vscode=acquireVsCodeApi(),composer=document.getElementById('composer'),messages=document.getElementById('messages'),mode=document.getElementById('mode'),model=document.getElementById('model'),approve=document.getElementById('approve'),startAgent=document.getElementById('startAgent'),startTeam=document.getElementById('startTeam');let planApproved=false;function post(type,extra={}){vscode.postMessage({type,...extra});}function add(text,kind='status'){const item=document.createElement('div');item.className='message '+kind;item.textContent=text;messages.appendChild(item);messages.scrollTop=messages.scrollHeight;}function actions(){approve.disabled=planApproved;startAgent.disabled=!planApproved||!model.value;startTeam.disabled=!planApproved;}composer.addEventListener('paste',event=>{const text=event.clipboardData?.getData('text/plain');if(typeof text!=='string')return;event.preventDefault();const start=composer.selectionStart,end=composer.selectionEnd;composer.setRangeText(text,start,end,'end');post('capturePaste',{text});});model.addEventListener('change',()=>post('selectModel',{text:model.value}));document.getElementById('new').addEventListener('click',()=>post('newSession',{mode:mode.value}));approve.addEventListener('click',()=>post('approvePlan'));startAgent.addEventListener('click',()=>post('startApprovedPlan',{mode:'agent'}));startTeam.addEventListener('click',()=>post('startApprovedPlan',{mode:'team'}));document.getElementById('send').addEventListener('click',()=>{const text=composer.value;if(!text.trim())return;post('send',{text,mode:mode.value});composer.value='';});window.addEventListener('message',event=>{const data=event.data;if(data?.type==='session'){document.getElementById('session').textContent=data.sessionId??'';mode.value=data.mode??'ask';planApproved=false;actions();}else if(data?.type==='models'){model.replaceChildren();for(const entry of data.models??[]){const option=document.createElement('option');option.value=entry.id;option.textContent=entry.name;model.appendChild(option);}if(model.options.length===0){const option=document.createElement('option');option.value='';option.textContent='No qualified model';model.appendChild(option);}model.value=data.selectedProfileId??'';actions();}else if(data?.type==='modelSelected'){model.value=data.profileId??'';actions();}else if(data?.type==='message'){add(data.text,data.role==='user'?'user':'assistant');}else if(data?.type==='artifact'){add(data.displayName+' · '+data.byteLength+' bytes · '+data.sourceSha256.slice(0,12),'artifacts');}else if(data?.type==='planState'){planApproved=Boolean(data.approved);actions();}else if(data?.type==='status'){add(data.code,'status');}});actions();post('ready');</script></body></html>`;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function asRecord(value: unknown): Record<string, unknown> | undefined {
  return isRecord(value) ? value : undefined;
}
