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
import type { HandoffReview } from "./handoff.js";
import type { RuntimeApprovalChallengeEnvelope } from "./runtime_transport.js";
import { VerifiedChatSurface } from "./verified_chat.js";
import {
  PARTICIPANT_ID,
  MAX_PARTICIPANT_TOTAL_BYTES,
  ParticipantIngressError,
  accountProviderParts,
  renderParticipantSource,
  runParticipantIngress,
  type ParticipantReferenceInput,
  type ParticipantReferenceResolver,
} from "./participant_ingress.js";

type AgentMageModelInformation = vscode.LanguageModelChatInformation &
  NativeModelInformation;

const registrationSlot = new RegistrationSlot();
let activeController: SecureReadController | undefined;
let participantRegistration: vscode.ChatParticipant | undefined;

class VsCodeParticipantResolver implements ParticipantReferenceResolver {
  async resolve(reference: ParticipantReferenceInput) {
    if (reference.uri === undefined) {
      throw new ParticipantIngressError("participant.source.uri-unavailable");
    }
    const uri = vscode.Uri.parse(reference.uri, true);
    const before = await vscode.workspace.fs.stat(uri);
    if (before.size <= 0 || before.size > MAX_PARTICIPANT_TOTAL_BYTES) {
      throw new ParticipantIngressError("participant.source.size-unsupported");
    }
    const bytes = await vscode.workspace.fs.readFile(uri);
    const after = await vscode.workspace.fs.stat(uri);
    return {
      bytes,
      displayName: reference.displayName,
      mediaType: reference.mediaType,
      revisionBefore: `${before.type.toString()}:${before.size.toString()}:${before.mtime.toString()}`,
      revisionAfter: `${after.type.toString()}:${after.size.toString()}:${after.mtime.toString()}`,
    };
  }
}

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
      "Review Read Preview",
    );
    return selection === "Review Read Preview";
  }

  async confirmRead(preview: ReadPreview): Promise<boolean> {
    const selection = await vscode.window.showWarningMessage(
      `Read ${preview.components.join("/")} (${preview.byte_len.toString()} bytes, ${preview.content_sha256})? No state change is permitted.`,
      { modal: true },
      "Approve Read",
    );
    return selection === "Approve Read";
  }

  async selectDiagnosticDestination(): Promise<string | undefined> {
    const destination = await vscode.window.showSaveDialog({
      saveLabel: "Select Diagnostic Export Destination",
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
      "Write Diagnostic Export",
    );
    return selection === "Write Diagnostic Export";
  }

  async confirmHandoff(review: HandoffReview): Promise<{
    readonly approved: boolean;
    readonly nonPublicAcknowledged: boolean;
  }> {
    const approveLabel = review.manifest.acknowledgment_required
      ? "Acknowledge and Render Local Packet"
      : "Render Local Packet";
    const selection = await vscode.window.showWarningMessage(
      [
        review.local_only_notice,
        "",
        review.packet_markdown,
        "",
        `Packet: ${review.manifest.packet_sha256}`,
        `Size: ${review.manifest.packet_bytes.toString()} bytes`,
      ].join("\n"),
      { modal: true },
      approveLabel,
    );
    return {
      approved: selection === approveLabel,
      nonPublicAcknowledged:
        selection === approveLabel && review.manifest.acknowledgment_required,
    };
  }

  async confirmRuntimeApproval(
    challenge: RuntimeApprovalChallengeEnvelope,
  ): Promise<"allow" | "deny"> {
    const selection = await vscode.window.showWarningMessage(
      [
        `Allow this ${challenge.operation.replaceAll("_", " ")} operation once?`,
        `Preview: ${challenge.preview_sha256}.`,
        `Approval: ${challenge.approval_id}.`,
      ].join(" "),
      { modal: true },
      "Allow Once",
      "Deny",
    );
    return selection === "Allow Once" ? "allow" : "deny";
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
  const verifiedChat = new VerifiedChatSurface(
    context.extensionUri,
    bridge,
    controller,
  );
  verifiedChat.register(context);
  participantRegistration = vscode.chat.createChatParticipant(
    PARTICIPANT_ID,
    async (request, _chatContext, response, token) => {
      const requestId = `participant-${stableHash(
        `${Date.now().toString()}:${request.prompt}:${request.references.length.toString()}`,
      )}`;
      const snapshot = await controller.discoverModels(token);
      const selected = snapshot?.entries.find(
        (entry) =>
          entry.profile_id === request.model.id &&
          entry.disposition === "selectable",
      );
      if (selected === undefined) {
        response.markdown(
          "# Request stopped\n\nThe selected model is not an exact currently admitted AgentMage profile. No source, model, or tool work started.\n\n- Code: `vscode.participant.model-unavailable`",
        );
        return;
      }
      const stopped = await controller.revalidateSelectedModel(
        selected.profile_id,
        selected.entry_sha256,
        token,
      );
      if (stopped !== undefined) {
        response.markdown(stopped.text);
        return;
      }
      const references = request.references.map((reference, index) =>
        participantReference(reference, index),
      );
      try {
        const result = await runParticipantIngress(
          async (hostRequest) => {
            const hostResponse = await bridge.engineering(hostRequest);
            return hostResponse.kind === "engineering"
              ? hostResponse
              : {
                  kind: "denied" as const,
                  code:
                    hostResponse.kind === "denied"
                      ? hostResponse.code
                      : "vscode.participant.host-response-invalid",
                };
          },
          new VsCodeParticipantResolver(),
          {
            requestId,
            prompt: request.prompt,
            command: request.command,
            references,
          },
          token,
          (record) => response.progress(renderParticipantSource(record)),
          Date.now,
          async () => {
            const current = await controller.revalidateSelectedModel(
              selected.profile_id,
              selected.entry_sha256,
              token,
            );
            if (current !== undefined) {
              throw new ParticipantIngressError(
                "vscode.participant.model-stale-before-submit",
              );
            }
          },
        );
        response.markdown(
          `${result.outputText}\n\n---\nSource manifest: \`${result.sourceManifestSha256}\`; ${result.sources.length.toString()} supplied reference(s) completely accounted for; ${result.totalBytes.toString()} / ${MAX_PARTICIPANT_TOTAL_BYTES.toString()} bytes used.`,
        );
      } catch (error) {
        const code =
          error instanceof ParticipantIngressError
            ? error.code
            : "vscode.participant.failed";
        response.markdown(
          `# Request stopped\n\nAgentMage could not safely account for this participant request. No unsupported source was treated as available.\n\n- Code: \`${code}\``,
        );
      }
    },
  );
  participantRegistration.iconPath = vscode.Uri.joinPath(
    context.extensionUri,
    "media",
    "agentmage.svg",
  );
  context.subscriptions.push(participantRegistration);
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
          for (const part of stopped.parts) {
            progress.report(new vscode.LanguageModelTextPart(part));
          }
          return;
        }
        const providerInput = lastUserParts(messages);
        if (providerInput.reasonCode !== null) {
          progress.report(
            new vscode.LanguageModelTextPart(
              `# Request stopped\n\nThe provider received ${providerInput.unsupportedParts.toString()} unsupported part(s) out of ${providerInput.totalParts.toString()}. AgentMage did not silently discard them. Retry through \`@agentmage\`, which accounts for every stable request reference.\n\n- Code: \`${providerInput.reasonCode}\``,
            ),
          );
          return;
        }
        const prompt = providerInput.text;
        let reportedParts = 0;
        const response = await controller.respond(
          prompt,
          token,
          {
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
          },
          (part) => {
            reportedParts += 1;
            progress.report(new vscode.LanguageModelTextPart(part));
          },
        );
        for (
          let index = reportedParts;
          index < response.parts.length;
          index += 1
        ) {
          const part = response.parts[index];
          if (part !== undefined) {
            progress.report(new vscode.LanguageModelTextPart(part));
          }
        }
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
  participantRegistration?.dispose();
  participantRegistration = undefined;
  await controller?.dispose();
}

