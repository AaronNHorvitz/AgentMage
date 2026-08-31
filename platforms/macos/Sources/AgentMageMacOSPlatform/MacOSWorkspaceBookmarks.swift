import AppKit
import Foundation
import Security

public let agentMageMacOSMaximumBookmarkBytes = 64 * 1024
public let agentMageMacOSMaximumWorkspaceRootEntries = 4_096

public struct MacOSWorkspaceBookmarkID: Hashable, Sendable {
    public let value: String

    public init(_ value: String) throws {
        guard value == value.lowercased(),
              let parsed = UUID(uuidString: value),
              parsed.uuidString.lowercased() == value
        else {
            throw MacOSWorkspaceBookmarkFailure.invalidBookmarkIdentifier
        }
        self.value = value
    }

    static func fresh() throws -> Self {
        try Self(UUID().uuidString.lowercased())
    }
}

public enum MacOSWorkspaceBookmarkFailure: String, Error, CaseIterable, Sendable {
    case invalidBookmarkIdentifier = "macos.workspace.invalid-bookmark-identifier"
    case pickerCancelled = "macos.workspace.picker-cancelled"
    case wrongSelectionCount = "macos.workspace.wrong-selection-count"
    case nonFileURL = "macos.workspace.non-file-url"
    case notDirectory = "macos.workspace.not-directory"
    case symbolicLink = "macos.workspace.symbolic-link"
    case aliasFile = "macos.workspace.alias-file"
    case resourceIdentityUnavailable = "macos.workspace.resource-identity-unavailable"
    case resourceIdentityChanged = "macos.workspace.resource-identity-changed"
    case volumeIdentityChanged = "macos.workspace.volume-identity-changed"
    case nonCanonicalName = "macos.workspace.noncanonical-name"
    case caseCollision = "macos.workspace.case-collision"
    case resourceLimitExceeded = "macos.workspace.resource-limit-exceeded"
    case bookmarkCreationFailed = "macos.workspace.bookmark-creation-failed"
    case malformedBookmarkRecord = "macos.workspace.malformed-bookmark-record"
    case duplicateBookmark = "macos.workspace.duplicate-bookmark"
    case bookmarkNotFound = "macos.workspace.bookmark-not-found"
    case bookmarkLoadFailed = "macos.workspace.bookmark-load-failed"
    case bookmarkResolutionFailed = "macos.workspace.bookmark-resolution-failed"
    case staleBookmarkRefreshFailed = "macos.workspace.stale-bookmark-refresh-failed"
    case securityScopeDenied = "macos.workspace.security-scope-denied"
    case bookmarkRevocationFailed = "macos.workspace.bookmark-revocation-failed"
}

struct MacOSWorkspaceObservation: Equatable, Sendable {
    var isFileURL: Bool
    var isDirectory: Bool
    var isSymbolicLink: Bool
    var isAliasFile: Bool
    var resourceIdentityPresent: Bool
    var volumeIdentityPresent: Bool
    var resourceIdentityStable: Bool
    var volumeIdentityStable: Bool
    var allNamesCanonical: Bool
    var hasCaseCollision: Bool
    var rootEntryCount: Int
}

enum MacOSWorkspaceAdmission {
    static func validate(_ observation: MacOSWorkspaceObservation) throws {
        guard observation.isFileURL else {
            throw MacOSWorkspaceBookmarkFailure.nonFileURL
        }
        guard observation.isDirectory else {
            throw MacOSWorkspaceBookmarkFailure.notDirectory
        }
        guard !observation.isSymbolicLink else {
            throw MacOSWorkspaceBookmarkFailure.symbolicLink
        }
        guard !observation.isAliasFile else {
            throw MacOSWorkspaceBookmarkFailure.aliasFile
        }
        guard observation.resourceIdentityPresent, observation.volumeIdentityPresent else {
            throw MacOSWorkspaceBookmarkFailure.resourceIdentityUnavailable
        }
        guard observation.resourceIdentityStable else {
            throw MacOSWorkspaceBookmarkFailure.resourceIdentityChanged
        }
        guard observation.volumeIdentityStable else {
            throw MacOSWorkspaceBookmarkFailure.volumeIdentityChanged
        }
        guard observation.allNamesCanonical else {
            throw MacOSWorkspaceBookmarkFailure.nonCanonicalName
        }
        guard !observation.hasCaseCollision else {
            throw MacOSWorkspaceBookmarkFailure.caseCollision
        }
        guard observation.rootEntryCount <= agentMageMacOSMaximumWorkspaceRootEntries else {
            throw MacOSWorkspaceBookmarkFailure.resourceLimitExceeded
        }
    }
}

