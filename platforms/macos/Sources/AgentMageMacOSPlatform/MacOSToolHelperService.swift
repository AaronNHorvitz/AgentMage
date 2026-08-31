import Darwin
import Foundation
import Security

public struct MacOSToolHelperExpectedIdentity: Equatable, Sendable {
    public let helperBundleIdentifier: String
    public let hostBundleIdentifier: String
    public let teamIdentifier: String
    public let appGroupIdentifier: String
    public let keychainAccessGroup: String
    public let hostDesignatedRequirement: String
    public let effectiveUserIdentifier: uid_t
    public let effectiveGroupIdentifier: gid_t

    public init(
        helperBundleIdentifier: String,
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
        guard !helperBundleIdentifier.isEmpty,
              !hostBundleIdentifier.isEmpty,
              teamIdentifier.utf8.count == 10,
              !appGroupIdentifier.isEmpty,
              !keychainAccessGroup.isEmpty,
              hostDesignatedRequirement == exactRequirement
        else {
            throw MacOSToolHelperFailure.invalidIdentity
        }
        self.helperBundleIdentifier = helperBundleIdentifier
        self.hostBundleIdentifier = hostBundleIdentifier
        self.teamIdentifier = teamIdentifier
        self.appGroupIdentifier = appGroupIdentifier
        self.keychainAccessGroup = keychainAccessGroup
        self.hostDesignatedRequirement = hostDesignatedRequirement
        self.effectiveUserIdentifier = effectiveUserIdentifier
        self.effectiveGroupIdentifier = effectiveGroupIdentifier
    }
}

struct MacOSToolHelperIdentityObservation: Equatable, Sendable {
    var signatureValid: Bool
    var hardenedRuntimePresent: Bool
    var bundleIdentifier: String?
    var teamIdentifier: String?
    var sandboxEnabled: Bool
    var appScopedBookmarksEnabled: Bool
    var unexpectedEntitlementKeys: Set<String>
}

struct MacOSToolHostIdentityObservation: Equatable, Sendable {
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

enum MacOSToolIdentityAdmission {
    static func admitHelper(
        _ observation: MacOSToolHelperIdentityObservation,
        expected: MacOSToolHelperExpectedIdentity
    ) throws {
        guard observation.signatureValid,
              observation.hardenedRuntimePresent,
              observation.bundleIdentifier == expected.helperBundleIdentifier,
              observation.teamIdentifier == expected.teamIdentifier,
              observation.sandboxEnabled,
              observation.appScopedBookmarksEnabled,
              observation.unexpectedEntitlementKeys.isEmpty
        else {
            throw MacOSToolHelperFailure.helperIdentityRejected
        }
    }

    static func admitHost(
        _ observation: MacOSToolHostIdentityObservation,
        expected: MacOSToolHelperExpectedIdentity
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
            throw MacOSToolHelperFailure.hostIdentityRejected
        }
    }
}

enum SecurityFrameworkMacOSToolIdentityObserver {
    private static let helperEntitlements: Set<String> = [
        "com.apple.security.app-sandbox",
        "com.apple.security.files.bookmarks.app-scope",
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

    static func observeHelper() throws -> MacOSToolHelperIdentityObservation {
        var code: SecCode?
        guard SecCodeCopySelf(SecCSFlags(), &code) == errSecSuccess, let code else {
            throw MacOSToolHelperFailure.helperIdentityRejected
        }
        let signing = try signingInformation(code)
        let entitlements = signing[kSecCodeInfoEntitlementsDict as String]
            as? [String: Any] ?? [:]
        return MacOSToolHelperIdentityObservation(
            signatureValid: SecCodeCheckValidity(code, SecCSFlags(), nil) == errSecSuccess,
            hardenedRuntimePresent: signing[kSecCodeInfoRuntimeVersion as String] != nil,
            bundleIdentifier: signing[kSecCodeInfoIdentifier as String] as? String,
            teamIdentifier: signing[kSecCodeInfoTeamIdentifier as String] as? String,
            sandboxEnabled: entitlements["com.apple.security.app-sandbox"] as? Bool == true,
            appScopedBookmarksEnabled:
                entitlements["com.apple.security.files.bookmarks.app-scope"] as? Bool == true,
            unexpectedEntitlementKeys: Set(entitlements.keys).subtracting(
                helperEntitlements.union(systemEntitlements)
            )
        )
    }

    static func observeHost(
        connection: NSXPCConnection,
        designatedRequirement: String
    ) throws -> MacOSToolHostIdentityObservation {
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
            throw MacOSToolHelperFailure.hostIdentityRejected
        }
        var requirement: SecRequirement?
        guard SecRequirementCreateWithString(
            designatedRequirement as CFString,
            SecCSFlags(),
            &requirement
        ) == errSecSuccess,
            let requirement
        else {
            throw MacOSToolHelperFailure.hostIdentityRejected
        }
        let signing = try signingInformation(code)
        let entitlements = signing[kSecCodeInfoEntitlementsDict as String]
            as? [String: Any] ?? [:]
        return MacOSToolHostIdentityObservation(
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
            throw MacOSToolHelperFailure.helperIdentityRejected
        }
        return signing
    }

