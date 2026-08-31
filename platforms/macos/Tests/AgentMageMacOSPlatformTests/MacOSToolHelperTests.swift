import CryptoKit
import Foundation
import Testing
@testable import AgentMageMacOSPlatform

private func toolInvocation() -> MacOSToolInvocation {
    let request = Data("{\"schema_version\":1}".utf8)
    return MacOSToolInvocation(
        schemaVersion: 1,
        grantState: "consumed",
        grantIdentifier: "grant-operation-0001",
        attemptIdentifier: "attempt-operation-0001",
        operation: "workspace_read",
        toolIdentifier: "agentmage.workspace.read-file",
        toolVersion: "1.0.0",
        bookmarkIdentifier: "123e4567-e89b-12d3-a456-426614174000",
        bookmarkData: Data(repeating: 7, count: 128),
        bookmarkSHA256: SHA256.hash(data: Data(repeating: 7, count: 128))
            .map { String(format: "%02x", $0) }.joined(),
        requestData: request,
        requestSHA256: SHA256.hash(data: request)
            .map { String(format: "%02x", $0) }.joined(),
        consumedAtEpochMilliseconds: 1_000,
        expiresAtEpochMilliseconds: 16_000,
        deadlineMilliseconds: 15_000
    )
}

private func toolObservation() -> MacOSToolInvocationObservation {
    let invocation = toolInvocation()
    return MacOSToolInvocationObservation(
        schemaVersion: invocation.schemaVersion,
        grantState: invocation.grantState,
        grantIdentifierValid: true,
        attemptIdentifierValid: true,
        operation: invocation.operation,
        toolIdentifierValid: true,
        toolVersionValid: true,
        bookmarkIdentifierValid: true,
        bookmarkByteCount: invocation.bookmarkData.count,
        bookmarkDigestMatches: true,
        requestByteCount: invocation.requestData.count,
        requestDigestMatches: true,
        consumedAtEpochMilliseconds: invocation.consumedAtEpochMilliseconds,
        expiresAtEpochMilliseconds: invocation.expiresAtEpochMilliseconds,
        deadlineMilliseconds: invocation.deadlineMilliseconds,
        nowEpochMilliseconds: 1_001
    )
}

@Test("one invocation requires an already-consumed exact read grant")
func oneInvocationRequiresConsumedReadGrant() throws {
    _ = try MacOSToolInvocationAdmission.admit(
        invocation: toolInvocation(),
        observation: toolObservation()
    )

    var cases: [(MacOSToolInvocationObservation, MacOSToolHelperFailure)] = []
    var item = toolObservation()
    item.schemaVersion = 2
    cases.append((item, .unsupportedSchema))
    item = toolObservation()
    item.grantIdentifierValid = false
    cases.append((item, .invalidIdentity))
    item = toolObservation()
    item.attemptIdentifierValid = false
    cases.append((item, .invalidIdentity))
    item = toolObservation()
    item.bookmarkIdentifierValid = false
    cases.append((item, .invalidIdentity))
    item = toolObservation()
    item.grantState = "issued"
    cases.append((item, .grantNotConsumed))
    item = toolObservation()
    item.operation = "workspace_write"
    cases.append((item, .wrongOperation))
    item = toolObservation()
    item.toolIdentifierValid = false
    cases.append((item, .invalidTool))
    item = toolObservation()
    item.bookmarkByteCount = 0
    cases.append((item, .invalidBookmark))
    item = toolObservation()
    item.bookmarkDigestMatches = false
    cases.append((item, .invalidBookmark))
    item = toolObservation()
    item.requestByteCount = agentMageMacOSToolRequestMaximumBytes + 1
    cases.append((item, .requestLimitExceeded))
    item = toolObservation()
    item.requestDigestMatches = false
    cases.append((item, .requestDigestMismatch))
    item = toolObservation()
    item.deadlineMilliseconds = agentMageMacOSToolMaximumDeadlineMilliseconds + 1
    cases.append((item, .invalidDeadline))
    item = toolObservation()
    item.nowEpochMilliseconds = item.expiresAtEpochMilliseconds
    cases.append((item, .expired))

    #expect(cases.count == 13)
    for (observation, failure) in cases {
        #expect(throws: failure) {
            _ = try MacOSToolInvocationAdmission.admit(
                invocation: toolInvocation(),
                observation: observation
            )
        }
    }
}

@Test("one endpoint consumes at most one invocation")
func endpointGateRejectsReplay() throws {
    let gate = MacOSToolOneShotGate()
    try gate.consume()
    #expect(throws: MacOSToolHelperFailure.replay) {
        try gate.consume()
    }
}

