import Testing
@testable import AgentMageMacOSPlatform

@Test("macOS scaffold remains explicitly blocked")
func macOSScaffoldRemainsBlocked() {
    #expect(AgentMageMacOSPlatformBoundary.implementationStatus == "blocked-macos")
    #expect(
        AgentMageMacOSPlatformBoundary.kernelHostSourceStatus
            == "implemented-source-unverified"
    )
}