function lastUserParts(
  messages: readonly vscode.LanguageModelChatRequestMessage[],
) {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index];
    if (message?.role === vscode.LanguageModelChatMessageRole.User) {
      return accountProviderParts(
        message.content.map((part) =>
          part instanceof vscode.LanguageModelTextPart
            ? { kind: "text" as const, text: part.value }
            : { kind: "unknown" as const },
        ),
      );
    }
  }
  return accountProviderParts([]);
}

function requestText(message: vscode.LanguageModelChatRequestMessage): string {
  const accounting = accountProviderParts(
    message.content.map((part) =>
      part instanceof vscode.LanguageModelTextPart
        ? { kind: "text" as const, text: part.value }
        : { kind: "unknown" as const },
    ),
  );
  return accounting.text;
}

function participantReference(
  reference: vscode.ChatPromptReference,
  index: number,
): ParticipantReferenceInput {
  const referenceId = `reference-${(index + 1).toString().padStart(4, "0")}`;
  const range = reference.range as readonly [number, number] | undefined;
  if (typeof reference.value === "string") {
    return {
      referenceId,
      kind: "text",
      text: reference.value,
      displayName: `Text reference ${index + 1}`,
      mediaType: "text/plain; charset=utf-8",
      ...(range === undefined ? {} : { range }),
    };
  }
  const uri =
    reference.value instanceof vscode.Uri
      ? reference.value
      : reference.value instanceof vscode.Location
        ? reference.value.uri
        : undefined;
  if (uri !== undefined) {
    return {
      referenceId,
      kind: reference.value instanceof vscode.Location ? "location" : "uri",
      uri: uri.toString(),
      displayName: `Attached reference ${index + 1}`,
      mediaType: mediaType(uri.path),
      ...(range === undefined ? {} : { range }),
    };
  }
  return {
    referenceId,
    kind: "unknown",
    displayName: `Unsupported reference ${index + 1}`,
    mediaType: "application/octet-stream",
    ...(range === undefined ? {} : { range }),
  };
}

function mediaType(path: string): string {
  const suffix = path.toLowerCase().split(".").pop();
  return (
    {
      json: "application/json",
      log: "text/plain; charset=utf-8",
      md: "text/markdown; charset=utf-8",
      txt: "text/plain; charset=utf-8",
    }[suffix ?? ""] ?? "application/octet-stream"
  );
}

function stableHash(value: string): string {
  let hash = 0x811c9dc5;
  for (const byte of Buffer.from(value, "utf8")) {
    hash ^= byte;
    hash = Math.imul(hash, 0x01000193);
  }
  return (hash >>> 0).toString(16).padStart(8, "0");
}
