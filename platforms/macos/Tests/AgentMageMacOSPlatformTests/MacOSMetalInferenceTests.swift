import Darwin
import Foundation
import Testing
@testable import AgentMageMacOSPlatform

private func metalRequest() -> MacOSMetalInferenceRequest {
    let input = Data("bounded prompt bytes".utf8)
    return MacOSMetalInferenceRequest(
        schemaVersion: 1,
        requestIdentifier: "request-inference-0001",
        profileIdentifier: "agentmage.muse-glimmer.text-8k",
        manifestSHA256: String(repeating: "1", count: 64),
        artifactSHA256: String(repeating: "2", count: 64),
        artifactByteCount: 17 * 1024 * 1024 * 1024,
        runtimeSHA256: String(repeating: "3", count: 64),
        tokenizerSHA256: String(repeating: "4", count: 64),
        templateSHA256: String(repeating: "5", count: 64),
        codecSHA256: String(repeating: "6", count: 64),
        inputData: input,
        inputSHA256: metalSHA256(input),
        contextTokens: 8_192,
        maximumOutputTokens: 1_024,
        seed: 42,
        temperatureBasisPoints: 7_000,
        deadlineMilliseconds: 120_000
    )
}

private func metalObservation() -> MacOSMetalInferenceObservation {
    let request = metalRequest()
    return MacOSMetalInferenceObservation(
        schemaVersion: request.schemaVersion,
        requestIdentifierValid: true,
        profileIdentifierValid: true,
        manifestDigestValid: true,
        artifactDigestValid: true,
        artifactByteCount: request.artifactByteCount,
        runtimeDigestValid: true,
        tokenizerDigestValid: true,
        templateDigestValid: true,
        codecDigestValid: true,
        inputByteCount: request.inputData.count,
        inputDigestMatches: true,
        contextTokens: request.contextTokens,
        maximumOutputTokens: request.maximumOutputTokens,
        temperatureBasisPoints: request.temperatureBasisPoints,
        deadlineMilliseconds: request.deadlineMilliseconds
    )
}

@Test("closed inference admission rejects every profile and budget mutation")
func inferenceAdmissionMutationsFailClosed() throws {
    _ = try MacOSMetalInferenceAdmission.admit(
        request: metalRequest(),
        observation: metalObservation()
    )
    var cases: [(MacOSMetalInferenceObservation, MacOSMetalInferenceFailure)] = []
    var item = metalObservation()
    item.schemaVersion = 2
    cases.append((item, .unsupportedSchema))
    item = metalObservation()
    item.requestIdentifierValid = false
    cases.append((item, .invalidIdentity))
    item = metalObservation()
    item.profileIdentifierValid = false
    cases.append((item, .invalidProfile))
    item = metalObservation()
    item.manifestDigestValid = false
    cases.append((item, .invalidManifestDigest))
    item = metalObservation()
    item.artifactDigestValid = false
    cases.append((item, .invalidArtifactDigest))
    item = metalObservation()
    item.runtimeDigestValid = false
    cases.append((item, .invalidRuntimeDigest))
    item = metalObservation()
    item.tokenizerDigestValid = false
    cases.append((item, .invalidCodecDigest))
    item = metalObservation()
    item.inputByteCount = 0
    cases.append((item, .invalidInput))
    item = metalObservation()
    item.inputByteCount = agentMageMacOSMetalInputMaximumBytes + 1
    cases.append((item, .inputLimitExceeded))
    item = metalObservation()
    item.inputDigestMatches = false
    cases.append((item, .inputDigestMismatch))
    item = metalObservation()
    item.contextTokens = agentMageMacOSMetalMaximumContextTokens + 1
    cases.append((item, .invalidContext))
    item = metalObservation()
    item.maximumOutputTokens = item.contextTokens
    cases.append((item, .invalidOutputLimit))
    item = metalObservation()
    item.temperatureBasisPoints = 20_001
    cases.append((item, .invalidSampling))
    item = metalObservation()
    item.deadlineMilliseconds = agentMageMacOSMetalMaximumDeadlineMilliseconds + 1
    cases.append((item, .invalidDeadline))

    #expect(cases.count == 14)
    for (observation, failure) in cases {
        #expect(throws: failure) {
            _ = try MacOSMetalInferenceAdmission.admit(
                request: metalRequest(),
                observation: observation
            )
        }
    }
}

@Test("codec is closed and binds input bytes to their digest")
func inferenceCodecRejectsUnknownFieldsAndInputSubstitution() throws {
    let encoded = try JSONEncoder().encode(metalRequest())
    _ = try MacOSMetalInferenceCodec.decodeClosed(encoded)
    var object = try #require(
        JSONSerialization.jsonObject(with: encoded) as? [String: Any]
    )
    object["workspaceBookmark"] = "forbidden"
    let unknown = try JSONSerialization.data(withJSONObject: object)
    #expect(throws: MacOSMetalInferenceFailure.malformedEnvelope) {
        _ = try MacOSMetalInferenceCodec.decodeClosed(unknown)
    }
    object.removeValue(forKey: "workspaceBookmark")
    object["inputSHA256"] = String(repeating: "0", count: 64)
    let changed = try JSONSerialization.data(withJSONObject: object)
    #expect(throws: MacOSMetalInferenceFailure.inputDigestMismatch) {
        _ = try MacOSMetalInferenceCodec.decodeClosed(changed)
    }
}

