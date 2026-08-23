# AgentMage Model Provenance and Admission Policy

| Field | Value |
|---|---|
| Status | Active product policy; zero enabled models; candidate evaluation planned under Decision 0027 |
| Effective date | 2026-08-12 |
| Product authority | `PRD.md` |
| Security authority | `SECURITY-REVIEW.md` |
| Model-management architecture | `TRUSTED-OPERATIONS.md` |
| Model-route implementation | `MODEL-GATEWAY.md` and Decision 0045 |
| Applies to | Models, family codecs, adapters, tokenizers, templates, conversions, quantizations, decoding profiles, context profiles, and derived artifacts |

## 1. Purpose

Gateway codec availability and endpoint protocol compatibility are not admission. Each selectable
route must bind a model admitted under this policy to an independently qualified runtime, codec,
endpoint, operator, disclosure policy, and route record.

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
| Runtime and codec | Family codec, tokenizer/template protocol, adapter identity, native build or immutable Open Container Initiative digest, backend, context and decoding profiles, modality, platform, hardware/driver envelope, configuration, and isolation boundary |
| Resources | Memory, disk, acceleration, context, concurrency, and cancellation limits measured on each reference platform |
| Quality | Pinned role-specific task corpus, quality profile, diagnostic-repeatability profile, repeated trials, tool-call validity, grounding, uncertainty, citation, false-completion, latency, resource, and intervention results |
| Security | Prompt-injection, malformed output, resource exhaustion, egress, file/tool/credential authority, and local endpoint exposure results |
| Decision | `PASS`, `BLOCKED`, or `REJECTED`; reviewer, date, limitations, evidence index, expiry or re-review trigger, and fallback status |

Mutable tags such as `latest` or `e4b` may be shown to users but never serve as release identity. Docker Model Runner artifacts are pinned by immutable OCI digest; native artifacts are pinned by GGUF and supporting-file hashes.

## 6. Runtime Parity

AgentMage implements one `LocalModelRuntime` contract with separately attributable adapters:

- Native `llama.cpp` is the Fedora, Ubuntu, and Windows 11 x64 first-GA security reference.
- Native `llama.cpp` with Metal remains the Apple Silicon macOS post-GA security reference.
- Docker Model Runner with its `llama.cpp` backend is a supported Fedora and Ubuntu compatibility adapter when its additional admission gates pass.
- Docker Model Runner on macOS is separately gated and is not required for the reference installation.

When AgentMage makes a cross-adapter parity claim, the Docker and native adapters use the same approved model profile, tokenizer, template, context policy, decoding profile, tool schemas, cancellation contract, and evaluation corpus. Hardware and backend differences may prevent byte-identical output; behavioral, authority, evidence, and threshold parity remain mandatory. A candidate may be evaluated on one declared development adapter without creating a parity, support, or platform claim.

Docker Model Runner is treated as an unauthenticated local inference service. Loopback binding alone is not proof of exclusive access. Its adapter remains blocked from the strict-local release profile unless tests demonstrate the declared process, socket, namespace, container, and egress boundary and no undeclared client can exercise AgentMage authority through it.

## 7. Muse-First, Multi-Model Candidate Strategy

Decision 0027 makes the model layer candidate-neutral while selecting Meta Muse Glimmer as the
primary implementation and deep-evaluation candidate. Priority is not approval. No candidate can
become a product prerequisite merely because it is evaluated first, loads successfully, or has a
strong publisher-reported benchmark.

### 7.1 Meta Muse Glimmer

The first Muse candidate tuple is an exact first-party text-only artifact through a pinned native
llama.cpp build on the Fedora development workstation. It uses one inference slot, zero egress,
synthetic data, no workspace handle, no tools, no grants, no credentials, no vision projection, no
speculative draft, and an initially bounded 8k context. Separately measured 16k and 32k profiles may
follow. Vision, draft/speculative decoding, dynamic quantization, Docker Model Runner, larger
contexts, and other platforms are separate tuples that inherit no result.

Before any supported, downloadable, compatible, open-source, open-weight, or hardware-fit claim is
made, the Muse record must verify from first-party evidence the exact developer and publisher,
release and artifact revisions, model card, license and separate use terms, origin and upstream
lineage, formats, transformations, hashes, tokenizer, template, codec, runtime compatibility,
measured platform resources, coding and tool quality, security behavior, and support state.

The disposition may be `PASS`, `BLOCKED`, or `REJECTED`. A non-pass does not block another eligible
candidate or first GA. Mutable names, community mirrors, secondary reports, or descriptive use of
the term open source cannot substitute for first-party and artifact evidence.

### 7.2 Comprehensive First-Party Gemma Inventory

