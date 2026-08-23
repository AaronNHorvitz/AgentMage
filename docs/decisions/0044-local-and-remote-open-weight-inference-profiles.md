# Decision 0044: Local and Remote Open-Weight Inference Profiles

| Field | Value |
|---|---|
| Status | Accepted additive architecture and planning refinement |
| Date | 2026-08-22 |
| Scope | Candidate-neutral model gateway, local and remote endpoint profiles, qualification, routing, disclosure, fallback, streaming, cancellation, health, usage, and cost controls |
| Adds | Normative Model Gateway specification, explicit route taxonomy, endpoint and route qualification, remote-inference threat controls, stable requirements and tests, Foundational Runtime Epic F4, schemas, and dependency-ordered work in existing sprints |
| Preserves | Strict-local target completeness; zero enabled models; rejected historical Gemma evidence; Muse-first evaluation priority; license and provenance gates; no-silent-fallback policy; platform evidence separation; and deterministic tool and completion authority |
| Does not authorize | Any model or endpoint activation, artifact download, private endpoint publication, credential storage in configuration, cloud support claims, automatic local-to-remote fallback, model-owned routing, tool execution, or completion |

## Context

AgentMage's candidate-neutral model contract and strict local policy separate
model output from authority, but the target product needs to use qualified
models across a local process, a private local-network server, a private remote
deployment, or an explicitly managed service. Provider-compatible wire formats
do not prove semantic parity, privacy, reliability, latency, license approval,
or suitability for an engineering role.

Model identity, endpoint identity, runtime identity, protocol codec, route, and
policy therefore require separate records and independent qualification.

## Decision

1. `MODEL-GATEWAY.md` is the normative authority for model and endpoint
   mediation. Engineering workflows depend only on an internal canonical
   request, event, usage, error, cancellation, and route-receipt contract.
2. Four profile classes are defined: `strict_local`,
   `local_network_private`, `remote_private`, and `remote_managed`. Strict local
   remains a complete target profile and never requires a cloud account.
3. Model profile, endpoint profile, runtime adapter, protocol codec, and route
   profile are distinct. Model publisher and endpoint operator are distinct
   identities. Open weights do not imply an open-source license.
4. Local runtime adapters may include qualified llama.cpp, Ollama, LM Studio,
   vLLM, SGLang, TGI, Ray Serve, KServe, or future implementations. Naming an
   adapter indicates planned compatibility evaluation, not support.
5. Remote routes are opt-in and policy-bound. They require explicit endpoint
   identity, TLS policy, host and address controls, credential reference,
   disclosure classification, region, retention, logging, training-use,
   quota, cost, timeout, health, and emergency-disable records.
6. A remote-inference worker is the only process allowed to resolve an endpoint
   credential. Credentials are never sent to models, model-visible context,
   logs, traces, receipts, or committed files.
7. Routing is deterministic over qualified capabilities and current policy.
   The selected route and reason are visible and receipted. Endpoint health can
   make a route unavailable but cannot widen authority or select a fallback.
8. Fallback is disabled by default. Any fallback requires an explicit ordered
   policy, equivalent data authorization for the destination, current
   qualification, a new route decision, and user-visible disclosure. There is
   no silent local-to-remote or remote-to-different-remote transition.
9. Qualification is three-dimensional: model qualification, endpoint
   qualification, and route qualification. Changes to artifact, tokenizer,
   template, codec, quantization, context, decoding, runtime, endpoint,
   transport, or policy invalidate the affected evidence.
10. Gateway success means a valid model proposal or advisory result, never tool
    execution or task completion. The Rust runtime remains the only authority
    for tools, verification, and terminal workflow state.

## Consequences

- Local and remote open-weight inference can share one workflow contract without
  pretending that compatible HTTP shapes are identical implementations.
- Remote inference can improve capacity but is never assumed to improve
  latency; first-token, throughput, queue, transfer, cancellation, and total
  workflow time must be measured.
- Private and managed endpoints remain optional capability profiles with
  separate privacy and support evidence.
- No model, endpoint, route, or fallback becomes enabled through this decision.
- Historical Gemma records remain immutable evidence; Muse remains the first
  named evaluation focus under Decision 0027, not an enabled model.

## Approval Record

On 2026-08-22, the user explicitly instructed AgentMage to plan and reconcile
local and remote open-weight model routing as part of the complete Engineering
Runtime assignment while preserving stricter security and provenance rules.