    private static func stringSet(_ value: Any?) -> Set<String> {
        guard let values = value as? [String] else { return [] }
        return Set(values)
    }
}

public struct MacOSToolResourceLimits: Equatable, Sendable {
    public static let maximumOpenFiles: rlim_t = 64
    public static let maximumProcesses: rlim_t = 1
    public static let maximumCPUSeconds: rlim_t = 16
    public static let maximumAddressSpaceBytes: rlim_t = 512 * 1024 * 1024
    public static let maximumFileBytes: rlim_t = rlim_t(agentMageMacOSToolResultMaximumBytes)

    public static func apply() throws {
        try set(RLIMIT_CORE, 0)
        try set(RLIMIT_NOFILE, maximumOpenFiles)
        try set(RLIMIT_NPROC, maximumProcesses)
        try set(RLIMIT_CPU, maximumCPUSeconds)
        try set(RLIMIT_AS, maximumAddressSpaceBytes)
        try set(RLIMIT_FSIZE, maximumFileBytes)
    }

    private static func set(_ resource: Int32, _ ceiling: rlim_t) throws {
        var current = rlimit()
        guard getrlimit(resource, &current) == 0 else {
            throw MacOSToolHelperFailure.resourceLimitFailed
        }
        let bounded = min(current.rlim_max, ceiling)
        var limit = rlimit(rlim_cur: bounded, rlim_max: bounded)
        guard setrlimit(resource, &limit) == 0 else {
            throw MacOSToolHelperFailure.resourceLimitFailed
        }
    }
}

public final class MacOSToolScratchDirectory: @unchecked Sendable {
    public let fileDescriptor: Int32
    private let url: URL
    private let lock = NSLock()
    private var descriptorClosed = false
    private var removed = false

    public init() throws {
        let base = FileManager.default.temporaryDirectory
        guard base.isFileURL else {
            throw MacOSToolHelperFailure.scratchCreationFailed
        }
        var template = Array(
            base.appendingPathComponent("agentmage-xpc-XXXXXX", isDirectory: true)
                .path.utf8CString
        )
        let created = template.withUnsafeMutableBufferPointer { buffer in
            mkdtemp(buffer.baseAddress)
        }
        guard let created else {
            throw MacOSToolHelperFailure.scratchCreationFailed
        }
        url = URL(fileURLWithPath: String(cString: created), isDirectory: true)
        guard chmod(url.path, S_IRWXU) == 0 else {
            try? FileManager.default.removeItem(at: url)
            throw MacOSToolHelperFailure.scratchValidationFailed
        }
        var status = stat()
        guard lstat(url.path, &status) == 0,
              (status.st_mode & S_IFMT) == S_IFDIR,
              status.st_uid == geteuid(),
              (status.st_mode & 0o777) == 0o700
        else {
            try? FileManager.default.removeItem(at: url)
            throw MacOSToolHelperFailure.scratchValidationFailed
        }
        fileDescriptor = open(url.path, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC)
        guard fileDescriptor >= 0 else {
            try? FileManager.default.removeItem(at: url)
            throw MacOSToolHelperFailure.scratchValidationFailed
        }
    }

    deinit {
        try? closeAndRemove()
    }

    public func closeAndRemove() throws {
        lock.lock()
        if !descriptorClosed {
            close(fileDescriptor)
            descriptorClosed = true
        }
        guard !removed else {
            lock.unlock()
            return
        }
        do {
            try FileManager.default.removeItem(at: url)
            removed = true
            lock.unlock()
        } catch {
            lock.unlock()
            throw MacOSToolHelperFailure.scratchCleanupFailed
        }
    }

