import Testing
@testable import AgentMageMacOSPlatform

@Test("macOS scaffold remains explicitly blocked")
func macOSScaffoldRemainsBlocked() {
    #expect(AgentMageMacOSPlatformBoundary.implementationStatus == "blocked-macos")
    #expect(
        AgentMageMacOSPlatformBoundary.kernelHostSourceStatus
            == "implemented-source-unverified"
    )
    #expect(
        AgentMageMacOSPlatformBoundary.bridgeSourceStatus
            == "implemented-source-unverified"
    )
    #expect(
        AgentMageMacOSPlatformBoundary.peerVerificationSourceStatus
            == "implemented-source-unverified"
    )
    #expect(
        AgentMageMacOSPlatformBoundary.workspaceBookmarkSourceStatus
            == "implemented-source-unverified"
    )
    #expect(
        AgentMageMacOSPlatformBoundary.toolHelperSourceStatus
            == "implemented-source-unverified"
    )
    #expect(
        AgentMageMacOSPlatformBoundary.metalInferenceSourceStatus
            == "implemented-source-unverified"
    )
}
