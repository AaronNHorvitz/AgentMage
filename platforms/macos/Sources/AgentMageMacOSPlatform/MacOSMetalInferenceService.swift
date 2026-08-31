import CryptoKit
import Darwin
import Foundation
import Metal
import Security

public struct MacOSMetalInferenceExpectedIdentity: Equatable, Sendable {
    public let serviceBundleIdentifier: String
    public let hostBundleIdentifier: String
    public let teamIdentifier: String
    public let appGroupIdentifier: String
    public let keychainAccessGroup: String
    public let hostDesignatedRequirement: String
    public let effectiveUserIdentifier: uid_t
    public let effectiveGroupIdentifier: gid_t

    public init(
        serviceBundleIdentifier: String,
        hostBundleIdentifier: String,
        teamIdentifier: String,
        appGroupIdentifier: String,
        keychainAccessGroup: String,
        hostDesignatedRequirement: String,
        effectiveUserIdentifier: uid_t,
        effectiveGroupIdentifier: gid_t
    ) throws {
        let exactRequirement = "anchor apple generic and identifier \(hostBundleIdentifier) "
            + "and certificate leaf[subject.OU] = \(teamIdentifier)"
        guard !serviceBundleIdentifier.isEmpty,
              !hostBundleIdentifier.isEmpty,
              teamIdentifier.utf8.count == 10,
              !appGroupIdentifier.isEmpty,
              !keychainAccessGroup.isEmpty,
              hostDesignatedRequirement == exactRequirement
        else {
            throw MacOSMetalInferenceFailure.invalidIdentity
        }
        self.serviceBundleIdentifier = serviceBundleIdentifier
        self.hostBundleIdentifier = hostBundleIdentifier
        self.teamIdentifier = teamIdentifier
        self.appGroupIdentifier = appGroupIdentifier
        self.keychainAccessGroup = keychainAccessGroup
        self.hostDesignatedRequirement = hostDesignatedRequirement
        self.effectiveUserIdentifier = effectiveUserIdentifier
        self.effectiveGroupIdentifier = effectiveGroupIdentifier
    }
}

struct MacOSMetalServiceIdentityObservation: Equatable, Sendable {
    var signatureValid: Bool
    var hardenedRuntimePresent: Bool
    var bundleIdentifier: String?
    var teamIdentifier: String?
    var sandboxEnabled: Bool
    var unexpectedEntitlementKeys: Set<String>
}

struct MacOSMetalHostIdentityObservation: Equatable, Sendable {
    var processIdentifier: pid_t
    var effectiveUserIdentifier: uid_t
    var effectiveGroupIdentifier: gid_t
    var signatureValid: Bool
    var designatedRequirementSatisfied: Bool
    var bundleIdentifier: String?
    var teamIdentifier: String?
    var sandboxEnabled: Bool
    var appGroups: Set<String>
    var appScopedBookmarksEnabled: Bool
    var userSelectedReadOnlyEnabled: Bool
    var keychainAccessGroups: Set<String>
    var unexpectedEntitlementKeys: Set<String>
}

enum MacOSMetalIdentityAdmission {
    static func admitService(
        _ observation: MacOSMetalServiceIdentityObservation,
        expected: MacOSMetalInferenceExpectedIdentity
    ) throws {
        guard observation.signatureValid,
              observation.hardenedRuntimePresent,
              observation.bundleIdentifier == expected.serviceBundleIdentifier,
              observation.teamIdentifier == expected.teamIdentifier,
              observation.sandboxEnabled,
              observation.unexpectedEntitlementKeys.isEmpty
        else {
            throw MacOSMetalInferenceFailure.serviceIdentityRejected
        }
    }

    static func admitHost(
        _ observation: MacOSMetalHostIdentityObservation,
        expected: MacOSMetalInferenceExpectedIdentity
    ) throws {
        guard observation.processIdentifier > 1,
              observation.effectiveUserIdentifier == expected.effectiveUserIdentifier,
              observation.effectiveGroupIdentifier == expected.effectiveGroupIdentifier,
              observation.signatureValid,
              observation.designatedRequirementSatisfied,
              observation.bundleIdentifier == expected.hostBundleIdentifier,
              observation.teamIdentifier == expected.teamIdentifier,
              observation.sandboxEnabled,
              observation.appGroups == [expected.appGroupIdentifier],
              observation.appScopedBookmarksEnabled,
              observation.userSelectedReadOnlyEnabled,
              observation.keychainAccessGroups == [expected.keychainAccessGroup],
              observation.unexpectedEntitlementKeys.isEmpty
        else {
            throw MacOSMetalInferenceFailure.hostIdentityRejected
        }
    }
}

