import CryptoKit
import Foundation

public let agentMageMacOSToolRequestMaximumBytes = 64 * 1024
public let agentMageMacOSToolResultMaximumBytes = 4 * 1024 * 1024
public let agentMageMacOSToolXPCMaximumBytes = 192 * 1024
public let agentMageMacOSToolMaximumDeadlineMilliseconds: UInt64 = 15_000
public let agentMageMacOSToolScratchMaximumBytes: UInt64 = 16 * 1024 * 1024
public let agentMageMacOSToolScratchMaximumEntries = 256

public enum MacOSToolHelperFailure: String, Error, CaseIterable, Sendable {
    case malformedEnvelope = "macos.tool-helper.malformed-envelope"
    case unsupportedSchema = "macos.tool-helper.unsupported-schema"
    case invalidIdentity = "macos.tool-helper.invalid-identity"
    case grantNotConsumed = "macos.tool-helper.grant-not-consumed"
    case wrongOperation = "macos.tool-helper.wrong-operation"
    case invalidTool = "macos.tool-helper.invalid-tool"
    case invalidBookmark = "macos.tool-helper.invalid-bookmark"
    case requestLimitExceeded = "macos.tool-helper.request-limit-exceeded"
    case requestDigestMismatch = "macos.tool-helper.request-digest-mismatch"
    case invalidDeadline = "macos.tool-helper.invalid-deadline"
    case expired = "macos.tool-helper.expired"
    case replay = "macos.tool-helper.replay"
    case resultLimitExceeded = "macos.tool-helper.result-limit-exceeded"
    case resultEncodingFailed = "macos.tool-helper.result-encoding-failed"
    case helperIdentityRejected = "macos.tool-helper.helper-identity-rejected"
    case hostIdentityRejected = "macos.tool-helper.host-identity-rejected"
    case connectionRejected = "macos.tool-helper.connection-rejected"
    case resourceLimitFailed = "macos.tool-helper.resource-limit-failed"
    case scratchCreationFailed = "macos.tool-helper.scratch-creation-failed"
    case scratchValidationFailed = "macos.tool-helper.scratch-validation-failed"
    case scratchCleanupFailed = "macos.tool-helper.scratch-cleanup-failed"
    case bookmarkResolutionFailed = "macos.tool-helper.bookmark-resolution-failed"
    case staleBookmark = "macos.tool-helper.stale-bookmark"
    case securityScopeDenied = "macos.tool-helper.security-scope-denied"
    case workspaceValidationFailed = "macos.tool-helper.workspace-validation-failed"
    case workspaceDescriptorFailed = "macos.tool-helper.workspace-descriptor-failed"
    case executorFailed = "macos.tool-helper.executor-failed"
    case timeout = "macos.tool-helper.timeout"
    case cancelled = "macos.tool-helper.cancelled"
}

/// Closed, content-bound input for one already-consumed read-only operation grant.
public struct MacOSToolInvocation: Codable, Equatable, Sendable {
    public let schemaVersion: UInt16
    public let grantState: String
    public let grantIdentifier: String
    public let attemptIdentifier: String
    public let operation: String
    public let toolIdentifier: String
    public let toolVersion: String
    public let bookmarkIdentifier: String
    public let bookmarkData: Data
    public let bookmarkSHA256: String
    public let requestData: Data
    public let requestSHA256: String
    public let consumedAtEpochMilliseconds: UInt64
    public let expiresAtEpochMilliseconds: UInt64
    public let deadlineMilliseconds: UInt64

    public init(
        schemaVersion: UInt16,
        grantState: String,
        grantIdentifier: String,
        attemptIdentifier: String,
        operation: String,
        toolIdentifier: String,
        toolVersion: String,
        bookmarkIdentifier: String,
        bookmarkData: Data,
        bookmarkSHA256: String,
        requestData: Data,
        requestSHA256: String,
        consumedAtEpochMilliseconds: UInt64,
        expiresAtEpochMilliseconds: UInt64,
        deadlineMilliseconds: UInt64
    ) {
        self.schemaVersion = schemaVersion
        self.grantState = grantState
        self.grantIdentifier = grantIdentifier
        self.attemptIdentifier = attemptIdentifier
        self.operation = operation
        self.toolIdentifier = toolIdentifier
        self.toolVersion = toolVersion
        self.bookmarkIdentifier = bookmarkIdentifier
        self.bookmarkData = bookmarkData
        self.bookmarkSHA256 = bookmarkSHA256
        self.requestData = requestData
        self.requestSHA256 = requestSHA256
        self.consumedAtEpochMilliseconds = consumedAtEpochMilliseconds
        self.expiresAtEpochMilliseconds = expiresAtEpochMilliseconds
        self.deadlineMilliseconds = deadlineMilliseconds
    }
}

