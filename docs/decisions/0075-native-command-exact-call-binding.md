# Decision 0075: Native Command Exact-Call Binding

| Field | Value |
|---|---|
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-23 |
| Authority | Decision 0054 and the owner's real-model integration assignment |
| Scope | Registered native command adaptation in the existing Linux effect boundary |
| Preserves | Exact approvals, operation plans, grants, confinement, output binding and verifier ownership |

## Evidence

Muse campaign7 new-file1 at `ef4a05d8` proposed valid native arguments to
`agentmage.command.run-template`, with argument SHA-256
`87565c772c7ae8b4c39bfe07ac130422afadac4bb90b44e3ceaccc73a0aad922`.
After approval the host returned `Dependency(Uncertain)`. This was not a decoder,
generation-budget or model-capability failure. Both complete responses, exact
prompts and the failed collector report remain in the private campaign logs.

The codec serializes JSON object keys in sorted order, beginning with
`command_attempt_id`. Command preparation parses the closed request and serializes
the internal Rust struct beginning with `schema_version`. The Direct command
driver correctly rejects the different bytes, but the host had incorrectly
selected that driver for a native adapted request. A host regression using the
same key-order difference reproduced `Uncertain` before the correction.

## Decision

Use the existing `RegisteredCommandWrapperBinding`, already used for targeted
validation, for the native registered-command adapter. Bind the exact original
tool/version/argument digest and the prepared operation-plan digest rendered in
the approval. The plan includes the complete original call, profile, operation,
target and expected state change; the frozen profile resolves the exact registered
command. The kernel still checks consumed authority, the held root, one exact
side effect and its matching plan before the executor can launch.

Do not compare only semantic JSON equality, rewrite approved bytes, relax Direct
driver equality, add a second runner or promote generic command output to
validation evidence. Command receipt and full captured-output hashes remain
mandatory. Targeted validation remains the only relevant validation owner.
No model, context, generation, resource or correction budget changes.

## Verification

Regression coverage includes compact sorted and pretty JSON approved in that
exact form, and refusal when equivalent JSON is reordered after approval, with
zero executor launches. The existing wrong-operation-plan kernel regression
must continue to refuse. An explicitly scripted `native-command-failure` scenario
submits the exact retained Muse argument bytes. The generic nonzero command must
terminate with `runtime.tool.failed`, a receipt and verified full failure output,
without validation evidence or a subsequent write. Its assessor binds the failure
output to the command turn and CLI-verified full artifact. Separate targeted
validation cases prove the ordinary failed-test/inspection/repair/pass/diff chain.

The first added matrix case incorrectly expected a generic failed command to
continue into repair. Matrix9 correctly terminated it; its failed assessment is
retained. Inspection of `finish_tool_non_success` and `validation_outcome` confirms
the existing owner distinction. Correct only the diagnostic expectation, not
generic failure semantics, targeted-validation ownership or model pass criteria.

Run the connected CLI/host regression and the separate real-model campaigns;
scripted success does not qualify either model. All admission, independent-review,
daily-use, platform and release gates remain open.
