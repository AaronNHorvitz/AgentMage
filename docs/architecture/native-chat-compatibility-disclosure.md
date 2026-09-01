# Native Chat Compatibility and Disclosure

## Supported stable surface

Story 23.8 provides two compatibility adapters through stable VS Code 1.125 APIs:

- the registered `@agentmage` Chat Participant accepts the current prompt, command, public request
  references, progress sink, selected model, and cancellation token; and
- the registered `agentmage` Language Model Chat Provider exposes only exact currently selectable
  local profiles and accepts bounded user/assistant text history.

Neither adapter is the canonical AgentMage interface. Verified Chat owns persistent lifecycle,
complete artifact selection, the full inspector, exact approvals, and reconstructable runtime
state. Native adapters cannot claim those guarantees and always name
`agentmage.openVerifiedChat` as the transition when a required semantic is unavailable.

## Participant reference closure

Every public request reference receives an ordered visible state and one content-free canonical
record. Text, URI, Location, virtual, remote, stale, changed, empty, oversized, cancelled, failed,
and unknown values are counted. Resolvable bytes use stat-read-stat revision checks and the same
authenticated chunked Engineering artifact RPC as Verified Chat.

Because the stable Chat API does not identify optional versus required references, AgentMage treats
every supplied reference as required. Any `unsupported`, `stale`, `omitted`, `failed`,
`cancelled`, or otherwise non-included result stops before the model turn. The participant never
runs on a silently weakened subset. Its terminal result names the source-manifest digest and byte
accounting, followed by the persistent-lifecycle and inspector limitation.

## Provider normalization

The provider advertises text-only native capabilities even when the selected AgentMage runtime can
internally propose tools or consume other modalities. This distinction prevents VS Code callers
from supplying external tools or images that the adapter cannot preserve. The exact internal model
profile still reaches the shared runtime, where model tool requests remain untrusted proposals
subject to the normal policy, grant, approval, dispatcher, receipt, and verifier boundaries.

For a supported native request, the adapter preserves every message, role, optional bounded name,
text part, and part boundary in one closed versioned JSON prompt projection. The last message must
be a user message. It refuses empty, oversized, control-containing, unknown-role, unknown-part,
data, image, external tool-call, external tool-result, caller-tool, non-default tool-mode, and
model-option inputs before model execution. The refusal is both visible Markdown and a structured
stable `LanguageModelDataPart` record with the Verified Chat transition command.

After exact model revalidation, the stream begins with visible and structured requested-route
disclosure: profile, manifest, runtime adapter, runtime digest, strict-local endpoint class,
request digest, no permitted fallback, and all limitations. `route_state` remains
`requested_profile`; it does not claim the host actually launched until subsequent verified runtime
events say so. Existing runtime progress and result parts follow in order.
Cancellation uses the existing runtime token and durable cancellation request. Unexpected adapter
errors become a closed no-fallback terminal message rather than raw exception detail.

The final structured usage disclosure reports exact message, part, input-byte, output-part, and
output-byte counts. The current shared runtime does not return exact tokenizer usage to this
adapter, so input and output token counts remain explicitly `null` with
`vscode.provider.exact-token-usage-unavailable`. `provideTokenCount` returns the conservative UTF-8
byte count rather than an underestimated synthetic tokenizer result. No exact-token claim is made.

## Versioned capability matrix

`architecture/vscode-api-surfaces.json` schema 2 is the authoritative production matrix. It pins
the minimum VS Code/API engine, stable roles and parts, explicit unsupported families, tool and
model-option behavior, route and usage disclosures, conservative token-count behavior, canonical
transition command, and the explicit non-Agent-Host claim. Preview, proposed, and private channels
remain outside the supported production path and require separate experiment controls.

The extension uses no proposed or private API. Removing either experimental channel leaves the
stable participant and provider behavior unchanged. A newly introduced VS Code part is `unknown`
until this matrix, projector, fixtures, and runtime path explicitly admit it.

## Evidence and limitations

The local Story 23.8 campaign covers complete participant accounting, unresolved-reference
blocking, history and part preservation, tool/options/role/size refusal, structured route and usage
disclosure, ordered runtime streaming, cancellation, stable API registration, matrix mutation, and
source/API lint. Retained evidence is under
`artifacts/sprints/sprint-23/story-23.8/`.

Installed VSIX interaction across the declared VS Code version range, remote placement, physical
platform parity, an independently qualified production model, and independent accessibility and
security review remain external evidence. Windows is reserved for the final same-candidate local
campaign. macOS and GitHub-hosted validation remain deferred by the current run instructions.
