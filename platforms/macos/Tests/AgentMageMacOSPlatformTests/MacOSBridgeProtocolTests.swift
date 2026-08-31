import Foundation
import Testing
@testable import AgentMageMacOSPlatform

private func bridgeIdentity() throws -> MacOSBridgeIdentity {
    try MacOSBridgeIdentity(
        bridgeBundleIdentifier: "com.example.agentmage.contractfixture.bridge",
        hostBundleIdentifier: "com.example.agentmage.contractfixture.host",
        teamIdentifier: "AAAAAAAAAA",
        appGroupIdentifier: "group.com.example.agentmage.contractfixture"
    )
}

private func socketBaseline() -> AppGroupSocketObservation {
    AppGroupSocketObservation(
        insideExpectedAppGroupContainer: true,
        parentIsDirectory: true,
        parentIsSymbolicLink: false,
        parentOwnerMatchesEffectiveUser: true,
        parentMode: 0o700,
        socketPathAbsentBeforeBind: true,
        socketIsUnixDomainSocket: true,
        socketIsSymbolicLink: false,
        socketOwnerMatchesEffectiveUser: true,
        socketMode: 0o600
    )
}

private func verifiedPeer(_ identity: MacOSBridgeIdentity) throws -> VerifiedMacOSBridgePeer {
    let expected = try MacOSPeerExpectation(
        bridgeIdentity: identity,
        designatedRequirement: MacOSPeerExpectation.requirement(for: identity),
        processIdentifier: 42,
        effectiveUserIdentifier: 501,
        effectiveGroupIdentifier: 20
    )
    return try MacOSPeerAdmission.admit(
        observation: MacOSPeerObservation(
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
        ),
        expected: expected
    )
}

@Test("fresh exact launch material authenticates once")
func freshExactLaunchAuthenticatesOnce() throws {
    let identity = try bridgeIdentity()
    let credentials = try MacOSLaunchCredentials(
        challenge: [UInt8](repeating: 3, count: 32),
        launchSecret: [UInt8](repeating: 4, count: 32),
        identity: identity
    )
    let frame = try credentials.request()
    #expect(frame.encoded().count == agentMageMacOSHandshakeBytes)
    #expect(try MacOSHandshakeFrame.decode(frame.encoded()) == frame)
    let authenticator = try MacOSBridgeAuthenticator(
        expectedIdentity: identity,
        challenge: [UInt8](repeating: 3, count: 32),
        launchSecret: [UInt8](repeating: 4, count: 32)
    )
    try authenticator.authenticate(verifiedPeer: verifiedPeer(identity), frame: frame)
    #expect(throws: MacOSBridgeFailure.replay) {
        try authenticator.authenticate(verifiedPeer: verifiedPeer(identity), frame: frame)
    }
}

