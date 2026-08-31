import CryptoKit
import Foundation

public let agentMageMacOSMetalInputMaximumBytes = 2 * 1024 * 1024
public let agentMageMacOSMetalResultMaximumBytes = 4 * 1024 * 1024
public let agentMageMacOSMetalXPCMaximumBytes = 3 * 1024 * 1024
public let agentMageMacOSMetalMaximumContextTokens: UInt32 = 32_768
public let agentMageMacOSMetalMaximumOutputTokens: UInt32 = 4_096
public let agentMageMacOSMetalMaximumDeadlineMilliseconds: UInt64 = 120_000
public let agentMageMacOSMetalMaximumModelBytes: UInt64 = 64 * 1024 * 1024 * 1024
public let agentMageMacOSMetalMaximumRequestIdentities = 1_024

public enum MacOSMetalInferenceFailure: String, Error, CaseIterable, Sendable {
    case malformedEnvelope = "macos.metal-inference.malformed-envelope"
    case unsupportedSchema = "macos.metal-inference.unsupported-schema"
    case invalidIdentity = "macos.metal-inference.invalid-identity"
    case invalidProfile = "macos.metal-inference.invalid-profile"
    case invalidManifestDigest = "macos.metal-inference.invalid-manifest-digest"
    case invalidArtifactDigest = "macos.metal-inference.invalid-artifact-digest"
    case invalidRuntimeDigest = "macos.metal-inference.invalid-runtime-digest"
    case invalidCodecDigest = "macos.metal-inference.invalid-codec-digest"
    case invalidInput = "macos.metal-inference.invalid-input"
    case inputLimitExceeded = "macos.metal-inference.input-limit-exceeded"
    case inputDigestMismatch = "macos.metal-inference.input-digest-mismatch"
    case invalidContext = "macos.metal-inference.invalid-context"
    case invalidOutputLimit = "macos.metal-inference.invalid-output-limit"
    case invalidSampling = "macos.metal-inference.invalid-sampling"
    case invalidDeadline = "macos.metal-inference.invalid-deadline"
    case replay = "macos.metal-inference.replay"
    case capacityExceeded = "macos.metal-inference.capacity-exceeded"
    case serviceIdentityRejected = "macos.metal-inference.service-identity-rejected"
    case hostIdentityRejected = "macos.metal-inference.host-identity-rejected"
    case connectionRejected = "macos.metal-inference.connection-rejected"
    case resourceLimitFailed = "macos.metal-inference.resource-limit-failed"
    case metalUnavailable = "macos.metal-inference.metal-unavailable"
    case modelDescriptorRejected = "macos.metal-inference.model-descriptor-rejected"
    case modelDigestMismatch = "macos.metal-inference.model-digest-mismatch"
    case profileSubstitution = "macos.metal-inference.profile-substitution"
    case executorFailed = "macos.metal-inference.executor-failed"
    case timeout = "macos.metal-inference.timeout"
    case cancelled = "macos.metal-inference.cancelled"
    case resultLimitExceeded = "macos.metal-inference.result-limit-exceeded"
    case resultEncodingFailed = "macos.metal-inference.result-encoding-failed"
}

/// One exact, inert inference request. It deliberately has no path, bookmark,
/// tool, grant, credential, environment, endpoint, or network field.
public struct MacOSMetalInferenceRequest: Codable, Equatable, Sendable {
    public let schemaVersion: UInt16
    public let requestIdentifier: String
    public let profileIdentifier: String
    public let manifestSHA256: String
    public let artifactSHA256: String
    public let artifactByteCount: UInt64
    public let runtimeSHA256: String
    public let tokenizerSHA256: String
    public let templateSHA256: String
    public let codecSHA256: String
    public let inputData: Data
    public let inputSHA256: String
    public let contextTokens: UInt32
    public let maximumOutputTokens: UInt32
    public let seed: UInt64
    public let temperatureBasisPoints: UInt16
    public let deadlineMilliseconds: UInt64