@Test("request registry serializes work and rejects identity replay")
func requestRegistryRejectsBusyAndReplay() throws {
    let registry = MacOSMetalRequestRegistry()
    try registry.begin("request-inference-0001")
    #expect(throws: MacOSMetalInferenceFailure.executorFailed) {
        try registry.begin("request-inference-0002")
    }
    registry.cancel("request-inference-0001")
    #expect(registry.isCancelled("request-inference-0001"))
    registry.finish("request-inference-0001")
    #expect(throws: MacOSMetalInferenceFailure.replay) {
        try registry.begin("request-inference-0001")
    }
}

@Test("service and host code identities fail closed independently")
func metalServiceIdentityMutationsFailClosed() throws {
    let expected = try MacOSMetalInferenceExpectedIdentity(
        serviceBundleIdentifier: "com.example.agentmage.contractfixture.inference",
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
    let service = MacOSMetalServiceIdentityObservation(
        signatureValid: true,
        hardenedRuntimePresent: true,
        bundleIdentifier: expected.serviceBundleIdentifier,
        teamIdentifier: expected.teamIdentifier,
        sandboxEnabled: true,
        unexpectedEntitlementKeys: []
    )
    let host = MacOSMetalHostIdentityObservation(
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
    try MacOSMetalIdentityAdmission.admitService(service, expected: expected)
    try MacOSMetalIdentityAdmission.admitHost(host, expected: expected)
    var changedService = service
    changedService.unexpectedEntitlementKeys = ["com.apple.security.network.client"]
    #expect(throws: MacOSMetalInferenceFailure.serviceIdentityRejected) {
        try MacOSMetalIdentityAdmission.admitService(changedService, expected: expected)
    }
    var changedHost = host
    changedHost.designatedRequirementSatisfied = false
    #expect(throws: MacOSMetalInferenceFailure.hostIdentityRejected) {
        try MacOSMetalIdentityAdmission.admitHost(changedHost, expected: expected)
    }
}

@Test("model admission accepts only an exact owner read-only regular descriptor")
func modelDescriptorMutationsFailClosed() throws {
    let valid = MacOSMetalModelDescriptorObservation(
        descriptorValid: true,
        accessMode: O_RDONLY,
        regularFile: true,
        ownerMatches: true,
        linkCount: 1,
        groupOrWorldWritable: false,
        byteCount: 4_096,
        expectedByteCount: 4_096,
        digestMatches: true
    )
    try MacOSMetalModelDescriptorAdmission.admit(valid)
    var cases = [MacOSMetalModelDescriptorObservation]()
    var item = valid
    item.accessMode = O_RDWR
    cases.append(item)
    item = valid
    item.regularFile = false
    cases.append(item)
    item = valid
    item.ownerMatches = false
    cases.append(item)
    item = valid
    item.linkCount = 2
    cases.append(item)
    item = valid
    item.groupOrWorldWritable = true
    cases.append(item)
    item = valid
    item.expectedByteCount = 4_095
    cases.append(item)
    for observation in cases {
        #expect(throws: MacOSMetalInferenceFailure.modelDescriptorRejected) {
            try MacOSMetalModelDescriptorAdmission.admit(observation)
        }
    }
    item = valid
    item.digestMatches = false
    #expect(throws: MacOSMetalInferenceFailure.modelDigestMismatch) {
        try MacOSMetalModelDescriptorAdmission.admit(item)
    }
}

@Test("profile latch and bounded replies reject substitution and oversized output")
func profileLatchAndReplyAreBounded() throws {
    let first = try MacOSMetalInferenceAdmission.admit(
        request: metalRequest(),
        observation: metalObservation()
    ).profile
    let latch = MacOSMetalProfileLatch()
    #expect(try latch.latch(first))
    #expect(try !latch.latch(first))
    var object = try #require(
        JSONSerialization.jsonObject(with: JSONEncoder().encode(metalRequest()))
            as? [String: Any]
    )
    object["artifactSHA256"] = String(repeating: "9", count: 64)
    let changed = try MacOSMetalInferenceCodec.decodeClosed(
        JSONSerialization.data(withJSONObject: object)
    ).profile
    #expect(throws: MacOSMetalInferenceFailure.profileSubstitution) {
        _ = try latch.latch(changed)
    }
    let output = Data("untrusted model output".utf8)
    let reply = try MacOSMetalInferenceReply.succeeded(output)
    #expect(reply.resultSHA256 == metalSHA256(output))
    #expect(throws: MacOSMetalInferenceFailure.resultLimitExceeded) {
        _ = try MacOSMetalInferenceReply.succeeded(
            Data(count: agentMageMacOSMetalResultMaximumBytes + 1)
        )
    }
}

@Test("Metal inference failure taxonomy is closed and content free")
func metalInferenceFailureTaxonomyIsClosed() {
    #expect(MacOSMetalInferenceFailure.allCases.count == 30)
    #expect(
        MacOSMetalInferenceFailure.allCases.allSatisfy {
            $0.rawValue.hasPrefix("macos.metal-inference.")
                && !$0.rawValue.contains("/")
                && !$0.rawValue.contains(" ")
        }
    )
}
