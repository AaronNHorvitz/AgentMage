/// Source-level marker for the retained macOS platform boundary.
public enum AgentMageMacOSPlatformBoundary {
    /// Native execution remains blocked until Apple Silicon evidence exists.
    public static let implementationStatus = "blocked-macos"

    /// Repository-controlled host admission source is present but not native evidence.
    public static let kernelHostSourceStatus = "implemented-source-unverified"

    /// Bridge protocol and App Group socket policy source are not native evidence.
    public static let bridgeSourceStatus = "implemented-source-unverified"

    /// Audit-token and peer code-identity verification source is not native evidence.
    public static let peerVerificationSourceStatus = "implemented-source-unverified"

    /// Picker and read-only app-scoped bookmark lifecycle source is not native evidence.
    public static let workspaceBookmarkSourceStatus = "implemented-source-unverified"

    /// Signed stateless XPC tool-helper source is present but not native evidence.
    public static let toolHelperSourceStatus = "implemented-source-unverified"

    /// Sandboxed Metal inference-service source is present but not native evidence.
    public static let metalInferenceSourceStatus = "implemented-source-unverified"
}
