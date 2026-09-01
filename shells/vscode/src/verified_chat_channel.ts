import type { EngineeringMode } from "./verified_chat_protocol.js";

export type VerifiedChatWebviewMessage =
  | Readonly<{ type: "ready"; channel: string; sequence: number }>
  | Readonly<{
      type: "newSession";
      channel: string;
      sequence: number;
      mode: EngineeringMode;
    }>
  | Readonly<{
      type: "capturePaste";
      channel: string;
      sequence: number;
      text: string;
    }>
  | Readonly<{
      type: "attachFiles";
      channel: string;
      sequence: number;
      mode: EngineeringMode;
    }>
  | Readonly<{
      type: "send";
      channel: string;
      sequence: number;
      text: string;
      mode: EngineeringMode;
    }>
  | Readonly<{
      type: "approvePlan";
      channel: string;
      sequence: number;
    }>
  | Readonly<{
      type: "startApprovedPlan";
      channel: string;
      sequence: number;
      mode: "agent" | "team";
    }>
  | Readonly<{
      type: "selectModel";
      channel: string;
      sequence: number;
      text: string;
    }>
  | Readonly<{
      type: "pause" | "resume" | "cancel";
      channel: string;
      sequence: number;
    }>;

export class VerifiedChatChannelError extends Error {
  constructor(code: string) {
    super(code);
  }
}

/** Presentation-only single-run gate; it can cancel but cannot launch or complete runtime work. */
export class VerifiedChatRunGate {
  private active:
    | {
        readonly runId: string;
        readonly sessionId: string;
        readonly cancel: () => void;
      }
    | undefined;

  get isActive(): boolean {
    return this.active !== undefined;
  }

  begin(runId: string, sessionId: string, cancel: () => void): void {
    if (
      this.active !== undefined ||
      !validIdentifier(runId) ||
      !validIdentifier(sessionId)
    ) {
      throw new VerifiedChatChannelError("verified-chat.agent.run-active");
    }
    this.active = { runId, sessionId, cancel };
  }

  allows(command: VerifiedChatWebviewMessage["type"]): boolean {
    return this.active === undefined || command === "cancel";
  }

  cancel(sessionId: string): boolean {
    if (this.active?.sessionId !== sessionId) return false;
    this.active.cancel();
    return true;
  }

  cancelActive(): void {
    this.active?.cancel();
  }

  finish(runId: string): void {
    if (this.active?.runId === runId) this.active = undefined;
  }
}

/** One-use ordered command channel for a disposable webview instance. */
export class VerifiedChatCommandChannel {
  private nextSequence = 0;

  constructor(private readonly channel: string) {
    if (!validIdentifier(channel)) {
      throw new VerifiedChatChannelError("verified-chat.channel.invalid");
    }
  }

  accept(value: unknown): VerifiedChatWebviewMessage {
    const message = record(value);
    if (
      message.channel !== this.channel ||
      message.sequence !== this.nextSequence ||
      !Number.isSafeInteger(message.sequence) ||
      typeof message.type !== "string"
    ) {
      throw new VerifiedChatChannelError("verified-chat.message.replayed");
    }
    const common = ["channel", "sequence", "type"];
    switch (message.type) {
      case "ready":
      case "approvePlan":
      case "pause":
      case "resume":
      case "cancel":
        exactKeys(message, common);
        break;
      case "newSession":
      case "attachFiles":
        exactKeys(message, [...common, "mode"]);
        requireMode(message.mode);
        break;
      case "capturePaste":
      case "selectModel":
        exactKeys(message, [...common, "text"]);
        requireText(message.text);
        break;
      case "send":
        exactKeys(message, [...common, "mode", "text"]);
        requireMode(message.mode);
        requireText(message.text);
        break;
      case "startApprovedPlan":
        exactKeys(message, [...common, "mode"]);
        if (message.mode !== "agent" && message.mode !== "team") invalid();
        break;
      default:
        return invalid();
    }
    this.nextSequence += 1;
    return message as unknown as VerifiedChatWebviewMessage;
  }
}

function requireMode(value: unknown): asserts value is EngineeringMode {
  if (!new Set(["ask", "plan", "agent", "team"]).has(value as string)) {
    invalid();
  }
}

function requireText(value: unknown): asserts value is string {
  if (typeof value !== "string" || value.length > 64 * 1024 * 1024) invalid();
}

function record(value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    invalid();
  }
  return value as Record<string, unknown>;
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
    invalid();
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

function invalid(): never {
  throw new VerifiedChatChannelError("verified-chat.message.invalid");
}
