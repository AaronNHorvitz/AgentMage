import Darwin
import Foundation
import Security

public let agentMageMacOSAuditTokenBytes = MemoryLayout<audit_token_t>.size

/// Release and launch values that the kernel-observed bridge peer must match.
public struct MacOSPeerExpectation: Equatable, Sendable {
    public let bridgeIdentity: MacOSBridgeIdentity
    public let designatedRequirement: String
    public let processIdentifier: pid_t
    public let effectiveUserIdentifier: uid_t
    public let effectiveGroupIdentifier: gid_t

    public init(
        bridgeIdentity: MacOSBridgeIdentity,
        designatedRequirement: String,
        processIdentifier: pid_t,
        effectiveUserIdentifier: uid_t,
        effectiveGroupIdentifier: gid_t
    ) throws {
        let exactRequirement = Self.requirement(for: bridgeIdentity)
        guard designatedRequirement == exactRequirement,
              processIdentifier > 1
        else {
            throw MacOSPeerFailure.invalidExpectation
        }
        self.bridgeIdentity = bridgeIdentity
        self.designatedRequirement = designatedRequirement
        self.processIdentifier = processIdentifier
        self.effectiveUserIdentifier = effectiveUserIdentifier
        self.effectiveGroupIdentifier = effectiveGroupIdentifier
    }

    public static func requirement(for identity: MacOSBridgeIdentity) -> String {
        "anchor apple generic and identifier \(identity.bridgeBundleIdentifier) "
            + "and certificate leaf[subject.OU] = \(identity.teamIdentifier)"
    }
}

/// Allowlisted, content-free facts derived from one accepted Unix socket peer.
struct MacOSPeerObservation: Equatable, Sendable {
    var auditTokenPresent: Bool
    var auditTokenByteCount: Int
    var processIdentifier: pid_t
    var effectiveUserIdentifier: uid_t
    var effectiveGroupIdentifier: gid_t
    var codeSignatureValid: Bool
    var designatedRequirementSatisfied: Bool
    var bundleIdentifier: String?
    var teamIdentifier: String?
    var sandboxEnabled: Bool
    var appGroups: Set<String>
    var unexpectedEntitlementKeys: Set<String>
}

public enum MacOSPeerFailure: String, Error, CaseIterable, Sendable {
    case invalidExpectation = "macos.ipc.peer.invalid-expectation"
    case auditTokenMissing = "macos.ipc.peer.audit-token-missing"
    case auditTokenSizeMismatch = "macos.ipc.peer.audit-token-size-mismatch"
    case invalidPeerProcess = "macos.ipc.peer.invalid-process"
    case wrongProcessIdentifier = "macos.ipc.peer.wrong-process"
    case wrongEffectiveUser = "macos.ipc.peer.wrong-effective-user"
    case wrongEffectiveGroup = "macos.ipc.peer.wrong-effective-group"
    case invalidCodeSignature = "macos.ipc.peer.invalid-code-signature"
    case designatedRequirementMismatch = "macos.ipc.peer.designated-requirement-mismatch"
    case wrongBundleIdentifier = "macos.ipc.peer.wrong-bundle-identifier"
    case wrongTeamIdentifier = "macos.ipc.peer.wrong-team-identifier"
    case appSandboxMissing = "macos.ipc.peer.app-sandbox-missing"
    case wrongAppGroup = "macos.ipc.peer.wrong-app-group"
    case entitlementClosureMismatch = "macos.ipc.peer.entitlement-closure-mismatch"
}

/// Opaque proof that the socket peer passed every identity and signature check.
public struct VerifiedMacOSBridgePeer: Equatable, Sendable {
    public let processIdentifier: pid_t
    public let bridgeIdentity: MacOSBridgeIdentity

    fileprivate init(expectation: MacOSPeerExpectation) {
        processIdentifier = expectation.processIdentifier
        bridgeIdentity = expectation.bridgeIdentity
    }
}

