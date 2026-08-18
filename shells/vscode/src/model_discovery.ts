import { createHash } from "node:crypto";

export type ModelRole =
  | "dialogue"
  | "coding_planner"
  | "tool_selection"
  | "safety_classification"
  | "embedding"
  | "reranking"
  | "multimodal";

export type ModelCapabilityState =
  "not_evaluated" | "blocked" | "failed" | "passed" | "not_applicable";

export type ModelLifecycleState =
  | "candidate"
  | "evaluating"
  | "approved"
  | "degraded"
  | "quarantined"
  | "rejected"
  | "retired";

export type ModelHealthState =
  "unloaded" | "loading" | "ready" | "degraded" | "quarantined" | "failed";

export interface ModelPickerCapability {
  readonly role: ModelRole;
  readonly state: ModelCapabilityState;
  readonly limitations: readonly string[];
}

export interface ModelPickerEntry {
  readonly profile_id: string;
  readonly display_name: string;
  readonly family: string;
  readonly manifest_sha256: string;
  readonly artifact_sha256: string;
  readonly publisher: string;
  readonly publisher_control: string;
  readonly lineage: readonly string[];
  readonly license_spdx: string;
  readonly license_terms_sha256: string;
  readonly source_revision: string;
  readonly artifact_format: string;
  readonly artifact_bytes: number;
  readonly quantization: string;
  readonly codec_id: string;
  readonly codec_sha256: string;
  readonly tokenizer_sha256: string;
  readonly template_sha256: string;
  readonly runtime_adapter_id: string;
  readonly runtime_kind:
    | "deterministic_fake"
    | "native_llama_cpp"
    | "docker_model_runner"
    | "macos_metal_llama_cpp";
  readonly runtime_contract_version: number;
  readonly runtime_build: string;
  readonly runtime_sha256: string;
  readonly platform:
    "deterministic_fake" | "fedora" | "ubuntu" | "mac_os_apple_silicon";
  readonly architecture: "x86_64" | "aarch64";
  readonly modalities: readonly ("text" | "image" | "audio" | "embedding")[];
  readonly max_context_tokens: number;
  readonly max_input_bytes: number;
  readonly max_messages: number;
  readonly context_sha256: string;
  readonly max_output_tokens: number;
  readonly decoding_sha256: string;
  readonly hardware_sha256: string;
  readonly policy_sha256: string;
  readonly tool_calling: boolean;
  readonly capabilities: readonly ModelPickerCapability[];
  readonly lifecycle: ModelLifecycleState;
  readonly runtime_health: ModelHealthState;
  readonly activation: "activated" | "inactive" | "blocked" | "stale";
  readonly compatibility: "compatible" | "incompatible" | "blocked" | "stale";
  readonly support: "supported" | "limited" | "unsupported" | "stale";
  readonly limitations: readonly string[];
  readonly requires_user_decision: boolean;
  readonly disposition: "selectable" | "management_only";
  readonly entry_sha256: string;
}

export interface ModelPickerSnapshot {
  readonly schema_version: 1;
  readonly catalog_sha256: string;
  readonly catalog_signature_verified: true;
  readonly observed_at_ms: number;
  readonly entries: readonly ModelPickerEntry[];
  readonly snapshot_sha256: string;
}

export interface ModelSelectionRevalidation {
  readonly schema_version: 1;
  readonly profile_id: string;
  readonly expected_entry_sha256: string;
  readonly current_snapshot_sha256: string;
  readonly admitted: boolean;
  readonly result_code: string;
}

export interface NativeModelInformation {
  readonly id: string;
  readonly name: string;
  readonly family: string;
  readonly tooltip: string;
  readonly detail: string;
  readonly version: string;
  readonly maxInputTokens: number;
  readonly maxOutputTokens: number;
  readonly capabilities: {
    readonly imageInput: boolean;
    readonly toolCalling: boolean;
  };
  readonly entrySha256: string;
  readonly discoverySha256: string;
  readonly artifactSha256: string;
  readonly runtimeAdapterId: string;
  readonly runtimeSha256: string;
  readonly visionInput: boolean;
}

