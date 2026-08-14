//! Closed catalog for the deterministic read-only workspace tool pack.

use std::fmt::Write;

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, GrantOperation, OperationBinding, RequiredGrantTemplate, SchemaId,
    SchemaReference, ToolDefinition, ToolId, ToolRiskLevel,
};
use sha2::{Digest, Sha256};

/// Immutable contract version shared by the initial read-only tool pack.
pub const READ_ONLY_TOOL_VERSION: &str = "1.0.0";

/// Shared closed input-schema identity for the initial read-only tool pack.
pub const READ_ONLY_INPUT_SCHEMA_ID: &str = "agentmage.workspace.read-only.input";

/// Shared closed output-schema identity for the initial read-only tool pack.
pub const READ_ONLY_OUTPUT_SCHEMA_ID: &str = "agentmage.workspace.read-only.output";

/// Canonical JSON Schema bytes hashed into every read-only tool definition.
pub const READ_ONLY_INPUT_SCHEMA_JSON: &str = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","$id":"agentmage.workspace.read-only.input","type":"object","additionalProperties":false,"required":["schema_version","paths","query","byte_offset","byte_count","encoding","limits","call_depth"],"properties":{"schema_version":{"const":1},"paths":{"type":"array","minItems":1,"maxItems":256,"items":{"type":"array","minItems":1,"maxItems":256,"items":{"type":"string","minLength":1,"maxLength":255}}},"query":{"type":["string","null"],"maxLength":4096},"byte_offset":{"type":["integer","null"],"minimum":0},"byte_count":{"type":["integer","null"],"minimum":1},"encoding":{"enum":["utf8","binary"]},"limits":{"type":"object","additionalProperties":false,"required":["files","input_bytes","depth","matches","output_bytes"],"properties":{"files":{"type":"integer","minimum":1,"maximum":256},"input_bytes":{"type":"integer","minimum":1,"maximum":16777216},"depth":{"type":"integer","minimum":0,"maximum":32},"matches":{"type":"integer","minimum":1,"maximum":1000},"output_bytes":{"type":"integer","minimum":1,"maximum":2097152}}},"call_depth":{"type":"integer","minimum":0,"maximum":8}}}"#;

/// Canonical JSON Schema bytes hashed into every read-only tool definition.
pub const READ_ONLY_OUTPUT_SCHEMA_JSON: &str = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","$id":"agentmage.workspace.read-only.output","type":"object","additionalProperties":false,"required":["schema_version","tool","outcome","items","observed_files","observed_bytes","output_bytes","truncated","result_sha256"],"properties":{"schema_version":{"const":1},"tool":{"type":"string"},"outcome":{"enum":["succeeded","no_result","denied","partial","truncated","malformed","cancelled","failed"]},"items":{"type":"array","maxItems":1000},"observed_files":{"type":"integer","minimum":0},"observed_bytes":{"type":"integer","minimum":0},"output_bytes":{"type":"integer","minimum":0,"maximum":2097152},"truncated":{"type":"boolean"},"result_sha256":{"type":"string","pattern":"^[0-9a-f]{64}$"}}}"#;

/// Closed operation identities admitted by the v0.1 read-only registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReadOnlyToolKind {
    /// List direct children of one exact directory.
    ListDirectory,
    /// Produce a bounded recursive directory tree.
    DirectoryTree,
    /// Read one bounded UTF-8 range from one exact file.
    ReadText,
    /// Read bounded UTF-8 ranges from multiple exact files.
    ReadMultiple,
    /// Search bounded relative filenames.
    SearchFilenames,
    /// Search bounded UTF-8 file content.
    SearchText,
    /// Return bounded content-free filesystem metadata.
    Metadata,
    /// Hash one exact regular file.
    HashFile,
    /// Hash one canonical bounded tree projection.
    HashTree,
    /// Return bounded content-free metadata for a binary file.
    BinaryMetadata,
}

