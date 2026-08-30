import Testing
@testable import AgentMageMacOSPlatform

private let expected = KernelHostExpectedIdentity(
    bundleIdentifier: "com.example.agentmage.contractfixture.host",
    teamIdentifier: "AAAAAAAAAA",
    appGroupIdentifier: "group.com.example.agentmage.contractfixture",
    keychainAccessGroup: "AAAAAAAAAA.com.example.agentmage.contractfixture.keys"
)

private func baseline() -> KernelHostObservation {
    KernelHostObservation(
        architectureIsArm64: true,
        minimumMacOSVersionMet: true,
        signatureValid: true,
        hardenedRuntimePresent: true,
        bundleIdentifier: expected.bundleIdentifier,
        teamIdentifier: expected.teamIdentifier,
        sandboxEnabled: true,
        appGroups: [expected.appGroupIdentifier],
        appScopedBookmarksEnabled: true,
        userSelectedReadOnlyEnabled: true,
        keychainAccessGroups: [expected.keychainAccessGroup],
        unexpectedEntitlementKeys: []
    )
}

private func assertRefusal(
    _ observation: KernelHostObservation,
    equals expectedFailure: KernelHostAdmissionFailure
) {
    do {
        _ = try KernelHostAdmission.admit(observation: observation, expected: expected)
        Issue.record("mutated host observation was admitted")
    } catch let failure as KernelHostAdmissionFailure {
        #expect(failure == expectedFailure)
    } catch {
        Issue.record("unexpected error type: \(error)")
    }
}

@Test("exact signed sandboxed arm64 identity is admitted")
func exactIdentityIsAdmitted() throws {
    let verified = try KernelHostAdmission.admit(
        observation: baseline(),
        expected: expected
    )
    #expect(verified.bundleIdentifier == expected.bundleIdentifier)
    #expect(verified.teamIdentifier == expected.teamIdentifier)
    #expect(verified.appGroupIdentifier == expected.appGroupIdentifier)
}

@Test("every host identity and confinement dimension fails closed")
func everyDimensionFailsClosed() {
    var cases: [(KernelHostObservation, KernelHostAdmissionFailure)] = []

    var item = baseline()
    item.architectureIsArm64 = false
    cases.append((item, .wrongArchitecture))
    item = baseline()
    item.minimumMacOSVersionMet = false
    cases.append((item, .unsupportedOperatingSystem))
    item = baseline()
    item.signatureValid = false
    cases.append((item, .invalidCodeSignature))
    item = baseline()
    item.hardenedRuntimePresent = false
    cases.append((item, .hardenedRuntimeMissing))
    item = baseline()
    item.bundleIdentifier = "com.example.agentmage.wrong"
    cases.append((item, .wrongBundleIdentifier))
    item = baseline()
    item.teamIdentifier = "BBBBBBBBBB"
    cases.append((item, .wrongTeamIdentifier))
    item = baseline()
    item.sandboxEnabled = false
    cases.append((item, .appSandboxMissing))
    item = baseline()
    item.appGroups = ["group.com.example.agentmage.wrong"]
    cases.append((item, .wrongAppGroup))
    item = baseline()
    item.appScopedBookmarksEnabled = false
    cases.append((item, .appScopedBookmarksMissing))
    item = baseline()
    item.userSelectedReadOnlyEnabled = false
    cases.append((item, .userSelectedReadOnlyMissing))
    item = baseline()
    item.keychainAccessGroups = ["BBBBBBBBBB.com.example.agentmage.wrong"]
    cases.append((item, .wrongKeychainAccessGroup))
    item = baseline()
    item.unexpectedEntitlementKeys = ["com.apple.security.network.client"]
    cases.append((item, .entitlementClosureMismatch))

    #expect(cases.count == KernelHostAdmissionFailure.allCases.count)
    for (observation, failure) in cases {
        assertRefusal(observation, equals: failure)
    }
}

@Test("extra identity or authority is rejected rather than ignored")
func extraIdentityOrAuthorityIsRejected() {
    var extraAppGroup = baseline()
    extraAppGroup.appGroups.insert("group.com.example.agentmage.extra")
    assertRefusal(extraAppGroup, equals: .wrongAppGroup)

    var extraKeychainGroup = baseline()
    extraKeychainGroup.keychainAccessGroups.insert("AAAAAAAAAA.com.example.extra")
    assertRefusal(extraKeychainGroup, equals: .wrongKeychainAccessGroup)

    var networkAuthority = baseline()
    networkAuthority.unexpectedEntitlementKeys.insert("com.apple.security.network.client")
    assertRefusal(networkAuthority, equals: .entitlementClosureMismatch)
}