const SHA256 = /^[0-9a-f]{64}$/;
const IDENTIFIER = /^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/;
const CODE = /^[a-z0-9][a-z0-9._-]{0,127}$/;

const ROLES: readonly ModelRole[] = [
  "dialogue",
  "coding_planner",
  "tool_selection",
  "safety_classification",
  "embedding",
  "reranking",
  "multimodal",
];
const CAPABILITY_STATES: readonly ModelCapabilityState[] = [
  "not_evaluated",
  "blocked",
  "failed",
  "passed",
  "not_applicable",
];
const LIFECYCLE_STATES: readonly ModelLifecycleState[] = [
  "candidate",
  "evaluating",
  "approved",
  "degraded",
  "quarantined",
  "rejected",
  "retired",
];
const HEALTH_STATES: readonly ModelHealthState[] = [
  "unloaded",
  "loading",
  "ready",
  "degraded",
  "quarantined",
  "failed",
];

export function parseModelPickerSnapshot(
  candidate: unknown,
): ModelPickerSnapshot {
  const value = record(candidate);
  requireKeys(value, [
    "catalog_sha256",
    "catalog_signature_verified",
    "entries",
    "observed_at_ms",
    "schema_version",
    "snapshot_sha256",
  ]);
  if (
    value.schema_version !== 1 ||
    value.catalog_signature_verified !== true ||
    !sha(value.catalog_sha256) ||
    !positiveInteger(value.observed_at_ms) ||
    !Array.isArray(value.entries) ||
    value.entries.length > 64 ||
    !sha(value.snapshot_sha256)
  ) {
    throw new Error("model.discovery.snapshot-invalid");
  }
  const entries = value.entries.map(parseEntry);
  if (
    entries.some(
      (entry, index) =>
        index > 0 && entries[index - 1]!.profile_id >= entry.profile_id,
    )
  ) {
    throw new Error("model.discovery.snapshot-invalid");
  }
  const snapshot: ModelPickerSnapshot = {
    schema_version: 1,
    catalog_sha256: value.catalog_sha256,
    catalog_signature_verified: true,
    observed_at_ms: value.observed_at_ms,
    entries,
    snapshot_sha256: value.snapshot_sha256,
  };
  const unsigned = {
    schema_version: snapshot.schema_version,
    catalog_sha256: snapshot.catalog_sha256,
    catalog_signature_verified: snapshot.catalog_signature_verified,
    observed_at_ms: snapshot.observed_at_ms,
    entries: snapshot.entries,
  };
  if (digest(unsigned) !== snapshot.snapshot_sha256) {
    throw new Error("model.discovery.snapshot-invalid");
  }
  return snapshot;
}

export function parseModelSelectionRevalidation(
  candidate: unknown,
): ModelSelectionRevalidation {
  const value = record(candidate);
  requireKeys(value, [
    "admitted",
    "current_snapshot_sha256",
    "expected_entry_sha256",
    "profile_id",
    "result_code",
    "schema_version",
  ]);
  if (
    value.schema_version !== 1 ||
    !identifier(value.profile_id) ||
    !sha(value.expected_entry_sha256) ||
    !sha(value.current_snapshot_sha256) ||
    typeof value.admitted !== "boolean" ||
    !code(value.result_code)
  ) {
    throw new Error("model.selection.revalidation-invalid");
  }
  return value as unknown as ModelSelectionRevalidation;
}

