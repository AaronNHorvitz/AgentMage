import Testing
@testable import AgentMageMacOSPlatform

private func workspaceBaseline() -> MacOSWorkspaceObservation {
    MacOSWorkspaceObservation(
        isFileURL: true,
        isDirectory: true,
        isSymbolicLink: false,
        isAliasFile: false,
        resourceIdentityPresent: true,
        volumeIdentityPresent: true,
        resourceIdentityStable: true,
        volumeIdentityStable: true,
        allNamesCanonical: true,
        hasCaseCollision: false,
        rootEntryCount: 12
    )
}

@Test("bookmark identifiers are canonical UUIDs only")
func bookmarkIdentifiersAreCanonical() throws {
    let value = "123e4567-e89b-12d3-a456-426614174000"
    #expect(try MacOSWorkspaceBookmarkID(value).value == value)
    #expect(throws: MacOSWorkspaceBookmarkFailure.invalidBookmarkIdentifier) {
        try MacOSWorkspaceBookmarkID(value.uppercased())
    }
    #expect(throws: MacOSWorkspaceBookmarkFailure.invalidBookmarkIdentifier) {
        try MacOSWorkspaceBookmarkID("workspace-one")
    }
}

@Test("every workspace root observation fails independently")
func everyWorkspaceRootMutationFailsClosed() throws {
    try MacOSWorkspaceAdmission.validate(workspaceBaseline())

    var cases: [(MacOSWorkspaceObservation, MacOSWorkspaceBookmarkFailure)] = []
    var item = workspaceBaseline()
    item.isFileURL = false
    cases.append((item, .nonFileURL))
    item = workspaceBaseline()
    item.isDirectory = false
    cases.append((item, .notDirectory))
    item = workspaceBaseline()
    item.isSymbolicLink = true
    cases.append((item, .symbolicLink))
    item = workspaceBaseline()
    item.isAliasFile = true
    cases.append((item, .aliasFile))
    item = workspaceBaseline()
    item.resourceIdentityPresent = false
    cases.append((item, .resourceIdentityUnavailable))
    item = workspaceBaseline()
    item.volumeIdentityPresent = false
    cases.append((item, .resourceIdentityUnavailable))
    item = workspaceBaseline()
    item.resourceIdentityStable = false
    cases.append((item, .resourceIdentityChanged))
    item = workspaceBaseline()
    item.volumeIdentityStable = false
    cases.append((item, .volumeIdentityChanged))
    item = workspaceBaseline()
    item.allNamesCanonical = false
    cases.append((item, .nonCanonicalName))
    item = workspaceBaseline()
    item.hasCaseCollision = true
    cases.append((item, .caseCollision))
    item = workspaceBaseline()
    item.rootEntryCount = agentMageMacOSMaximumWorkspaceRootEntries + 1
    cases.append((item, .resourceLimitExceeded))

    #expect(cases.count == 11)
    for (observation, failure) in cases {
        #expect(throws: failure) {
            try MacOSWorkspaceAdmission.validate(observation)
        }
    }
}

@Test("failure taxonomy is closed and content free")
func workspaceFailureTaxonomyIsClosed() {
    #expect(MacOSWorkspaceBookmarkFailure.allCases.count == 22)
    #expect(
        MacOSWorkspaceBookmarkFailure.allCases.allSatisfy {
            $0.rawValue.hasPrefix("macos.workspace.")
                && !$0.rawValue.contains("/")
                && !$0.rawValue.contains(" ")
        }
    )
}