@MainActor
public enum MacOSWorkspacePicker {
    public static func selectDirectory() async throws -> URL {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        panel.resolvesAliases = false
        panel.canCreateDirectories = false
        panel.treatsFilePackagesAsDirectories = false
        panel.prompt = "Authorize Read-Only Workspace"

        return try await withCheckedThrowingContinuation {
            (continuation: CheckedContinuation<URL, Error>) in
            panel.begin { response in
                guard response == .OK else {
                    continuation.resume(throwing: MacOSWorkspaceBookmarkFailure.pickerCancelled)
                    return
                }
                guard panel.urls.count == 1, let selected = panel.urls.first else {
                    continuation.resume(throwing: MacOSWorkspaceBookmarkFailure.wrongSelectionCount)
                    return
                }
                continuation.resume(returning: selected)
            }
        }
    }
}

struct MacOSStoredBookmark: Codable {
    let schemaVersion: UInt32
    let bookmarkIdentifier: String
    let bookmarkData: Data
}

/// Stores bookmark bytes in the signed host's access-group-scoped Keychain only.
final class KeychainMacOSWorkspaceBookmarkStore {
    private let lock = NSLock()
    private let service: String
    private let accessGroup: String

    init(service: String, accessGroup: String) throws {
        guard !service.isEmpty, service.utf8.count <= 255, !service.contains("\0"),
              !accessGroup.isEmpty, accessGroup.utf8.count <= 255,
              !accessGroup.contains("\0")
        else {
            throw MacOSWorkspaceBookmarkFailure.malformedBookmarkRecord
        }
        self.service = service
        self.accessGroup = accessGroup
    }

    func insert(_ record: MacOSStoredBookmark) throws {
        let value = try encode(record)
        lock.lock()
        defer { lock.unlock() }
        var query = baseQuery(record.bookmarkIdentifier)
        query[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        query[kSecValueData as String] = value
        let status = SecItemAdd(query as CFDictionary, nil)
        guard status != errSecDuplicateItem else {
            throw MacOSWorkspaceBookmarkFailure.duplicateBookmark
        }
        guard status == errSecSuccess else {
            throw MacOSWorkspaceBookmarkFailure.bookmarkCreationFailed
        }
    }

    func load(_ identifier: MacOSWorkspaceBookmarkID) throws -> MacOSStoredBookmark {
        lock.lock()
        defer { lock.unlock() }
        var query = baseQuery(identifier.value)
        query[kSecMatchLimit as String] = kSecMatchLimitOne
        query[kSecReturnData as String] = true
        var rawResult: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &rawResult)
        guard status != errSecItemNotFound else {
            throw MacOSWorkspaceBookmarkFailure.bookmarkNotFound
        }
        guard status == errSecSuccess, let value = rawResult as? Data else {
            throw MacOSWorkspaceBookmarkFailure.bookmarkLoadFailed
        }
        let record = try decode(value)
        guard record.bookmarkIdentifier == identifier.value else {
            throw MacOSWorkspaceBookmarkFailure.malformedBookmarkRecord
        }
        return record
    }

    func replace(_ record: MacOSStoredBookmark) throws {
        let value = try encode(record)
        lock.lock()
        defer { lock.unlock() }
        let status = SecItemUpdate(
            baseQuery(record.bookmarkIdentifier) as CFDictionary,
            [kSecValueData as String: value] as CFDictionary
        )
        guard status == errSecSuccess else {
            throw MacOSWorkspaceBookmarkFailure.staleBookmarkRefreshFailed
        }
    }

    func delete(_ identifier: MacOSWorkspaceBookmarkID) throws {
        lock.lock()
        defer { lock.unlock() }
        let status = SecItemDelete(baseQuery(identifier.value) as CFDictionary)
        guard status == errSecSuccess else {
            throw MacOSWorkspaceBookmarkFailure.bookmarkRevocationFailed
        }
    }