    public init(
        schemaVersion: UInt16,
        requestIdentifier: String,
        profileIdentifier: String,
        manifestSHA256: String,
        artifactSHA256: String,
        artifactByteCount: UInt64,
        runtimeSHA256: String,
        tokenizerSHA256: String,
        templateSHA256: String,
        codecSHA256: String,
        inputData: Data,
        inputSHA256: String,
        contextTokens: UInt32,
        maximumOutputTokens: UInt32,
        seed: UInt64,
        temperatureBasisPoints: UInt16,
        deadlineMilliseconds: UInt64
    ) {
        self.schemaVersion = schemaVersion
        self.requestIdentifier = requestIdentifier
        self.profileIdentifier = profileIdentifier
        self.manifestSHA256 = manifestSHA256
        self.artifactSHA256 = artifactSHA256
        self.artifactByteCount = artifactByteCount
        self.runtimeSHA256 = runtimeSHA256
        self.tokenizerSHA256 = tokenizerSHA256
        self.templateSHA256 = templateSHA256
        self.codecSHA256 = codecSHA256
        self.inputData = inputData
        self.inputSHA256 = inputSHA256
        self.contextTokens = contextTokens
        self.maximumOutputTokens = maximumOutputTokens
        self.seed = seed
        self.temperatureBasisPoints = temperatureBasisPoints
        self.deadlineMilliseconds = deadlineMilliseconds
    }
}

public struct MacOSMetalVerifiedProfile: Equatable, Sendable {
    public let profileIdentifier: String
    public let manifestSHA256: String
    public let artifactSHA256: String
    public let artifactByteCount: UInt64
    public let runtimeSHA256: String
    public let tokenizerSHA256: String
    public let templateSHA256: String
    public let codecSHA256: String

    public init(
        profileIdentifier: String,
        manifestSHA256: String,
        artifactSHA256: String,
        artifactByteCount: UInt64,
        runtimeSHA256: String,
        tokenizerSHA256: String,
        templateSHA256: String,
        codecSHA256: String
    ) throws {
        guard profileIdentifier.utf8.count >= 8,
              profileIdentifier.utf8.count <= 128,
              profileIdentifier.unicodeScalars.allSatisfy({
                CharacterSet(
                    charactersIn: "abcdefghijklmnopqrstuvwxyz0123456789._-"
                ).contains($0)
              }),
              validSHA256(manifestSHA256),
              validSHA256(artifactSHA256),
              artifactByteCount > 0,
              artifactByteCount <= agentMageMacOSMetalMaximumModelBytes,
              validSHA256(runtimeSHA256),
              validSHA256(tokenizerSHA256),
              validSHA256(templateSHA256),
              validSHA256(codecSHA256)
        else {
            throw MacOSMetalInferenceFailure.invalidProfile
        }
        self.profileIdentifier = profileIdentifier
        self.manifestSHA256 = manifestSHA256
        self.artifactSHA256 = artifactSHA256
        self.artifactByteCount = artifactByteCount
        self.runtimeSHA256 = runtimeSHA256
        self.tokenizerSHA256 = tokenizerSHA256
        self.templateSHA256 = templateSHA256
        self.codecSHA256 = codecSHA256
    }

    fileprivate init(_ request: MacOSMetalInferenceRequest) {
        profileIdentifier = request.profileIdentifier
        manifestSHA256 = request.manifestSHA256
        artifactSHA256 = request.artifactSHA256
        artifactByteCount = request.artifactByteCount
        runtimeSHA256 = request.runtimeSHA256
        tokenizerSHA256 = request.tokenizerSHA256
        templateSHA256 = request.templateSHA256
        codecSHA256 = request.codecSHA256
    }
}

public struct VerifiedMacOSMetalInferenceRequest: Sendable {
    public let request: MacOSMetalInferenceRequest
    public let profile: MacOSMetalVerifiedProfile

    fileprivate init(_ request: MacOSMetalInferenceRequest) {
        self.request = request
        profile = MacOSMetalVerifiedProfile(request)
    }
}

