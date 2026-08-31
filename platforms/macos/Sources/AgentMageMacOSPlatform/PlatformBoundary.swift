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
}