    private func baseQuery(_ account: String) -> [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            kSecAttrAccessGroup as String: accessGroup,
        ]
    }

    private func encode(_ record: MacOSStoredBookmark) throws -> Data {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys]
        let value: Data
        do {
            value = try encoder.encode(record)
        } catch {
            throw MacOSWorkspaceBookmarkFailure.malformedBookmarkRecord
        }
        guard value.count <= agentMageMacOSMaximumBookmarkBytes * 2 else {
            throw MacOSWorkspaceBookmarkFailure.resourceLimitExceeded
        }
        return value
    }

    private func decode(_ value: Data) throws -> MacOSStoredBookmark {
        guard value.count <= agentMageMacOSMaximumBookmarkBytes * 2,
              let record = try? JSONDecoder().decode(MacOSStoredBookmark.self, from: value),
              record.schemaVersion == 1,
              record.bookmarkData.count <= agentMageMacOSMaximumBookmarkBytes,
              !record.bookmarkData.isEmpty,
              (try? MacOSWorkspaceBookmarkID(record.bookmarkIdentifier)) != nil
        else {
            throw MacOSWorkspaceBookmarkFailure.malformedBookmarkRecord
        }
        return record
    }
}

/// Balanced access token. Deinitialization always relinquishes kernel scope.
public final class MacOSWorkspaceAccess: @unchecked Sendable {
    public let bookmarkIdentifier: MacOSWorkspaceBookmarkID
    private let lock = NSLock()
    private var scopedURL: URL?

    fileprivate init(identifier: MacOSWorkspaceBookmarkID, scopedURL: URL) {
        bookmarkIdentifier = identifier
        self.scopedURL = scopedURL
    }

    deinit {
        close()
    }

    public func withSecurityScopedURL<T>(_ body: (URL) throws -> T) throws -> T {
        lock.lock()
        guard let url = scopedURL else {
            lock.unlock()
            throw MacOSWorkspaceBookmarkFailure.securityScopeDenied
        }
        defer { lock.unlock() }
        return try body(url)
    }

    public func close() {
        lock.lock()
        let url = scopedURL
        scopedURL = nil
        lock.unlock()
        url?.stopAccessingSecurityScopedResource()
    }
}

/// Native picker, persistent app-scoped bookmark, resolution, refresh, and revoke lifecycle.
public final class MacOSWorkspaceBookmarkLifecycle: @unchecked Sendable {
    private final class WeakAccess {
        weak var value: MacOSWorkspaceAccess?

        init(_ value: MacOSWorkspaceAccess) {
            self.value = value
        }
    }

    private let store: KeychainMacOSWorkspaceBookmarkStore
    private let activeAccessLock = NSLock()
    private var activeAccesses: [String: [WeakAccess]] = [:]

    public init(keychainService: String, keychainAccessGroup: String) throws {
        store = try KeychainMacOSWorkspaceBookmarkStore(
            service: keychainService,
            accessGroup: keychainAccessGroup
        )
    }

    @MainActor
    public func selectAndPersistWorkspace() async throws -> MacOSWorkspaceBookmarkID {
        let selected = try await MacOSWorkspacePicker.selectDirectory()
        let before = try Self.snapshot(selected)
        try MacOSWorkspaceAdmission.validate(before.observation(comparedWith: before))
        let bookmarkData: Data
        do {
            bookmarkData = try selected.bookmarkData(
                options: [.withSecurityScope, .securityScopeAllowOnlyReadAccess],
                includingResourceValuesForKeys: Self.resourceKeyList,
                relativeTo: nil
            )
        } catch {
            throw MacOSWorkspaceBookmarkFailure.bookmarkCreationFailed
        }
        guard !bookmarkData.isEmpty,
              bookmarkData.count <= agentMageMacOSMaximumBookmarkBytes
        else {
            throw MacOSWorkspaceBookmarkFailure.resourceLimitExceeded
        }
        let after = try Self.snapshot(selected)
        try MacOSWorkspaceAdmission.validate(after.observation(comparedWith: before))
        let identifier = try MacOSWorkspaceBookmarkID.fresh()
        try store.insert(
            MacOSStoredBookmark(
                schemaVersion: 1,
                bookmarkIdentifier: identifier.value,
                bookmarkData: bookmarkData
            )
        )
        return identifier
    }