struct MacOSMetalInferenceObservation: Equatable, Sendable {
    var schemaVersion: UInt16
    var requestIdentifierValid: Bool
    var profileIdentifierValid: Bool
    var manifestDigestValid: Bool
    var artifactDigestValid: Bool
    var artifactByteCount: UInt64
    var runtimeDigestValid: Bool
    var tokenizerDigestValid: Bool
    var templateDigestValid: Bool
    var codecDigestValid: Bool
    var inputByteCount: Int
    var inputDigestMatches: Bool
    var contextTokens: UInt32
    var maximumOutputTokens: UInt32
    var temperatureBasisPoints: UInt16
    var deadlineMilliseconds: UInt64
}

enum MacOSMetalInferenceAdmission {
    static func admit(
        request: MacOSMetalInferenceRequest,
        observation: MacOSMetalInferenceObservation
    ) throws -> VerifiedMacOSMetalInferenceRequest {
        guard observation.schemaVersion == 1 else {
            throw MacOSMetalInferenceFailure.unsupportedSchema
        }
        guard observation.requestIdentifierValid else {
            throw MacOSMetalInferenceFailure.invalidIdentity
        }
        guard observation.profileIdentifierValid else {
            throw MacOSMetalInferenceFailure.invalidProfile
        }
        guard observation.manifestDigestValid else {
            throw MacOSMetalInferenceFailure.invalidManifestDigest
        }
        guard observation.artifactDigestValid,
              observation.artifactByteCount > 0,
              observation.artifactByteCount <= agentMageMacOSMetalMaximumModelBytes
        else {
            throw MacOSMetalInferenceFailure.invalidArtifactDigest
        }
        guard observation.runtimeDigestValid else {
            throw MacOSMetalInferenceFailure.invalidRuntimeDigest
        }
        guard observation.tokenizerDigestValid,
              observation.templateDigestValid,
              observation.codecDigestValid
        else {
            throw MacOSMetalInferenceFailure.invalidCodecDigest
        }
        guard observation.inputByteCount > 0 else {
            throw MacOSMetalInferenceFailure.invalidInput
        }
        guard observation.inputByteCount <= agentMageMacOSMetalInputMaximumBytes else {
            throw MacOSMetalInferenceFailure.inputLimitExceeded
        }
        guard observation.inputDigestMatches else {
            throw MacOSMetalInferenceFailure.inputDigestMismatch
        }
        guard observation.contextTokens > 0,
              observation.contextTokens <= agentMageMacOSMetalMaximumContextTokens
        else {
            throw MacOSMetalInferenceFailure.invalidContext
        }
        guard observation.maximumOutputTokens > 0,
              observation.maximumOutputTokens <= agentMageMacOSMetalMaximumOutputTokens,
              observation.maximumOutputTokens < observation.contextTokens
        else {
            throw MacOSMetalInferenceFailure.invalidOutputLimit
        }
        guard observation.temperatureBasisPoints <= 20_000 else {
            throw MacOSMetalInferenceFailure.invalidSampling
        }
        guard observation.deadlineMilliseconds > 0,
              observation.deadlineMilliseconds
                <= agentMageMacOSMetalMaximumDeadlineMilliseconds
        else {
            throw MacOSMetalInferenceFailure.invalidDeadline
        }
        return VerifiedMacOSMetalInferenceRequest(request)
    }
}

enum MacOSMetalInferenceCodec {
    private static let keys: Set<String> = [
        "schemaVersion", "requestIdentifier", "profileIdentifier",
        "manifestSHA256", "artifactSHA256", "artifactByteCount",
        "runtimeSHA256", "tokenizerSHA256", "templateSHA256", "codecSHA256",
        "inputData", "inputSHA256", "contextTokens", "maximumOutputTokens",
        "seed", "temperatureBasisPoints", "deadlineMilliseconds",
    ]

