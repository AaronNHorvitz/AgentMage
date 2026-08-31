import Foundation
import Testing
@testable import AgentMageMacOSPlatform

private func peerIdentity() throws -> MacOSBridgeIdentity {
    try MacOSBridgeIdentity(
        bridgeBundleIdentifier: "com.example.agentmage.contractfixture.bridge",
        hostBundleIdentifier: "com.example.agentmage.contractfixture.host",
        teamIdentifier: "AAAAAAAAAA",
        appGroupIdentifier: "group.com.example.agentmage.contractfixture"
    )
}

private func peerExpectation() throws -> MacOSPeerExpectation {
    let identity = try peerIdentity()
    return try MacOSPeerExpectation(
        bridgeIdentity: identity,
        designatedRequirement: MacOSPeerExpectation.requirement(for: identity),
        processIdentifier: 42,
        effectiveUserIdentifier: 501,
        effectiveGroupIdentifier: 20
    )
}

private func peerBaseline() throws -> MacOSPeerObservation {
    let identity = try peerIdentity()
    return MacOSPeerObservation(
        auditTokenPresent: true,
        auditTokenByteCount: agentMageMacOSAuditTokenBytes,
        processIdentifier: 42,
        effectiveUserIdentifier: 501,
        effectiveGroupIdentifier: 20,
        codeSignatureValid: true,
        designatedRequirementSatisfied: true,
        bundleIdentifier: identity.bridgeBundleIdentifier,
        teamIdentifier: identity.teamIdentifier,
        sandboxEnabled: true,
        appGroups: [identity.appGroupIdentifier],
        unexpectedEntitlementKeys: []
    )
}

@Test("release requirement must bind the exact bridge bundle and Team ID")
func exactDesignatedRequirementIsRequired() throws {
    let identity = try peerIdentity()
    #expect(throws: MacOSPeerFailure.invalidExpectation) {
        try MacOSPeerExpectation(
            bridgeIdentity: identity,
            designatedRequirement: "anchor apple generic",
            processIdentifier: 42,
            effectiveUserIdentifier: 501,
            effectiveGroupIdentifier: 20
        )
    }
}

@Test("every peer identity and code substitution fails closed")
func everyPeerMutationFailsClosed() throws {
    let expected = try peerExpectation()
    _ = try MacOSPeerAdmission.admit(observation: peerBaseline(), expected: expected)

    var cases: [(MacOSPeerObservation, MacOSPeerFailure)] = []
    var item = try peerBaseline()
    item.auditTokenPresent = false
    cases.append((item, .auditTokenMissing))
    item = try peerBaseline()
    item.auditTokenByteCount -= 1
    cases.append((item, .auditTokenSizeMismatch))
    item = try peerBaseline()
    item.processIdentifier = 0
    cases.append((item, .invalidPeerProcess))
    item = try peerBaseline()
    item.processIdentifier += 1
    cases.append((item, .wrongProcessIdentifier))
    item = try peerBaseline()
    item.effectiveUserIdentifier += 1
    cases.append((item, .wrongEffectiveUser))
    item = try peerBaseline()
    item.effectiveGroupIdentifier += 1
    cases.append((item, .wrongEffectiveGroup))
    item = try peerBaseline()
    item.codeSignatureValid = false
    cases.append((item, .invalidCodeSignature))
    item = try peerBaseline()
    item.designatedRequirementSatisfied = false
    cases.append((item, .designatedRequirementMismatch))
    item = try peerBaseline()
    item.bundleIdentifier = "com.example.agentmage.wrong.bridge"
    cases.append((item, .wrongBundleIdentifier))
    item = try peerBaseline()
    item.teamIdentifier = "BBBBBBBBBB"
    cases.append((item, .wrongTeamIdentifier))
    item = try peerBaseline()
    item.sandboxEnabled = false
    cases.append((item, .appSandboxMissing))
    item = try peerBaseline()
    item.appGroups = ["group.com.example.agentmage.wrong"]
    cases.append((item, .wrongAppGroup))
    item = try peerBaseline()
    item.unexpectedEntitlementKeys = ["com.apple.security.network.client"]
    cases.append((item, .entitlementClosureMismatch))

    #expect(cases.count + 1 == MacOSPeerFailure.allCases.count)
    for (observation, failure) in cases {
        #expect(throws: failure) {
            _ = try MacOSPeerAdmission.admit(
                observation: observation,
                expected: expected
            )
        }
    }
}

@Test("verified peer and fresh launch challenge compose into one admission")
func verifiedPeerAndFreshChallengeCompose() throws {
    let identity = try peerIdentity()
    let credentials = try MacOSLaunchCredentials(
        challenge: [UInt8](repeating: 3, count: 32),
        launchSecret: [UInt8](repeating: 4, count: 32),
        identity: identity
    )
    let frame = try credentials.request()
    let admission = try MacOSBridgeSessionAdmission(
        expectedPeer: peerExpectation(),
        challenge: [UInt8](repeating: 3, count: 32),
        launchSecret: [UInt8](repeating: 4, count: 32)
    )

    var wrongPeer = try peerBaseline()
    wrongPeer.processIdentifier += 1
    #expect(throws: MacOSPeerFailure.wrongProcessIdentifier) {
        _ = try admission.authenticate(observation: wrongPeer, frame: frame)
    }
    let verified = try admission.authenticate(observation: peerBaseline(), frame: frame)
    #expect(verified.bridgeIdentity == identity)
    #expect(throws: MacOSBridgeFailure.replay) {
        _ = try admission.authenticate(observation: peerBaseline(), frame: frame)
    }

    let staleFrame = try MacOSHandshakeFrame(
        protocolVersion: agentMageMacOSIPCProtocolVersion,
        challenge: [UInt8](repeating: 9, count: 32),
        response: frame.response
    )
    let freshAdmission = try MacOSBridgeSessionAdmission(
        expectedPeer: peerExpectation(),
        challenge: [UInt8](repeating: 3, count: 32),
        launchSecret: [UInt8](repeating: 4, count: 32)
    )
    #expect(throws: MacOSBridgeFailure.challengeMismatch) {
        _ = try freshAdmission.authenticate(
            observation: peerBaseline(),
            frame: staleFrame
        )
    }
}