struct MacOSToolInvocationObservation: Equatable, Sendable {
    var schemaVersion: UInt16
    var grantState: String
    var grantIdentifierValid: Bool
    var attemptIdentifierValid: Bool
    var operation: String
    var toolIdentifierValid: Bool
    var toolVersionValid: Bool
    var bookmarkIdentifierValid: Bool
    var bookmarkByteCount: Int
    var bookmarkDigestMatches: Bool
    var requestByteCount: Int
    var requestDigestMatches: Bool
    var consumedAtEpochMilliseconds: UInt64
    var expiresAtEpochMilliseconds: UInt64
    var deadlineMilliseconds: UInt64
    var nowEpochMilliseconds: UInt64
}

/// Opaque admission proof. It carries no minting or reusable grant authority.
public struct VerifiedMacOSToolInvocation: Sendable {
    public let invocation: MacOSToolInvocation

    fileprivate init(invocation: MacOSToolInvocation) {
        self.invocation = invocation
    }
}

enum MacOSToolInvocationAdmission {
    static func admit(
        invocation: MacOSToolInvocation,
        observation: MacOSToolInvocationObservation
    ) throws -> VerifiedMacOSToolInvocation {
        guard observation.schemaVersion == 1 else {
            throw MacOSToolHelperFailure.unsupportedSchema
        }
        guard observation.grantIdentifierValid,
              observation.attemptIdentifierValid,
              observation.bookmarkIdentifierValid
        else {
            throw MacOSToolHelperFailure.invalidIdentity
        }
        guard observation.grantState == "consumed" else {
            throw MacOSToolHelperFailure.grantNotConsumed
        }
        guard observation.operation == "workspace_read" else {
            throw MacOSToolHelperFailure.wrongOperation
        }
        guard observation.toolIdentifierValid, observation.toolVersionValid else {
            throw MacOSToolHelperFailure.invalidTool
        }
        guard observation.bookmarkByteCount > 0,
              observation.bookmarkByteCount <= agentMageMacOSMaximumBookmarkBytes,
              observation.bookmarkDigestMatches
        else {
            throw MacOSToolHelperFailure.invalidBookmark
        }
        guard observation.requestByteCount > 0,
              observation.requestByteCount <= agentMageMacOSToolRequestMaximumBytes
        else {
            throw MacOSToolHelperFailure.requestLimitExceeded
        }
        guard observation.requestDigestMatches else {
            throw MacOSToolHelperFailure.requestDigestMismatch
        }
        guard observation.deadlineMilliseconds > 0,
              observation.deadlineMilliseconds
                <= agentMageMacOSToolMaximumDeadlineMilliseconds,
              observation.consumedAtEpochMilliseconds
                < observation.expiresAtEpochMilliseconds,
              observation.expiresAtEpochMilliseconds
                - observation.consumedAtEpochMilliseconds
                <= agentMageMacOSToolMaximumDeadlineMilliseconds
        else {
            throw MacOSToolHelperFailure.invalidDeadline
        }
        guard observation.nowEpochMilliseconds
                >= observation.consumedAtEpochMilliseconds,
              observation.nowEpochMilliseconds
                < observation.expiresAtEpochMilliseconds
        else {
            throw MacOSToolHelperFailure.expired
        }
        return VerifiedMacOSToolInvocation(invocation: invocation)
    }
}

enum MacOSToolInvocationCodec {
    private static let keys: Set<String> = [
        "schemaVersion",
        "grantState",
        "grantIdentifier",
        "attemptIdentifier",
        "operation",
        "toolIdentifier",
        "toolVersion",
        "bookmarkIdentifier",
        "bookmarkData",
        "bookmarkSHA256",
        "requestData",
        "requestSHA256",
        "consumedAtEpochMilliseconds",
        "expiresAtEpochMilliseconds",
        "deadlineMilliseconds",
    ]

