import Foundation
import Security

/// The release-derived identity values that a running kernel host must match.
public struct KernelHostExpectedIdentity: Equatable, Sendable {
    public let bundleIdentifier: String
    public let teamIdentifier: String
    public let appGroupIdentifier: String
    public let keychainAccessGroup: String

    public init(
        bundleIdentifier: String,
        teamIdentifier: String,
        appGroupIdentifier: String,
        keychainAccessGroup: String
    ) {
        self.bundleIdentifier = bundleIdentifier
        self.teamIdentifier = teamIdentifier
        self.appGroupIdentifier = appGroupIdentifier
        self.keychainAccessGroup = keychainAccessGroup
    }
}

/// Content-free facts observed from the running host and its embedded signature.
public struct KernelHostObservation: Equatable, Sendable {
    public var architectureIsArm64: Bool
    public var minimumMacOSVersionMet: Bool
    public var signatureValid: Bool
    public var hardenedRuntimePresent: Bool
    public var bundleIdentifier: String?
    public var teamIdentifier: String?
    public var sandboxEnabled: Bool
    public var appGroups: Set<String>
    public var appScopedBookmarksEnabled: Bool
    public var userSelectedReadOnlyEnabled: Bool
    public var keychainAccessGroups: Set<String>
    public var unexpectedEntitlementKeys: Set<String>

    public init(
        architectureIsArm64: Bool,
        minimumMacOSVersionMet: Bool,
        signatureValid: Bool,
        hardenedRuntimePresent: Bool,
        bundleIdentifier: String?,
        teamIdentifier: String?,
        sandboxEnabled: Bool,
        appGroups: Set<String>,
        appScopedBookmarksEnabled: Bool,
        userSelectedReadOnlyEnabled: Bool,
        keychainAccessGroups: Set<String>,
        unexpectedEntitlementKeys: Set<String>
    ) {
        self.architectureIsArm64 = architectureIsArm64
        self.minimumMacOSVersionMet = minimumMacOSVersionMet
        self.signatureValid = signatureValid
        self.hardenedRuntimePresent = hardenedRuntimePresent
        self.bundleIdentifier = bundleIdentifier
        self.teamIdentifier = teamIdentifier
        self.sandboxEnabled = sandboxEnabled
        self.appGroups = appGroups
        self.appScopedBookmarksEnabled = appScopedBookmarksEnabled
        self.userSelectedReadOnlyEnabled = userSelectedReadOnlyEnabled
        self.keychainAccessGroups = keychainAccessGroups
        self.unexpectedEntitlementKeys = unexpectedEntitlementKeys
    }
}

/// Stable, content-free startup refusal classes evaluated in fixed order.
public enum KernelHostAdmissionFailure: String, Error, CaseIterable, Sendable {
    case wrongArchitecture = "macos-host.wrong-architecture"
    case unsupportedOperatingSystem = "macos-host.unsupported-operating-system"
    case invalidCodeSignature = "macos-host.invalid-code-signature"
    case hardenedRuntimeMissing = "macos-host.hardened-runtime-missing"
    case wrongBundleIdentifier = "macos-host.wrong-bundle-identifier"
    case wrongTeamIdentifier = "macos-host.wrong-team-identifier"
    case appSandboxMissing = "macos-host.app-sandbox-missing"
    case wrongAppGroup = "macos-host.wrong-app-group"
    case appScopedBookmarksMissing = "macos-host.app-scoped-bookmarks-missing"
    case userSelectedReadOnlyMissing = "macos-host.user-selected-read-only-missing"
    case wrongKeychainAccessGroup = "macos-host.wrong-keychain-access-group"
    case entitlementClosureMismatch = "macos-host.entitlement-closure-mismatch"
}

/// The only constructor for a host identity that passed every startup check.
public struct VerifiedKernelHostIdentity: Equatable, Sendable {
    public let bundleIdentifier: String
    public let teamIdentifier: String
    public let appGroupIdentifier: String

    fileprivate init(expected: KernelHostExpectedIdentity) {
        bundleIdentifier = expected.bundleIdentifier
        teamIdentifier = expected.teamIdentifier
        appGroupIdentifier = expected.appGroupIdentifier
    }
}