    static func decodeClosed(_ data: Data) throws -> VerifiedMacOSMetalInferenceRequest {
        guard !data.isEmpty, data.count <= agentMageMacOSMetalXPCMaximumBytes,
              let object = try? JSONSerialization.jsonObject(with: data),
              let dictionary = object as? [String: Any],
              Set(dictionary.keys) == keys,
              let request = try? JSONDecoder().decode(
                MacOSMetalInferenceRequest.self,
                from: data
              )
        else {
            throw MacOSMetalInferenceFailure.malformedEnvelope
        }
        let observation = MacOSMetalInferenceObservation(
            schemaVersion: request.schemaVersion,
            requestIdentifierValid: validIdentifier(request.requestIdentifier),
            profileIdentifierValid: validIdentifier(request.profileIdentifier),
            manifestDigestValid: validSHA256(request.manifestSHA256),
            artifactDigestValid: validSHA256(request.artifactSHA256),
            artifactByteCount: request.artifactByteCount,
            runtimeDigestValid: validSHA256(request.runtimeSHA256),
            tokenizerDigestValid: validSHA256(request.tokenizerSHA256),
            templateDigestValid: validSHA256(request.templateSHA256),
            codecDigestValid: validSHA256(request.codecSHA256),
            inputByteCount: request.inputData.count,
            inputDigestMatches: metalSHA256(request.inputData) == request.inputSHA256,
            contextTokens: request.contextTokens,
            maximumOutputTokens: request.maximumOutputTokens,
            temperatureBasisPoints: request.temperatureBasisPoints,
            deadlineMilliseconds: request.deadlineMilliseconds
        )
        return try MacOSMetalInferenceAdmission.admit(
            request: request,
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
}

final class MacOSMetalRequestRegistry: @unchecked Sendable {
    private let lock = NSLock()
    private var seen = Set<String>()
    private var active: String?
    private var cancelled = Set<String>()

    func begin(_ requestIdentifier: String) throws {
        lock.lock()
        defer { lock.unlock() }
        guard seen.count < agentMageMacOSMetalMaximumRequestIdentities else {
            throw MacOSMetalInferenceFailure.capacityExceeded
        }
        guard !seen.contains(requestIdentifier) else {
            throw MacOSMetalInferenceFailure.replay
        }
        guard active == nil else {
            throw MacOSMetalInferenceFailure.executorFailed
        }
        seen.insert(requestIdentifier)
        active = requestIdentifier
    }

    func cancel(_ requestIdentifier: String) {
        lock.lock()
        if active == requestIdentifier {
            cancelled.insert(requestIdentifier)
        }
        lock.unlock()
    }

    func isCancelled(_ requestIdentifier: String) -> Bool {
        lock.lock()
        defer { lock.unlock() }
        return cancelled.contains(requestIdentifier)
    }

    func finish(_ requestIdentifier: String) {
        lock.lock()
        if active == requestIdentifier {
            active = nil
        }
        cancelled.remove(requestIdentifier)
        lock.unlock()
    }
}

public struct MacOSMetalInferenceReply: Codable, Equatable, Sendable {
    public let schemaVersion: UInt16
    public let outcome: String
    public let failureCode: String?
    public let resultData: Data?
    public let resultSHA256: String?

    public static func succeeded(_ data: Data) throws -> Self {
        guard data.count <= agentMageMacOSMetalResultMaximumBytes else {
            throw MacOSMetalInferenceFailure.resultLimitExceeded
        }
        return Self(
            schemaVersion: 1,
            outcome: "succeeded",
            failureCode: nil,
            resultData: data,
            resultSHA256: metalSHA256(data)
        )
    }

    public static func failed(_ failure: MacOSMetalInferenceFailure) -> Self {
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
        guard data.count <= agentMageMacOSMetalResultMaximumBytes else {
            throw MacOSMetalInferenceFailure.resultEncodingFailed
        }
        return data
    }
}

func validSHA256(_ value: String) -> Bool {
    value.utf8.count == 64 && value.allSatisfy { $0.isHexDigit && !$0.isUppercase }
}

func metalSHA256(_ data: Data) -> String {
    SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
}
