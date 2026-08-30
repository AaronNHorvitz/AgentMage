import Foundation

public let agentMageMacOSSocketRelativePath = "Library/Application Support/AgentMage/ipc/host.sock"

/// Allowlisted observations for the App Group socket before a peer can authenticate.
public struct AppGroupSocketObservation: Equatable, Sendable {
    public var insideExpectedAppGroupContainer: Bool
    public var parentIsDirectory: Bool
    public var parentIsSymbolicLink: Bool
    public var parentOwnerMatchesEffectiveUser: Bool
    public var parentMode: UInt16
    public var socketPathAbsentBeforeBind: Bool
    public var socketIsUnixDomainSocket: Bool
    public var socketIsSymbolicLink: Bool
    public var socketOwnerMatchesEffectiveUser: Bool
    public var socketMode: UInt16

    public init(
        insideExpectedAppGroupContainer: Bool,
        parentIsDirectory: Bool,
        parentIsSymbolicLink: Bool,
        parentOwnerMatchesEffectiveUser: Bool,
        parentMode: UInt16,
        socketPathAbsentBeforeBind: Bool,
        socketIsUnixDomainSocket: Bool,
        socketIsSymbolicLink: Bool,
        socketOwnerMatchesEffectiveUser: Bool,
        socketMode: UInt16
    ) {
        self.insideExpectedAppGroupContainer = insideExpectedAppGroupContainer
        self.parentIsDirectory = parentIsDirectory
        self.parentIsSymbolicLink = parentIsSymbolicLink
        self.parentOwnerMatchesEffectiveUser = parentOwnerMatchesEffectiveUser
        self.parentMode = parentMode
        self.socketPathAbsentBeforeBind = socketPathAbsentBeforeBind
        self.socketIsUnixDomainSocket = socketIsUnixDomainSocket
        self.socketIsSymbolicLink = socketIsSymbolicLink
        self.socketOwnerMatchesEffectiveUser = socketOwnerMatchesEffectiveUser
        self.socketMode = socketMode
    }
}

public enum AppGroupSocketFailure: String, Error, CaseIterable, Sendable {
    case outsideAppGroup = "macos.ipc.socket.outside-app-group"
    case unsafeParentType = "macos.ipc.socket.unsafe-parent-type"
    case wrongParentOwner = "macos.ipc.socket.wrong-parent-owner"
    case wrongParentMode = "macos.ipc.socket.wrong-parent-mode"
    case preexistingSocketPath = "macos.ipc.socket.preexisting-path"
    case wrongSocketType = "macos.ipc.socket.wrong-socket-type"
    case wrongSocketOwner = "macos.ipc.socket.wrong-socket-owner"
    case wrongSocketMode = "macos.ipc.socket.wrong-socket-mode"
}

/// Fail-closed App Group path and mode admission; native binding evidence remains blocked.
public enum AppGroupSocketBoundary {
    public static func validate(_ observation: AppGroupSocketObservation) throws {
        guard observation.insideExpectedAppGroupContainer else {
            throw AppGroupSocketFailure.outsideAppGroup
        }
        guard observation.parentIsDirectory, !observation.parentIsSymbolicLink else {
            throw AppGroupSocketFailure.unsafeParentType
        }
        guard observation.parentOwnerMatchesEffectiveUser else {
            throw AppGroupSocketFailure.wrongParentOwner
        }
        guard observation.parentMode == 0o700 else {
            throw AppGroupSocketFailure.wrongParentMode
        }
        guard observation.socketPathAbsentBeforeBind else {
            throw AppGroupSocketFailure.preexistingSocketPath
        }
        guard observation.socketIsUnixDomainSocket, !observation.socketIsSymbolicLink else {
            throw AppGroupSocketFailure.wrongSocketType
        }
        guard observation.socketOwnerMatchesEffectiveUser else {
            throw AppGroupSocketFailure.wrongSocketOwner
        }
        guard observation.socketMode == 0o600 else {
            throw AppGroupSocketFailure.wrongSocketMode
        }
    }
}