export function selectableModelInformation(
  snapshot: ModelPickerSnapshot,
): readonly NativeModelInformation[] {
  return snapshot.entries
    .filter((entry) => entry.disposition === "selectable")
    .map((entry) => ({
      id: entry.profile_id,
      name: entry.display_name,
      family: entry.family,
      tooltip: tooltip(entry),
      detail: `Local | ${label(entry.runtime_health)} | ${entry.max_context_tokens.toLocaleString("en-US")} context tokens`,
      version: entry.manifest_sha256,
      maxInputTokens: entry.max_context_tokens,
      maxOutputTokens: entry.max_output_tokens,
      capabilities: {
        imageInput: entry.modalities.includes("image"),
        toolCalling: entry.tool_calling,
      },
      entrySha256: entry.entry_sha256,
      discoverySha256: snapshot.snapshot_sha256,
      artifactSha256: entry.artifact_sha256,
      runtimeAdapterId: entry.runtime_adapter_id,
      runtimeSha256: entry.runtime_sha256,
      visionInput: entry.modalities.includes("image"),
    }));
}

export function renderModelManagementReport(
  snapshot: ModelPickerSnapshot,
): string {
  const lines = [
    "# Local Model Profiles",
    "",
    `Catalog: verified (${shortHash(snapshot.catalog_sha256)})`,
    `Profiles: ${snapshot.entries.length.toString()}`,
  ];
  if (snapshot.entries.length === 0) {
    lines.push("", "No exact local model profile is currently registered.");
    return lines.join("\n");
  }
  for (const entry of snapshot.entries) {
    const selection =
      entry.disposition === "selectable" ? "Available" : "Unavailable";
    lines.push(
      "",
      `## ${escapeMarkdown(entry.display_name)}`,
      "",
      `- Selection: ${selection}; explicit user decision ${entry.requires_user_decision ? "required" : "not required"}`,
      `- Exact profile: \`${entry.profile_id}\``,
      `- Manifest: \`${entry.manifest_sha256}\``,
      `- Publisher: ${escapeMarkdown(entry.publisher)} (${escapeMarkdown(entry.publisher_control)})`,
      `- Lineage: ${entry.lineage.map((item) => escapeMarkdown(item)).join(" -> ")}`,
      `- License: \`${entry.license_spdx}\` (terms ${entry.license_terms_sha256})`,
      `- Artifact: ${escapeMarkdown(entry.artifact_format)}, ${entry.artifact_bytes.toLocaleString("en-US")} bytes, \`${entry.artifact_sha256}\``,
      `- Source revision: \`${entry.source_revision}\``,
      `- Quantization: ${escapeMarkdown(entry.quantization)}`,
      `- Runtime: \`${entry.runtime_adapter_id}\` contract ${entry.runtime_contract_version.toString()} (${label(entry.runtime_kind)}, ${entry.platform}/${entry.architecture}, ${entry.runtime_sha256})`,
      `- State: lifecycle ${label(entry.lifecycle)}; health ${label(entry.runtime_health)}; activation ${label(entry.activation)}; compatibility ${label(entry.compatibility)}; support ${label(entry.support)}`,
      `- Limits: ${entry.max_context_tokens.toLocaleString("en-US")} context tokens; ${entry.max_output_tokens.toLocaleString("en-US")} output tokens; ${entry.max_messages.toString()} messages`,
      `- Bound evidence: context ${shortHash(entry.context_sha256)}; decoding ${shortHash(entry.decoding_sha256)}; hardware ${shortHash(entry.hardware_sha256)}; policy ${shortHash(entry.policy_sha256)}`,
      `- Modalities: ${entry.modalities.map(label).join(", ")}`,
      `- Roles: ${entry.capabilities.map((item) => `${label(item.role)} (${label(item.state)})`).join(", ")}`,
      `- Tool selection: ${entry.tool_calling ? "available" : "unavailable"}`,
      `- Limitations: ${entry.limitations.length === 0 ? "none" : entry.limitations.map((item) => `\`${item}\``).join(", ")}`,
    );
  }
  return lines.join("\n");
}

/** Renders one exact review-only acquisition preflight and license screen. */
export function renderModelAcquisitionReview(
  snapshot: ModelPickerSnapshot,
  profileId: string,
): string | undefined {
  if (!identifier(profileId)) {
    return undefined;
  }
  const entry = snapshot.entries.find(
    (candidate) => candidate.profile_id === profileId,
  );
  if (entry === undefined) {
    return undefined;
  }
  return [
    "# Model Acquisition Review",
    "",
    `Exact profile: \`${entry.profile_id}\``,
    `Catalog: verified (${shortHash(snapshot.catalog_sha256)})`,
    "",
    "## Preflight Review",
    "",
    `- Compatibility: ${label(entry.compatibility)}`,
    `- Platform: ${entry.platform}/${entry.architecture}`,
    `- Runtime: \`${entry.runtime_adapter_id}\` contract ${entry.runtime_contract_version.toString()} (${entry.runtime_sha256})`,
    `- Artifact: ${escapeMarkdown(entry.artifact_format)}, ${entry.artifact_bytes.toLocaleString("en-US")} bytes, \`${entry.artifact_sha256}\``,
    `- Context limit: ${entry.max_context_tokens.toLocaleString("en-US")} tokens`,
    `- Hardware evidence: \`${entry.hardware_sha256}\``,
    `- Limitations: ${entry.limitations.length === 0 ? "none" : entry.limitations.map((item) => `\`${item}\``).join(", ")}`,
    "- Source opened: no",
    "- Destination changed: no",
    "- Acquisition started: no",
    "",
    "## License Review",
    "",
    `- Publisher: ${escapeMarkdown(entry.publisher)} (${escapeMarkdown(entry.publisher_control)})`,
    `- Lineage: ${entry.lineage.map((item) => escapeMarkdown(item)).join(" -> ")}`,
    `- License: \`${entry.license_spdx}\``,
    `- Reviewed terms digest: \`${entry.license_terms_sha256}\``,
    `- Source revision: \`${entry.source_revision}\``,
    `- Acceptance: ${entry.requires_user_decision ? "required before any acquisition" : "not available through this review"}`,
    "",
    "This screen is review-only. Import, download, activation, rollback, and model substitution are unavailable here.",
  ].join("\n");
}

