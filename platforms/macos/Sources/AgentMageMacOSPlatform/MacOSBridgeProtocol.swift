import CryptoKit
import Foundation

private extension Array where Element == UInt8 {
    mutating func erase() {
        withUnsafeMutableBytes { bytes in
            bytes.initializeMemory(as: UInt8.self, repeating: 0)
        }
    }
}

public let agentMageMacOSIPCProtocolVersion: UInt32 = 1
public let agentMageMacOSHandshakeBytes = 68
public let agentMageMacOSMaximumRequestBytes = 64 * 1024
public let agentMageMacOSMaximumResponseBytes = 4 * 1024 * 1024

/// Release-declared identities bound into the one-launch bridge authenticator.
public struct MacOSBridgeIdentity: Equatable, Sendable {
    public let bridgeBundleIdentifier: String
    public let hostBundleIdentifier: String
    public let teamIdentifier: String
    public let appGroupIdentifier: String

    public init(
        bridgeBundleIdentifier: String,
        hostBundleIdentifier: String,
        teamIdentifier: String,
        appGroupIdentifier: String
    ) throws {
        guard Self.validIdentifier(bridgeBundleIdentifier),
              Self.validIdentifier(hostBundleIdentifier),
              teamIdentifier.count == 10,
              teamIdentifier.allSatisfy({ $0.isASCII && ($0.isUppercase || $0.isNumber) }),
              appGroupIdentifier.hasPrefix("group."),
              Self.validIdentifier(appGroupIdentifier)
        else {
            throw MacOSBridgeFailure.invalidIdentity
        }
        self.bridgeBundleIdentifier = bridgeBundleIdentifier
        self.hostBundleIdentifier = hostBundleIdentifier
        self.teamIdentifier = teamIdentifier
        self.appGroupIdentifier = appGroupIdentifier
    }

    fileprivate var authenticationBytes: [UInt8] {
        [bridgeBundleIdentifier, hostBundleIdentifier, teamIdentifier, appGroupIdentifier]
            .flatMap { value in
                let bytes = Array(value.utf8)
                var length = UInt16(bytes.count).bigEndian
                let prefix = withUnsafeBytes(of: &length) { Array($0) }
                return prefix + bytes
            }
    }

    private static func validIdentifier(_ value: String) -> Bool {
        !value.isEmpty && value.utf8.count <= 255 && !value.contains("\0")
            && value.allSatisfy {
                $0.isASCII && ($0.isLetter || $0.isNumber || $0 == "." || $0 == "-")
            }
    }
}

public enum MacOSBridgeFailure: String, Error, CaseIterable, Sendable {
    case invalidIdentity = "macos.ipc.invalid-identity"
    case invalidLaunchMaterial = "macos.ipc.invalid-launch-material"
    case versionMismatch = "macos.ipc.version-mismatch"
    case challengeMismatch = "macos.ipc.challenge-mismatch"
    case bridgeIdentityMismatch = "macos.ipc.bridge-identity-mismatch"
    case authenticationFailed = "macos.ipc.authentication-failed"
    case replay = "macos.ipc.replay"
    case malformedFrame = "macos.ipc.malformed-frame"
    case resourceLimitExceeded = "macos.ipc.resource-limit-exceeded"
}

/// Fixed 68-byte launch authentication request.
public struct MacOSHandshakeFrame: Equatable, Sendable {
    public let protocolVersion: UInt32
    public let challenge: [UInt8]
    public let response: [UInt8]

    public init(
        protocolVersion: UInt32,
        challenge: [UInt8],
        response: [UInt8]
    ) throws {
        guard challenge.count == 32, response.count == 32 else {
            throw MacOSBridgeFailure.malformedFrame
        }
        self.protocolVersion = protocolVersion
        self.challenge = challenge
        self.response = response
    }

    public func encoded() -> Data {
        var version = protocolVersion.bigEndian
        var result = Data(bytes: &version, count: MemoryLayout<UInt32>.size)
        result.append(contentsOf: challenge)
        result.append(contentsOf: response)
        return result
    }

    public static func decode(_ data: Data) throws -> Self {
        guard data.count == agentMageMacOSHandshakeBytes else {
            throw MacOSBridgeFailure.malformedFrame
        }
        let version = data.prefix(4).reduce(UInt32.zero) {
            ($0 << 8) | UInt32($1)
        }
        return try Self(
            protocolVersion: version,
            challenge: Array(data[4..<36]),
            response: Array(data[36..<68])
        )
    }
}