@Test("version challenge identity and response substitutions fail")
func authenticationSubstitutionsFail() throws {
    let identity = try bridgeIdentity()
    let credential = try MacOSLaunchCredentials(
        challenge: [UInt8](repeating: 3, count: 32),
        launchSecret: [UInt8](repeating: 4, count: 32),
        identity: identity
    )
    let baseline = try credential.request()

    let wrongVersion = try MacOSHandshakeFrame(
        protocolVersion: 2,
        challenge: baseline.challenge,
        response: baseline.response
    )
    #expect(throws: MacOSBridgeFailure.versionMismatch) {
        try MacOSBridgeAuthenticator(
            expectedIdentity: identity,
            challenge: [UInt8](repeating: 3, count: 32),
            launchSecret: [UInt8](repeating: 4, count: 32)
        ).authenticate(verifiedPeer: verifiedPeer(identity), frame: wrongVersion)
    }

    let wrongChallenge = try MacOSHandshakeFrame(
        protocolVersion: agentMageMacOSIPCProtocolVersion,
        challenge: [UInt8](repeating: 9, count: 32),
        response: baseline.response
    )
    #expect(throws: MacOSBridgeFailure.challengeMismatch) {
        try MacOSBridgeAuthenticator(
            expectedIdentity: identity,
            challenge: [UInt8](repeating: 3, count: 32),
            launchSecret: [UInt8](repeating: 4, count: 32)
        ).authenticate(verifiedPeer: verifiedPeer(identity), frame: wrongChallenge)
    }

    let wrongIdentity = try MacOSBridgeIdentity(
        bridgeBundleIdentifier: "com.example.agentmage.wrong.bridge",
        hostBundleIdentifier: identity.hostBundleIdentifier,
        teamIdentifier: identity.teamIdentifier,
        appGroupIdentifier: identity.appGroupIdentifier
    )
    #expect(throws: MacOSBridgeFailure.bridgeIdentityMismatch) {
        try MacOSBridgeAuthenticator(
            expectedIdentity: identity,
            challenge: [UInt8](repeating: 3, count: 32),
            launchSecret: [UInt8](repeating: 4, count: 32)
        ).authenticate(verifiedPeer: verifiedPeer(wrongIdentity), frame: baseline)
    }

    var wrongResponse = baseline.response
    wrongResponse[0] ^= 1
    let corrupted = try MacOSHandshakeFrame(
        protocolVersion: baseline.protocolVersion,
        challenge: baseline.challenge,
        response: wrongResponse
    )
    #expect(throws: MacOSBridgeFailure.authenticationFailed) {
        try MacOSBridgeAuthenticator(
            expectedIdentity: identity,
            challenge: [UInt8](repeating: 3, count: 32),
            launchSecret: [UInt8](repeating: 4, count: 32)
        ).authenticate(verifiedPeer: verifiedPeer(identity), frame: corrupted)
    }
}

@Test("frame bounds are closed")
func frameBoundsAreClosed() throws {
    #expect(throws: MacOSBridgeFailure.malformedFrame) {
        try MacOSHandshakeFrame.decode(Data(repeating: 0, count: 67))
    }
    #expect(throws: MacOSBridgeFailure.resourceLimitExceeded) {
        try MacOSIPCFrameBoundary.encode(Data(), maximumBytes: 1)
    }
    #expect(throws: MacOSBridgeFailure.resourceLimitExceeded) {
        try MacOSIPCFrameBoundary.encode(
            Data(repeating: 1, count: agentMageMacOSMaximumResponseBytes + 1),
            maximumBytes: agentMageMacOSMaximumResponseBytes
        )
    }
    let body = Data("bounded".utf8)
    #expect(try MacOSIPCFrameBoundary.encode(body, maximumBytes: 64).count == body.count + 4)
}

@Test("App Group socket identity and modes fail closed independently")
func appGroupSocketMutationsFailClosed() throws {
    try AppGroupSocketBoundary.validate(socketBaseline())

    var cases: [(AppGroupSocketObservation, AppGroupSocketFailure)] = []
    var item = socketBaseline()
    item.insideExpectedAppGroupContainer = false
    cases.append((item, .outsideAppGroup))
    item = socketBaseline()
    item.parentIsSymbolicLink = true
    cases.append((item, .unsafeParentType))
    item = socketBaseline()
    item.parentOwnerMatchesEffectiveUser = false
    cases.append((item, .wrongParentOwner))
    item = socketBaseline()
    item.parentMode = 0o755
    cases.append((item, .wrongParentMode))
    item = socketBaseline()
    item.socketPathAbsentBeforeBind = false
    cases.append((item, .preexistingSocketPath))
    item = socketBaseline()
    item.socketIsUnixDomainSocket = false
    cases.append((item, .wrongSocketType))
    item = socketBaseline()
    item.socketOwnerMatchesEffectiveUser = false
    cases.append((item, .wrongSocketOwner))
    item = socketBaseline()
    item.socketMode = 0o660
    cases.append((item, .wrongSocketMode))

    #expect(cases.count == AppGroupSocketFailure.allCases.count)
    for (observation, failure) in cases {
        #expect(throws: failure) { try AppGroupSocketBoundary.validate(observation) }
    }
}