    static func decodeClosed(
        _ data: Data,
        nowEpochMilliseconds: UInt64
    ) throws -> VerifiedMacOSToolInvocation {
        guard !data.isEmpty, data.count <= agentMageMacOSToolXPCMaximumBytes,
              let object = try? JSONSerialization.jsonObject(with: data),
              let dictionary = object as? [String: Any],
              Set(dictionary.keys) == keys,
              let invocation = try? JSONDecoder().decode(MacOSToolInvocation.self, from: data)
        else {
            throw MacOSToolHelperFailure.malformedEnvelope
        }
        let observation = MacOSToolInvocationObservation(
            schemaVersion: invocation.schemaVersion,
            grantState: invocation.grantState,
            grantIdentifierValid: validIdentifier(invocation.grantIdentifier),
            attemptIdentifierValid: validIdentifier(invocation.attemptIdentifier),
            operation: invocation.operation,
            toolIdentifierValid: validToolIdentifier(invocation.toolIdentifier),
            toolVersionValid: validToolVersion(invocation.toolVersion),
            bookmarkIdentifierValid:
                (try? MacOSWorkspaceBookmarkID(invocation.bookmarkIdentifier)) != nil,
            bookmarkByteCount: invocation.bookmarkData.count,
            bookmarkDigestMatches:
                sha256(invocation.bookmarkData) == invocation.bookmarkSHA256,
            requestByteCount: invocation.requestData.count,
            requestDigestMatches:
                sha256(invocation.requestData) == invocation.requestSHA256,
            consumedAtEpochMilliseconds: invocation.consumedAtEpochMilliseconds,
            expiresAtEpochMilliseconds: invocation.expiresAtEpochMilliseconds,
            deadlineMilliseconds: invocation.deadlineMilliseconds,
            nowEpochMilliseconds: nowEpochMilliseconds
        )
        return try MacOSToolInvocationAdmission.admit(
            invocation: invocation,
            observation: observation
        )
    }

    private static func validIdentifier(_ value: String) -> Bool {
        value.utf8.count >= 8 && value.utf8.count <= 128
            && value.unicodeScalars.allSatisfy {
                CharacterSet(charactersIn: "abcdefghijklmnopqrstuvwxyz0123456789._-")
                    .contains($0)
            }
    }

    private static func validToolIdentifier(_ value: String) -> Bool {
        value.hasPrefix("agentmage.") && validIdentifier(value)
    }

    private static func validToolVersion(_ value: String) -> Bool {
        let parts = value.split(separator: ".", omittingEmptySubsequences: false)
        return parts.count == 3 && parts.allSatisfy {
            !$0.isEmpty && $0.count <= 5 && $0.allSatisfy(\.isNumber)
        }
    }
}

public struct MacOSToolHelperReply: Codable, Equatable, Sendable {
    public let schemaVersion: UInt16
    public let outcome: String
    public let failureCode: String?
    public let resultData: Data?
    public let resultSHA256: String?

    public static func succeeded(_ data: Data) throws -> Self {
        guard data.count <= agentMageMacOSToolResultMaximumBytes else {
            throw MacOSToolHelperFailure.resultLimitExceeded
        }
        let reply = Self(
            schemaVersion: 1,
            outcome: "succeeded",
            failureCode: nil,
            resultData: data,
            resultSHA256: sha256(data)
        )
        guard (try? reply.encoded()) != nil else {
            throw MacOSToolHelperFailure.resultLimitExceeded
        }
        return reply
    }

    public static func failed(_ failure: MacOSToolHelperFailure) -> Self {
        Self(
            schemaVersion: 1,
            outcome: "failed",
            failureCode: failure.rawValue,
            resultData: nil,
            resultSHA256: nil
        )
    }

    func encoded() throws -> Data {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys]
        let data = try encoder.encode(self)
        guard data.count <= agentMageMacOSToolResultMaximumBytes else {
            throw MacOSToolHelperFailure.resultEncodingFailed
        }
        return data
    }
}

final class MacOSToolOneShotGate: @unchecked Sendable {
    private let lock = NSLock()
    private var consumed = false

    func consume() throws {
        lock.lock()
        defer { lock.unlock() }
        guard !consumed else {
            throw MacOSToolHelperFailure.replay
        }
        consumed = true
    }
}

private func sha256(_ data: Data) -> String {
    SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
}