enum MacOSPeerAdmission {
    static func admit(
        observation: MacOSPeerObservation,
        expected: MacOSPeerExpectation
    ) throws -> VerifiedMacOSBridgePeer {
        guard observation.auditTokenPresent else {
            throw MacOSPeerFailure.auditTokenMissing
        }
        guard observation.auditTokenByteCount == agentMageMacOSAuditTokenBytes else {
            throw MacOSPeerFailure.auditTokenSizeMismatch
        }
        guard observation.processIdentifier > 1 else {
            throw MacOSPeerFailure.invalidPeerProcess
        }
        guard observation.processIdentifier == expected.processIdentifier else {
            throw MacOSPeerFailure.wrongProcessIdentifier
        }
        guard observation.effectiveUserIdentifier == expected.effectiveUserIdentifier else {
            throw MacOSPeerFailure.wrongEffectiveUser
        }
        guard observation.effectiveGroupIdentifier == expected.effectiveGroupIdentifier else {
            throw MacOSPeerFailure.wrongEffectiveGroup
        }
        guard observation.codeSignatureValid else {
            throw MacOSPeerFailure.invalidCodeSignature
        }
        guard observation.designatedRequirementSatisfied else {
            throw MacOSPeerFailure.designatedRequirementMismatch
        }
        guard observation.bundleIdentifier == expected.bridgeIdentity.bridgeBundleIdentifier else {
            throw MacOSPeerFailure.wrongBundleIdentifier
        }
        guard observation.teamIdentifier == expected.bridgeIdentity.teamIdentifier else {
            throw MacOSPeerFailure.wrongTeamIdentifier
        }
        guard observation.sandboxEnabled else {
            throw MacOSPeerFailure.appSandboxMissing
        }
        guard observation.appGroups == [expected.bridgeIdentity.appGroupIdentifier] else {
            throw MacOSPeerFailure.wrongAppGroup
        }
        guard observation.unexpectedEntitlementKeys.isEmpty else {
            throw MacOSPeerFailure.entitlementClosureMismatch
        }
        return VerifiedMacOSBridgePeer(expectation: expected)
    }
}

public enum MacOSPeerObservationError: String, Error, Sendable {
    case invalidSocket = "macos.ipc.peer-observation.invalid-socket"
    case cannotReadAuditToken = "macos.ipc.peer-observation.cannot-read-audit-token"
    case malformedAuditToken = "macos.ipc.peer-observation.malformed-audit-token"
    case cannotReadPeerIdentity = "macos.ipc.peer-observation.cannot-read-peer-identity"
    case cannotResolvePeerCode = "macos.ipc.peer-observation.cannot-resolve-peer-code"
    case cannotCompileRequirement = "macos.ipc.peer-observation.cannot-compile-requirement"
    case cannotReadSigningInformation = "macos.ipc.peer-observation.cannot-read-signing-information"
}

/// Reads the audit token and peer credentials from the accepted local socket itself.
public enum SecurityFrameworkMacOSPeerObserver {
    private static let declaredEntitlementKeys: Set<String> = [
        "com.apple.security.app-sandbox",
        "com.apple.security.application-groups",
    ]

    private static let systemIdentityEntitlementKeys: Set<String> = [
        "application-identifier",
        "com.apple.application-identifier",
        "com.apple.developer.team-identifier",
    ]