    public func openWorkspace(
        _ identifier: MacOSWorkspaceBookmarkID
    ) throws -> MacOSWorkspaceAccess {
        let record = try store.load(identifier)
        var stale = false
        let resolved: URL
        do {
            resolved = try URL(
                resolvingBookmarkData: record.bookmarkData,
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
            throw MacOSWorkspaceBookmarkFailure.bookmarkResolutionFailed
        }
        guard resolved.isFileURL else {
            throw MacOSWorkspaceBookmarkFailure.nonFileURL
        }
        guard resolved.startAccessingSecurityScopedResource() else {
            throw MacOSWorkspaceBookmarkFailure.securityScopeDenied
        }
        do {
            let before = try Self.snapshot(resolved)
            try MacOSWorkspaceAdmission.validate(before.observation(comparedWith: before))
            let after = try Self.snapshot(resolved)
            try MacOSWorkspaceAdmission.validate(after.observation(comparedWith: before))
            if stale {
                let refreshed: Data
                do {
                    refreshed = try resolved.bookmarkData(
                        options: [.withSecurityScope, .securityScopeAllowOnlyReadAccess],
                        includingResourceValuesForKeys: Self.resourceKeyList,
                        relativeTo: nil
                    )
                } catch {
                    throw MacOSWorkspaceBookmarkFailure.staleBookmarkRefreshFailed
                }
                guard !refreshed.isEmpty,
                      refreshed.count <= agentMageMacOSMaximumBookmarkBytes
                else {
                    throw MacOSWorkspaceBookmarkFailure.resourceLimitExceeded
                }
                try store.replace(
                    MacOSStoredBookmark(
                        schemaVersion: 1,
                        bookmarkIdentifier: identifier.value,
                        bookmarkData: refreshed
                    )
                )
            }
            let access = MacOSWorkspaceAccess(identifier: identifier, scopedURL: resolved)
            register(access)
            return access
        } catch {
            resolved.stopAccessingSecurityScopedResource()
            throw error
        }
    }

    public func revokeWorkspace(_ identifier: MacOSWorkspaceBookmarkID) throws {
        activeAccessLock.lock()
        let active = activeAccesses.removeValue(forKey: identifier.value) ?? []
        activeAccessLock.unlock()
        for item in active {
            item.value?.close()
        }
        try store.delete(identifier)
    }

    private func register(_ access: MacOSWorkspaceAccess) {
        activeAccessLock.lock()
        var current = activeAccesses[access.bookmarkIdentifier.value] ?? []
        current.removeAll { $0.value == nil }
        current.append(WeakAccess(access))
        activeAccesses[access.bookmarkIdentifier.value] = current
        activeAccessLock.unlock()
    }

    private static let resourceKeyList: [URLResourceKey] = [
        .isDirectoryKey,
        .isSymbolicLinkKey,
        .isAliasFileKey,
        .fileResourceIdentifierKey,
        .volumeIdentifierKey,
    ]

    private static let resourceKeys = Set(resourceKeyList)

    private struct Snapshot {
        let isFileURL: Bool
        let isDirectory: Bool
        let isSymbolicLink: Bool
        let isAliasFile: Bool
        let resourceIdentity: NSObject?
        let volumeIdentity: NSObject?
        let allNamesCanonical: Bool
        let hasCaseCollision: Bool
        let rootEntryCount: Int

        func observation(comparedWith prior: Snapshot) -> MacOSWorkspaceObservation {
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

    private static func snapshot(_ url: URL) throws -> Snapshot {
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
            throw MacOSWorkspaceBookmarkFailure.resourceIdentityUnavailable
        }
        guard entries.count <= agentMageMacOSMaximumWorkspaceRootEntries else {
            throw MacOSWorkspaceBookmarkFailure.resourceLimitExceeded
        }
        var foldedNames = Set<String>()
        var canonical = true
        var collision = false
        let locale = Locale(identifier: "en_US_POSIX")
        for entry in entries {
            let name = entry.lastPathComponent
            let normalized = name.precomposedStringWithCanonicalMapping
            canonical = canonical && normalized == name
            let folded = normalized.folding(options: [.caseInsensitive], locale: locale)
            if !foldedNames.insert(folded).inserted {
                collision = true
            }
        }
        return Snapshot(
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