At each declared evaluation freeze, AgentMage inventories every eligible official first-party Gemma
model discoverable from the pinned Google catalog evidence. Each exact profile records its role,
publisher-controlled source, revision, license and use terms, lineage, artifact, tokenizer,
template, transformation, runtime, modality, context, hardware preflight, applicable test suites,
and result.

Evaluation is role-specific:

- general, instruction, reasoning, coding, and multimodal generative profiles receive applicable
  repository, planning, coding, tool, evidence, context, security, and resource tests;
- function-specialized profiles receive tool-selection and structured-proposal tests;
- safety profiles receive advisory-classification tests and can only deny, narrow, redact, isolate,
  or escalate;
- embedding profiles receive retrieval, source-provenance, contamination, invalidation, and
  resource tests; and
- specialist, research, interpretability, translation, medical, current, and legacy profiles
  receive applicable role tests and cannot silently become the coding planner.

An exact profile that cannot run within the reference-machine envelope receives a visible
`BLOCKED-HARDWARE` evaluation result. It is not silently omitted, represented as tested, or treated
as a family-wide rejection. Community conversions, fine-tunes, adapters, merges, and mirrors do not
inherit a first-party result.

The existing Gemma 4 E4B and Gemma 4 12B Unified rejected feasibility records remain historical
evidence. Their exact text and prior requirement identities remain protected. A new revision,
artifact, transformation, runtime, codec, or profile requires new admission and cannot relabel an
old result.

### 7.3 Other Eligible Candidates

Other first-party candidates may enter the development evaluation inventory through the same
origin, jurisdiction, lineage, license, use-policy, provenance, artifact, codec, runtime, hardware,
quality, security, and evidence gates. Section 4's non-Chinese and non-Chinese-derived rule remains
in force. Runtime compatibility alone creates no approval.

User-selected arbitrary, community, or provenance-incomplete artifacts remain limited to the
post-GA Experimental Model Lab. They cannot enter the ordinary candidate store merely because the
runtime can parse them.

### 7.4 Candidate and Product State

The development inventory and signed product catalog use exact lifecycle states:

- `candidate`: attributable profile awaiting evidence;
- `evaluating`: exact profile allowed only in its declared isolated evidence run;
- `approved`: exact profile allowed only for the proved capabilities and platforms;
- `degraded`: explicitly supported limited profile;
- `quarantined`: unusable pending investigation or re-review;
- `rejected`: exact profile failed a non-waivable gate; and
- `retired`: no new use after support or policy withdrawal.

`BLOCKED-HARDWARE` is an evaluation result attached to an exact profile/platform/hardware tuple, not
an activation state. Only `approved` and explicitly supported `degraded` profiles can enter ordinary
operation. The current state remains zero enabled models.

### 7.5 Quality and Diagnostic Repeatability

Every serious generative candidate uses separately identified profiles:

1. A first-party-recommended quality profile measures intended capability and may use stochastic
   generation.
2. A diagnostic-repeatability profile pins the artifact, tokenizer, template, codec, runtime,
   sampler order, top-k one, seed, reasoning controls, context, slot count, speculative state,
   platform, hardware, driver, prompt, tools, and limits.

The profiles use separately reported results. Temperature zero, top-k one, a fixed seed, greedy
sampling, or repeated output does not prove cross-runtime, cross-driver, cross-device, cross-release,
or universal model determinism. AgentMage distinguishes narrow token repeatability from
deterministic policy, deterministic authority and effect mediation, independently verified outcome
state, and reproducible audit evidence.

Stochastic evaluations run repeated trials and report the declared pass-at-one, pass-at-k,
pass-to-the-k, variance, confidence interval, tool-validity, false-completion, latency, memory, and
user-intervention measures. Incomparable tuples are labeled and never merged.

## 8. Re-Review Triggers

Admission expires and the profile is disabled or quarantined when any of the following changes:

- Model, tokenizer, template, family codec, encoder, draft model, adapter, conversion, quantization, runtime, decoding, context, modality, or artifact identity.
- Publisher, ownership, license, lineage, origin evidence, support status, or vulnerability disposition.
- Platform, hardware, driver, acceleration backend, context policy, reasoning control, sampler order, tool grammar, sandbox, socket, or network behavior.
- Quality corpus, acceptance threshold, threat model, or supported AgentMage capability.
- A vulnerability, compromise, revocation, or unexplained reproducibility difference affects the admitted profile.

No cached approval, user preference, compatibility alias, or successful prior run overrides a re-review trigger.

## 9. Approved Catalog and Guided Installation

The signed model catalog uses exact `candidate`, `evaluating`, `approved`, `degraded`, `quarantined`,
`rejected`, and `retired` states. Only `approved` profiles and explicitly supported `degraded`
profiles can be activated for ordinary AgentMage use. Every catalog transition records its source
evidence, decision, reviewer, date, limitations, expiry, and triggering release identity.

