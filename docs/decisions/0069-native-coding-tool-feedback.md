# Decision 0069: Native Coding Tool Feedback

| Field | Value |
|---|---|
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-23 |
| Authority | Decision 0054 and the owner's real-model integration assignment |
| Scope | Exact native call/result context and safe continuation for the coding harness |
| Preserves | Existing coordinator, canonical journal/artifacts, schema checks, grants, confinement, verifier and resource limits |

## Evidence

Muse native-2 completed a genuine failing validation, then echoed a repository
context item in reasoning without an actionable proposal. GPT native-5 completed
a hash operation, then selected an unsupported Python text fallback. Both failed
attempts remain retained. These observations do not establish model incapability.

The context interface supplied results without their original calls. Consequently
Muse used an unnamed tool role, and Harmony used the invented
`functions.agentmage_native` name without a preceding assistant call. Neither is
the exact feedback format of the pinned upstream template. The syntax-language
restriction was also absent from the native patch tool's own description.

## Decision

Retain completed calls alongside results in the existing coordinator and its
hash-bound continuation artifact. Validate count, ordered call/result identities,
correlation, argument bytes and repeated-call semantic digests. Resume preserves
the same observations; it does not dispatch historical calls. Old continuation
payloads lacking the required binding fail closed rather than inventing history.

The coding context supplies a verified call/result pair as one source-selection
unit. Selected pairs render in execution order after supporting sources. Native
codecs render the actual frozen tool recipient, assistant call and tool result
using their family-specific syntax. They reject unknown or mismatched identities;
tool output cannot choose the recipient. No separate store or model loop is added.

Expose the unchanged syntax-language restrictions directly in the patch tool's
description, including the rejection of Python text fallback. Preserve the actual
GPT proposal as a negative regression. No argument repair, output-budget change,
model substitution or completion relaxation is authorized by this decision.