enum SecurityFrameworkMacOSMetalIdentityObserver {
    private static let serviceEntitlements: Set<String> = [
        "com.apple.security.app-sandbox",
    ]
    private static let hostEntitlements: Set<String> = [
        "com.apple.security.app-sandbox",
        "com.apple.security.application-groups",
        "com.apple.security.files.bookmarks.app-scope",
        "com.apple.security.files.user-selected.read-only",
        "keychain-access-groups",
    ]
    private static let systemEntitlements: Set<String> = [
        "application-identifier",
        "com.apple.application-identifier",
        "com.apple.developer.team-identifier",
    ]

    static func observeService() throws -> MacOSMetalServiceIdentityObservation {
        var code: SecCode?
        guard SecCodeCopySelf(SecCSFlags(), &code) == errSecSuccess, let code else {
            throw MacOSMetalInferenceFailure.serviceIdentityRejected
        }
        let signing = try signingInformation(code)
        let entitlements = signing[kSecCodeInfoEntitlementsDict as String]
            as? [String: Any] ?? [:]
        return MacOSMetalServiceIdentityObservation(
            signatureValid: SecCodeCheckValidity(code, SecCSFlags(), nil) == errSecSuccess,
            hardenedRuntimePresent: signing[kSecCodeInfoRuntimeVersion as String] != nil,
            bundleIdentifier: signing[kSecCodeInfoIdentifier as String] as? String,
            teamIdentifier: signing[kSecCodeInfoTeamIdentifier as String] as? String,
            sandboxEnabled: entitlements["com.apple.security.app-sandbox"] as? Bool == true,
            unexpectedEntitlementKeys: Set(entitlements.keys).subtracting(
                serviceEntitlements.union(systemEntitlements)
            )
        )
    }

    static func observeHost(
        connection: NSXPCConnection,
        designatedRequirement: String
    ) throws -> MacOSMetalHostIdentityObservation {
        let attributes = [
            kSecGuestAttributePid as String: NSNumber(value: connection.processIdentifier),
        ] as CFDictionary
        var code: SecCode?
        guard SecCodeCopyGuestWithAttributes(
            nil,
            attributes,
            SecCSFlags(),
            &code
        ) == errSecSuccess,
            let code
        else {
            throw MacOSMetalInferenceFailure.hostIdentityRejected
        }
        var requirement: SecRequirement?
        guard SecRequirementCreateWithString(
            designatedRequirement as CFString,
            SecCSFlags(),
            &requirement
        ) == errSecSuccess,
            let requirement
        else {
            throw MacOSMetalInferenceFailure.hostIdentityRejected
        }
        let signing = try signingInformation(code)
        let entitlements = signing[kSecCodeInfoEntitlementsDict as String]
            as? [String: Any] ?? [:]
        return MacOSMetalHostIdentityObservation(
            processIdentifier: connection.processIdentifier,
            effectiveUserIdentifier: connection.effectiveUserIdentifier,
            effectiveGroupIdentifier: connection.effectiveGroupIdentifier,
            signatureValid: SecCodeCheckValidity(code, SecCSFlags(), nil) == errSecSuccess,
            designatedRequirementSatisfied:
                SecCodeCheckValidity(code, SecCSFlags(), requirement) == errSecSuccess,
            bundleIdentifier: signing[kSecCodeInfoIdentifier as String] as? String,
            teamIdentifier: signing[kSecCodeInfoTeamIdentifier as String] as? String,
            sandboxEnabled: entitlements["com.apple.security.app-sandbox"] as? Bool == true,
            appGroups: stringSet(entitlements["com.apple.security.application-groups"]),
            appScopedBookmarksEnabled:
                entitlements["com.apple.security.files.bookmarks.app-scope"] as? Bool == true,
            userSelectedReadOnlyEnabled:
                entitlements["com.apple.security.files.user-selected.read-only"] as? Bool == true,
            keychainAccessGroups: stringSet(entitlements["keychain-access-groups"]),
            unexpectedEntitlementKeys: Set(entitlements.keys).subtracting(
                hostEntitlements.union(systemEntitlements)
            )
        )
    }

    private static func signingInformation(_ code: SecCode) throws -> [String: Any] {
        var raw: CFDictionary?
        let flags = SecCSFlags(
            rawValue: kSecCSSigningInformation | kSecCSRequirementInformation
        )
        guard SecCodeCopySigningInformation(code, flags, &raw) == errSecSuccess,
              let signing = raw as? [String: Any]
        else {
            throw MacOSMetalInferenceFailure.serviceIdentityRejected
        }
        return signing
    }

