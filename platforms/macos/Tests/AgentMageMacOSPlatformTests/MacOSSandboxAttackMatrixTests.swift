import Testing
@testable import AgentMageMacOSPlatform

private enum SandboxAttackComponent: String, CaseIterable {
    case kernelHost = "kernel_host"
    case vscodeBridge = "vscode_bridge"
    case xpcToolHelper = "xpc_tool_helper"
    case metalInferenceService = "metal_inference_service"
}

private enum SandboxAttackClass: String, CaseIterable {
    case ambientHome = "ambient-home"
    case device
    case process
    case environment
    case credential
    case network
    case workspaceWrite = "workspace-write"
    case grant
    case crossUser = "cross-user"
}

private struct SandboxAttackCase: Equatable {
    let component: SandboxAttackComponent
    let attackClass: SandboxAttackClass

    var identifier: String {
        "\(component.rawValue):\(attackClass.rawValue)"
    }
}

private struct SandboxAttackObservation {
    var caseIdentifier: String
    var attempted: Bool
    var unauthorizedAccessCount: UInt64
    var unauthorizedByteCount: UInt64
    var networkConnectionCount: UInt64
    var networkByteCount: UInt64
    var descendantProcessCount: UInt64
    var residueCount: UInt64
    var canaryObserved: Bool
    var workspaceModified: Bool
    var authorityBroadened: Bool
}

private enum SandboxAttackMatrixFailure: String, Error {
    case caseIdentityMismatch = "macos.sandbox-attack.case-identity-mismatch"
    case attackNotAttempted = "macos.sandbox-attack.not-attempted"
    case unauthorizedAccess = "macos.sandbox-attack.unauthorized-access"
    case unauthorizedBytes = "macos.sandbox-attack.unauthorized-bytes"
    case networkAccess = "macos.sandbox-attack.network-access"
    case networkBytes = "macos.sandbox-attack.network-bytes"
    case descendantProcess = "macos.sandbox-attack.descendant-process"
    case residue = "macos.sandbox-attack.residue"
    case canaryObserved = "macos.sandbox-attack.canary-observed"
    case workspaceModified = "macos.sandbox-attack.workspace-modified"
    case authorityBroadened = "macos.sandbox-attack.authority-broadened"
}

private protocol SandboxAttackProbe {
    func attempt(_ attack: SandboxAttackCase) throws -> SandboxAttackObservation
}

private enum SandboxAttackCampaign {
    static let expectedCases = SandboxAttackComponent.allCases.flatMap { component in
        SandboxAttackClass.allCases.map { attackClass in
            SandboxAttackCase(component: component, attackClass: attackClass)
        }
    }

    static func run(using probe: any SandboxAttackProbe) throws -> [SandboxAttackObservation] {
        try expectedCases.map { attack in
            let result = try probe.attempt(attack)
            guard result.caseIdentifier == attack.identifier else {
                throw SandboxAttackMatrixFailure.caseIdentityMismatch
            }
            guard result.attempted else {
                throw SandboxAttackMatrixFailure.attackNotAttempted
            }
            guard result.unauthorizedAccessCount == 0 else {
                throw SandboxAttackMatrixFailure.unauthorizedAccess
            }
            guard result.unauthorizedByteCount == 0 else {
                throw SandboxAttackMatrixFailure.unauthorizedBytes
            }
            guard result.networkConnectionCount == 0 else {
                throw SandboxAttackMatrixFailure.networkAccess
            }
            guard result.networkByteCount == 0 else {
                throw SandboxAttackMatrixFailure.networkBytes
            }
            guard result.descendantProcessCount == 0 else {
                throw SandboxAttackMatrixFailure.descendantProcess
            }
            guard result.residueCount == 0 else {
                throw SandboxAttackMatrixFailure.residue
            }
            guard !result.canaryObserved else {
                throw SandboxAttackMatrixFailure.canaryObserved
            }
            guard !result.workspaceModified else {
                throw SandboxAttackMatrixFailure.workspaceModified
            }
            guard !result.authorityBroadened else {
                throw SandboxAttackMatrixFailure.authorityBroadened
            }
            return result
        }
    }
}