A user may ask Chat to list compatible approved profiles and to download, import, verify, activate,
compare, roll back, remove, or clean up one. Before acquisition, the deterministic model manager
shows the exact artifact, tokenizer, template, codec, runtime, context, decoding, modality, platform,
and hardware identities plus publisher, license, source, size, disk and memory requirements,
measured hardware fit, expected network use, destination, verification sequence, limitations, and
rollback. The user confirms that exact plan.

The separate installer/importer performs bounded acquisition or import into quarantine, resume,
hash and signature verification, malware and format scanning, admission self-tests, atomic
activation, cancellation, crash recovery, rollback, and cleanup. The model runtime, selected model,
model output, prompt, webpage, repository, or provider response cannot approve, download, replace,
activate, or fall back to a model.

## 10. Experimental Model Lab

The post-GA Experimental Model Lab may inspect user-selected unapproved artifacts only inside the
boundary in `TRUSTED-OPERATIONS.md`. It retains Section 4's origin and lineage rule. An imported
artifact records its source, observed license, provenance gaps, hashes, quarantine state, format,
resource preflight, and explicit warnings.

An experimental model has no network, credential, command, connector, messaging, finance, delivery,
cloud, backup, operational-memory, approved-model-store, or canonical-workspace-write authority. It
uses synthetic evaluation data and disposable bounded scratch space. Experimental quality or
security results do not create approval. Promotion requires a new complete Section 5 admission
record, independent review, and an approved catalog transition.

## 11. Whole-Codebase Audit Model Use

[`CODEBASE-AUDIT.md`](./CODEBASE-AUDIT.md) governs model use during a comprehensive repository
audit. A large context window, retrieval score, or model-generated summary is never evidence that a
model understands an entire repository. The deterministic census, structural index, source hashes,
evidence ledger, reconciliation state, and coverage records remain authoritative.

Every model profile promoted for audit work must pass an audit-specific corpus covering bounded
source packets, exact source grounding, structural-fact adherence, uncertainty, contradictory
evidence, malformed or adversarial repository content, context pressure, cancellation, resume, and
stable structured output. The admission record stores the measured packet, context, latency,
memory, and quality limits. A profile is not approved merely because it accepts a large prompt.

Every semantic evidence card records the exact model, artifact, tokenizer, template, codec,
runtime, adapter, context, decoding, modality, platform, hardware/driver, packet, proposal, and
policy identities that produced it. Changing any of those identities invalidates affected semantic
cards and their dependent findings, reconciliation records, and reports. It does not invalidate an
unchanged deterministic structural index unless that index's own parser or source identity changed.

An audit model receives only the bounded, secret-safe packet selected by the audit coordinator. It
never receives a repository handle, canonical workspace write grant, credential, network authority,
hosted-service authority, command authority, coverage authority, approval authority, or checkpoint
decryption key. Model output remains untrusted data until schema validation, source resolution, and
cross-module reconciliation succeed.

## 12. Local and Remote Gateway Admission

Decision 0044 extends admission from a local model/runtime tuple to distinct model, runtime,
protocol-codec, endpoint, operator, route, and policy records. It does not change Section 4. The
instruction's broad reference to future model families conflicts with the current non-Chinese and
non-Chinese-derived rule; the stricter existing rule remains authoritative unless a separately
approved owner decision changes it.

A local or remote endpoint can enter evaluation only after the model itself is eligible under this
policy. Endpoint compatibility, private deployment, self-hosting, open weights, a familiar API, or
provider marketing does not repair a license, origin, lineage, artifact, runtime, quality, or
security gap.

Each endpoint profile additionally records:

- Endpoint operator, deployment owner, immutable or otherwise exact deployment identity, profile
  class, protocol and version, runtime and codec;
- Endpoint reference separately from any credential, exact TLS and mutually authenticated TLS
  policy, host and address policy, redirect and proxy policy, DNS and SSRF controls;
- Region, residency, retention, logging, training-use, incident, emergency-disable, quota, cost,
  concurrency, context, streaming, cancellation, error, and health behavior;
- Credential reference type and exact worker allowed to resolve it, never the credential value;
- Endpoint conformance, route conformance, privacy, latency, throughput, resource, failure,
  cancellation, version-skew, removal, and strict-local restoration evidence.

Route admission binds one exact model profile, endpoint profile, runtime adapter, codec, role,
disclosure policy, resource policy, and fallback policy. Technical compatibility is not approval.
Model qualification, endpoint qualification, and route qualification remain separate. Changing any
material identity invalidates every safely identified dependent route; uncertain reach requires a
broader requalification.

No endpoint or route is enabled by documentation or schema validation. `strict_local` remains a
complete target profile. `local_network_private`, `remote_private`, and `remote_managed` are
optional profiles that require their own evidence. Fallback remains disabled by default; no route
may silently move from local to remote or between remote operators.