function parseEntry(candidate: unknown): ModelPickerEntry {
  const value = record(candidate);
  requireKeys(value, [
    "activation",
    "architecture",
    "artifact_sha256",
    "publisher",
    "publisher_control",
    "lineage",
    "license_spdx",
    "license_terms_sha256",
    "source_revision",
    "artifact_format",
    "artifact_bytes",
    "quantization",
    "capabilities",
    "codec_id",
    "codec_sha256",
    "compatibility",
    "context_sha256",
    "decoding_sha256",
    "display_name",
    "disposition",
    "entry_sha256",
    "family",
    "hardware_sha256",
    "lifecycle",
    "limitations",
    "manifest_sha256",
    "max_context_tokens",
    "max_input_bytes",
    "max_messages",
    "max_output_tokens",
    "modalities",
    "platform",
    "policy_sha256",
    "profile_id",
    "requires_user_decision",
    "runtime_adapter_id",
    "runtime_build",
    "runtime_contract_version",
    "runtime_health",
    "runtime_kind",
    "runtime_sha256",
    "support",
    "template_sha256",
    "tokenizer_sha256",
    "tool_calling",
  ]);
  const capabilities = array(value.capabilities).map(parseCapability);
  const lineage = textArray(value.lineage);
  const limitations = codeArray(value.limitations);
  const modalities = enumArray(value.modalities, [
    "text",
    "image",
    "audio",
    "embedding",
  ] as const);
  if (
    !identifier(value.profile_id) ||
    !text(value.display_name) ||
    !text(value.family) ||
    !sha(value.manifest_sha256) ||
    !sha(value.artifact_sha256) ||
    !text(value.publisher) ||
    !text(value.publisher_control) ||
    lineage.length === 0 ||
    new Set(lineage).size !== lineage.length ||
    !text(value.license_spdx) ||
    !sha(value.license_terms_sha256) ||
    !text(value.source_revision) ||
    !text(value.artifact_format) ||
    !positiveInteger(value.artifact_bytes) ||
    !text(value.quantization) ||
    !identifier(value.codec_id) ||
    !sha(value.codec_sha256) ||
    !sha(value.tokenizer_sha256) ||
    !sha(value.template_sha256) ||
    !identifier(value.runtime_adapter_id) ||
    !oneOf(value.runtime_kind, [
      "deterministic_fake",
      "native_llama_cpp",
      "docker_model_runner",
      "macos_metal_llama_cpp",
    ] as const) ||
    !positiveInteger(value.runtime_contract_version) ||
    !text(value.runtime_build) ||
    !sha(value.runtime_sha256) ||
    !oneOf(value.platform, [
      "deterministic_fake",
      "fedora",
      "ubuntu",
      "mac_os_apple_silicon",
    ] as const) ||
    !oneOf(value.architecture, ["x86_64", "aarch64"] as const) ||
    modalities.length === 0 ||
    !positiveInteger(value.max_context_tokens) ||
    !positiveInteger(value.max_input_bytes) ||
    !positiveInteger(value.max_messages) ||
    !sha(value.context_sha256) ||
    !positiveInteger(value.max_output_tokens) ||
    !sha(value.decoding_sha256) ||
    !sha(value.hardware_sha256) ||
    !sha(value.policy_sha256) ||
    typeof value.tool_calling !== "boolean" ||
    capabilities.length === 0 ||
    !oneOf(value.lifecycle, LIFECYCLE_STATES) ||
    !oneOf(value.runtime_health, HEALTH_STATES) ||
    !oneOf(value.activation, [
      "activated",
      "inactive",
      "blocked",
      "stale",
    ] as const) ||
    !oneOf(value.compatibility, [
      "compatible",
      "incompatible",
      "blocked",
      "stale",
    ] as const) ||
    !oneOf(value.support, [
      "supported",
      "limited",
      "unsupported",
      "stale",
    ] as const) ||
    typeof value.requires_user_decision !== "boolean" ||
    !oneOf(value.disposition, ["selectable", "management_only"] as const) ||
    !sha(value.entry_sha256)
  ) {
    throw new Error("model.discovery.entry-invalid");
  }
  const entry = {
    ...value,
    capabilities,
    limitations,
    lineage,
    modalities,
  } as unknown as ModelPickerEntry;
  const toolCalling = capabilities.some(
    (item) => item.role === "tool_selection" && item.state === "passed",
  );
  const selectable =
    (entry.lifecycle === "approved" || entry.lifecycle === "degraded") &&
    entry.runtime_health === "ready" &&
    entry.activation === "activated" &&
    entry.compatibility === "compatible" &&
    (entry.support === "supported" || entry.support === "limited") &&
    capabilities.some((item) => item.state === "passed") &&
    entry.requires_user_decision;
  if (
    entry.tool_calling !== toolCalling ||
    (entry.disposition === "selectable" && !selectable)
  ) {
    throw new Error("model.discovery.entry-invalid");
  }
  const { entry_sha256: expectedDigest, ...unsigned } = entry;
  if (digest(unsigned) !== expectedDigest) {
    throw new Error("model.discovery.entry-invalid");
  }
  return entry;
}