    static func observe(
        socketFileDescriptor: Int32,
        designatedRequirement: String
    ) throws -> MacOSPeerObservation {
        guard socketFileDescriptor >= 0 else {
            throw MacOSPeerObservationError.invalidSocket
        }

        var auditToken = audit_token_t()
        var auditTokenLength = socklen_t(agentMageMacOSAuditTokenBytes)
        guard getsockopt(
            socketFileDescriptor,
            SOL_LOCAL,
            LOCAL_PEERTOKEN,
            &auditToken,
            &auditTokenLength
        ) == 0 else {
            throw MacOSPeerObservationError.cannotReadAuditToken
        }
        guard Int(auditTokenLength) == agentMageMacOSAuditTokenBytes else {
            throw MacOSPeerObservationError.malformedAuditToken
        }

        var peerPID = pid_t.zero
        var peerPIDLength = socklen_t(MemoryLayout<pid_t>.size)
        guard getsockopt(
            socketFileDescriptor,
            SOL_LOCAL,
            LOCAL_PEERPID,
            &peerPID,
            &peerPIDLength
        ) == 0,
            Int(peerPIDLength) == MemoryLayout<pid_t>.size
        else {
            throw MacOSPeerObservationError.cannotReadPeerIdentity
        }
        var effectiveUID = uid_t.zero
        var effectiveGID = gid_t.zero
        guard getpeereid(socketFileDescriptor, &effectiveUID, &effectiveGID) == 0 else {
            throw MacOSPeerObservationError.cannotReadPeerIdentity
        }

        let auditTokenData = withUnsafeBytes(of: &auditToken) { Data($0) }
        let attributes = [kSecGuestAttributeAudit as String: auditTokenData] as CFDictionary
        var peerCode: SecCode?
        guard SecCodeCopyGuestWithAttributes(
            nil,
            attributes,
            SecCSFlags(),
            &peerCode
        ) == errSecSuccess,
            let peerCode
        else {
            throw MacOSPeerObservationError.cannotResolvePeerCode
        }

        var requirement: SecRequirement?
        guard SecRequirementCreateWithString(
            designatedRequirement as CFString,
            SecCSFlags(),
            &requirement
        ) == errSecSuccess,
            let requirement
        else {
            throw MacOSPeerObservationError.cannotCompileRequirement
        }

        var rawSigningInformation: CFDictionary?
        guard SecCodeCopySigningInformation(
            peerCode,
            SecCSFlags(rawValue: kSecCSSigningInformation),
            &rawSigningInformation
        ) == errSecSuccess,
            let signingInformation = rawSigningInformation as? [String: Any]
        else {
            throw MacOSPeerObservationError.cannotReadSigningInformation
        }
        let entitlements = signingInformation[kSecCodeInfoEntitlementsDict as String]
            as? [String: Any] ?? [:]
        let allowedEntitlementKeys = declaredEntitlementKeys.union(
            systemIdentityEntitlementKeys
        )

        return MacOSPeerObservation(
            auditTokenPresent: true,
            auditTokenByteCount: Int(auditTokenLength),
            processIdentifier: peerPID,
            effectiveUserIdentifier: effectiveUID,
            effectiveGroupIdentifier: effectiveGID,
            codeSignatureValid:
                SecCodeCheckValidity(peerCode, SecCSFlags(), nil) == errSecSuccess,
            designatedRequirementSatisfied:
                SecCodeCheckValidity(peerCode, SecCSFlags(), requirement) == errSecSuccess,
            bundleIdentifier: signingInformation[kSecCodeInfoIdentifier as String] as? String,
            teamIdentifier: signingInformation[kSecCodeInfoTeamIdentifier as String] as? String,
            sandboxEnabled: entitlements["com.apple.security.app-sandbox"] as? Bool == true,
            appGroups: stringSet(entitlements["com.apple.security.application-groups"]),
            unexpectedEntitlementKeys: Set(entitlements.keys).subtracting(
                allowedEntitlementKeys
            )
        )
    }

    private static func stringSet(_ value: Any?) -> Set<String> {
        guard let values = value as? [String] else { return [] }
        return Set(values)
    }
}

/// The only public host-side route to consume one launch authenticator.
public final class MacOSBridgeSessionAdmission: @unchecked Sendable {
    private let expectedPeer: MacOSPeerExpectation
    private let authenticator: MacOSBridgeAuthenticator

    public init(
        expectedPeer: MacOSPeerExpectation,
        challenge: [UInt8],
        launchSecret: [UInt8]
    ) throws {
        self.expectedPeer = expectedPeer
        authenticator = try MacOSBridgeAuthenticator(
            expectedIdentity: expectedPeer.bridgeIdentity,
            challenge: challenge,
            launchSecret: launchSecret
        )
    }

    public func authenticate(
        socketFileDescriptor: Int32,
        frame: MacOSHandshakeFrame
    ) throws -> VerifiedMacOSBridgePeer {
        let observation = try SecurityFrameworkMacOSPeerObserver.observe(
            socketFileDescriptor: socketFileDescriptor,
            designatedRequirement: expectedPeer.designatedRequirement
        )
        return try authenticate(observation: observation, frame: frame)
    }

    func authenticate(
        observation: MacOSPeerObservation,
        frame: MacOSHandshakeFrame
    ) throws -> VerifiedMacOSBridgePeer {
        let verifiedPeer = try MacOSPeerAdmission.admit(
            observation: observation,
            expected: expectedPeer
        )
        try authenticator.authenticate(verifiedPeer: verifiedPeer, frame: frame)
        return verifiedPeer
    }
}