impl ReadOnlyToolKind {
    /// Every admitted read-only tool in stable registry order.
    pub const ALL: [Self; 10] = [
        Self::ListDirectory,
        Self::DirectoryTree,
        Self::ReadText,
        Self::ReadMultiple,
        Self::SearchFilenames,
        Self::SearchText,
        Self::Metadata,
        Self::HashFile,
        Self::HashTree,
        Self::BinaryMetadata,
    ];

    /// Stable exact tool identity.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::ListDirectory => "agentmage.workspace.list-directory",
            Self::DirectoryTree => "agentmage.workspace.directory-tree",
            Self::ReadText => "agentmage.workspace.read-file",
            Self::ReadMultiple => "agentmage.workspace.read-multiple",
            Self::SearchFilenames => "agentmage.workspace.search-filenames",
            Self::SearchText => "agentmage.workspace.search-text",
            Self::Metadata => "agentmage.workspace.metadata",
            Self::HashFile => "agentmage.workspace.hash-file",
            Self::HashTree => "agentmage.workspace.hash-tree",
            Self::BinaryMetadata => "agentmage.workspace.binary-metadata",
        }
    }

    const fn display_name(self) -> &'static str {
        match self {
            Self::ListDirectory => "List workspace directory",
            Self::DirectoryTree => "Build workspace directory tree",
            Self::ReadText => "Read workspace file",
            Self::ReadMultiple => "Read multiple workspace files",
            Self::SearchFilenames => "Search workspace filenames",
            Self::SearchText => "Search workspace text",
            Self::Metadata => "Read workspace metadata",
            Self::HashFile => "Hash workspace file",
            Self::HashTree => "Hash workspace tree",
            Self::BinaryMetadata => "Read binary metadata",
        }
    }

    const fn description(self) -> &'static str {
        match self {
            Self::ListDirectory => "Lists bounded direct children of one exact approved directory",
            Self::DirectoryTree => {
                "Builds a bounded deterministic tree beneath one exact approved directory"
            }
            Self::ReadText => "Reads one exact approved workspace file as bounded UTF-8 text",
            Self::ReadMultiple => "Reads bounded UTF-8 ranges from exact approved workspace files",
            Self::SearchFilenames => {
                "Searches bounded approved relative filenames in deterministic order"
            }
            Self::SearchText => "Searches bounded approved UTF-8 content in deterministic order",
            Self::Metadata => "Returns content-free metadata for exact approved workspace objects",
            Self::HashFile => "Computes SHA-256 for one exact approved regular file",
            Self::HashTree => "Computes SHA-256 for one canonical bounded approved tree projection",
            Self::BinaryMetadata => {
                "Returns size, digest, and format hint for one approved binary file"
            }
        }
    }

    const fn target_scope(self) -> &'static str {
        match self {
            Self::ListDirectory | Self::DirectoryTree | Self::HashTree => {
                "one-exact-held-workspace-directory"
            }
            Self::ReadMultiple | Self::SearchFilenames | Self::SearchText | Self::Metadata => {
                "bounded-exact-held-workspace-objects"
            }
            Self::ReadText | Self::HashFile | Self::BinaryMetadata => {
                "one-exact-held-workspace-file"
            }
        }
    }
}

/// Returns every declarative read-only definition in stable identity order.
#[must_use]
pub fn read_only_tool_definitions() -> Vec<ToolDefinition> {
    ReadOnlyToolKind::ALL
        .into_iter()
        .map(read_only_tool_definition)
        .collect()
}

/// Returns one exact declarative read-only definition.
#[must_use]
pub fn read_only_tool_definition(kind: ReadOnlyToolKind) -> ToolDefinition {
    let operation = OperationBinding::new(GrantOperation::WorkspaceRead);
    ToolDefinition {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_id: ToolId::from_raw(kind.id()),
        tool_version: READ_ONLY_TOOL_VERSION.to_owned(),
        display_name: kind.display_name().to_owned(),
        description: kind.description().to_owned(),
        input_schema: schema(
            READ_ONLY_INPUT_SCHEMA_ID,
            READ_ONLY_INPUT_SCHEMA_JSON.as_bytes(),
        ),
        output_schema: schema(
            READ_ONLY_OUTPUT_SCHEMA_ID,
            READ_ONLY_OUTPUT_SCHEMA_JSON.as_bytes(),
        ),
        risk_level: ToolRiskLevel::Low,
        declared_effects: vec![operation],
        required_grant: RequiredGrantTemplate {
            operation,
            target_scope: kind.target_scope().to_owned(),
            single_use: true,
        },
        timeout_ms: 15_000,
    }
}