@Test("closed codec rejects an unknown field and digest substitution")
func closedCodecRejectsUnknownFieldsAndMutations() throws {
    let encoder = JSONEncoder()
    let encoded = try encoder.encode(toolInvocation())
    _ = try MacOSToolInvocationCodec.decodeClosed(
        encoded,
        nowEpochMilliseconds: 1_001
    )

    var object = try #require(
        JSONSerialization.jsonObject(with: encoded) as? [String: Any]
    )
    object["unknown"] = true
    let unknown = try JSONSerialization.data(withJSONObject: object)
    #expect(throws: MacOSToolHelperFailure.malformedEnvelope) {
        _ = try MacOSToolInvocationCodec.decodeClosed(
            unknown,
            nowEpochMilliseconds: 1_001
        )
    }

    object.removeValue(forKey: "unknown")
    object["requestSHA256"] = String(repeating: "0", count: 64)
    let substituted = try JSONSerialization.data(withJSONObject: object)
    #expect(throws: MacOSToolHelperFailure.requestDigestMismatch) {
        _ = try MacOSToolInvocationCodec.decodeClosed(
            substituted,
            nowEpochMilliseconds: 1_001
        )
    }
}

@Test("helper and host identities fail closed independently")
func helperAndHostIdentityMutationsFailClosed() throws {
    let expected = try MacOSToolHelperExpectedIdentity(
        helperBundleIdentifier: "com.example.agentmage.contractfixture.tool",
        hostBundleIdentifier: "com.example.agentmage.contractfixture.host",
        teamIdentifier: "AAAAAAAAAA",
        appGroupIdentifier: "group.com.example.agentmage.contractfixture",
        keychainAccessGroup: "AAAAAAAAAA.com.example.agentmage.contractfixture.keys",
        hostDesignatedRequirement: "anchor apple generic and identifier "
            + "com.example.agentmage.contractfixture.host "
            + "and certificate leaf[subject.OU] = AAAAAAAAAA",
        effectiveUserIdentifier: 501,
        effectiveGroupIdentifier: 20
    )
    let helper = MacOSToolHelperIdentityObservation(
        signatureValid: true,
        hardenedRuntimePresent: true,
        bundleIdentifier: expected.helperBundleIdentifier,
        teamIdentifier: expected.teamIdentifier,
        sandboxEnabled: true,
        appScopedBookmarksEnabled: true,
        unexpectedEntitlementKeys: []
    )
    let host = MacOSToolHostIdentityObservation(
        processIdentifier: 42,
        effectiveUserIdentifier: 501,
        effectiveGroupIdentifier: 20,
        signatureValid: true,
        designatedRequirementSatisfied: true,
        bundleIdentifier: expected.hostBundleIdentifier,
        teamIdentifier: expected.teamIdentifier,
        sandboxEnabled: true,
        appGroups: [expected.appGroupIdentifier],
        appScopedBookmarksEnabled: true,
        userSelectedReadOnlyEnabled: true,
        keychainAccessGroups: [expected.keychainAccessGroup],
        unexpectedEntitlementKeys: []
    )
    try MacOSToolIdentityAdmission.admitHelper(helper, expected: expected)
    try MacOSToolIdentityAdmission.admitHost(host, expected: expected)

    var changedHelper = helper
    changedHelper.unexpectedEntitlementKeys = ["com.apple.security.network.client"]
    #expect(throws: MacOSToolHelperFailure.helperIdentityRejected) {
        try MacOSToolIdentityAdmission.admitHelper(changedHelper, expected: expected)
    }
    var changedHost = host
    changedHost.designatedRequirementSatisfied = false
    #expect(throws: MacOSToolHelperFailure.hostIdentityRejected) {
        try MacOSToolIdentityAdmission.admitHost(changedHost, expected: expected)
    }
}

@Test("result bytes are bounded and digest bound")
func resultBytesAreBoundedAndDigestBound() throws {
    let bytes = Data("result".utf8)
    let result = try MacOSToolHelperReply.succeeded(bytes)
    #expect(result.resultData == bytes)
    #expect(result.resultSHA256?.count == 64)
    #expect(throws: MacOSToolHelperFailure.resultLimitExceeded) {
        _ = try MacOSToolHelperReply.succeeded(
            Data(count: agentMageMacOSToolResultMaximumBytes + 1)
        )
    }
}

@Test("failure taxonomy is closed and content free")
func toolHelperFailureTaxonomyIsClosed() {
    #expect(MacOSToolHelperFailure.allCases.count == 29)
    #expect(
        MacOSToolHelperFailure.allCases.allSatisfy {
            $0.rawValue.hasPrefix("macos.tool-helper.")
                && !$0.rawValue.contains("/")
                && !$0.rawValue.contains(" ")
        }
    )
}
