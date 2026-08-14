import * as vscode from "vscode";

import { RegistrationSlot } from "./index.js";
import { launchInstalledHost } from "./host_bootstrap.js";
import {
  PROVIDER_VENDOR,
  type DiagnosticExportPreview,
  SecureReadController,
  SessionRequestIdentitySource,
  type ApprovalUi,
  type LocalWorkspace,
  type ReadPreview,
  type WorkspaceSource,
} from "./provider.js";
import {
  selectableModelInformation,
  type NativeModelInformation,
} from "./model_discovery.js";

type AgentMageModelInformation = vscode.LanguageModelChatInformation &
  NativeModelInformation;

const registrationSlot = new RegistrationSlot();
let activeController: SecureReadController | undefined;

class VsCodeWorkspaceSource implements WorkspaceSource {
  selectedLocalWorkspace(): LocalWorkspace | undefined {
    const folders = vscode.workspace.workspaceFolders;
    if (vscode.env.remoteName !== undefined || folders?.length !== 1) {
      return undefined;
    }
    const folder = folders[0];
    if (folder === undefined || folder.uri.scheme !== "file") {
      return undefined;
    }
    return {
      id: `workspace-${stableHash(folder.uri.toString())}`,
      name: folder.name,
      root: folder.uri.fsPath,
    };
  }
}

class VsCodeApprovalUi implements ApprovalUi {
  async confirmWorkspace(
    workspace: LocalWorkspace,
    components: readonly string[],
  ): Promise<boolean> {
    const selection = await vscode.window.showWarningMessage(
      `Allow a bounded read preview for ${workspace.name}/${components.join("/")}?`,
      { modal: true },
      "Continue",
    );
    return selection === "Continue";
  }

  async confirmRead(preview: ReadPreview): Promise<boolean> {
    const selection = await vscode.window.showWarningMessage(
      `Read ${preview.components.join("/")} (${preview.byte_len.toString()} bytes, ${preview.content_sha256})? No state change is permitted.`,
      { modal: true },
      "Read",
    );
    return selection === "Read";
  }

  async selectDiagnosticDestination(): Promise<string | undefined> {
    const destination = await vscode.window.showSaveDialog({
      saveLabel: "Review Export",
      filters: { JSON: ["json"] },
      title: "Select a private local diagnostic export destination",
    });
    return destination?.scheme === "file" ? destination.fsPath : undefined;
  }

  async confirmDiagnosticExport(
    preview: DiagnosticExportPreview,
    destination: string,
  ): Promise<boolean> {
    const selection = await vscode.window.showWarningMessage(
      [
        `Write ${preview.payload_bytes.toString()} diagnostic bytes to ${destination}?`,
        `Fields: ${preview.included_fields.join(", ")}.`,
        `Redactions: ${preview.redactions.join(", ")}.`,
        `Sensitivity: ${preview.sensitivity}. Retention: ${preview.retention}.`,
        `Payload: ${preview.payload_sha256}.`,
      ].join(" "),
      { modal: true },
      "Export",
    );
    return selection === "Export";
  }
}

/** Activates the real VS Code provider with a fail-closed host bridge. */
export async function activate(
  context: vscode.ExtensionContext,
): Promise<void> {
  await deactivate();
  const bridge = await launchInstalledHost();
  const controller = new SecureReadController(
    bridge,
    new VsCodeWorkspaceSource(),
    new VsCodeApprovalUi(),
    new SessionRequestIdentitySource(),
  );
  activeController = controller;
  const provider: vscode.LanguageModelChatProvider<AgentMageModelInformation> =
    {
      provideLanguageModelChatInformation: async (_options, token) => {
        if (token.isCancellationRequested) {
          return [];
        }
        const snapshot = await controller.discoverModels(token);
        return snapshot === undefined
          ? []
          : selectableModelInformation(snapshot).map((model) => ({ ...model }));
      },
      provideLanguageModelChatResponse: async (
        model,
        messages,
        _options,
        progress,
        token,
      ) => {
        const stopped = await controller.revalidateSelectedModel(
          model.id,
          model.entrySha256,
          token,
        );
        if (stopped !== undefined) {
          progress.report(new vscode.LanguageModelTextPart(stopped.text));
          return;
        }
        const prompt = lastUserText(messages);
        const response = await controller.respond(prompt, token);
        progress.report(new vscode.LanguageModelTextPart(response.text));
      },
      provideTokenCount: (_model, value) => {
        const text = typeof value === "string" ? value : requestText(value);
        return Promise.resolve(
          Math.max(1, Math.ceil(Buffer.byteLength(text, "utf8") / 4)),
        );
      },
    };
  const registration = vscode.lm.registerLanguageModelChatProvider(
    PROVIDER_VENDOR,
    provider,
  );
  registrationSlot.replace(registration);
  context.subscriptions.push(registration);
}

/** Disposes provider registration and any retained local bridge state. */
export async function deactivate(): Promise<void> {
  const controller = activeController;
  activeController = undefined;
  registrationSlot.dispose();
  await controller?.dispose();
}

function lastUserText(
  messages: readonly vscode.LanguageModelChatRequestMessage[],
): string {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index];
    if (message?.role === vscode.LanguageModelChatMessageRole.User) {
      return requestText(message);
    }
  }
  return "";
}

function requestText(message: vscode.LanguageModelChatRequestMessage): string {
  return message.content
    .filter(
      (part): part is vscode.LanguageModelTextPart =>
        part instanceof vscode.LanguageModelTextPart,
    )
    .map((part) => part.value)
    .join("");
}

function stableHash(value: string): string {
  let hash = 0x811c9dc5;
  for (const byte of Buffer.from(value, "utf8")) {
    hash ^= byte;
    hash = Math.imul(hash, 0x01000193);
  }
  return (hash >>> 0).toString(16).padStart(8, "0");
}