function parseCapability(candidate: unknown): ModelPickerCapability {
  const value = record(candidate);
  requireKeys(value, ["limitations", "role", "state"]);
  if (!oneOf(value.role, ROLES) || !oneOf(value.state, CAPABILITY_STATES)) {
    throw new Error("model.discovery.capability-invalid");
  }
  return {
    role: value.role,
    state: value.state,
    limitations: codeArray(value.limitations),
  };
}

function tooltip(entry: ModelPickerEntry): string {
  const roles = entry.capabilities
    .filter((item) => item.state === "passed")
    .map((item) => label(item.role))
    .join(", ");
  const limitations =
    entry.limitations.length === 0 ? "none" : entry.limitations.join(", ");
  return [
    `Exact profile: ${entry.profile_id}`,
    `Runtime: ${entry.runtime_adapter_id} contract ${entry.runtime_contract_version.toString()} (${entry.runtime_kind}, ${entry.runtime_sha256})`,
    `Manifest: ${entry.manifest_sha256}`,
    `Publisher: ${entry.publisher} (${entry.publisher_control}); license: ${entry.license_spdx}`,
    `Artifact: ${entry.artifact_format} ${entry.artifact_bytes.toString()} bytes; ${entry.artifact_sha256}`,
    `Source: ${entry.source_revision}; quantization: ${entry.quantization}`,
    `Bound evidence: context ${entry.context_sha256}; decoding ${entry.decoding_sha256}; hardware ${entry.hardware_sha256}; policy ${entry.policy_sha256}`,
    `Roles: ${roles || "none"}`,
    `Limits: ${entry.max_context_tokens.toString()} context / ${entry.max_output_tokens.toString()} output tokens`,
    `Support: ${entry.support}; limitations: ${limitations}`,
    "Selection requires explicit user action; AgentMage never substitutes another model.",
  ].join("\n");
}