    public func validateBudget() throws {
        let keys: Set<URLResourceKey> = [
            .isRegularFileKey,
            .isDirectoryKey,
            .isSymbolicLinkKey,
            .fileSizeKey,
        ]
        var pending = [url]
        var entries = 0
        var bytes: UInt64 = 0
        while let directory = pending.popLast() {
            guard let children = try? FileManager.default.contentsOfDirectory(
                at: directory,
                includingPropertiesForKeys: Array(keys),
                options: []
            ) else {
                throw MacOSToolHelperFailure.scratchValidationFailed
            }
            for item in children {
                entries += 1
                guard entries <= agentMageMacOSToolScratchMaximumEntries,
                      let values = try? item.resourceValues(forKeys: keys),
                      values.isSymbolicLink != true,
                      values.isRegularFile == true || values.isDirectory == true
                else {
                    throw MacOSToolHelperFailure.scratchValidationFailed
                }
                if values.isDirectory == true {
                    pending.append(item)
                } else {
                    let fileSize = values.fileSize ?? -1
                    guard fileSize >= 0 else {
                        throw MacOSToolHelperFailure.scratchValidationFailed
                    }
                    let sum = bytes.addingReportingOverflow(UInt64(fileSize))
                    guard !sum.overflow,
                          sum.partialValue <= agentMageMacOSToolScratchMaximumBytes
                    else {
                        throw MacOSToolHelperFailure.scratchValidationFailed
                    }
                    bytes = sum.partialValue
                }
            }
        }
    }
}

public final class MacOSToolReadOnlyWorkspaceScope: @unchecked Sendable {
    public let rootFileDescriptor: Int32
    private let url: URL
    private let lock = NSLock()
    private var closed = false

    public init(bookmarkData: Data) throws {
        guard !bookmarkData.isEmpty,
              bookmarkData.count <= agentMageMacOSMaximumBookmarkBytes
        else {
            throw MacOSToolHelperFailure.invalidBookmark
        }
        var stale = false
        let resolved: URL
        do {
            resolved = try URL(
                resolvingBookmarkData: bookmarkData,
                options: [
                    .withSecurityScope,
                    .withoutUI,
                    .withoutMounting,
                    .withoutImplicitStartAccessing,
                ],
                relativeTo: nil,
                bookmarkDataIsStale: &stale
            )
        } catch {
            throw MacOSToolHelperFailure.bookmarkResolutionFailed
        }
        guard !stale else {
            throw MacOSToolHelperFailure.staleBookmark
        }
        guard resolved.startAccessingSecurityScopedResource() else {
            throw MacOSToolHelperFailure.securityScopeDenied
        }
        do {
            let before = try Self.snapshot(resolved)
            try MacOSWorkspaceAdmission.validate(before.observation(comparedWith: before))
            let descriptor = open(
                resolved.path,
                O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC
            )
            guard descriptor >= 0 else {
                throw MacOSToolHelperFailure.workspaceDescriptorFailed
            }
            do {
                let after = try Self.snapshot(resolved)
                try MacOSWorkspaceAdmission.validate(after.observation(comparedWith: before))
                var descriptorStatus = stat()
                var pathStatus = stat()
                guard fstat(descriptor, &descriptorStatus) == 0,
                      lstat(resolved.path, &pathStatus) == 0,
                      (descriptorStatus.st_mode & S_IFMT) == S_IFDIR,
                      (pathStatus.st_mode & S_IFMT) == S_IFDIR,
                      descriptorStatus.st_dev == pathStatus.st_dev,
                      descriptorStatus.st_ino == pathStatus.st_ino
                else {
                    throw MacOSToolHelperFailure.workspaceValidationFailed
                }
                rootFileDescriptor = descriptor
                url = resolved
            } catch {
                close(descriptor)
                throw error
            }
        } catch {
            resolved.stopAccessingSecurityScopedResource()
            if error is MacOSWorkspaceBookmarkFailure {
                throw MacOSToolHelperFailure.workspaceValidationFailed
            }
            throw error
        }
    }

    deinit {
        closeScope()
    }