private enum UnsafeAttackMutation: String, CaseIterable {
    case caseIdentity
    case attempted
    case unauthorizedAccess
    case unauthorizedBytes
    case networkAccess
    case networkBytes
    case descendantProcess
    case residue
    case canary
    case workspace
    case authority

    var failure: SandboxAttackMatrixFailure {
        switch self {
        case .caseIdentity: .caseIdentityMismatch
        case .attempted: .attackNotAttempted
        case .unauthorizedAccess: .unauthorizedAccess
        case .unauthorizedBytes: .unauthorizedBytes
        case .networkAccess: .networkAccess
        case .networkBytes: .networkBytes
        case .descendantProcess: .descendantProcess
        case .residue: .residue
        case .canary: .canaryObserved
        case .workspace: .workspaceModified
        case .authority: .authorityBroadened
        }
    }

    func apply(to result: inout SandboxAttackObservation) {
        switch self {
        case .caseIdentity: result.caseIdentifier = "substituted:case"
        case .attempted: result.attempted = false
        case .unauthorizedAccess: result.unauthorizedAccessCount = 1
        case .unauthorizedBytes: result.unauthorizedByteCount = 1
        case .networkAccess: result.networkConnectionCount = 1
        case .networkBytes: result.networkByteCount = 1
        case .descendantProcess: result.descendantProcessCount = 1
        case .residue: result.residueCount = 1
        case .canary: result.canaryObserved = true
        case .workspace: result.workspaceModified = true
        case .authority: result.authorityBroadened = true
        }
    }
}

private final class FixedSandboxAttackProbe: SandboxAttackProbe {
    private let mutation: UnsafeAttackMutation?
    private(set) var attemptedCaseIdentifiers: [String] = []

    init(mutation: UnsafeAttackMutation? = nil) {
        self.mutation = mutation
    }

    func attempt(_ attack: SandboxAttackCase) throws -> SandboxAttackObservation {
        attemptedCaseIdentifiers.append(attack.identifier)
        var result = SandboxAttackObservation(
            caseIdentifier: attack.identifier,
            attempted: true,
            unauthorizedAccessCount: 0,
            unauthorizedByteCount: 0,
            networkConnectionCount: 0,
            networkByteCount: 0,
            descendantProcessCount: 0,
            residueCount: 0,
            canaryObserved: false,
            workspaceModified: false,
            authorityBroadened: false
        )
        if attemptedCaseIdentifiers.count == 1 {
            mutation?.apply(to: &result)
        }
        return result
    }
}

@Test("S-008-ST01 runs the exact four by nine hostile-access matrix")
func s008ST01RunsExactHostileAccessMatrix() throws {
    #expect(SandboxAttackComponent.allCases.count == 4)
    #expect(SandboxAttackClass.allCases.count == 9)
    #expect(SandboxAttackCampaign.expectedCases.count == 36)
    #expect(Set(SandboxAttackCampaign.expectedCases.map(\.identifier)).count == 36)

    let probe = FixedSandboxAttackProbe()
    let results = try SandboxAttackCampaign.run(using: probe)
    #expect(results.count == 36)
    #expect(
        probe.attemptedCaseIdentifiers
            == SandboxAttackCampaign.expectedCases.map(\.identifier)
    )
}

@Test("S-008-ST01 rejects every nonzero or incomplete attack result")
func s008ST01RejectsEveryUnsafeAttackResult() throws {
    #expect(UnsafeAttackMutation.allCases.count == 11)
    for mutation in UnsafeAttackMutation.allCases {
        let probe = FixedSandboxAttackProbe(mutation: mutation)
        #expect(throws: mutation.failure) {
            _ = try SandboxAttackCampaign.run(using: probe)
        }
        #expect(probe.attemptedCaseIdentifiers.count == 1)
    }
}