function digest(value: unknown): string {
  return createHash("sha256")
    .update(JSON.stringify(value), "utf8")
    .digest("hex");
}

function record(value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error("model.discovery.shape-invalid");
  }
  return value as Record<string, unknown>;
}

function requireKeys(
  value: Record<string, unknown>,
  expected: readonly string[],
): void {
  const actual = Object.keys(value).sort();
  const ordered = [...expected].sort();
  if (
    actual.length !== ordered.length ||
    actual.some((item, index) => item !== ordered[index])
  ) {
    throw new Error("model.discovery.shape-invalid");
  }
}

function array(value: unknown): readonly unknown[] {
  if (!Array.isArray(value) || value.length > 128) {
    throw new Error("model.discovery.array-invalid");
  }
  return value;
}

function codeArray(value: unknown): readonly string[] {
  const values = array(value);
  if (!values.every(code)) {
    throw new Error("model.discovery.code-invalid");
  }
  return values;
}

function textArray(value: unknown): readonly string[] {
  const values = array(value);
  if (!values.every(text) || values.length > 32) {
    throw new Error("model.discovery.text-array-invalid");
  }
  return values;
}

function enumArray<const T extends string>(
  value: unknown,
  choices: readonly T[],
): readonly T[] {
  const values = array(value);
  if (
    !values.every((item) => oneOf(item, choices)) ||
    new Set(values).size !== values.length
  ) {
    throw new Error("model.discovery.enum-invalid");
  }
  return values;
}

function oneOf<const T extends string>(
  value: unknown,
  choices: readonly T[],
): value is T {
  return typeof value === "string" && choices.includes(value as T);
}

function sha(value: unknown): value is string {
  return typeof value === "string" && SHA256.test(value);
}

function identifier(value: unknown): value is string {
  return typeof value === "string" && IDENTIFIER.test(value);
}

function code(value: unknown): value is string {
  return typeof value === "string" && CODE.test(value);
}

function text(value: unknown): value is string {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    Buffer.byteLength(value, "utf8") <= 256 &&
    !Array.from(value).some((character) => {
      const point = character.codePointAt(0) ?? 0;
      return point <= 0x1f || point === 0x7f;
    })
  );
}

function positiveInteger(value: unknown): value is number {
  return Number.isSafeInteger(value) && (value as number) > 0;
}

function shortHash(value: string): string {
  return value.slice(0, 12);
}

function label(value: string): string {
  return value.replaceAll("_", " ");
}

function escapeMarkdown(value: string): string {
  return value.replace(/[\\`*_{}[\]()#+.!|>-]/gu, "\\$&");
}