    private static func stringSet(_ value: Any?) -> Set<String> {
        guard let values = value as? [String] else { return [] }
        return Set(values)
    }
}

public struct MacOSMetalResourceLimits: Equatable, Sendable {
    public static let maximumOpenFiles: rlim_t = 32
    public static let maximumProcesses: rlim_t = 1
    public static let maximumCPUSeconds: rlim_t = 300
    public static let maximumAddressSpaceBytes: rlim_t = 48 * 1024 * 1024 * 1024

    public static func apply() throws {
        try set(RLIMIT_CORE, 0)
        try set(RLIMIT_NOFILE, maximumOpenFiles)
        try set(RLIMIT_NPROC, maximumProcesses)
        try set(RLIMIT_CPU, maximumCPUSeconds)
        try set(RLIMIT_AS, maximumAddressSpaceBytes)
        try set(RLIMIT_FSIZE, 0)
    }

    private static func set(_ resource: Int32, _ ceiling: rlim_t) throws {
        var current = rlimit()
        guard getrlimit(resource, &current) == 0 else {
            throw MacOSMetalInferenceFailure.resourceLimitFailed
        }
        let bounded = min(current.rlim_max, ceiling)
        var limit = rlimit(rlim_cur: bounded, rlim_max: bounded)
        guard setrlimit(resource, &limit) == 0 else {
            throw MacOSMetalInferenceFailure.resourceLimitFailed
        }
    }
}

struct MacOSMetalModelDescriptorObservation: Equatable, Sendable {
    var descriptorValid: Bool
    var accessMode: Int32
    var regularFile: Bool
    var ownerMatches: Bool
    var linkCount: UInt64
    var groupOrWorldWritable: Bool
    var byteCount: UInt64
    var expectedByteCount: UInt64
    var digestMatches: Bool
}

enum MacOSMetalModelDescriptorAdmission {
    static func admit(_ observation: MacOSMetalModelDescriptorObservation) throws {
        guard observation.descriptorValid,
              observation.accessMode == O_RDONLY,
              observation.regularFile,
              observation.ownerMatches,
              observation.linkCount == 1,
              !observation.groupOrWorldWritable,
              observation.byteCount > 0,
              observation.byteCount <= agentMageMacOSMetalMaximumModelBytes,
              observation.byteCount == observation.expectedByteCount
        else {
            throw MacOSMetalInferenceFailure.modelDescriptorRejected
        }
        guard observation.digestMatches else {
            throw MacOSMetalInferenceFailure.modelDigestMismatch
        }
    }
}

public final class MacOSMetalReadOnlyModel: @unchecked Sendable {
    public let fileDescriptor: Int32
    private let lock = NSLock()
    private var closed = false

    public init(fileHandle: FileHandle, expected: MacOSMetalVerifiedProfile) throws {
        let duplicated = dup(fileHandle.fileDescriptor)
        guard duplicated >= 0 else {
            throw MacOSMetalInferenceFailure.modelDescriptorRejected
        }
        do {
            let flags = fcntl(duplicated, F_GETFL)
            var status = stat()
            let statusRead = fstat(duplicated, &status) == 0
            let byteCount = status.st_size > 0 ? UInt64(status.st_size) : 0
            let observedDigest = try Self.sha256(
                fileDescriptor: duplicated,
                byteCount: byteCount
            )
            let observation = MacOSMetalModelDescriptorObservation(
                descriptorValid: flags >= 0 && statusRead,
                accessMode: flags & O_ACCMODE,
                regularFile: (status.st_mode & S_IFMT) == S_IFREG,
                ownerMatches: status.st_uid == geteuid(),
                linkCount: UInt64(status.st_nlink),
                groupOrWorldWritable:
                    (status.st_mode & (S_IWGRP | S_IWOTH)) != 0,
                byteCount: byteCount,
                expectedByteCount: expected.artifactByteCount,
                digestMatches: observedDigest == expected.artifactSHA256
            )
            try MacOSMetalModelDescriptorAdmission.admit(observation)
            fileDescriptor = duplicated
        } catch {
            close(duplicated)
            throw error
        }
    }

    deinit {
        closeDescriptor()
    }

    public func closeDescriptor() {
        lock.lock()
        guard !closed else {
            lock.unlock()
            return
        }
        closed = true
        lock.unlock()
        close(fileDescriptor)
    }

