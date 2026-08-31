import Testing
@testable import AgentMageMacOSPlatform

private enum LifecycleRecoveryScenario: String, CaseIterable {
    case staleBookmark = "stale-bookmark"
    case revokedBookmark = "revoked-bookmark"
    case helperCrash = "helper-crash"
    case hostCrash = "host-crash"
    case interruptedInstall = "interrupted-install"
    case failedLaunch = "failed-launch"
    case uninstall = "uninstall"
    case rollback = "rollback"

    var expectedTerminalClass: String {
        switch self {
        case .staleBookmark: "refreshed-after-revalidation"
        case .revokedBookmark: "revoked-access-denied"
        case .helperCrash: "helper-terminated-cleanly"
        case .hostCrash: "host-terminated-cleanly"
        case .interruptedInstall: "prior-package-preserved"
        case .failedLaunch: "prior-package-restored"
        case .uninstall: "candidate-removed"
        case .rollback: "prior-package-restored-and-launched"
        }
    }
}

private struct LifecycleRecoveryObservation {
    var scenario: String
    var terminalClass: String
    var attempted: Bool
    var cleanupVerified: Bool
    var priorValidStateRecovered: Bool
    var descendantProcessCount: UInt64
    var residueCount: UInt64
    var workspaceModified: Bool
    var authorityBroadened: Bool
}

private enum LifecycleRecoveryFailure: String, Error {
    case scenarioMismatch = "macos.lifecycle.scenario-mismatch"
    case terminalClassMismatch = "macos.lifecycle.terminal-class-mismatch"
    case notAttempted = "macos.lifecycle.not-attempted"
    case cleanupIncomplete = "macos.lifecycle.cleanup-incomplete"
    case priorStateNotRecovered = "macos.lifecycle.prior-state-not-recovered"
    case descendantsRemain = "macos.lifecycle.descendants-remain"
    case residueRemains = "macos.lifecycle.residue-remains"
    case workspaceModified = "macos.lifecycle.workspace-modified"
    case authorityBroadened = "macos.lifecycle.authority-broadened"
}

private protocol LifecycleRecoveryProbe {
    func exercise(_ scenario: LifecycleRecoveryScenario) throws
        -> LifecycleRecoveryObservation
}

private enum LifecycleRecoveryCampaign {
    static func run(using probe: any LifecycleRecoveryProbe) throws
        -> [LifecycleRecoveryObservation]
    {
        try LifecycleRecoveryScenario.allCases.map { scenario in
            let result = try probe.exercise(scenario)
            guard result.scenario == scenario.rawValue else {
                throw LifecycleRecoveryFailure.scenarioMismatch
            }
            guard result.terminalClass == scenario.expectedTerminalClass else {
                throw LifecycleRecoveryFailure.terminalClassMismatch
            }
            guard result.attempted else {
                throw LifecycleRecoveryFailure.notAttempted
            }
            guard result.cleanupVerified else {
                throw LifecycleRecoveryFailure.cleanupIncomplete
            }
            guard result.priorValidStateRecovered else {
                throw LifecycleRecoveryFailure.priorStateNotRecovered
            }
            guard result.descendantProcessCount == 0 else {
                throw LifecycleRecoveryFailure.descendantsRemain
            }
            guard result.residueCount == 0 else {
                throw LifecycleRecoveryFailure.residueRemains
            }
            guard !result.workspaceModified else {
                throw LifecycleRecoveryFailure.workspaceModified
            }
            guard !result.authorityBroadened else {
                throw LifecycleRecoveryFailure.authorityBroadened
            }
            return result
        }
    }
}

private enum UnsafeRecoveryMutation: CaseIterable {
    case scenario
    case terminalClass
    case attempted
    case cleanup
    case priorState
    case descendants
    case residue
    case workspace
    case authority

    var failure: LifecycleRecoveryFailure {
        switch self {
        case .scenario: .scenarioMismatch
        case .terminalClass: .terminalClassMismatch
        case .attempted: .notAttempted
        case .cleanup: .cleanupIncomplete
        case .priorState: .priorStateNotRecovered
        case .descendants: .descendantsRemain
        case .residue: .residueRemains
        case .workspace: .workspaceModified
        case .authority: .authorityBroadened
        }
    }

    func apply(to result: inout LifecycleRecoveryObservation) {
        switch self {
        case .scenario: result.scenario = "substituted-scenario"
        case .terminalClass: result.terminalClass = "substituted-terminal"
        case .attempted: result.attempted = false
        case .cleanup: result.cleanupVerified = false
        case .priorState: result.priorValidStateRecovered = false
        case .descendants: result.descendantProcessCount = 1
        case .residue: result.residueCount = 1
        case .workspace: result.workspaceModified = true
        case .authority: result.authorityBroadened = true
        }
    }
}

private final class FixedLifecycleRecoveryProbe: LifecycleRecoveryProbe {
    private let mutation: UnsafeRecoveryMutation?
    private(set) var attemptedScenarios: [String] = []

    init(mutation: UnsafeRecoveryMutation? = nil) {
        self.mutation = mutation
    }

    func exercise(_ scenario: LifecycleRecoveryScenario) throws
        -> LifecycleRecoveryObservation
    {
        attemptedScenarios.append(scenario.rawValue)
        var result = LifecycleRecoveryObservation(
            scenario: scenario.rawValue,
            terminalClass: scenario.expectedTerminalClass,
            attempted: true,
            cleanupVerified: true,
            priorValidStateRecovered: true,
            descendantProcessCount: 0,
            residueCount: 0,
            workspaceModified: false,
            authorityBroadened: false
        )
        if attemptedScenarios.count == 1 {
            mutation?.apply(to: &result)
        }
        return result
    }
}

@Test("S-008-RT01 runs all eight lifecycle recovery scenarios")
func s008RT01RunsAllLifecycleRecoveryScenarios() throws {
    #expect(LifecycleRecoveryScenario.allCases.count == 8)
    #expect(Set(LifecycleRecoveryScenario.allCases.map(\.rawValue)).count == 8)
    let probe = FixedLifecycleRecoveryProbe()
    let results = try LifecycleRecoveryCampaign.run(using: probe)
    #expect(results.count == 8)
    #expect(
        probe.attemptedScenarios
            == LifecycleRecoveryScenario.allCases.map(\.rawValue)
    )
}

@Test("S-008-RT01 rejects every incomplete cleanup or recovery result")
func s008RT01RejectsEveryUnsafeRecoveryResult() throws {
    #expect(UnsafeRecoveryMutation.allCases.count == 9)
    for mutation in UnsafeRecoveryMutation.allCases {
        let probe = FixedLifecycleRecoveryProbe(mutation: mutation)
        #expect(throws: mutation.failure) {
            _ = try LifecycleRecoveryCampaign.run(using: probe)
        }
        #expect(probe.attemptedScenarios.count == 1)
    }
}