/// Fail-closed admission for the signed, sandboxed arm64 kernel host.
public enum KernelHostAdmission {
    public static func admit(
        observation: KernelHostObservation,
        expected: KernelHostExpectedIdentity
    ) throws -> VerifiedKernelHostIdentity {
        guard observation.architectureIsArm64 else {
            throw KernelHostAdmissionFailure.wrongArchitecture
        }
        guard observation.minimumMacOSVersionMet else {
            throw KernelHostAdmissionFailure.unsupportedOperatingSystem
        }
        guard observation.signatureValid else {
            throw KernelHostAdmissionFailure.invalidCodeSignature
        }
        guard observation.hardenedRuntimePresent else {
            throw KernelHostAdmissionFailure.hardenedRuntimeMissing
        }
        guard observation.bundleIdentifier == expected.bundleIdentifier else {
            throw KernelHostAdmissionFailure.wrongBundleIdentifier
        }
        guard observation.teamIdentifier == expected.teamIdentifier else {
            throw KernelHostAdmissionFailure.wrongTeamIdentifier
        }
        guard observation.sandboxEnabled else {
            throw KernelHostAdmissionFailure.appSandboxMissing
        }
        guard observation.appGroups == [expected.appGroupIdentifier] else {
            throw KernelHostAdmissionFailure.wrongAppGroup
        }
        guard observation.appScopedBookmarksEnabled else {
            throw KernelHostAdmissionFailure.appScopedBookmarksMissing
        }
        guard observation.userSelectedReadOnlyEnabled else {
            throw KernelHostAdmissionFailure.userSelectedReadOnlyMissing
        }
        guard observation.keychainAccessGroups == [expected.keychainAccessGroup] else {
            throw KernelHostAdmissionFailure.wrongKeychainAccessGroup
        }
        guard observation.unexpectedEntitlementKeys.isEmpty else {
            throw KernelHostAdmissionFailure.entitlementClosureMismatch
        }
        return VerifiedKernelHostIdentity(expected: expected)
    }
}

public enum KernelHostObservationError: String, Error, Sendable {
    case cannotCopySelf = "macos-host.cannot-copy-self-code"
    case invalidCodeSignature = "macos-host.invalid-code-signature"
    case cannotReadSigningInformation = "macos-host.cannot-read-signing-information"
    case malformedSigningInformation = "macos-host.malformed-signing-information"
}

/// Reads only allowlisted, content-free identity facts from Security.framework.
public enum SecurityFrameworkKernelHostObserver {
    private static let requestedEntitlementKeys: Set<String> = [
        "com.apple.security.app-sandbox",
        "com.apple.security.application-groups",
        "com.apple.security.files.bookmarks.app-scope",
        "com.apple.security.files.user-selected.read-only",
        "keychain-access-groups",
    ]

    private static let systemIdentityEntitlementKeys: Set<String> = [
        "application-identifier",
        "com.apple.application-identifier",
        "com.apple.developer.team-identifier",
    ]

    public static func observeCurrentProcess() throws -> KernelHostObservation {
        var runningCode: SecCode?
        guard SecCodeCopySelf(SecCSFlags(), &runningCode) == errSecSuccess,
              let runningCode
        else {
            throw KernelHostObservationError.cannotCopySelf
        }
        guard SecCodeCheckValidity(runningCode, SecCSFlags(), nil) == errSecSuccess else {
            throw KernelHostObservationError.invalidCodeSignature
        }

        let requestedFlags = SecCSFlags(
            rawValue: kSecCSSigningInformation | kSecCSRequirementInformation
        )
        var rawSigningInformation: CFDictionary?
        guard SecCodeCopySigningInformation(
            runningCode,
            requestedFlags,
            &rawSigningInformation
        ) == errSecSuccess,
            let signingInformation = rawSigningInformation as? [String: Any]
        else {
            throw KernelHostObservationError.cannotReadSigningInformation
        }

        guard let entitlements = signingInformation[kSecCodeInfoEntitlementsDict as String]
            as? [String: Any]
        else {
            throw KernelHostObservationError.malformedSigningInformation
        }

        let entitlementKeys = Set(entitlements.keys)
        let allowedKeys = requestedEntitlementKeys.union(systemIdentityEntitlementKeys)
        let operatingSystem = ProcessInfo.processInfo.operatingSystemVersion

        return KernelHostObservation(
            architectureIsArm64: currentBuildIsArm64,
            minimumMacOSVersionMet: operatingSystem.majorVersion >= 15,
            signatureValid: true,
            hardenedRuntimePresent: signingInformation[kSecCodeInfoRuntimeVersion as String] != nil,
            bundleIdentifier: signingInformation[kSecCodeInfoIdentifier as String] as? String,
            teamIdentifier: signingInformation[kSecCodeInfoTeamIdentifier as String] as? String,
            sandboxEnabled: entitlements["com.apple.security.app-sandbox"] as? Bool == true,
            appGroups: stringSet(entitlements["com.apple.security.application-groups"]),
            appScopedBookmarksEnabled:
                entitlements["com.apple.security.files.bookmarks.app-scope"] as? Bool == true,
            userSelectedReadOnlyEnabled:
                entitlements["com.apple.security.files.user-selected.read-only"] as? Bool == true,
            keychainAccessGroups: stringSet(entitlements["keychain-access-groups"]),
            unexpectedEntitlementKeys: entitlementKeys.subtracting(allowedKeys)
        )
    }

    private static var currentBuildIsArm64: Bool {
        #if arch(arm64)
        true
        #else
        false
        #endif
    }

    private static func stringSet(_ value: Any?) -> Set<String> {
        guard let values = value as? [String] else { return [] }
        return Set(values)
    }
}