    public func closeScope() {
        lock.lock()
        guard !closed else {
            lock.unlock()
            return
        }
        closed = true
        lock.unlock()
        close(rootFileDescriptor)
        url.stopAccessingSecurityScopedResource()
    }

    private static let resourceKeys: Set<URLResourceKey> = [
        .isDirectoryKey,
        .isSymbolicLinkKey,
        .isAliasFileKey,
        .fileResourceIdentifierKey,
        .volumeIdentifierKey,
    ]

    private struct WorkspaceSnapshot {
        let isFileURL: Bool
        let isDirectory: Bool
        let isSymbolicLink: Bool
        let isAliasFile: Bool
        let resourceIdentity: NSObject?
        let volumeIdentity: NSObject?
        let allNamesCanonical: Bool
        let hasCaseCollision: Bool
        let rootEntryCount: Int

        func observation(comparedWith prior: WorkspaceSnapshot) -> MacOSWorkspaceObservation {
            MacOSWorkspaceObservation(
                isFileURL: isFileURL,
                isDirectory: isDirectory,
                isSymbolicLink: isSymbolicLink,
                isAliasFile: isAliasFile,
                resourceIdentityPresent: resourceIdentity != nil,
                volumeIdentityPresent: volumeIdentity != nil,
                resourceIdentityStable: Self.same(resourceIdentity, prior.resourceIdentity),
                volumeIdentityStable: Self.same(volumeIdentity, prior.volumeIdentity),
                allNamesCanonical: allNamesCanonical,
                hasCaseCollision: hasCaseCollision,
                rootEntryCount: rootEntryCount
            )
        }

        private static func same(_ left: NSObject?, _ right: NSObject?) -> Bool {
            guard let left, let right else { return left == nil && right == nil }
            return left.isEqual(right)
        }
    }

    private static func snapshot(_ url: URL) throws -> WorkspaceSnapshot {
        let values: URLResourceValues
        let entries: [URL]
        do {
            values = try url.resourceValues(forKeys: resourceKeys)
            entries = try FileManager.default.contentsOfDirectory(
                at: url,
                includingPropertiesForKeys: nil,
                options: []
            )
        } catch {
            throw MacOSToolHelperFailure.workspaceValidationFailed
        }
        guard entries.count <= agentMageMacOSMaximumWorkspaceRootEntries else {
            throw MacOSToolHelperFailure.workspaceValidationFailed
        }
        let locale = Locale(identifier: "en_US_POSIX")
        var folded = Set<String>()
        var canonical = true
        var collision = false
        for entry in entries {
            let name = entry.lastPathComponent
            let normalized = name.precomposedStringWithCanonicalMapping
            canonical = canonical && name == normalized
            if !folded.insert(
                normalized.folding(options: [.caseInsensitive], locale: locale)
            ).inserted {
                collision = true
            }
        }
        return WorkspaceSnapshot(
            isFileURL: url.isFileURL,
            isDirectory: values.isDirectory == true,
            isSymbolicLink: values.isSymbolicLink == true,
            isAliasFile: values.isAliasFile == true,
            resourceIdentity: values.fileResourceIdentifier as? NSObject,
            volumeIdentity: values.volumeIdentifier as? NSObject,
            allNamesCanonical: canonical,
            hasCaseCollision: collision,
            rootEntryCount: entries.count
        )
    }
}

public protocol MacOSToolReadOnlyExecutor: AnyObject {
    func execute(
        requestData: Data,
        workspaceRootFileDescriptor: Int32,
        scratchFileDescriptor: Int32,
        deadlineMilliseconds: UInt64
    ) throws -> Data
}

@objc public protocol MacOSToolXPCProtocol {
    func execute(_ request: NSData, withReply reply: @escaping (NSData) -> Void)
}