/// Fresh material delivered through a direct inherited channel, never arguments or environment.
public final class MacOSLaunchCredentials: @unchecked Sendable {
    private var challengeBytes: [UInt8]
    private var secretBytes: [UInt8]
    public let identity: MacOSBridgeIdentity

    public init(
        challenge: [UInt8],
        launchSecret: [UInt8],
        identity: MacOSBridgeIdentity
    ) throws {
        guard challenge.count == 32, launchSecret.count == 32 else {
            throw MacOSBridgeFailure.invalidLaunchMaterial
        }
        challengeBytes = challenge
        secretBytes = launchSecret
        self.identity = identity
    }

    deinit {
        challengeBytes.erase()
        secretBytes.erase()
    }

    public func request() throws -> MacOSHandshakeFrame {
        try MacOSHandshakeFrame(
            protocolVersion: agentMageMacOSIPCProtocolVersion,
            challenge: challengeBytes,
            response: Self.authenticationDigest(
                protocolVersion: agentMageMacOSIPCProtocolVersion,
                challenge: challengeBytes,
                launchSecret: secretBytes,
                identity: identity
            )
        )
    }

    fileprivate static func authenticationDigest(
        protocolVersion: UInt32,
        challenge: [UInt8],
        launchSecret: [UInt8],
        identity: MacOSBridgeIdentity
    ) -> [UInt8] {
        var version = protocolVersion.bigEndian
        var message = Data("agentmage-macos-ipc-auth-v1\0".utf8)
        message.append(Data(bytes: &version, count: MemoryLayout<UInt32>.size))
        message.append(contentsOf: challenge)
        message.append(contentsOf: identity.authenticationBytes)
        let key = SymmetricKey(data: launchSecret)
        return Array(HMAC<SHA256>.authenticationCode(for: message, using: key))
    }
}

/// One-use host-side authenticator reachable only after native peer admission.
final class MacOSBridgeAuthenticator: @unchecked Sendable {
    private let lock = NSLock()
    private let expectedIdentity: MacOSBridgeIdentity
    private var challenge: [UInt8]
    private var launchSecret: [UInt8]
    private var consumed = false

    init(
        expectedIdentity: MacOSBridgeIdentity,
        challenge: [UInt8],
        launchSecret: [UInt8]
    ) throws {
        guard challenge.count == 32, launchSecret.count == 32 else {
            throw MacOSBridgeFailure.invalidLaunchMaterial
        }
        self.expectedIdentity = expectedIdentity
        self.challenge = challenge
        self.launchSecret = launchSecret
    }

    deinit {
        challenge.erase()
        launchSecret.erase()
    }

    func authenticate(
        verifiedPeer: VerifiedMacOSBridgePeer,
        frame: MacOSHandshakeFrame
    ) throws {
        lock.lock()
        defer { lock.unlock() }
        guard !consumed else { throw MacOSBridgeFailure.replay }
        guard frame.protocolVersion == agentMageMacOSIPCProtocolVersion else {
            throw MacOSBridgeFailure.versionMismatch
        }
        guard frame.challenge == challenge else {
            throw MacOSBridgeFailure.challengeMismatch
        }
        guard verifiedPeer.bridgeIdentity == expectedIdentity else {
            throw MacOSBridgeFailure.bridgeIdentityMismatch
        }
        let expectedResponse = MacOSLaunchCredentials.authenticationDigest(
            protocolVersion: frame.protocolVersion,
            challenge: frame.challenge,
            launchSecret: launchSecret,
            identity: verifiedPeer.bridgeIdentity
        )
        guard Self.constantTimeEqual(frame.response, expectedResponse) else {
            throw MacOSBridgeFailure.authenticationFailed
        }
        consumed = true
        challenge.erase()
        launchSecret.erase()
    }

    private static func constantTimeEqual(_ left: [UInt8], _ right: [UInt8]) -> Bool {
        guard left.count == right.count else { return false }
        return zip(left, right).reduce(UInt8.zero) { $0 | ($1.0 ^ $1.1) } == 0
    }
}

/// Applies the same closed bounds to every authenticated application frame.
public enum MacOSIPCFrameBoundary {
    public static func encode(_ body: Data, maximumBytes: Int) throws -> Data {
        guard maximumBytes > 0,
              maximumBytes <= agentMageMacOSMaximumResponseBytes,
              !body.isEmpty,
              body.count <= maximumBytes,
              body.count <= Int(UInt32.max)
        else {
            throw MacOSBridgeFailure.resourceLimitExceeded
        }
        var length = UInt32(body.count).bigEndian
        var result = Data(bytes: &length, count: MemoryLayout<UInt32>.size)
        result.append(body)
        return result
    }
}
