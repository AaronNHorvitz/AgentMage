# Decision 0163: Explicit native endpoint read development launch

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-08 |
| Authority | Local agent mission; T-303/T-306; Decision 0162 |
| Scope | Disposable read-only configured-endpoint service activation |

Connect trusted endpoint preparation and its concrete file-observation verifier
to the existing authenticated HTTP owner through one development launch. Reuse
the same coordinator, context, credential-owning transport, native file/grant
boundary, durable kernel authority, worker manifest and sandbox. There is no
second loop, executor, permission system or completion parser.

The executable requires an explicit startup approval flag and a private launch
document. This consents to reading that configuration and its optional private
endpoint-key file, selecting the marked disposable workspace, and creating
private development state. It does not approve a model disclosure or a file
read. Every advance still requires separate disclosure consent and every file
effect still asks for its exact native grant. Observation delegation stays off.

The startup document contains a sealed original endpoint request, its original
instruction ledger and exact file-observation checks already selected by the
trusted preparer. It is never an HTTP/model tool argument. No inline key or
source excerpts are accepted. Fresh service-issued run/session identities and
derived task/packet identities replace the template identities before original
startup narrowing and policy binding. All other request content is preserved;
original seals are checked before identity binding. No resume is implied.

Use the existing disposable activation marker and Linux development adapter.
By default, the worker and fixed system executables/libraries must pass the
original root-owned manifest admission. Unavailable native resources refuse before
opening endpoint credentials or creating authority state. This launch does not
relax worker ownership, impersonate a qualified native model profile, or claim
production/package activation. An installed user-owned worker is insufficient
under this default; the explicit development selection below changes only
worker sourcing.

The concrete verifier certifies only the declared complete UTF-8 observations,
not summary meaning or repository correctness. Native success is unobserved in
the builder lane. Edit/test/retry, durable coordinator recovery, approved Git,
parent measurements and T-303/T-306 closure remain open. No row is added or
closed; independent core review is required.

## Explicit development worker selection, 2026-11-08

The private launch document may explicitly select `sealed_sibling_development`
for `worker_trust`. Omission keeps the original `root_owned` selection. Reuse
the existing development worker snapshot mechanism with a closed launcher
mapping: only the running canonical local API executable's exact sibling
`agentmage-read-only-worker` is selectable. Retain original owner/mode/ancestry,
bounded-copy, drift and immutable descriptor checks. The existing host launcher
continues to select its own sibling through the same mechanism.

Neither selection falls back to the other. Fixed system executables and runtime
libraries still require original root-owned admission; runner, supervision,
offline confinement, credentials/state ordering and exact grants are unchanged.
This removes a worker-selection prerequisite only. Positive native execution
and model measurements remain unobserved in this lane; parent rows remain open.