/// Resolves only an exact admitted identity and version.
#[must_use]
pub fn read_only_tool_kind(tool_id: &ToolId, tool_version: &str) -> Option<ReadOnlyToolKind> {
    if tool_version != READ_ONLY_TOOL_VERSION {
        return None;
    }
    ReadOnlyToolKind::ALL
        .into_iter()
        .find(|kind| kind.id() == tool_id.as_str())
}

fn schema(id: &str, bytes: &[u8]) -> SchemaReference {
    SchemaReference {
        schema_id: SchemaId::from_raw(id),
        schema_version: 1,
        schema_sha256: sha256_hex(bytes),
    }
}

fn sha256_hex(value: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use agentmage_kernel_contracts::{GrantOperation, ToolId, ToolRiskLevel};

    use super::{
        READ_ONLY_INPUT_SCHEMA_JSON, READ_ONLY_OUTPUT_SCHEMA_JSON, READ_ONLY_TOOL_VERSION,
        ReadOnlyToolKind, read_only_tool_definition, read_only_tool_definitions,
        read_only_tool_kind,
    };

    #[test]
    fn catalog_is_closed_stable_and_exclusively_read_only() {
        let definitions = read_only_tool_definitions();
        assert_eq!(definitions.len(), ReadOnlyToolKind::ALL.len());
        assert_eq!(definitions.len(), 10);
        assert_eq!(
            definitions
                .iter()
                .map(|definition| definition.tool_id.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            definitions.len()
        );
        for (kind, definition) in ReadOnlyToolKind::ALL.into_iter().zip(definitions) {
            assert_eq!(definition.tool_id.as_str(), kind.id());
            assert_eq!(definition.tool_version, READ_ONLY_TOOL_VERSION);
            assert_eq!(definition.risk_level, ToolRiskLevel::Low);
            assert_eq!(definition.declared_effects.len(), 1);
            assert_eq!(
                definition.declared_effects[0].operation(),
                GrantOperation::WorkspaceRead
            );
            assert!(definition.required_grant.single_use);
            assert_eq!(
                read_only_tool_kind(&definition.tool_id, &definition.tool_version),
                Some(kind)
            );
        }
    }

    #[test]
    fn schemas_are_real_closed_json_schema_documents_and_hashes_are_content_bound() {
        for schema in [READ_ONLY_INPUT_SCHEMA_JSON, READ_ONLY_OUTPUT_SCHEMA_JSON] {
            let value: serde_json::Value = serde_json::from_str(schema).expect("schema JSON");
            assert_eq!(value["additionalProperties"], false);
            assert_eq!(
                value["$schema"],
                "https://json-schema.org/draft/2020-12/schema"
            );
        }
        let definition = read_only_tool_definition(ReadOnlyToolKind::ReadText);
        assert_ne!(
            definition.input_schema.schema_sha256,
            definition.output_schema.schema_sha256
        );
        assert_eq!(definition.input_schema.schema_sha256.len(), 64);
        assert_eq!(definition.output_schema.schema_sha256.len(), 64);
    }

    #[test]
    fn unknown_identity_or_version_never_resolves() {
        assert_eq!(
            read_only_tool_kind(&ToolId::from_raw("agentmage.workspace.write-file"), "1.0.0"),
            None
        );
        assert_eq!(
            read_only_tool_kind(&ToolId::from_raw(ReadOnlyToolKind::ReadText.id()), "2.0.0"),
            None
        );
    }
}