    private static func sha256(
        fileDescriptor: Int32,
        byteCount: UInt64
    ) throws -> String {
        guard byteCount > 0, byteCount <= agentMageMacOSMetalMaximumModelBytes else {
            throw MacOSMetalInferenceFailure.modelDescriptorRejected
        }
        var hasher = SHA256()
        var offset: UInt64 = 0
        var buffer = [UInt8](repeating: 0, count: 1024 * 1024)
        while offset < byteCount {
            let requested = min(UInt64(buffer.count), byteCount - offset)
            let count = buffer.withUnsafeMutableBytes { bytes in
                pread(
                    fileDescriptor,
                    bytes.baseAddress,
                    Int(requested),
                    off_t(offset)
                )
            }
            guard count > 0 else {
                throw MacOSMetalInferenceFailure.modelDescriptorRejected
            }
            hasher.update(data: Data(buffer.prefix(count)))
            offset += UInt64(count)
        }
        return hasher.finalize().map { String(format: "%02x", $0) }.joined()
    }
}

final class MacOSMetalProfileLatch: @unchecked Sendable {
    private let lock = NSLock()
    private var profile: MacOSMetalVerifiedProfile?

    func latch(_ candidate: MacOSMetalVerifiedProfile) throws -> Bool {
        lock.lock()
        defer { lock.unlock() }
        if let profile {
            guard profile == candidate else {
                throw MacOSMetalInferenceFailure.profileSubstitution
            }
            return false
        }
        profile = candidate
        return true
    }
}

public protocol MacOSMetalInferenceExecutor: AnyObject {
    func loadModel(
        readOnlyModelFileDescriptor: Int32,
        profile: MacOSMetalVerifiedProfile
    ) throws

    func infer(
        inputData: Data,
        contextTokens: UInt32,
        maximumOutputTokens: UInt32,
        seed: UInt64,
        temperatureBasisPoints: UInt16,
        deadlineMilliseconds: UInt64,
        cancellationRequested: @escaping @Sendable () -> Bool
    ) throws -> Data
}

@objc public protocol MacOSMetalInferenceXPCProtocol {
    func infer(
        _ request: NSData,
        modelFile: FileHandle,
        withReply reply: @escaping (NSData) -> Void
    )
    func cancel(_ requestIdentifier: NSString)
}

public final class MacOSMetalInferenceXPCServiceEndpoint: NSObject,
    MacOSMetalInferenceXPCProtocol, @unchecked Sendable
{
    private let executor: MacOSMetalInferenceExecutor
    private let expectedProfile: MacOSMetalVerifiedProfile
    private let registry = MacOSMetalRequestRegistry()
    private let profileLatch = MacOSMetalProfileLatch()
    private let stateLock = NSLock()
    private var loadedModel: MacOSMetalReadOnlyModel?

    public init(
        expectedProfile: MacOSMetalVerifiedProfile,
        executor: MacOSMetalInferenceExecutor
    ) {
        self.expectedProfile = expectedProfile
        self.executor = executor
    }

    public func infer(
        _ request: NSData,
        modelFile: FileHandle,
        withReply reply: @escaping (NSData) -> Void
    ) {
        DispatchQueue.global(qos: .userInitiated).async { [self] in
            let response: MacOSMetalInferenceReply
            var requestIdentifier: String?
            do {
                let verified = try MacOSMetalInferenceCodec.decodeClosed(request as Data)
                guard verified.profile == expectedProfile else {
                    throw MacOSMetalInferenceFailure.profileSubstitution
                }
                requestIdentifier = verified.request.requestIdentifier
                try registry.begin(verified.request.requestIdentifier)
                defer { registry.finish(verified.request.requestIdentifier) }
                let firstProfileUse = try profileLatch.latch(verified.profile)
                if firstProfileUse {
                    let model = try MacOSMetalReadOnlyModel(
                        fileHandle: modelFile,
                        expected: verified.profile
                    )
                    do {
                        try executor.loadModel(
                            readOnlyModelFileDescriptor: model.fileDescriptor,
                            profile: verified.profile
                        )
                    } catch {
                        model.closeDescriptor()
                        throw error
                    }
                    stateLock.lock()
                    loadedModel = model
                    stateLock.unlock()
                } else {
                    modelFile.closeFile()
                }
                let watchdog = DispatchSource.makeTimerSource(
                    queue: DispatchQueue.global(qos: .userInitiated)
                )
                watchdog.schedule(
                    deadline: .now() + .milliseconds(
                        Int(verified.request.deadlineMilliseconds)
                    )
                )
                watchdog.setEventHandler { _exit(124) }
                watchdog.resume()
                defer { watchdog.cancel() }
                let started = DispatchTime.now().uptimeNanoseconds
                let result = try executor.infer(
                    inputData: verified.request.inputData,
                    contextTokens: verified.request.contextTokens,
                    maximumOutputTokens: verified.request.maximumOutputTokens,
                    seed: verified.request.seed,
                    temperatureBasisPoints: verified.request.temperatureBasisPoints,
                    deadlineMilliseconds: verified.request.deadlineMilliseconds,
                    cancellationRequested: {
                        self.registry.isCancelled(verified.request.requestIdentifier)
                    }
                )
                guard !registry.isCancelled(verified.request.requestIdentifier) else {
                    throw MacOSMetalInferenceFailure.cancelled
                }
                let elapsed = (DispatchTime.now().uptimeNanoseconds - started) / 1_000_000
                guard elapsed <= verified.request.deadlineMilliseconds else {
                    throw MacOSMetalInferenceFailure.timeout
                }
                response = try MacOSMetalInferenceReply.succeeded(result)
            } catch let failure as MacOSMetalInferenceFailure {
                response = .failed(failure)
            } catch {
                response = .failed(.executorFailed)
            }
            if let requestIdentifier,
               registry.isCancelled(requestIdentifier),
               response.outcome == "succeeded"
            {
                reply(
                    ((try? MacOSMetalInferenceReply.failed(.cancelled).encoded()) ?? Data())
                        as NSData
                )
                return
            }
            let encoded = (try? response.encoded())
                ?? (try? MacOSMetalInferenceReply.failed(.resultEncodingFailed).encoded())
                ?? Data()
            reply(encoded as NSData)
        }
    }

    public func cancel(_ requestIdentifier: NSString) {
        registry.cancel(requestIdentifier as String)
    }
}