public final class MacOSToolXPCServiceEndpoint: NSObject, MacOSToolXPCProtocol,
    @unchecked Sendable
{
    private let executor: MacOSToolReadOnlyExecutor
    private let gate = MacOSToolOneShotGate()

    public init(executor: MacOSToolReadOnlyExecutor) {
        self.executor = executor
    }

    public func execute(_ request: NSData, withReply reply: @escaping (NSData) -> Void) {
        let response: MacOSToolHelperReply
        do {
            try gate.consume()
            let now = UInt64(Date().timeIntervalSince1970 * 1_000)
            let verified = try MacOSToolInvocationCodec.decodeClosed(
                request as Data,
                nowEpochMilliseconds: now
            )
            let remaining = verified.invocation.expiresAtEpochMilliseconds - now
            let effectiveDeadline = min(
                verified.invocation.deadlineMilliseconds,
                remaining
            )
            let watchdog = DispatchSource.makeTimerSource(
                queue: DispatchQueue.global(qos: .userInitiated)
            )
            watchdog.schedule(deadline: .now() + .milliseconds(Int(effectiveDeadline)))
            watchdog.setEventHandler {
                _exit(124)
            }
            watchdog.resume()
            defer { watchdog.cancel() }
            let started = DispatchTime.now().uptimeNanoseconds
            let scratch = try MacOSToolScratchDirectory()
            defer { try? scratch.closeAndRemove() }
            let workspace = try MacOSToolReadOnlyWorkspaceScope(
                bookmarkData: verified.invocation.bookmarkData
            )
            defer { workspace.closeScope() }
            let result = try executor.execute(
                requestData: verified.invocation.requestData,
                workspaceRootFileDescriptor: workspace.rootFileDescriptor,
                scratchFileDescriptor: scratch.fileDescriptor,
                deadlineMilliseconds: effectiveDeadline
            )
            try scratch.validateBudget()
            workspace.closeScope()
            try scratch.closeAndRemove()
            let elapsed = (DispatchTime.now().uptimeNanoseconds - started) / 1_000_000
            guard elapsed <= effectiveDeadline else {
                throw MacOSToolHelperFailure.timeout
            }
            response = try MacOSToolHelperReply.succeeded(result)
        } catch let failure as MacOSToolHelperFailure {
            response = .failed(failure)
        } catch {
            response = .failed(.executorFailed)
        }
        let encoded = (try? response.encoded())
            ?? (try? MacOSToolHelperReply.failed(.resultEncodingFailed).encoded())
            ?? Data()
        reply(encoded as NSData)
        DispatchQueue.global(qos: .utility).asyncAfter(deadline: .now() + .milliseconds(100)) {
            _exit(0)
        }
    }
}

public final class MacOSToolHelperListenerDelegate: NSObject, NSXPCListenerDelegate {
    private let expectedIdentity: MacOSToolHelperExpectedIdentity
    private let executor: MacOSToolReadOnlyExecutor
    private let connectionGate = MacOSToolOneShotGate()

    public init(
        expectedIdentity: MacOSToolHelperExpectedIdentity,
        executor: MacOSToolReadOnlyExecutor
    ) throws {
        self.expectedIdentity = expectedIdentity
        self.executor = executor
        super.init()
        try MacOSToolIdentityAdmission.admitHelper(
            SecurityFrameworkMacOSToolIdentityObserver.observeHelper(),
            expected: expectedIdentity
        )
        try MacOSToolResourceLimits.apply()
    }

    public func listener(
        _ listener: NSXPCListener,
        shouldAcceptNewConnection newConnection: NSXPCConnection
    ) -> Bool {
        do {
            let observation = try SecurityFrameworkMacOSToolIdentityObserver.observeHost(
                connection: newConnection,
                designatedRequirement: expectedIdentity.hostDesignatedRequirement
            )
            try MacOSToolIdentityAdmission.admitHost(
                observation,
                expected: expectedIdentity
            )
            try connectionGate.consume()
        } catch {
            return false
        }
        newConnection.interruptionHandler = {
            _exit(125)
        }
        newConnection.invalidationHandler = {
            _exit(125)
        }
        newConnection.exportedInterface = NSXPCInterface(with: MacOSToolXPCProtocol.self)
        newConnection.exportedObject = MacOSToolXPCServiceEndpoint(executor: executor)
        newConnection.resume()
        return true
    }
}

public enum MacOSToolHelperServiceMain {
    public static func run(
        expectedIdentity: MacOSToolHelperExpectedIdentity,
        executor: MacOSToolReadOnlyExecutor
    ) throws -> Never {
        let delegate = try MacOSToolHelperListenerDelegate(
            expectedIdentity: expectedIdentity,
            executor: executor
        )
        let listener = NSXPCListener.service()
        listener.delegate = delegate
        listener.resume()
        dispatchMain()
    }
}
