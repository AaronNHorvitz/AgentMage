import Foundation
import Testing
@testable import AgentMageMacOSPlatform

private let refusalChallenge = [UInt8](repeating: 3, count: 32)
private let refusalSecret = [UInt8](repeating: 4, count: 32)

private func refusalIdentity() throws -> MacOSBridgeIdentity {
    try MacOSBridgeIdentity(
        bridgeBundleIdentifier: "com.example.agentmage.contractfixture.bridge",
        hostBundleIdentifier: "com.example.agentmage.contractfixture.host",
        teamIdentifier: "AAAAAAAAAA",
        appGroupIdentifier: "group.com.example.agentmage.contractfixture"
    )
}

private func refusalExpectation() throws -> MacOSPeerExpectation {
    let identity = try refusalIdentity()
    return try MacOSPeerExpectation(
        bridgeIdentity: identity,
        designatedRequirement: MacOSPeerExpectation.requirement(for: identity),
        processIdentifier: 42,
        effectiveUserIdentifier: 501,
        effectiveGroupIdentifier: 20
    )
}

private struct HandshakeRefusalFixture {
    var socket: AppGroupSocketObservation
    var peer: MacOSPeerObservation
    var frameBytes: Data
}

private func refusalBaseline() throws -> HandshakeRefusalFixture {
    let identity = try refusalIdentity()
    let credentials = try MacOSLaunchCredentials(
        challenge: refusalChallenge,
        launchSecret: refusalSecret,
        identity: identity
    )
    return HandshakeRefusalFixture(
        socket: AppGroupSocketObservation(
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
        ),
        peer: MacOSPeerObservation(
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
        frameBytes: try credentials.request().encoded()
    )
}

private func refusalAdmission() throws -> MacOSBridgeSessionAdmission {
    try MacOSBridgeSessionAdmission(
        expectedPeer: refusalExpectation(),
        challenge: refusalChallenge,
        launchSecret: refusalSecret
    )
}

private func attemptHandshake(
    _ fixture: HandshakeRefusalFixture,
    admission: MacOSBridgeSessionAdmission
) throws {
    try AppGroupSocketBoundary.validate(fixture.socket)
    let frame = try MacOSHandshakeFrame.decode(fixture.frameBytes)
    _ = try admission.authenticate(observation: fixture.peer, frame: frame)
}

private func refusalCode(_ error: any Error) -> String {
    if let value = error as? AppGroupSocketFailure { return value.rawValue }
    if let value = error as? MacOSPeerFailure { return value.rawValue }
    if let value = error as? MacOSBridgeFailure { return value.rawValue }
    return "unexpected-error-type"
}

private enum HandshakeRefusalMutation: String, CaseIterable {
    case codeIdentityAbsent = "code-identity.absent"
    case codeIdentityWrong = "code-identity.wrong"
    case auditTokenAbsent = "audit-token.absent"
    case auditTokenWrong = "audit-token.wrong"
    case teamIdentifierAbsent = "team-id.absent"
    case teamIdentifierWrong = "team-id.wrong"
    case bundleIdentifierAbsent = "bundle-id.absent"
    case bundleIdentifierWrong = "bundle-id.wrong"
    case appGroupAbsent = "app-group.absent"
    case appGroupWrong = "app-group.wrong"
    case protocolVersionAbsent = "protocol-version.absent"
    case protocolVersionWrong = "protocol-version.wrong"
    case freshChallengeAbsent = "fresh-challenge.absent"
    case freshChallengeWrong = "fresh-challenge.wrong"
    case freshChallengeReplay = "fresh-challenge.replay"
    case socketModeAbsent = "socket-mode.absent"
    case socketModeWrong = "socket-mode.wrong"

    var expectedFailure: String {
        switch self {
        case .codeIdentityAbsent:
            MacOSPeerFailure.invalidCodeSignature.rawValue
        case .codeIdentityWrong:
            MacOSPeerFailure.designatedRequirementMismatch.rawValue
        case .auditTokenAbsent:
            MacOSPeerFailure.auditTokenMissing.rawValue
        case .auditTokenWrong:
            MacOSPeerFailure.auditTokenSizeMismatch.rawValue
        case .teamIdentifierAbsent, .teamIdentifierWrong:
            MacOSPeerFailure.wrongTeamIdentifier.rawValue
        case .bundleIdentifierAbsent, .bundleIdentifierWrong:
            MacOSPeerFailure.wrongBundleIdentifier.rawValue
        case .appGroupAbsent, .appGroupWrong:
            MacOSPeerFailure.wrongAppGroup.rawValue
        case .protocolVersionAbsent, .freshChallengeAbsent:
            MacOSBridgeFailure.malformedFrame.rawValue
        case .protocolVersionWrong:
            MacOSBridgeFailure.versionMismatch.rawValue
        case .freshChallengeWrong:
            MacOSBridgeFailure.challengeMismatch.rawValue
        case .freshChallengeReplay:
            MacOSBridgeFailure.replay.rawValue
        case .socketModeAbsent:
            AppGroupSocketFailure.wrongSocketType.rawValue
        case .socketModeWrong:
            AppGroupSocketFailure.wrongSocketMode.rawValue
        }
    }

    func apply(to fixture: inout HandshakeRefusalFixture) throws {
        switch self {
        case .codeIdentityAbsent:
            fixture.peer.codeSignatureValid = false
        case .codeIdentityWrong:
            fixture.peer.designatedRequirementSatisfied = false
        case .auditTokenAbsent:
            fixture.peer.auditTokenPresent = false
        case .auditTokenWrong:
            fixture.peer.auditTokenByteCount -= 1
        case .teamIdentifierAbsent:
            fixture.peer.teamIdentifier = nil
        case .teamIdentifierWrong:
            fixture.peer.teamIdentifier = "BBBBBBBBBB"
        case .bundleIdentifierAbsent:
            fixture.peer.bundleIdentifier = nil
        case .bundleIdentifierWrong:
            fixture.peer.bundleIdentifier = "com.example.agentmage.wrong.bridge"
        case .appGroupAbsent:
            fixture.peer.appGroups = []
        case .appGroupWrong:
            fixture.peer.appGroups = ["group.com.example.agentmage.wrong"]
        case .protocolVersionAbsent:
            fixture.frameBytes.removeSubrange(0..<4)
        case .protocolVersionWrong:
            let baseline = try MacOSHandshakeFrame.decode(fixture.frameBytes)
            fixture.frameBytes = try MacOSHandshakeFrame(
                protocolVersion: agentMageMacOSIPCProtocolVersion + 1,
                challenge: baseline.challenge,
                response: baseline.response
            ).encoded()
        case .freshChallengeAbsent:
            fixture.frameBytes.removeSubrange(4..<36)
        case .freshChallengeWrong:
            let baseline = try MacOSHandshakeFrame.decode(fixture.frameBytes)
            fixture.frameBytes = try MacOSHandshakeFrame(
                protocolVersion: baseline.protocolVersion,
                challenge: [UInt8](repeating: 9, count: 32),
                response: baseline.response
            ).encoded()
        case .freshChallengeReplay:
            break
        case .socketModeAbsent:
            fixture.socket.socketIsUnixDomainSocket = false
        case .socketModeWrong:
            fixture.socket.socketMode = 0o660
        }
    }
}

@Test("S-008-UT01 refuses every absent or wrong handshake field")
func s008UT01RefusesEveryAbsentOrWrongHandshakeField() throws {
    #expect(HandshakeRefusalMutation.allCases.count == 17)
    #expect(Set(HandshakeRefusalMutation.allCases.map(\.rawValue)).count == 17)

    for mutation in HandshakeRefusalMutation.allCases {
        var fixture = try refusalBaseline()
        try mutation.apply(to: &fixture)
        let admission = try refusalAdmission()

        if mutation == .freshChallengeReplay {
            try attemptHandshake(fixture, admission: admission)
        }

        do {
            try attemptHandshake(fixture, admission: admission)
            Issue.record("accepted prohibited S-008-UT01 case: \(mutation.rawValue)")
        } catch {
            #expect(refusalCode(error) == mutation.expectedFailure)
        }

        if mutation != .freshChallengeReplay {
            try attemptHandshake(try refusalBaseline(), admission: admission)
        }
    }
}
