# AgentMage Model Provenance and Admission Policy

| Field | Value |
|---|---|
| Status | Active product policy; initial candidate evaluation is in progress |
| Effective date | 2026-08-10 |
| Product authority | `PRD.md` |
| Security authority | `SECURITY-REVIEW.md` |
| Applies to | Models, adapters, tokenizers, templates, conversions, quantizations, and derived artifacts |

## 1. Purpose

This policy defines the evidence required before AgentMage can enable a local model profile. It makes the project's model-origin requirement testable, rejects unknown or silently changed lineage, and binds every approved profile to exact artifacts and measured behavior.

This policy does not approve a model merely because it is local, open-weight, popular, or published by a familiar organization. Admission requires a complete disposition record and all release-specific quality, security, resource, platform, and license gates.

## 2. Authority and Change Control

The PRD controls product scope. `Agent-Scaffolding-Inventory.md` controls stable requirements and acceptance tests. `SECURITY-REVIEW.md` controls product-security requirements and reviewer protocols. This policy supplies the model-admission procedure under those documents and cannot weaken them.

Every admission decision records the policy version, model and runtime identities, evidence hashes, reviewer, date, result, limitations, and superseding decision when applicable. Unknown, contradictory, stale, unavailable, or unverifiable evidence produces `BLOCKED`, never conditional approval. A verified prohibited identity, integrity violation, or mandatory threshold failure produces `REJECTED`. Only complete conforming evidence can produce `PASS`.

## 3. Covered Supply Chain

One model profile includes all of the following:

- Base and instruction-tuned weights.
- Fine-tunes, adapters, merges, distillations, and synthetic-training lineage where disclosed or used.
- Tokenizer, vocabulary, chat template, system-role behavior, and tool-call grammar.
- Conversion and quantization inputs, tools, commands, settings, and outputs.
- Native and managed inference runtimes, including exact build or image identities.
- Model manifests, licenses, model cards, safety documentation, and artifact catalogs.
- Optional embedding models, rerankers, draft models, and multimodal encoders.

A conversion, repackaging, or quantization does not create new provenance. The resulting artifact inherits the source model's origin and adds its own conversion and toolchain provenance.

## 4. Origin and Lineage Rule

AgentMage does not admit a Chinese model or a model derived from a Chinese model. For this policy, a model is excluded when any of these conditions is true:

- Its model developer, weight publisher, or controlling supplier is based in or controlled from the People's Republic of China.
- Its weights are fine-tuned, merged, adapted, distilled, transferred, or otherwise derived from an excluded model.
- Its training or tuning process is known to depend materially on synthetic outputs from an excluded model.
- Its base-weight, fine-tune, adapter, or distillation lineage is undisclosed, contradictory, or cannot be verified to the degree required by the release profile.
- A silent artifact, tokenizer, template, conversion, quantization, or runtime substitution prevents the approved lineage from being reproduced.

Shared public training data, a compatible file format, or an independently implemented tokenizer algorithm does not by itself prove model derivation. Those dependencies still require provenance, integrity, license, and security review. When the available evidence cannot distinguish compatibility from derivation, admission remains blocked pending a documented decision.

## 5. Required Admission Record

Every candidate receives one signed or hash-bound record containing:

| Category | Required evidence |
|---|---|
| Identity | Canonical model name, variant, revision, publisher, developer, release date, and source locations |
| Ownership and origin | Developer and publisher control, applicable jurisdictions, upstream base models, fine-tunes, adapters, merges, distillations, and known synthetic-training lineage |
| License | Exact license text and version, redistribution obligations, notices, use restrictions, and recorded disposition |
| Artifacts | Original and packaged hashes, sizes, formats, tokenizer, templates, encoders, and auxiliary files |
| Transformation | Conversion and quantization tools, versions, source hashes, commands, settings, environment identity, and output hashes |
| Runtime | Adapter identity, native build or immutable Open Container Initiative digest, backend, configuration, platform, and isolation boundary |
| Resources | Memory, disk, acceleration, context, concurrency, and cancellation limits measured on each reference platform |
| Quality | Pinned task corpus, decoding profile, tool-call validity, grounding, uncertainty, citation, and reproducibility results |
| Security | Prompt-injection, malformed output, resource exhaustion, egress, file/tool/credential authority, and local endpoint exposure results |
| Decision | `PASS`, `BLOCKED`, or `REJECTED`; reviewer, date, limitations, evidence index, expiry or re-review trigger, and fallback status |

Mutable tags such as `latest` or `e4b` may be shown to users but never serve as release identity. Docker Model Runner artifacts are pinned by immutable OCI digest; native artifacts are pinned by GGUF and supporting-file hashes.

## 6. Runtime Parity

AgentMage implements one `LocalModelRuntime` contract with separately attributable adapters:

- Native `llama.cpp` with Metal is the macOS security reference.
- Native `llama.cpp` is the Fedora and Ubuntu security reference.
- Docker Model Runner with its `llama.cpp` backend is a supported Fedora and Ubuntu compatibility adapter when its additional admission gates pass.
- Docker Model Runner on macOS is separately gated and is not required for the reference installation.

The Docker and native adapters use the same approved model profile, tokenizer, template, context policy, decoding profile, tool schemas, cancellation contract, and evaluation corpus. Hardware and backend differences may prevent byte-identical output; behavioral, authority, evidence, and threshold parity remain mandatory.

Docker Model Runner is treated as an unauthenticated local inference service. Loopback binding alone is not proof of exclusive access. Its adapter remains blocked from the strict-local release profile unless tests demonstrate the declared process, socket, namespace, container, and egress boundary and no undeclared client can exercise AgentMage authority through it.

## 7. Initial and Fallback Profiles

### Gemma 4 E4B

Gemma 4 E4B is the initial candidate, not a pre-approved dependency. Its first admission record must verify the exact first-party artifact, Apache-2.0 license disposition, upstream and packaged hashes, tokenizer and template, official or reproducible GGUF, runtime compatibility, hardware fit, tool calling, evidence behavior, and every quantitative v0.1 model threshold.

The Docker compatibility path uses `ai/gemma4:e4b` only after resolving that mutable name to an approved immutable OCI digest. The native path uses an approved GGUF and supporting artifacts whose hashes resolve to the same admitted profile.

### Gemma 4 12B Unified

Gemma 4 12B Unified is the named fallback candidate if E4B fails a required quality or tool-calling threshold. It remains disabled unless a decision record promotes it after the complete, independent admission process. AgentMage never switches to it automatically, and a fallback decision cannot waive resource, license, origin, platform, security, or offline requirements.

Gemma 4 26B A4B and other later candidates remain disabled until separately promoted and admitted.

## 8. Re-Review Triggers

Admission expires and the profile is disabled or quarantined when any of the following changes:

- Model, tokenizer, template, encoder, adapter, conversion, quantization, runtime, or artifact identity.
- Publisher, ownership, license, lineage, origin evidence, support status, or vulnerability disposition.
- Platform, acceleration backend, context policy, tool grammar, sandbox, socket, or network behavior.
- Quality corpus, acceptance threshold, threat model, or supported AgentMage capability.
- A vulnerability, compromise, revocation, or unexplained reproducibility difference affects the admitted profile.

No cached approval, user preference, compatibility alias, or successful prior run overrides a re-review trigger.