final class MacOSMetalConnectionGate: @unchecked Sendable {
    private let lock = NSLock()
    private var consumed = false

    func consume() throws {
        lock.lock()
        defer { lock.unlock() }
        guard !consumed else {
            throw MacOSMetalInferenceFailure.connectionRejected
        }
        consumed = true
    }
}

public final class MacOSMetalInferenceListenerDelegate: NSObject,
    NSXPCListenerDelegate
{
    private let expectedIdentity: MacOSMetalInferenceExpectedIdentity
    private let expectedProfile: MacOSMetalVerifiedProfile
    private let executor: MacOSMetalInferenceExecutor
    private let connectionGate = MacOSMetalConnectionGate()

    public init(
        expectedIdentity: MacOSMetalInferenceExpectedIdentity,
        expectedProfile: MacOSMetalVerifiedProfile,
        executor: MacOSMetalInferenceExecutor
    ) throws {
        self.expectedIdentity = expectedIdentity
        self.expectedProfile = expectedProfile
        self.executor = executor
        super.init()
        try MacOSMetalIdentityAdmission.admitService(
            SecurityFrameworkMacOSMetalIdentityObserver.observeService(),
            expected: expectedIdentity
        )
        guard MTLCreateSystemDefaultDevice() != nil else {
            throw MacOSMetalInferenceFailure.metalUnavailable
        }
        try MacOSMetalResourceLimits.apply()
    }

    public func listener(
        _ listener: NSXPCListener,
        shouldAcceptNewConnection newConnection: NSXPCConnection
    ) -> Bool {
        do {
            let observation = try SecurityFrameworkMacOSMetalIdentityObserver.observeHost(
                connection: newConnection,
                designatedRequirement: expectedIdentity.hostDesignatedRequirement
            )
            try MacOSMetalIdentityAdmission.admitHost(
                observation,
                expected: expectedIdentity
            )
            try connectionGate.consume()
        } catch {
            return false
        }
        newConnection.interruptionHandler = { _exit(125) }
        newConnection.invalidationHandler = { _exit(125) }
        newConnection.exportedInterface = NSXPCInterface(
            with: MacOSMetalInferenceXPCProtocol.self
        )
        newConnection.exportedObject = MacOSMetalInferenceXPCServiceEndpoint(
            expectedProfile: expectedProfile,
            executor: executor
        )
        newConnection.resume()
        return true
    }
}

public enum MacOSMetalInferenceServiceMain {
    public static func run(
        expectedIdentity: MacOSMetalInferenceExpectedIdentity,
        expectedProfile: MacOSMetalVerifiedProfile,
        executor: MacOSMetalInferenceExecutor
    ) throws -> Never {
        let delegate = try MacOSMetalInferenceListenerDelegate(
            expectedIdentity: expectedIdentity,
            expectedProfile: expectedProfile,
            executor: executor
        )
        let listener = NSXPCListener.service()
        listener.delegate = delegate
        listener.resume()
        dispatchMain()
    }
}
