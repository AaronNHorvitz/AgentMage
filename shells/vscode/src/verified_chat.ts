import { randomBytes, randomUUID } from "node:crypto";

import * as vscode from "vscode";

import type { HostBridge, HostResponse } from "./provider.js";
import {
  captureExactText,
  request,
  type EngineeringHostResponse,
  type EngineeringMode,
} from "./verified_chat_protocol.js";

const VIEW_TYPE = "agentmage.verifiedChat";
const VIEW_ID = "agentmage.sessions";

interface WebviewMessage {
  readonly type: "ready" | "newSession" | "capturePaste" | "send";
  readonly text?: string;
  readonly mode?: EngineeringMode;
}

/** Dedicated, reconnectable AgentMage Verified Chat editor and Activity Bar surface. */
export class VerifiedChatSurface implements vscode.WebviewViewProvider {
  private panel: vscode.WebviewPanel | undefined;
  private view: vscode.WebviewView | undefined;
  private sessionId: string | undefined;
  private mode: EngineeringMode = "ask";

  constructor(
    private readonly extensionUri: vscode.Uri,
    private readonly bridge: HostBridge,
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
      return;
    }
    if (message.type === "newSession") {
      await this.createSession(message.mode ?? "ask");
      return;
    }
    const requestedMode = message.mode ?? this.mode;
    if (this.sessionId === undefined || requestedMode !== this.mode) {
      await this.createSession(requestedMode);
    }
    const sessionId = this.sessionId;
    if (sessionId === undefined) return;
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

  private async post(
    message: Readonly<Record<string, unknown>>,
  ): Promise<void> {
    await this.panel?.webview.postMessage(message);
  }
}

function parseWebviewMessage(value: unknown): WebviewMessage | undefined {
  if (!isRecord(value) || typeof value.type !== "string") return undefined;
  const allowed = new Set(["ready", "newSession", "capturePaste", "send"]);
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
  return `<!doctype html><html><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src ${cspSource}; style-src 'nonce-${nonce}'; script-src 'nonce-${nonce}'"><meta name="viewport" content="width=device-width,initial-scale=1"><style nonce="${nonce}">*{box-sizing:border-box}body{margin:0;color:var(--vscode-foreground);background:var(--vscode-editor-background);font-family:var(--vscode-font-family);height:100vh;display:grid;grid-template-rows:42px 1fr auto}.bar{display:flex;gap:8px;align-items:center;padding:6px 12px;border-bottom:1px solid var(--vscode-panel-border)}select,textarea,button{font:inherit;color:inherit}select,textarea{background:var(--vscode-input-background);border:1px solid var(--vscode-input-border);color:var(--vscode-input-foreground)}button{height:30px;border:0;background:var(--vscode-button-background);color:var(--vscode-button-foreground);padding:0 12px}.messages{overflow:auto;padding:16px;display:flex;flex-direction:column;gap:10px}.message{white-space:pre-wrap;line-height:1.45;max-width:900px}.user{border-left:3px solid var(--vscode-charts-blue);padding-left:10px}.status{color:var(--vscode-descriptionForeground);font-size:12px}.artifacts{color:var(--vscode-charts-green);font-size:12px}.composer{padding:10px 12px;border-top:1px solid var(--vscode-panel-border);display:grid;grid-template-columns:1fr auto;gap:8px}textarea{resize:vertical;min-height:72px;max-height:240px;padding:8px;letter-spacing:0}</style></head><body><header class="bar"><strong>AgentMage</strong><select id="mode" aria-label="Mode"><option value="ask">Ask</option><option value="plan">Plan</option><option value="agent">Agent</option><option value="team">Team</option></select><button id="new">New</button><span id="session" class="status"></span></header><main id="messages" class="messages" aria-live="polite"></main><footer class="composer"><textarea id="composer" aria-label="Message"></textarea><button id="send">Send</button></footer><script nonce="${nonce}">const vscode=acquireVsCodeApi(),composer=document.getElementById('composer'),messages=document.getElementById('messages'),mode=document.getElementById('mode');function post(type,extra={}){vscode.postMessage({type,...extra});}function add(text,kind='status'){const item=document.createElement('div');item.className='message '+kind;item.textContent=text;messages.appendChild(item);messages.scrollTop=messages.scrollHeight;}composer.addEventListener('paste',event=>{const text=event.clipboardData?.getData('text/plain');if(typeof text!=='string')return;event.preventDefault();const start=composer.selectionStart,end=composer.selectionEnd;composer.setRangeText(text,start,end,'end');post('capturePaste',{text});});document.getElementById('new').addEventListener('click',()=>post('newSession',{mode:mode.value}));document.getElementById('send').addEventListener('click',()=>{const text=composer.value;if(!text.trim())return;post('send',{text,mode:mode.value});composer.value='';});window.addEventListener('message',event=>{const data=event.data;if(data?.type==='session'){document.getElementById('session').textContent=data.sessionId??'';mode.value=data.mode??'ask';}else if(data?.type==='message'){add(data.text,data.role==='user'?'user':'assistant');}else if(data?.type==='artifact'){add(data.displayName+' · '+data.byteLength+' bytes · '+data.sourceSha256.slice(0,12),'artifacts');}else if(data?.type==='status'){add(data.code,'status');}});post('ready');</script></body></html>`;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function asRecord(value: unknown): Record<string, unknown> | undefined {
  return isRecord(value) ? value : undefined;
}
