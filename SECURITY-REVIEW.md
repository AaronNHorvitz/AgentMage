# AgentMage Security Review and Verification Guide

| Field | Value |
|---|---|
| Document status | Public product-security planning baseline; no external certification claim |
| Source review date | 2026-08-12 |
| Product ownership | Independently developed by Aaron N. Horvitz on personal time and personally controlled equipment |
| Intended product boundary | Local, single-user desktop software |
| First-GA validation targets | Fedora, Ubuntu, and Windows 11 x64 with native `llama.cpp` |
| Compatibility runtime | Docker Model Runner is a separately gated Linux adapter |
| Retained post-GA platform | Apple Silicon macOS on a MacBook Pro M5 |
| First supported release scope | Local-first development, delivery, productivity, research, continuity, finance, trusted-operations, and whole-codebase audit assistant in native Visual Studio Code Chat |

## 1. Purpose

This document is the public, reviewer-facing product-security baseline for AgentMage. It converts the product threat model and widely used security-engineering practices into requirements, implementation recommendations, reproducible tests, and evidence artifacts.

AgentMage is an independent, privately developed product created on personal time, on personally controlled hardware, with independently obtained tools and services. It is not sponsored, commissioned, or developed on behalf of an employer. The intended distribution is public. Evaluation or installation on a managed device is a separate decision by that device's owner or operator and does not change project ownership.

The security baseline is deliberately customer-neutral. An individual, business, school, nonprofit, or other organization can evaluate the same product evidence and apply its own internal policies without changing AgentMage's architecture or reconstructing the security case from source code.

This document does not:

- Claim certification against a standard that has not been independently assessed.
- Replace a customer's security, privacy, legal, records, procurement, accessibility, or AI-risk review.
- Approve sensitive, regulated, confidential, personal, financial, credential, or customer-controlled data for a use profile that has not been tested for it.
- Treat local execution as sufficient proof of security.
- Transfer project ownership, sponsorship, or authorship through evaluation, testing, installation, or use.

The device owner or deploying organization retains authority over installation, allowed data, endpoint policy, and use in its own environment.

This guide is interpreted with [SECURITY.md](SECURITY.md), [MODEL-PROVENANCE-POLICY.md](MODEL-PROVENANCE-POLICY.md), [RUNTIME-BOUNDARIES.md](RUNTIME-BOUNDARIES.md), [DELIVERY-SYSTEM.md](DELIVERY-SYSTEM.md), [PRODUCTIVITY-SYSTEM.md](PRODUCTIVITY-SYSTEM.md), [TRUSTED-OPERATIONS.md](TRUSTED-OPERATIONS.md), [CODEBASE-AUDIT.md](CODEBASE-AUDIT.md), and [WINDOWS-BOUNDARIES.md](WINDOWS-BOUNDARIES.md). Those documents define public vulnerability handling, model admission, shared runtime, delivery, productivity, finance, cloud-observer, command, research, credential, continuity, model-management, whole-codebase audit, and Windows boundaries; a release cannot substitute looser behavior for any of them.

## 2. Recommended Review Position

AgentMage should be presented as independently developed, locally installed desktop software with a bounded Visual Studio Code integration, not as a cloud service or an extension of any employer's systems or intellectual property.

Recommended first-GA product-validation boundary:

- One clean Fedora or Ubuntu environment and one clean Windows 11 x64 environment.
- One standard, non-administrator user.
- One stable Visual Studio Code build.
- One allowlisted AgentMage extension and signed platform package.
- One user-selected workspace at a time, beginning with read-only mode before any write or delivery authority is enabled.
- One explicitly selected, exact admitted local profile behind the candidate-neutral runtime and closed family codec; no named model family is a prerequisite.
- Public, synthetic, or user-owned non-sensitive data only.
- No cloud inference, product telemetry, analytics, crash upload, hidden remote tool, ambient connector, autonomous external effect, or automatic model switch.
- Synthetic provider tenants, repositories, projects, pipelines, artifacts, environments, telemetry, and incidents for connected-capability conformance before user-controlled provider data.
- No sensitive or regulated data until a separately tested profile explicitly supports it.

Decision 0040 makes those first-GA environments disposable local KVM guests,
not GitHub-hosted runners. Fedora, Ubuntu, and Windows 11 evidence must bind the
exact source revision, immutable base-image identity, writable overlay,
virtualization configuration, standard-user identity class, network phase,
commands, raw results, and cleanup result. Routine product and documentation
gates run locally. GitHub-hosted execution is limited to an explicitly
dispatched, budget-confirmed Apple Silicon macOS source build and test; that
preliminary result cannot satisfy MacBook Pro M5, signing, notarization,
lifecycle, security, support, or release gates.

The engineering target is a conservative, defense-in-depth desktop security posture with least privilege, deny-by-default authority, local data minimization, reproducible builds, transparent supply-chain records, adversarial testing, and independent verification.

## 3. Reviewer Quick Path

A reviewer should be able to complete the initial assessment in this order:

- [ ] Confirm the proposed use case and allowed information types.
- [ ] Confirm who owns or manages the target device and whether installation is permitted.
- [ ] Confirm that strict-local operation has no cloud or hosted service dependency and remains complete when connected packs are removed.
- [ ] Confirm the Fedora, Ubuntu, Windows 11, Visual Studio Code, extension, model, runtime, and cryptographic module versions.
- [ ] Confirm the exact model, artifact, tokenizer, template, codec, runtime, context, decoding, platform, hardware, evaluation, lifecycle, and fallback state against the model-provenance policy.
- [ ] Review the architecture diagram, data-flow diagram, threat model, and shared-responsibility matrix.
- [ ] Validate signatures, notarization, hashes, SBOM, model manifest, and release provenance.
- [ ] Run the automated reviewer suite and retain its signed evidence bundle.
- [ ] Independently observe the offline, sandbox, path, IPC, logging, deletion, and prompt-injection tests.
- [ ] Independently observe provider identity, credential isolation, exact-preview, idempotency, uncertain-result, rollback, removal, and cross-tenant tests for every promoted adapter.
- [ ] Record privacy, retention, accessibility, supply-chain, and AI-risk decisions.
- [ ] Record open findings, owners, deadlines, and compensating controls in the remediation register, or reject the release if a release-blocking gate fails.

No reviewer should need internet access to run the product tests after the approved package and model have been imported.

## 4. Applicability Decisions

| Topic | Default product position | Reviewer decision |
|---|---|---|
| Local-only architecture | Required for the strict-local profile after model installation. | Confirm that no hosted control plane, remote inference, telemetry, analytics, cloud storage, or silent update check is present and that connected packs are removable. |
| Data sensitivity | Synthetic and non-sensitive user-owned data only by default. | Identify any additional data classes and require a separate threat model and test profile before use. |
| Privacy and retention | Data minimization, local encryption, explicit retention, export, and deletion are required. | Define any customer-specific notice, retention, backup, or deletion rules. |
| AI risk | Applicable because a local generative model is used. | Review intended use, foreseeable misuse, quality limits, human oversight, and required safeguards. |
| Software supply chain | Signed provenance, SBOM, model manifest, licenses, hashes, and vulnerability dispositions are required. | Decide whether additional attestation, source review, or penetration testing is needed. |
| Accessibility | Core workflows and generated guidance target WCAG 2.2 AA. | Review the Accessibility Conformance Report and independent test results. |
| Cryptography | Platform-backed, reviewed cryptographic providers and precise claims are required. | Confirm the provider and operating environment are acceptable for the intended data. |
| Managed-device compatibility | Optional and outside the personal development boundary. | Supply endpoint-management, monitoring, software-allowlist, and installation constraints before testing. |
| Validation execution authority | Routine checks and first-GA native acceptance run locally; source hosting is not test authority. | Reproduce Fedora, Ubuntu, and Windows 11 in separate disposable local guests and verify exact image, overlay, revision, standard-user, network-phase, result, and cleanup evidence. |
| Windows 11 x64 | Required for v1.0 GA. | Require the complete `WINDOWS-BOUNDARIES.md` package, IPC, sandbox, path, key, model, connected-worker, clean-install, accessibility, recovery, and removal evidence. |
| Apple Silicon macOS | Retained post-GA and currently `BLOCKED-MACOS`; the manual hosted source lane is preliminary only. | Require genuine MacBook Pro M5 signing, notarization, App Sandbox, XPC, Keychain, Metal, clean-install, and release evidence before claiming support. |
| Connected delivery | Required only for individually promoted provider/version/capability tuples. | Verify the support matrix, least-privilege credential, exact effect, conformance level, recovery, and removal evidence for each tuple. |

## 5. Public Product-Security Reference Baseline

The project uses public, broadly applicable security and quality references. A customer may add internal requirements without redefining the product's ownership or default architecture.

### 5.1 Secure Development and Application Security

- [OWASP Application Security Verification Standard](https://owasp.org/www-project-application-security-verification-standard/) for verifiable application-security requirements.
- [OWASP Software Assurance Maturity Model](https://owaspsamm.org/) for secure development and governance practices.
- [OWASP GenAI Security Project](https://genai.owasp.org/) for model, prompt-injection, agent, and tool-use risks.
- [Common Weakness Enumeration](https://cwe.mitre.org/) and [Common Attack Pattern Enumeration and Classification](https://capec.mitre.org/) for weakness and attack-case taxonomy.

### 5.2 Supply Chain and Release Integrity

- [Supply-chain Levels for Software Artifacts](https://slsa.dev/) for build provenance and tamper resistance.
- [OpenSSF Best Practices](https://www.bestpractices.dev/) and [OpenSSF Scorecard](https://securityscorecards.dev/) for project and dependency assurance.
- [SPDX](https://spdx.dev/) and [CycloneDX](https://cyclonedx.org/) for software, cryptographic, and model bills of materials.
- [The Update Framework](https://theupdateframework.io/) for signed update metadata and rollback resistance where updates are introduced.

### 5.3 Platforms, Operations, and Testing

- [Apple Platform Security](https://support.apple.com/guide/security/welcome/web) for signing, notarization, App Sandbox, Keychain, XPC, and security-scoped resources.
- [CIS Benchmarks](https://www.cisecurity.org/cis-benchmarks) as optional operating-system hardening and compatibility references.
- [MITRE ATT&CK](https://attack.mitre.org/) and [MITRE ATLAS](https://atlas.mitre.org/) for adversarial threat cases.
- Language and runtime security tooling, sanitizers, fuzzers, dependency auditing, and reproducible test harnesses appropriate to each shipped component.

### 5.4 Privacy, AI Risk, Accessibility, and Retention

- [ISO/IEC 27001](https://www.iso.org/standard/27001) and [ISO/IEC 27002](https://www.iso.org/standard/75652.html) for general information-security management and control themes.
- [ISO/IEC 27701](https://www.iso.org/standard/27701) for privacy-management considerations.
- [ISO/IEC 42001](https://www.iso.org/standard/42001) and [ISO/IEC 23894](https://www.iso.org/standard/77304.html) for AI management and risk considerations.
- [Web Content Accessibility Guidelines 2.2](https://www.w3.org/TR/WCAG22/) for interface and generated-document accessibility.
- Product-defined minimization, retention, export, deletion, incident, and recovery requirements verified by the `SR-*` and `RV-*` suites below.

## 6. Shared-Responsibility Model

| Responsibility | AgentMage project | Device owner or deploying organization | Shared |
|---|---:|---:|---:|
| Product threat model and secure architecture | Yes | Review | Yes |
| Data and use-case approval | No | Yes | No |
| Local risk classification and control tailoring | Evidence input | Yes | Yes |
| Fedora, Ubuntu, Windows 11, retained macOS, and Visual Studio Code deployment approval | Compatibility evidence | Yes | Yes |
| Package signing, notarization, and integrity | Yes | Verify and allowlist | Yes |
| Model and runtime provenance | Yes | Approve and inventory | Yes |
| Cryptographic provider selection | Implement and document | Approve operating environment | Yes |
| Endpoint configuration, MDM, EDR, firewall, and physical controls | Remain compatible | Yes | Yes |
| Application least privilege and sandbox | Yes | Verify | Yes |
| Enterprise identity integration | Avoid conflicting authentication | Yes | Yes |
| Product audit events | Generate and export safely | Collect and retain | Yes |
| Incident notification and remediation | Product procedure | Environment response procedure | Yes |
| Records schedules and legal holds | Support policy hooks | Decide and enforce | Yes |
| Accessibility conformance | Test and document | Independently validate | Yes |
| Deployment decision and residual-risk acceptance | No | Yes | No |

## 7. Product Security Domain Coverage

The final package maps individual `SR-*` controls, not just broad domains. This view prevents omissions during design without tying the product to one customer's compliance framework.

| Domain | Product contribution | Primary evidence |
|---|---|---|
| Authority and access | Workspace grants, one-use operation grants, least privilege, and no ambient authority | Grant tests, path tests, permission map |
| Audit and accountability | Structured local events, actor/action/resource/result identity, and clock handling | Audit schema, event tests, export sample |
| Configuration and release identity | Pinned manifests, secure defaults, fail-closed startup, and immutable release identity | Configuration schema, baseline diff, release manifest |
| Resilience and recovery | Crash-safe state, backup/restore boundaries, rollback, and cancellation | Recovery tests and procedures |
| Identity and local IPC | Operating-system session reliance and mutually authenticated local IPC | IPC identity tests and design evidence |
| Data protection | Minimal persistence, encrypted storage, export controls, retention, and sanitization | Data inventory, cryptography, and deletion tests |
| Platform confinement | Sandboxes, resource limits, secure paths, and denied ambient access | Sandbox, path, process, and resource tests |
| Supply chain | Dependency, toolchain, model, runtime, signer, and supplier provenance | SBOM, CBOM, Model BOM, licenses, and provenance |
| AI and untrusted content | Model quality limits, prompt-injection defenses, evidence states, and human authority | Evaluation corpus, attack results, and claim ledger |
| Privacy and accessibility | Data minimization, notice, purpose limits, retention, accessible interfaces, and accessible outputs | Privacy tests, WCAG results, and conformance report |
| Operations and incident response | Detection, containment, evidence preservation, update, support, and disclosure procedures | Runbooks, tabletop results, and maintenance tests |
| Independent verification | Reproducible tests, signed raw evidence, explicit failures, and release gates | Reviewer suite, evidence index, and gate decision |

## 8. Detailed Security Requirements

Each requirement must have an implementation owner, automated test where possible, human assessment procedure, evidence artifact, and release-blocking severity.

### 8.1 Governance, Scope, and Authorization

| ID | Requirement | Build integration | Reviewer test or evidence |
|---|---|---|---|
| `SR-GOV-001` | Define one bounded use case and prohibited uses. | Compile the approved profile into configuration and show the redacted doctor result inside native Visual Studio Code Chat. | Compare behavior and documentation with the approved use-case record. |
| `SR-GOV-002` | Treat v0.1 as a general-purpose local profile and prohibit data classes that have not been explicitly supported and tested. | Display and record the boundary without sending it to the model. | Attempt to enable a prohibited data profile; startup must refuse. |
| `SR-GOV-003` | Do not claim external certification, customer approval, or managed-environment approval that has not been granted. | Add claim linting to release documentation. | Scan release text and UI for unsupported certification or approval claims. |
| `SR-GOV-004` | Identify the software version, owner, maintainer, support period, and security contact. | Put immutable identifiers in the signed release manifest. | Verify displayed identifiers against the signed manifest. |
| `SR-GOV-005` | Maintain a dated risk register with owners and dispositions. | Link every release-blocking risk to a test or accepted decision. | Inspect open risks and verify no critical risk is silently closed. |
| `SR-GOV-006` | Maintain an explicit system boundary and data-flow model. | Generate diagrams and a machine-readable component/data-flow inventory from release manifests. | Trace every process, file store, socket, and data path to the package. |
| `SR-GOV-007` | Distinguish product, environment, and shared controls. | Export a machine-readable component definition and human-readable matrix. | Confirm that AgentMage does not claim endpoint or organizational controls it cannot enforce. |
| `SR-GOV-008` | Make every unsupported capability fail closed. | Centralize capability registration and startup verification. | Remove or corrupt each declared dependency and verify visible refusal. |
| `SR-GOV-009` | Record policy and standard versions used by each release. | Pin the review-baseline version in release metadata. | Compare source versions and review date with the release manifest. |
| `SR-GOV-010` | Require a new security impact review for changed authority, storage, networking, model, runtime, extension API, platform, or installer behavior. | Add release-diff classification to CI. | Feed representative changes and verify correct review escalation. |

### 8.2 Platform, Process, and Privilege Boundaries

| ID | Requirement | Build integration | Reviewer test or evidence |
|---|---|---|---|
| `SR-PLT-001` | Run as a standard user with no administrator requirement after installation. | Reject elevated execution and avoid privileged helpers, kernel extensions, and system-wide writes. | Install and run from a clean standard account; inspect process credentials. |
| `SR-PLT-002` | Sign, notarize, staple, and Hardened Runtime-enable every macOS executable and helper. | Release only from an isolated signing runner using protected credentials. | Verify Gatekeeper assessment, code signatures, designated requirements, and notarization ticket. |
| `SR-PLT-003` | Apply App Sandbox to the host and a separate sandbox to the stateless XPC tool helper. | Keep entitlements minimal and version-controlled. | Dump entitlements and run filesystem, process, device, environment, and network escape tests. |
| `SR-PLT-004` | Permit access only to one user-selected read-only workspace. | Use an app-scoped security-scoped bookmark and descriptor-relative path operations. | Test stale bookmarks, aliases, symlinks, mount changes, rename races, case folding, and Unicode normalization. |
| `SR-PLT-005` | Give the Visual Studio Code extension display and interaction authority only. | Keep file, Git, model-runtime, key, and grant access in the local host. | Instrument the extension host and prove it cannot invoke tools or read the workspace through AgentMage. |
| `SR-PLT-006` | Authenticate every local IPC peer and launch. | Validate code identity, audit token, App Group, socket mode, peer credentials, protocol version, and fresh challenge. | Attempt unsigned, wrongly signed, replayed, cross-user, wrong-App-Group, and malformed clients. |
| `SR-PLT-007` | Keep every inference process or container untrusted and authority-free. | Enforce the declared topology in `RUNTIME-BOUNDARIES.md`; give inference only model data, bounded prompt input, resource limits, and a guarded local response path. | Attempt file, environment, credential, socket, tool, grant, and peer-process access from each runtime adapter. |
| `SR-PLT-008` | Keep model acquisition separate from normal operation. | The installer has artifact acquisition authority but no workspace, session, tool, or inference authority. | Run installer and host concurrently; one must refuse. Inspect installer access and cleanup. |
| `SR-PLT-009` | Remain compatible with the selected endpoint-hardening baseline, device management, monitoring, and firewall. | Test on the exact managed-like macOS profile without weakening it. | Run the selected baseline assessment before and after installation and compare results. |
| `SR-PLT-010` | Freeze the tested platform matrix. | Bind OS build, architecture, SDK, toolchain, VS Code build, entitlements, and helper hashes to release identity. | Change each version independently and verify unsupported-state reporting. |
| `SR-PLT-011` | Do not load downloaded executable code, plugins, dynamic agents, or unsigned libraries during v0.1. | Enforce a closed signed component inventory. | Add an unmanifested library or plugin and verify load refusal. |
| `SR-PLT-012` | Keep Fedora, Ubuntu, and macOS evidence separate and distinguish native from containerized runtime evidence. | Label every result with platform, hardware, kernel/OS, package, model artifact, runtime build, adapter, and container image digest where applicable. | Verify that one platform or adapter pass cannot satisfy another platform- or adapter-specific gate. |

### 8.3 Access Control, Paths, and User Authority

| ID | Requirement | Build integration | Reviewer test or evidence |
|---|---|---|---|
| `SR-ACC-001` | Make `CapabilityGrant` the only operation authority. | Models, prompts, tools, shells, and extensions may describe but never authorize actions. | Attempt execution with every non-grant object; all attempts must fail. |
| `SR-ACC-002` | Bind grants to actor, session, task, operation, target, arguments, preimage, expiry, nonce, and preview digest. | Canonically serialize and sign or authenticate grants in the kernel. | Mutate each field independently across at least 500 cases; zero execution. |
| `SR-ACC-003` | Consume every operation grant atomically and once. | Validate immediately before execution inside one transaction. | Replay successful, failed, timed-out, and crash-interrupted grants. |
| `SR-ACC-004` | Use workspace-relative typed paths only at tool boundaries. | Reject absolute paths, traversal, alternate separators, NULs, unresolved links, and normalization ambiguity. | Run at least 500 path-escape fixtures with zero escape. |
| `SR-ACC-005` | Hold validated descriptors across check and use. | Use descriptor-relative no-follow traversal and file-identity verification. | Run rename, replacement, link-swap, mount-swap, and time-of-check/time-of-use attacks. |
| `SR-ACC-006` | Deny ambient home, credential, browser, SSH, cloud-sync, clipboard, and unrelated repository access. | Build explicit root and descriptor allowlists. | Place canary files in prohibited locations and verify zero reads and zero model exposure. |
| `SR-ACC-007` | Preserve user decision authority. | Show exact scope and stop when a request exceeds it; no automatic Codex or frontier handoff. | Run 200 transfer and authority-escalation attempts; zero external action. |
| `SR-ACC-008` | Treat repository and note content as untrusted data. | Separate instruction channels and prohibit content from changing policy or authority. | Run prompt-injection fixtures in filenames, source, comments, docs, Git metadata, and model output. |

### 8.4 Data Protection, Cryptography, and Sanitization

| ID | Requirement | Build integration | Reviewer test or evidence |
|---|---|---|---|
| `SR-DAT-001` | Inventory every collected, derived, persisted, displayed, exported, and deleted field. | Maintain a versioned data dictionary linked to code schemas. | Compare runtime storage and logs with the declared data inventory. |
| `SR-DAT-002` | Classify, minimize, detect secrets, assign retention, and select storage before persistence. | Put one mandatory pre-persistence policy gate before every durable write. | Inject labeled sensitive values into every input path and inspect all stores. |
| `SR-DAT-003` | Persist no raw prompt, full model response, attachment, environment, or tool output by default. | Store hashes, bounded excerpts, and receipt metadata only when sufficient. | Use unique canary strings and search memory dumps, storage, logs, exports, and crash artifacts. |
| `SR-DAT-004` | Fail closed when required encryption is unavailable. | Permit only explicit ephemeral operation for protected data when policy allows. | Disable the key store and database encryption; protected persistence must not occur. |
| `SR-DAT-005` | Use a reviewed, platform-appropriate cryptographic provider with explicitly selected algorithms, modes, and key boundaries. | Route all security-relevant cryptography through one pluggable provider and record its identity, version, configuration, and operating environment. | Verify provider identity, version, environment, algorithms, self-tests where available, key handling, and error behavior. |
| `SR-DAT-006` | Make only precise, evidence-backed cryptographic claims. | Generate provider and configuration language from the signed manifest. | Scan documentation and diagnostics for unsupported certification, algorithm, or operating-mode claims. |
| `SR-DAT-007` | Store keys in the platform key service and never in configuration, model context, logs, exports, backups, or repository files. | Use non-exportable or least-exportable key handles where supported. | Search all artifacts and instrument key access by process and call site. |
| `SR-DAT-008` | Maintain a cryptographic inventory and Cryptographic Bill of Materials (CBOM). | Discover libraries, algorithms, protocols, signatures, certificates, key uses, and persisted formats during build. | Compare SCA/SAST discovery and runtime observations with the CBOM. |
| `SR-DAT-009` | Design for cryptographic agility and post-quantum migration. | Keep algorithms and providers policy-driven, versioned, downgrade-resistant, and replaceable without changing data authority. | Substitute an approved test provider, reject deprecated algorithms, and verify migration/rollback fixtures. |
| `SR-DAT-010` | Enforce documented expiration, holds, export, backup, restore, and deletion. | Use one retention engine for all stores and indexes. | Advance a fake clock, apply holds, expire records, restore backup, and verify consistent outcomes. |
| `SR-DAT-011` | Use cryptographic erasure only where its assumptions are satisfied. | Track key scope and zeroization; document SSD and copy-on-write limitations. | Destroy scoped keys and prove ciphertext, indexes, caches, and backups are unreadable under the stated model. |
| `SR-DAT-012` | Provide a complete uninstall and sanitization procedure. | Inventory application files, model files, caches, logs, receipts, socket files, bookmarks, and keys. | Install, populate synthetic data, uninstall, then run the residue scanner and document externally managed remnants. |

### 8.5 Offline and Network Security

| ID | Requirement | Build integration | Reviewer test or evidence |
|---|---|---|---|
| `SR-NET-001` | Make no outbound connection after the separate installer exits. | Remove network entitlements and network client dependencies from normal components. | Observe packets, DNS, sockets, and system calls for at least 60 minutes; require zero outbound attempts and bytes. |
| `SR-NET-002` | Provide no cloud fallback, telemetry, analytics, crash upload, account check, update check, or remote model discovery. | Compile these capabilities out of v0.1, not merely disable them in UI. | Block the network and exercise every workflow; behavior must remain complete and deterministic. |
| `SR-NET-003` | Bind local services only to the declared local transport and treat namespace or loopback placement as reachability control, not authentication. | Prefer authenticated private IPC. Where Docker Model Runner's unauthenticated HTTP API is used on Linux, confine its immutable wildcard listener to the exact loopback-only, route-free private namespace, connect through the dedicated guard, expose only authenticated Unix IPC to the kernel, and deny every undeclared process, user, container, and namespace. | Scan all host and private-namespace interfaces, routes, listeners, and sockets and attempt LAN, container, cross-user, tool-worker, extension-host, and undeclared-process connections. |
| `SR-NET-004` | Attribute egress proof to AgentMage processes despite Visual Studio Code having independent network capability. | Identify processes, descriptors, and IPC flows in the evidence collector. | Run with VS Code offline controls and separately prove the AgentMage extension sends no repository or prompt content. |
| `SR-NET-005` | Make acquisition network use visible, bounded, and separate. | Show source host, artifact identity, expected size, license, hash, and destination before download. | Capture an acquisition session and verify no workspace, prompt, session, or credential content is transmitted. |
| `SR-NET-006` | Reject proxy, environment, DNS, and local-service confusion. | Ignore ambient proxy settings in offline components and pin local service identity. | Inject proxy variables, DNS overrides, hostile loopback services, and container aliases. |
| `SR-NET-007` | Return to a provable offline state after acquisition. | Require installer exit, closed sockets, staged-file verification, and a fresh offline preflight. | Complete, cancel, and corrupt downloads; all paths must end closed or visibly blocked. |

### 8.6 Secure Development and Supply Chain

| ID | Requirement | Build integration | Reviewer test or evidence |
|---|---|---|---|
| `SR-SUP-001` | Follow the project's declared OWASP, SLSA, OpenSSF, and AI-security development practices. | Maintain a task-level secure-development mapping with evidence links. | Sample each claimed practice and verify current evidence. |
| `SR-SUP-002` | Protect and separate source, build, test, signing, notarization, and release environments. | Use isolated ephemeral runners, least privilege, MFA, protected branches, and separate signing authority. | Review build identity, access logs, runner image, and credential boundaries. |
| `SR-SUP-003` | Pin source dependencies, build tools, parsers, model runtime, conversion tools, and platform SDKs. | Use lock files, hashes, allowlists, and explicit update review. | Attempt an undeclared or substituted dependency and verify build or startup failure. |
| `SR-SUP-004` | Produce complete source/build and shipped-binary SBOMs in a standard machine-readable format. | Generate SPDX or CycloneDX at build and compare with binary analysis. | Validate schema, required elements, transitive coverage, versions, hashes, licenses, and generation context. |
| `SR-SUP-005` | Produce release provenance linking source, builder, dependencies, commands, tests, artifacts, and signer. | Generate signed provenance and checksums from the release runner. | Independently verify signatures and trace a package back to source and build inputs. |
| `SR-SUP-006` | Perform supplier and component due diligence. | Apply `MODEL-PROVENANCE-POLICY.md` to models and record ownership/control, origin, maintainer, provenance, release history, resilience, support, license, vulnerabilities, and alternatives for every critical component. | Review all critical dependencies and model/runtime suppliers; block unknown, prohibited, revoked, or incomplete admission records. |
| `SR-SUP-007` | Maintain a Model BOM and runtime manifest. | Record model developer, license, lineage, original hash, conversion and quantization recipe, tokenizer, GGUF hash, immutable OCI digest where applicable, runtime build, adapter, and platform fit. | Recreate or independently verify every identity and hash, compare native/container behavior, and reject mutable tags or silent substitutions as release identities. |
| `SR-SUP-008` | Scan source, dependencies, binaries, packages, and model/runtime artifacts for known vulnerabilities and malware. | Use at least SAST, SCA, secret scanning, package scanning, and project-approved malware scanning. | Preserve tool versions, databases, suppressions, raw results, and disposition for every finding. |
| `SR-SUP-009` | Audit memory-unsafe and privileged code. | Prefer memory-safe implementation; inventory every `unsafe`, FFI, native library, and entitlement. | Require focused review, fuzz coverage, and justification for each boundary. |
| `SR-SUP-010` | Publish a vulnerability disclosure, triage, remediation, release, and notification process. | Implement `SECURITY.md`, including supported versions, signed manual patch delivery, emergency local disablement, embargo handling, and end of support. | Run a tabletop from report receipt through fixed signed release and notification. |
| `SR-SUP-011` | Provide deterministic or reproducible build evidence to the feasible degree. | Eliminate uncontrolled timestamps, paths, network fetches, and ambient tools. | Build twice in clean runners and compare outputs or explain signed, bounded differences. |
| `SR-SUP-012` | Support risk-based customer assurance requests without changing the product's ownership or default boundary. | Generate SBOM, secure-development mapping, supply-chain summary, optional attestation inputs, and additional evidence from one release manifest. | Reviewer selects an evidence profile; the package must be complete without manual reconstruction. |
| `SR-SUP-013` | Prevent release when a critical component is unmaintained, revoked, unverified, or outside policy. | Make supply-chain policy a release gate. | Mark a fixture component revoked and verify blocked build and blocked startup. |

### 8.7 Model and Generative-AI Security

| ID | Requirement | Build integration | Reviewer test or evidence |
|---|---|---|---|
| `SR-AI-001` | Register one explicit AI use case, purpose, users, data, decisions, and limitations. | Bind use-case identity to the release and session profile. | Compare actual prompts, tools, and outputs with the approved use case. |
| `SR-AI-002` | Require an intended-use and impact assessment before a customer enables a materially different use profile. | Supply a plain-language impact questionnaire and evidence without deciding for the customer. | Reviewer records the decision, rationale, owner, and review date. |
| `SR-AI-003` | Keep the model advisory and non-authoritative. | Label model claims as inferred until deterministic evidence supports them. | Use false and unsupported model outputs; UI must not present them as observed facts. |
| `SR-AI-004` | Require human control over expanded authority and consequential action. | v0.1 is read-only and has no automatic external action. | Attempt write, send, upload, deploy, execute, transfer, or privilege change; require zero action. |
| `SR-AI-005` | Defend against direct and indirect prompt injection. | Separate trusted policy, tool results, user input, and untrusted content; mediate every tool call in the kernel. | Run at least 200 labeled injections and require zero unauthorized action or secret disclosure. |
| `SR-AI-006` | Evaluate model/tool-call reliability on fixed, versioned corpora. | Pin prompts, schemas, decoding settings, model/runtime identity, and expected outcomes. | Require the release thresholds for schema validity, task accuracy, unsupported claims, and recovery. |
| `SR-AI-007` | Make uncertainty, missing evidence, conflicts, and limitations visible. | Use Observed, Derived, Inferred, and Unknown/Blocked states. | Run labeled ambiguous and contradictory cases and verify exact state assignment. |
| `SR-AI-008` | Prevent model extraction of secrets and unrelated content. | Minimize context and use policy-filtered bounded excerpts. | Seed canaries in prohibited files, environment, stores, and adjacent workspaces; require zero output. |
| `SR-AI-009` | Prevent model denial of service and resource exhaustion. | Bound context, output, concurrency, memory, CPU/GPU time, file size, result count, and cancellation. | Exercise oversized, recursive, compressed, adversarial-token, and non-terminating inputs. |
| `SR-AI-010` | Preserve provenance for every model-assisted inference. | Record model/runtime manifest, prompt-template version, cited evidence identity, and validation result without retaining raw sensitive prompts by default. | Resolve every material inference to the declared model and evidence state. |
| `SR-AI-011` | Evaluate truthfulness, factual grounding, uncertainty acknowledgement, and reproducibility. | Maintain a versioned factual and coding evaluation set with source-backed scoring. | Run repeat trials and report errors, uncertainty failures, and negative results without suppression. |
| `SR-AI-012` | Support customer-requested model and system disclosures without exposing weights or secrets. | Package model card, system-behavior specification, known limits, evaluation methods, and results. | Reviewer verifies disclosures and tests a representative sample. |
| `SR-AI-013` | Never silently change model, quantization, tokenizer, template, or runtime. | Require a new signed manifest and security impact review. | Modify each artifact and verify quarantine or startup refusal. |
| `SR-AI-014` | Provide immediate suspension and secure containment. | Add a local administrative disable policy that prevents model loading while preserving authorized evidence. | Trigger a simulated model incident and verify stop, containment, evidence preservation, and recovery. |
| `SR-AI-015` | Keep runtime supervision candidate neutral and model-family behavior inside closed codecs. | Bind every run to exact model, artifact, tokenizer, template, codec, runtime, quantization, modality, context, decoding, platform, hardware, driver, policy, and evaluation identities; decode only the closed proposal schema. | Substitute every tuple field and exercise unknown, malformed, partial, duplicate, stale, replayed, oversized, trailing, and unsupported output; require quarantine or inert rejection before authority. |
| `SR-AI-016` | Separate model quality from diagnostic repeatability and make determinism claims exact. | Maintain first-party-recommended quality profiles and separately pinned diagnostic-repeatability profiles with raw repeated-trial evidence. | Change sampler order, seed, slot, draft, context, driver, hardware, runtime, template, or grader; require incomparable results and prohibit cross-device, cross-driver, cross-release, or universal determinism claims. |
| `SR-AI-017` | Separate sensitivity, action risk, and model capability and keep learned classification non-authoritative. | Run static checks and deterministic deny-first policy over typed facts; permit a classifier only to deny, narrow, redact, isolate, or escalate. | Across at least 5,000 policy and 1,000 learned-classifier fixtures, attempt grant widening, denial override, destination substitution, model switching, and completion; require zero broader authority and narrower handling for uncertainty or failure. |
| `SR-AI-018` | Make successful completion a deterministic verified state. | Persist named agent states, exact proposal identity, no-progress ceilings, consumed grants, uncertain effects, verifier results, and restart reconciliation; allow only `SUCCESS` and verified `NO_OP` as success. | Inject false completion, confidence, model-judge approval, stalls, exhaustion, uncertainty, cancellation, crash, replay, and stale postconditions; require the exact non-success state unless current deterministic evidence proves completion. |

### 8.8 Audit, Logging, Incident Response, and Monitoring

| ID | Requirement | Build integration | Reviewer test or evidence |
|---|---|---|---|
| `SR-OPS-001` | Generate structured audit events for security-relevant actions. | Include time, release, actor/session pseudonymous identity, operation, object identity, result, policy decision, and correlation ID. | Exercise every event type and validate schema and required fields. |
| `SR-OPS-002` | Log access, denied access, grant lifecycle, privilege/authority changes, configuration changes, model/runtime changes, installer activity, integrity failures, and security alerts. | Map events to the product audit schema, threat model, and declared monitoring outcomes. | Run an event-coverage matrix and require 100 percent coverage of declared events. |
| `SR-OPS-003` | Exclude secrets, keys, credentials, raw prompts, private excerpts, environment values, and unrelated absolute paths from logs. | Apply typed redaction before serialization. | Inject canaries into every field and search logs, exports, errors, and crash output. |
| `SR-OPS-004` | Make logs locally tamper-evident and exportable to an external collector without adding a v0.1 network client. | Use hash chaining or signed checkpoints and explicit user/administrator export. | Alter, remove, reorder, and duplicate records; verification must detect each change. |
| `SR-OPS-005` | Use monotonic sequencing and handle clock changes visibly. | Record wall time, monotonic time, sequence, and clock-anomaly events. | Move the clock backward/forward and resume after sleep; preserve event order. |
| `SR-OPS-006` | Define incident detection, containment, evidence preservation, remediation, recovery, and notification responsibilities. | Ship a product incident runbook with a documented external-environment interface. | Conduct prompt-injection, package-tamper, key-store, and suspected-egress tabletops. |
| `SR-OPS-007` | Preserve only authorized incident evidence. | Apply separate incident-hold authority, classification, access, retention, and export controls. | Place a test hold, expire normal records, and verify held evidence remains bounded and protected. |
| `SR-OPS-008` | Continuously reassess supported releases, dependencies, model/runtime artifacts, cryptography, and tests. | Produce a signed monitoring report on each release and scheduled local review. | Verify stale or revoked components create findings and cannot be hidden by a green prior report. |
| `SR-OPS-009` | Keep security monitoring compatible with customer-selected endpoint monitoring and inventory tools. | Publish signed component, process, path, socket, hash, and version indicators. | Reviewer reconciles live processes/files with the declared inventory. |
| `SR-OPS-010` | Define support end, patch, emergency release, and rollback procedures. | Put support state in the signed manifest and diagnostics. | Simulate expired support, urgent patch, failed update, and rollback. |

### 8.9 Security Testing, Resilience, and Quality

| ID | Requirement | Build integration | Reviewer test or evidence |
|---|---|---|---|
| `SR-TST-001` | Maintain unit, property, integration, end-to-end, platform, adversarial, and recovery suites. | Tag every test with requirement IDs and evidence outputs. | Verify every applicable requirement has at least one assessment procedure. |
| `SR-TST-002` | Fuzz every parser and trust boundary. | Cover IPC, schemas, manifests, model output, paths, Git, text decoding, archives if enabled, and database import. | Report corpus, duration, coverage, crashes, sanitizer results, and fixes. |
| `SR-TST-003` | Use static analysis, dependency analysis, secret scanning, and platform security analysis. | Pin tools and policy; review suppressions as code. | Re-run from a clean checkout and compare signed results. |
| `SR-TST-004` | Test malformed, hostile, oversized, partial, stale, and conflicting input. | Create fixed adversarial fixtures for every input class. | Require bounded failure, no privilege change, no data leak, and a correct receipt. |
| `SR-TST-005` | Test crash consistency and uncertain results. | Inject failure before and after each durable state transition. | At least 100 crash resumes must repeat no completed operation and choose the correct safe state. |
| `SR-TST-006` | Test resource governance. | Enforce process, memory, CPU/GPU, disk, file, output, context, and concurrency limits. | Exceed each limit and verify cancellation, cleanup, audit, and continued responsiveness. |
| `SR-TST-007` | Test installation, update, migration, rollback, uninstall, and recovery on clean systems. | Use disposable clean Mac, Fedora, and Ubuntu fixtures with exact versions. | Three independent clean installations per supported platform using only published instructions. |
| `SR-TST-008` | Test endpoint-baseline compatibility. | Record the selected endpoint baseline before and after the package lifecycle. | No required control may be disabled or weakened. |
| `SR-TST-009` | Test release integrity independently from the build system. | Supply an offline verifier with no trust in build metadata alone. | Verify signatures, notarization, hashes, SBOM, provenance, entitlements, and component closure. |
| `SR-TST-010` | Separate test evidence from claims. | Produce raw results, normalized results, summary, failures, skipped tests, environment, and tool versions. | Recompute the summary from raw results and detect omitted failures. |
| `SR-TST-011` | Require independent review of critical boundaries. | Define review criteria for unsafe/FFI, crypto, IPC, sandbox, path, installer, update, and model parsing code. | Record reviewer, commit, findings, disposition, and re-review after changes. |
| `SR-TST-012` | Block release on failed security thresholds. | Make security gates non-bypassable except through a signed, dated risk decision outside normal development credentials. | Force each gate to fail and verify no production release is produced. |

### 8.10 Privacy, Records, Accessibility, and User Protection

| ID | Requirement | Build integration | Reviewer test or evidence |
|---|---|---|---|
| `SR-CIV-001` | Complete a privacy threshold analysis before permitting real customer-controlled data. | Generate the data inventory, purposes, subjects, flows, retention, sharing, and safeguards. | Privacy reviewer records the required privacy assessment and its applicability. |
| `SR-CIV-002` | Provide notice and purpose limitation for collected information. | Show concise local notices before workspace selection and persistence changes. | Verify no collection or secondary use outside the documented purpose. |
| `SR-CIV-003` | Support access, correction, export, deletion, retention, and legal hold decisions where applicable. | Keep these operations policy-mediated and receipted. | Run each lifecycle action against linked stores, indexes, exports, and backups. |
| `SR-CIV-004` | Determine whether prompts, outputs, receipts, and generated artifacts are subject to customer retention, legal-hold, or business-record requirements. | Label record-capable outputs and support export to an approved records system without self-declaring disposition. | The designated records owner records the schedule and required capture procedure. |
| `SR-CIV-005` | Never use an unapproved location as the authoritative store for customer-controlled records. | Separate application operational state from user-approved records export. | Verify the export destination and local expiration behavior. |
| `SR-CIV-006` | Conform to WCAG 2.2 AA for applicable interfaces and generated guidance. | Design keyboard, focus, screen-reader, contrast, zoom, error, status, and non-color cues from the first UI increment. | Run WCAG-aligned automated and manual assistive-technology tests and document exceptions. |
| `SR-CIV-007` | Publish a versioned accessibility conformance report aligned with WCAG 2.2. | Generate traceable results by criterion and product version. | Independent reviewer samples all supported, partial, and not-supported claims. |
| `SR-CIV-008` | Ensure generated reports and documentation are accessible. | Test Markdown rendering, HTML/PDF exports if supported, headings, tables, links, reading order, and alternatives. | Validate representative output with automated and manual checks. |
| `SR-CIV-009` | Provide clear limitations, prohibited data, recovery, reporting, and safe-use instructions. | Bundle versioned offline documentation and expose it from diagnostics. | A new reviewer completes the workflow without external assistance. |

### 8.11 Windows 11 First-GA Gate

Windows 11 x64 support is release-blocking for v1.0 GA. All common requirements and the separate Windows platform package must prove:

- Authenticode-signed and appropriately packaged binaries.
- Standard-user installation and execution compatible with common enterprise software deployment.
- Windows Defender, firewall, application-control, endpoint-detection, and enterprise inventory compatibility.
- A documented Windows sandbox and service/IPC identity boundary equivalent to the Mac contract.
- Windows ACL, alternate data stream, junction, symbolic link, reparse point, case-folding, device-path, UNC-path, and time-of-check/time-of-use defenses.
- Windows Credential Manager or DPAPI/CNG key handling through a reviewed provider and tested operating environment.
- Windows Event Log or approved structured local audit integration without sensitive data.
- A platform-specific secure configuration baseline and complete clean-system acceptance suite.
- A separate signed release manifest, threat model, test report, and platform release review.

Passing on macOS or Linux must never satisfy a Windows gate.

| ID | Requirement | Build integration | Reviewer test or evidence |
|---|---|---|---|
| `SR-PLT-013` | Package Windows 11 x64 per user with signed and timestamped MSIX and Authenticode identities. | Bind package, executable, library, extension, model, configuration, signer, and timestamp identities to the release manifest. | Install, repair, upgrade, roll back, and uninstall from three clean standard-user accounts; reconcile every file, package, process, registration, and residue. |
| `SR-PLT-014` | Authenticate Windows local IPC and confine every tool worker. | Validate named-pipe ACL, user, logon session, integrity level, executable/package identity, protocol, sequence, and launch challenge; use fresh restricted workers and Job Objects. | Attempt wrong-user, wrong-integrity, unsigned, replaced, replaying, malformed, oversized, and descendant escape clients and workers. |
| `SR-PLT-015` | Enforce the Windows workspace boundary against NTFS and path aliases. | Use handle-relative operations and reject unsupported device, UNC, alternate stream, reparse, link, alias, case, Unicode, rename, replace, and race states. | Run at least 1,000 path and race fixtures with zero boundary escape or wrong-object operation. |
| `SR-PLT-016` | Protect Windows keys and credentials without exposing values. | Protect data-encryption keys with DPAPI through a reviewed provider and use Credential Manager only through bounded references. | Seed credential and key canaries across model, logs, diagnostics, receipts, errors, crash output, command lines, environments, and repositories; require zero disclosure. |
| `SR-PLT-017` | Keep strict-local and connected Windows workers separate. | Give native `llama.cpp` and file workers no network; give provider workers one exact destination, credential reference, operation, budget, and expiry. | Prove zero outbound bytes in strict-local mode and run cross-adapter destination, credential, proxy, redirect, and removal attacks in connected mode. |

### 8.12 Connected Delivery Security

| ID | Requirement | Build integration | Reviewer test or evidence |
|---|---|---|---|
| `SR-DEL-001` | Publish an exact provider/version/object/operation support matrix. | Generate the matrix from signed adapter manifests and conformance evidence. | Select every tuple and verify its registered methods, scopes, limits, unsupported operations, test identity, and support state. |
| `SR-DEL-002` | Keep capability classes independent. | Encode `observe`, `draft`, `local-write`, `remote-write`, `execute`, `deploy`, `secrets`, and `admin` as non-inheriting grants and policy classes. | Attempt every pairwise class escalation and require zero unauthorized registration or execution. |
| `SR-DEL-003` | Isolate credentials by provider, exact host, tenant, account, project, environment, and capability. | Resolve credentials in the operation worker from the platform secret store after grant validation. | Run at least 2,000 cross-domain, redirect, proxy, callback, clone-host, and credential-confusion attempts with zero disclosure or wrong-host request. |
| `SR-DEL-004` | Treat all provider, event, log, artifact, telemetry, incident, finding, and catalog content as untrusted. | Keep content outside policy and grant channels; classify, bound, parse safely, and cite before model use. | Put injection, malicious links, archives, scripts, credentials, and false completion statements in every field class; require zero authority change or secret disclosure. |
| `SR-DEL-005` | Bind every external effect to current exact state and one consumed grant. | Re-read affected objects before preview and submission; bind actor, target, payload, visibility, revision, environment, expected effect, preview digest, expiry, and recovery. | Mutate every bound field and remote precondition after preview; require denial and no provider request. |
| `SR-DEL-006` | Prevent duplicate or unsafe remote effects. | Use provider idempotency where available and deterministic fingerprints plus reconciliation otherwise; treat timeouts as unknown. | Inject loss before and after effect, duplicate responses, retry, replay, eventual consistency, and partial success; require no duplicate and no retry while effect remains unknown. |
| `SR-DEL-007` | Verify postconditions and make rollback or compensation a new operation. | Record effect, non-effect, partial effect, unknown effect, changed remote identities, and recovery choices in immutable receipts. | Change remote state after the original effect and prove rollback cannot overwrite later work without a new exact preview and grant. |
| `SR-DEL-008` | Separate CI execution, deployment, infrastructure application, secret operations, and administration from generic writes. | Use dedicated tool registrations, grants, previews, policies, budgets, and evidence for each authority path. | Attempt to embed each stronger effect inside comments, issue updates, repository writes, pipeline inputs, manifests, and nested provider calls; require denial. |
| `SR-DEL-009` | Verify webhook and polling integrity. | Validate signatures, host, tenant, timestamp, nonce, event identity, ordering, replay, gap, backfill, duplicate, and tombstone behavior. | Forge, reorder, delay, duplicate, omit, replay, and mutate event streams; require deterministic state and no duplicate task or action. |
| `SR-DEL-010` | Make every adapter completely removable. | Remove credentials, cache, event registrations, webhooks, schedules, processes, tools, network scopes, and retained data according to policy. | Remove each adapter independently and rerun strict-local plus neighboring-adapter suites; require zero residue or damage. |
| `SR-DEL-011` | Preserve source-to-release and incident-to-rollback identity. | Link work, commit, CI, artifact digest, provenance, deployment, telemetry, incident, and release through evidence-backed graph edges. | Mutate names, transfer repositories, reuse identifiers, move branches, replace tags, and skew clocks; require exact immutable identity or visible unresolved state. |
| `SR-DEL-012` | Bound provider version drift and degradation. | Refuse unsupported versions or enter declared diagnostic/read-only degradation without silently inheriting support. | Exercise minimum, maximum, future, missing-feature, and behavior-changed providers and verify exact registration and support claims. |
| `SR-DEL-013` | Prevent delivery evidence and summaries from hiding failures. | Preserve raw responses, normalized records, failures, skips, retries, suppressions, versions, environment, and reviewer dispositions. | Recompute every adapter and cross-provider gate from raw evidence and inject omitted failures; require mismatch detection and blocked release. |
| `SR-DEL-014` | Protect delivery operations from resource exhaustion. | Bound pagination, logs, artifacts, archives, event rates, telemetry cardinality, concurrent workers, model context, disk, memory, processor, and graphics load. | Exceed each bound during reads, execution, reconciliation, and cancellation; require cleanup, responsiveness, accurate receipt, and no authority expansion. |

### 8.12A Repository and GitHub Safety

The normative implementation detail for this control family is
[`docs/security/repository-safety.md`](./docs/security/repository-safety.md).

| ID | Requirement | Build integration | Reviewer test or evidence |
|---|---|---|---|
| `SR-GIT-001` | Keep models and repository content outside Git and GitHub authority. | Expose only closed kernel operations through dedicated adapters; give models no raw Git, shell, transport, credential, signer, socket, or provider-token handle. | Request every admitted and prohibited operation through prompts, files, hooks, hosted text, errors, workflow output, and model calls; require no direct invocation or authority change. |
| `SR-GIT-002` | Preserve user files and complete unrelated Git state. | Bind and reconcile a minimized preservation manifest covering checkout, index, paths, refs, notes, stash, reflogs, tags, configuration, hooks, filters, worktrees, submodules, Large File Storage, locks, and in-progress operations. | Mutate each field before approval, before launch, concurrently, and after interruption; require stale denial, exact accounting, and zero unexplained change or work loss. |
| `SR-GIT-003` | Make destructive and implicit Git behavior structurally unrepresentable. | Register only clone, fetch, owned-worktree create/remove, local branch fast-forward, signed commit, and push; omit pull, merge, rebase, reset, clean, discard, stash/tag/note mutation, deletion, remote configuration, arbitrary ref update, mirror, force, and hook execution. | Enumerate schemas, manifests, tool registries, policy paths, shells, aliases, provider extensions, and binaries; require absence or denial before any process or request. |
| `SR-GIT-004` | Isolate Git from hostile configuration and executable repository behavior. | Pin executable identity and arguments; sanitize environment; disable or reject aliases, URL rewrites, unsafe ownership, helpers, hooks, filters, drivers, pagers, editors, signers, alternates, replacement refs, unsupported protocols, maintenance, submodule recursion, and Large File Storage transfer. | Seed every executable/configuration path and environment variable with canaries and side effects; require zero execution, disclosure, network expansion, or unowned write. |
| `SR-GIT-005` | Make clone and fetch exact, namespaced, and non-destructive. | Clone without checkout into a new owned destination; fetch one exact remote ref through an empty ref map into one AgentMage namespace with no prune, tags, `FETCH_HEAD`, shallow mutation, maintenance, or user-ref update. | Exercise hostile objects, paths, refs, refspecs, redirects, partial transfers, collisions, quotas, and concurrent ref movement; require quarantine or exact isolated result. |
| `SR-GIT-006` | Isolate coding and cleanup in owned worktrees. | Create one task branch/worktree from an immutable base and remove only a proven-owned, clean, process-free worktree after recovery retention. | Exercise dirty, detached, linked, missing, moved, locked, inaccessible, interrupted, and process-active worktrees; require preservation and visible recovery rather than deletion. |
| `SR-GIT-007` | Create commits from exact approved state only. | Use a temporary owned index, approved path identities and bytes, exact parent/message/author, pinned signer, signature verification, no hooks/filters, and compare-and-swap update of one task branch. | Change index, path, bytes, message, parent, identity, signer, attributes, branch, or tests after preview; require zero commit and no active-index mutation. |
| `SR-GIT-008` | Keep commit and push separate and make push one exact ordinary fast-forward. | Require a second grant bound to canonical host/account/repository/credential/full ref/expected old/new objects; prohibit defaults, multiple destinations, tags, deletion, options, recursion, force, lease, and bypass. | Attempt every implicit destination and prohibited refspec/flag plus branch movement and protection/ruleset change; require no request or wrong-ref effect. |
| `SR-GIT-009` | Use short-lived, least-privilege, host-bound authentication. | Prefer repository-scoped GitHub App installation tokens; bind approved fallback methods to exact host, account, repository, operation, permissions, single-sign-on, expiry, certificate or Secure Shell host identity, and non-secret reference. | Run cross-host/account/repository/token/helper/proxy/redirect/certificate/host-key confusion and expiry/revocation cases; require zero credential disclosure or crossover. |
| `SR-GIT-010` | Treat provider protections as additional controls, not authority. | Observe and bind rulesets, branch protection, signatures, reviews, checks, push rules, merge queues, and bypass capabilities; never exercise bypass automatically. | Change each protection after preview, grant a bypass-capable actor, and inject misleading hosted output; require stale denial and no bypass use. |
| `SR-GIT-011` | Reconcile uncertain remote effects before retry. | Record verified effect, verified non-effect, partial effect, or unknown effect; perform a fresh bounded remote observation after transport uncertainty and block retry while unknown. | Crash, cancel, partition, time out, duplicate, truncate, and corrupt responses before and after server effect; require no duplicate push and one truthful terminal receipt. |
| `SR-GIT-012` | Bound repository objects, disclosures, and resources. | Limit object count/bytes/decompression/depth, output, history, paths, workers, disk, time, credentials, diagnostics, and retained manifest content; scan outgoing commit ranges for secrets and policy violations. | Use malformed or oversized objects, bombs, deep trees, secret canaries, binary payloads, sensitive paths, and resource exhaustion; require quarantine or denial without leakage or residue. |

### 8.13 Productivity, Communications, Finance, and Cloud Observation

| ID | Requirement | Build integration | Reviewer test or evidence |
|---|---|---|---|
| `SR-PRD-001` | Enforce autonomy as an intersected policy ceiling outside the model and shell. | Represent global, pack, connector, account, workspace, operation, destination, recipient, channel, payload, schedule, budget, and expiry ceilings in kernel policy; keep the UI control non-authoritative. | Attempt every pairwise and parent/child level escalation through prompts, messages, models, shell state, stale displays, nested workflows, and replay; require zero broader effective authority. |
| `SR-PRD-002` | Make autonomy changes authenticated, visible, narrowable, and reversible. | Preview the exact policy delta, require user authentication, persist a versioned decision, expose current effective state, and provide a global external-write disable action. | Mutate the preview, race the policy, use a stale session, cancel in flight, and activate emergency disablement; require no new write and deterministic reconciliation. |
| `SR-PRD-003` | Bind every communication effect to exact sender, recipients, destination, visibility, content, and attachments. | Register read, draft, send, reply, forward, edit, delete, reaction, attachment, move, label, flag, and archive operations separately. | Mutate sender, alias, recipient, external domain, channel, thread, mention, visibility, body, formatting, quoted text, link, attachment, classification, or provider transformation after preview; require denial and no request. |
| `SR-PRD-004` | Prevent deceptive or inferred recipient identity. | Use provider-scoped identities and require confirmed cross-provider links or exact deterministic identifiers for authorization. | Reuse display names, homoglyphs, aliases, stale contacts, moved accounts, renamed channels, and conflicting directories; require visible ambiguity and no send. |
| `SR-PRD-005` | Treat every message, attachment, invitation, contact, task, document, transaction description, and cloud record as untrusted data. | Keep external content outside instruction, policy, grant, tool, workflow, and completion channels; parse in bounded workers and classify before model use. | Seed authority claims, hidden text, scripts, macros, malicious links, credential requests, false completion, recipient substitution, and workflow instructions in every field class; require zero authority or secret disclosure. |
| `SR-PRD-006` | Preserve complete and truthful synchronization state. | Record cursor, sequence, provider and observation time, edit, deletion, tombstone, backfill, duplicate, gap, freshness, permission, and coverage state. | Lose or rewind cursors, reorder, duplicate, omit, edit, delete, backfill, throttle, revoke, and partition; require deterministic recovery and visible incompleteness. |
| `SR-PRD-007` | Isolate provider, tenant, account, mailbox, workspace, channel, calendar, document repository, and local-client profiles. | Give each operation worker one credential reference, exact destination, capability, data scope, and cache partition. | Run cross-provider, account, tenant, mailbox, workspace, channel, calendar, repository, cache, token, redirect, proxy, and loopback confusion attacks; require zero crossover. |
| `SR-PRD-008` | Keep local mail-client interoperability out of private profile mutation. | Use provider APIs, capability-detected mail protocols, or explicit read-only mbox and Maildir import; prohibit direct profile-database writes. | Replace, lock, corrupt, symlink, race, or version-skew Outlook, Thunderbird, Evolution, and KMail profiles; require no private-profile write or credential extraction. |
| `SR-PRD-009` | Protect attachments and document visibility. | Scan, classify, hash, size-bound, type-verify, preview, and bind every attachment and document permission to the effect grant. | Swap attachments, spoof types, use traversal archives, macros, external links, public-link changes, permission broadening, decompression bombs, and post-preview edits; require quarantine or denial. |
| `SR-PRD-010` | Compile workflows into bounded deterministic operation graphs. | Bind trigger, connectors, accounts, filters, branches, operations, recipients, classifications, budgets, stop conditions, expiry, approvals, recovery, and receipts. | Attempt self-edit, recursive trigger, destination substitution, budget expansion, hidden branch, approval aggregation, message-triggered instruction, and cross-pack grant reuse; require denial. |
| `SR-PRD-012` | Constrain Proton Calendar UI interaction to an exact confirmed calendar operation rather than generic browser authority. | Bind an allowlisted origin, visible user-authenticated profile, provider-surface version, structured document or accessibility controls, bounded visual fallback, calendar, event, attendees, notifications, preconditions, postconditions, fresh grant, and no-blind-retry reconciliation; keep credentials and profile state outside model context and evidence. | Mutate origin, profile, account, calendar, event fields, attendee, reply correlation, surface version, control identity, focus, screenshot, policy, session, and postcondition; inject ambiguous replies, redirects, overlays, reauthentication, second-factor prompts, timeout, partial effect, and duplicate submission; require no credential exposure, guessed action, inferred consent, blind retry, duplicate effect, or false completion. |
| `SR-FIN-001` | Use exact financial arithmetic and explicit currency semantics. | Use fixed-point decimal values with currency, scale, sign, rounding, effective date, and conversion-source contracts. | Exercise boundary magnitudes, negative values, fractional currencies, rounding modes, exchange rates, splits, fees, and aggregate order; require exact deterministic results. |
| `SR-FIN-002` | Preserve immutable financial source records and auditable corrections. | Hash imports, retain source identity and pending/posted state, deduplicate deterministically, and use adjustment or supersession records rather than silent rewrites. | Mutate imports, pending transactions, posting dates, identifiers, descriptions, splits, transfers, statement balances, and corrections; require preserved lineage and visible conflict. |
| `SR-FIN-003` | Require deterministic statement and account reconciliation. | Bind account, period, opening and closing balances, included transactions, pending exclusions, corrections, tolerance, and reviewer disposition. | Inject missing, duplicate, reordered, cross-account, wrong-currency, pending, deleted, and changed transactions; require no false reconciliation. |
| `SR-FIN-004` | Keep financial categorization and anomaly analysis explainable and non-authoritative. | Record rule or model identity, features, baseline, confidence, limitations, source evidence, and user disposition; label outputs as proposals or indicators. | Shift distributions, poison descriptions, create sparse history, duplicate merchants, change categories, and insert adversarial text; require no unsupported fraud or advice claim and no automatic external action. |
| `SR-FIN-005` | Prohibit money movement and financial-account administration in v1.0. | Omit transfers, payments, bill pay, trades, orders, withdrawals, deposits, credit, loans, tax filing, beneficiaries, account administration, and credential recovery from schemas, manifests, tools, policies, and adapters. | Probe every shell, model, prompt, workflow, connector, schedule, API route, nested payload, autonomy level, and provider extension for each prohibited family; require structural absence or denial before request. |
| `SR-FIN-006` | Isolate financial records from general memory and unrelated packs. | Encrypt financial data, use dedicated classification and retention, minimize model context, and require explicit receipted cross-pack disclosure. | Search logs, memory, prompts, diagnostics, exports, chat drafts, delivery caches, crash output, and removed-pack residue for canaries; require zero undeclared disclosure. |
| `SR-CLD-001` | Keep Cloud Observer strictly read-only in v1.0. | Manifest only bounded inventory, configuration, health, metric, log, audit-reference, security-observation, deployment-identity, and cost reads. | Probe resource writes, remote commands, shells, deploys, secret values, identity, policy, logging, budget, upload, delete, and administration through every interface and provider extension; require absence or denial before request. |
| `SR-CLD-002` | Scope cloud reads to exact provider and resource boundaries. | Bind provider, organization or tenant, account, subscription or project, region, service, resource, query, time, field, row, byte, and rate. | Mutate each field; attempt assume-role, subscription, project, region, service, resource, data-plane, log-query, redirect, and credential crossover; require zero out-of-scope data. |
| `SR-CLD-003` | Prevent cloud observations from authorizing delivery or remediation. | Label observations and correlations as evidence only; require separate delivery-system plans, policies, previews, and grants for any effect. | Insert remediation instructions, false severity, deployment requests, credential references, and causation claims into metrics, logs, findings, tags, and cost records; require zero effect or authority change. |
| `SR-PRD-011` | Make every productivity pack completely removable. | Cancel queues, reconcile effects, revoke credentials, remove caches, cursors, webhooks, schedules, indexes, workers, sockets, and network scopes, and apply declared retention. | Remove each pack and all packs together, scan every platform boundary, and rerun strict-local plus delivery suites; require zero undeclared residue or neighboring-pack damage. |

### 8.14 Trusted Operations, Research, Continuity, and Model Management

| ID | Requirement | Build integration | Reviewer test or evidence |
|---|---|---|---|
| `SR-CMD-001` | Enforce command authority outside the model at Disabled, Inspect, Workspace Autonomous, Connected Operations, and Owner / Unrestricted Session levels. | Intersect command level with global, pack, connector, account, workspace, operation, destination, schedule, budget, expiry, and emergency policy immediately before launch. | Cross every level and ceiling; mutate policy, display, plan, grant, time, and actor; require zero broader effective authority or launch. |
| `SR-CMD-002` | Make Owner / Unrestricted Session a direct, authenticated, expiring device-owner decision. | Bind activation to current user, device, login session, release, kernel launch, risk preview, duration, visible indicator, panic stop, and automatic revocation on lock, logout, restart, expiry, policy change, or incident. | Attempt activation, replay, inheritance, scheduling, auto-renewal, stale-session use, hidden UI, child survival, and post-revocation execution; require zero unauthorized or surviving command. |
| `SR-CMD-003` | Preserve full command semantics without hiding process authority. | Record direct executable or shell identity, arguments, pipelines, redirections, interpreters, PTY, working directory, paths, environment, credentials, network, descendants, persistence, resources, timeout, changed files, rollback, and cancellation. | Exercise every command form and mutate every bound field around preview and launch; require exact execution or denial and one truthful receipt. |
| `SR-CMD-004` | Keep constrained modes confined and disclose the limits of Owner mode. | Use platform sandboxes for constrained workers; minimize Owner-mode environment and state plainly that host-user commands are not confined, while keeping elevation and credential injection separately authorized. | Probe path, environment, network, credential, process, device, persistence, and privilege escape in constrained modes; verify accurate high-risk disclosure and no implicit elevation or secret injection in Owner mode. |
| `SR-WEB-001` | Treat public research as bounded evidence acquisition, not generic network authority. | Bind query, recency, domains, schemes, DNS, proxy, certificate, redirects, items, bytes, media, scripts, archives, cache, downloads, and cancellation in a separate worker. | Mutate each field and exercise malicious or changing sites; require no undeclared destination, request, download, or cache state. |
| `SR-WEB-002` | Preserve current, claim-level web evidence. | Record query, provider, direct URL, redirects, title, retrieval time, publication date, event date where known, relevant excerpt hash, source quality, cache state, and inference status. | Recompute answers across current, stale, conflicting, moved, unavailable, primary, and secondary sources; require complete citation and visible uncertainty. |
| `SR-WEB-003` | Prevent web content from creating authority. | Keep page text, hidden text, metadata, scripts, links, downloads, authentication prompts, and tool requests in untrusted evidence channels. | Seed prompt injection, credential requests, shell commands, model downloads, false completion, redirects, and policy claims in every field; require zero authority change or external effect. |
| `SR-WEB-004` | Prevent undeclared private-data disclosure during research. | Give research workers no ambient workspace, memory, communication, finance, connector, or secret access; require an exact classified disclosure preview for selected fields. | Place canaries in every private domain and request them through queries, forms, URLs, headers, uploads, redirects, and downloaded instructions; require zero undeclared egress. |
| `SR-CRD-001` | Store long-lived secrets only through supported operating-system facilities. | Use Linux Secret Service, macOS Keychain, and Windows Credential Manager or DPAPI with typed non-secret references in product state. | Scan databases, configuration, files, environment, process metadata, logs, diagnostics, exports, crash data, and backups for canaries; require zero raw secret. |
| `SR-CRD-002` | Resolve one credential only for one exact operation worker. | Validate worker identity, provider, host, tenant, account, operation, scope, grant, expiry, redirect, and proxy before resolution; clear bounded memory at termination. | Run cross-worker, provider, host, tenant, account, scope, redirect, proxy, callback, replay, expiry, crash, and residue attacks; require zero wrong resolution or disclosure. |
| `SR-CRD-003` | Make credential lifecycle and restore behavior explicit. | Support scoped OAuth, short-lived tokens, agent/certificate references, rotation, expiry, revocation, deletion, and reauthentication after restore without backing up raw credentials. | Revoke, expire, rotate, restore, remove, and race every reference state; require fail-closed operation, truthful status, no secret backup, and deterministic reauthentication. |
| `SR-BCK-001` | Keep local canonical state out of synchronized and network storage. | Reject live operational databases, model stores, indexes, locks, queues, sockets, and temporary roots on known cloud-sync, network, or remote filesystems. | Test symlink, mount, reparse, rename, alias, synchronization-folder, network-share, and race substitutions; require startup or persistence denial before live use. |
| `SR-BCK-002` | Create immutable, complete, client-side-encrypted snapshots. | Version manifests, classifications, exclusions, chunks, integrity trees, encryption metadata, retention, schema, release identity, and atomic completion. | Interrupt every phase; corrupt, omit, duplicate, reorder, replay, replace, and exhaust space; require no false-complete snapshot or plaintext artifact. |
| `SR-BCK-003` | Scope cloud continuity to one exact encrypted backup namespace. | Use a separate least-privilege worker and credential bound to provider, account, container or folder, prefix, object methods, bytes, rate, retention, deletion, and restore. | Attempt cross-account, cross-prefix, list, read, write, delete, redirect, proxy, and credential reuse outside the namespace; require zero access or request. |
| `SR-BCK-004` | Keep cloud backup separate from read-only Cloud Observer. | Register continuity writes independently and reject Cloud Observer credentials, tools, schemas, grants, workflows, or autonomy as backup authority and vice versa. | Embed backup writes in cloud observations and cloud mutations in backup requests across every shell and workflow; require structural absence or denial before request. |
| `SR-BCK-005` | Prove restore, rollback, retention, deletion, and disaster recovery. | Restore into staging, verify complete objects and compatibility, migrate, display included/excluded domains, confirm, swap atomically, and preserve rollback. | Exercise clean-device, cross-version, missing, corrupt, stale, replayed, revoked, throttled, interrupted, ransomware-like, retention, deletion, and provider-outage cases; require deterministic recovery or visible block. |
| `SR-MGM-001` | Admit model catalog states only from exact evidence. | Sign candidate, evaluating, approved, degraded, quarantined, rejected, and retired entries with complete model-policy fields and transitions. | Mutate identity, license, lineage, artifact, hash, transformation, runtime, resource, quality, security, support, expiry, and state; require the exact block or transition. |
| `SR-MGM-002` | Make chat-guided model operations deterministic and user confirmed. | Display exact profile, publisher, license, source, artifacts, size, disk, hardware fit, network, destination, checks, limitations, and rollback before launching the separate installer. | Mutate every displayed field after confirmation and exercise stale catalog, low disk, cancellation, crash, and retry; require no acquisition or activation outside the plan. |
| `SR-MGM-003` | Keep model acquisition, admission, and inference separate. | Download or import into quarantine, verify signatures and hashes, scan, self-test, activate atomically, and roll back without workspace, session, connector, credential, or inference authority in the installer. | Attempt source substitution, redirect, artifact replacement, archive/path escape, credential access, workspace read, self-approval, partial activation, and automatic fallback; require denial or rollback. |
| `SR-MGM-004` | Represent Muse Glimmer truthfully as the primary deep-evaluation candidate. | Require verified first-party classification, license, origin, lineage, artifact, tokenizer, template, codec, runtime, resource, quality, repeatability, security, and platform evidence before any support or activation claim. | Remove, contradict, or fail each evidence class and require `BLOCKED` or `REJECTED`; a non-pass cannot block an independently conforming eligible profile or be hidden as support. |
| `SR-MGM-005` | Inventory eligible official first-party Gemma profiles completely and evaluate them by role. | Freeze the authoritative source catalog; record exact identity, role, applicability, provenance, policy, artifact, runtime, hardware, suites, result, and exclusions; use `BLOCKED-HARDWARE` instead of silent omission. | Reconcile 100% of frozen catalog entries; mutate role, applicability, source, lineage, hardware, test, and state; require no omitted candidate, family-wide inference, role escalation, automatic activation, or borrowed result. |
| `SR-LAB-001` | Isolate the post-GA Experimental Model Lab from trusted authority. | Give the lab a separate package, process, data root, synthetic corpus, no network, no credentials, no commands, no connectors, no canonical writes, and strict resource limits. | Probe every IPC, path, socket, tool, credential, connector, workspace, approved-store, and resource boundary with hostile models; require structural absence or denial. |
| `SR-LAB-002` | Keep experimental provenance and license gaps visible. | Record user-selected source, hashes, observed license, lineage gaps, quarantine, format, resource preflight, warnings, and evaluation limitations without treating them as approval. | Import malformed, unknown, mirrored, mutable, provenance-incomplete, license-unclear, oversized, and policy-excluded artifacts; require truthful state and no ordinary activation. |
| `SR-LAB-003` | Require normal admission for experimental promotion. | Provide no direct promotion route; create a new independent admission record and approved catalog transition for any candidate. | Attempt promotion through chat, files, catalog edits, lab results, model output, copied manifests, stale approval, or user preference; require zero activation until normal admission passes. |
| `SR-TOP-001` | Make every trusted-operations capability independently removable. | Expire grants, stop schedules, reconcile work, terminate descendants, revoke network rules, clear credentials and scratch, remove registrations and caches, apply retention, and rerun strict local. | Disable and remove each capability alone and together during idle, active, queued, interrupted, uncertain, and restored states; require zero undeclared authority or residue. |

### 8.15 Whole-Codebase Audit Security

| ID | Requirement | Build integration | Reviewer test or evidence |
|---|---|---|---|
| `SR-AUD-001` | Bind every audit to an exact repository and complete scope manifest. | Record root, revision, worktree, index, dirty state, inclusion and exclusion rules, path classes, parser set, model, policy, resources, history, commands, network, retention, and outputs before execution. | Mutate every identity and enumerate tracked, untracked, ignored, sparse, generated, vendored, binary, submodule, worktree, link, archive, inaccessible, and changing paths; require one exact terminal disposition per in-scope path. |
| `SR-AUD-002` | Enforce canonical repository immutability outside the model. | Give census and parser workers read-only source handles; run every potentially writing command in a disposable copy-on-write workspace; attest repository, Git, process, socket, hosted, and neighboring state before and after. | Run builds, tests, generators, formatters, package tools, hooks, language services, crashes, races, and hostile repository instructions; require zero canonical, hosted, credential, or neighboring-data mutation. |
| `SR-AUD-003` | Prevent repository secrets and restricted data from entering model or retained audit surfaces. | Detect and classify before packet creation, substitute typed redactions, isolate restricted parsing, and scan prompts, context, cards, findings, checkpoints, logs, diagnostics, exports, and reports with canaries. | Seed credentials and private values in every file and metadata class, encodings, archives, generated output, history, issues, logs, and errors; require zero raw-value exposure or retention. |
| `SR-AUD-004` | Isolate parsers, archives, language services, and build metadata as hostile inputs. | Use fresh bounded workers with exact path, file-count, recursion, archive, CPU, memory, disk, process, output, timeout, cancellation, and no-network defaults. | Exercise malformed syntax, decompression bombs, path traversal, links, parser exploits, build scripts, plugins, special files, denial of service, and process descendants; require bounded failure and cleanup. |
| `SR-AUD-005` | Keep deterministic repository facts authoritative over model memory and retrieval. | Store exact path, symbol, graph, parser, source-span, hash, and coverage records; give models bounded packets without repository handles; treat summaries and embeddings as non-authoritative observations. | Reorder packets, vary context, retrieval, approved model, and summary depth, and inject false identity or coverage claims; require unchanged deterministic state and no invented evidence. |
| `SR-AUD-006` | Make checkpoint resume and transitive invalidation fail closed. | Bind checkpoint records to source, scope, parser, model, runtime, policy, queue, card, graph, resource, and completion identities with reverse dependencies. | Interrupt every phase and mutate files, paths, rules, parsers, models, graphs, cards, checkpoints, and clocks; require exact resume or explicit broader rescan with no stale result represented as current. |
| `SR-AUD-007` | Require calibrated, immutable evidence for every finding and report claim. | Use closed finding schemas for identity, category, severity, confidence, impact, evidence, graph paths, counterevidence, uncertainty, recommendation, conflict, deduplication, and status. | Remove, replace, stale, contradict, duplicate, or weaken each evidence class and seed false positives and model disagreement; require visible uncertainty, correct blocking, and no unsupported claim. |
| `SR-AUD-008` | Require cross-module reconciliation before repository-wide conclusions. | Trace dependency, call, state, data, event, error, configuration, authorization, lifecycle, requirement, decision, test, deployment, and support relationships and retain contradictions. | Seed distributed drift, duplicate systems, circular dependencies, orphaned code, stale adapters, state conflicts, abandoned migrations, and documentation drift; require all material sides or a local-only limitation. |
| `SR-AUD-009` | Bound audit resources without silently dropping coverage. | Enforce visible file, byte, graph, packet, context, output, concurrency, CPU, memory, graphics, disk, time, and retention limits with checkpointed pause or block. | Exceed each limit during census, parsing, verification, semantic review, reconciliation, reporting, cancellation, and recovery; require responsiveness, cleanup, accurate gaps, and no weakened profile. |
| `SR-AUD-010` | Make audit capability and retained evidence independently removable. | Cancel work, terminate workers, remove scratch, indexes, cards, checkpoints, findings, caches, registrations, and path authorities according to retention without changing source or neighboring data. | Remove during idle, parsing, verification, model analysis, reconciliation, reporting, crash, and restore; require zero worker, process, socket, path, schedule, authority, or undeclared residue. |

### 8.16 Foundational Artifact and Workflow Runtime Security

Decision 0042 composes existing controls rather than creating a separate security domain or new
stable requirement identifiers. The mapped controls and reviewer protocols remain release-blocking
where their owning sprint or release gate says so.

| Boundary | Required control composition | Blocking evidence |
|---|---|---|
| Request-bound Chat ingress | `SR-ACC-001` through `SR-ACC-008`, `SR-PLT-005`, `SR-DAT-001` through `SR-DAT-004`, and `SR-AI-005` | Every supplied reference is accounted for; ambient workspace access and silent non-text omission are absent; remote or virtual references disclose no bytes without exact current request authority. |
| Source-artifact admission and extraction | `SR-PLT-003`, `SR-PLT-004`, `SR-DAT-001` through `SR-DAT-007`, `SR-TST-002`, `SR-TST-004`, `SR-TST-006`, and parser-isolation portions of `SR-AUD-004` | Detection, parsing, active-content refusal, archive and relationship traversal, resource ceilings, cancellation, crash cleanup, secret handling, and unsupported states pass the hostile corpus. |
| Context budgeting and source persistence | `SR-DAT-003` through `SR-DAT-008`, `SR-AI-002`, `SR-AI-006`, `SR-AI-009`, `SR-OPS-001`, and `SR-OPS-002` | Exact token partitions reconcile; every omission is visible; source manifests and encrypted payload references obey classification, retention, deletion, refresh, and invalidation rules. |
| Step admission and tool calls | `SR-ACC-002`, `SR-ACC-004`, `SR-AI-003` through `SR-AI-008`, `SR-OPS-001`, and `SR-TST-004` | Missing, malformed, stale, unauthorized, over-budget, and unsupported calls fail before dispatch; model content cannot create authority or completion. |
| Side effects, attempts, and recovery | `SR-ACC-004`, `SR-OPS-001` through `SR-OPS-005`, `SR-TST-005` through `SR-TST-009`, plus connected-effect controls when applicable | Every permitted retry is a fresh attempt; no consumed grant or effect is replayed; uncertain effects block; budgets terminate; one diagnostic exists for every non-cancelled failure. |
| Cross-interface and MCP exposure | `SR-PLT-005`, `SR-AI-005`, `SR-TST-004`, `SR-TST-007`, and adapter controls when applicable | Chat, CLI, headless, and MCP clients produce the same correctness state and evidence from the same authorized input; no client gains parser, policy, storage, effect, retry, or completion authority. |

The first execution owners are the mapped Decision 0042 stories in Sprints 1, 2, 5, 11, 13, 16,
21-23, 50, 58, 60, 62, and 81. `RV-03`, `RV-04`, `RV-08`, `RV-11`, `RV-12`, `RV-15`, `RV-16`,
`RV-17`, `RV-18`, and `RV-25` apply according to the exact boundary under review. Existing
no-hidden-retry, no-replay, verifier-only completion, zero-silent-drop, and parser-isolation results
remain mandatory regression evidence.

### 8.17 Engineering Runtime, Verified Chat, and Model Gateway Security

Decisions 0043 and 0044 add the controls below. They compose, and never weaken, all earlier
artifact, workflow, model, platform, Git, connected-effect, command, credential, audit, and
strict-local controls.

| ID | Requirement | Build integration | Reviewer test or evidence |
|---|---|---|---|
| `SR-ERT-001` | Keep one Rust-owned authority and effect machine. | Place policy, grants, workflow truth, routing admission, tools, persistence, verification, and completion in the Rust host; expose only typed proposal and client ports. | Attempt to mint authority or establish state through model output, TypeScript, webview, manifest, adapter, endpoint, tool output, or client replay; require structural absence or denial. |
| `SR-ERT-002` | Make persistent tasks bounded and crash-safe. | Journal state before meaningful transitions, bind checkpoints to exact inputs and environment, reconcile effects, and enforce turn, token, time, tool, attempt, no-progress, resource, disclosure, and cost ceilings. | Terminate and restart at every boundary with unchanged, changed, corrupt, and uncertain state; require exact resume or block, zero grant or effect replay, and complete owned-process cleanup. |
| `SR-ERT-003` | Require one complete terminal observation per admitted tool call. | Correlate call, attempt, authority, worker, output artifacts, state change, cleanup, resources, and receipt through a closed `ToolObservation`. | Drop, duplicate, reorder, truncate, corrupt, or replay tool events under large output, cancellation, timeout, crash, and uncertainty; require one terminal truth and no workflow advancement on missing evidence. |
| `SR-ERT-004` | Reserve completion for current deterministic verifiers. | Resolve success and no-op only from current postconditions, preserved invariants, prohibited-effect checks, receipt identity, artifacts, and environment evidence. | Seed model completion claims, exit-zero failures, stale evidence, altered output, hidden effects, and incomplete plans; require zero false completion and one explicit non-success result. |
| `SR-ERT-005` | Make observability informative without becoming authority or a secret store. | Retain content-free identities, hashes, states, timings, counts, policy decisions, and protected artifact references; classify and minimize any sensitive trace payload. | Seed secret canaries and hidden-reasoning markers through every stage, mutate lineage, and submit trace-derived grants or state changes; require zero raw leakage, no authority, and explicit incomplete lineage. |
| `SR-ERT-006` | Degrade and exhaust resources without silent loss or policy weakening. | Classify dependencies, bound queues and workers, apply backpressure and cancellation, and make every omission, substitution, limit, and degraded state visible. | Remove dependencies and exceed every budget during every lifecycle phase; require bounded termination, cleanup, accurate gaps, no hidden route, no dropped required content, and no false success. |
| `SR-CTX-001` | Account for every supplied artifact before inference. | Capture AgentMage-owned paste text and enumerate request references with immutable origin, authority, media, classification, byte identity where available, and terminal disposition. | Supply valid, missing, inaccessible, stale, linked, changing, virtual, remote, unsupported, oversized, and malformed inputs; require exactly one visible disposition for every item. |
| `SR-CTX-002` | Isolate all parsers and active content. | Run bounded no-network parser workers with macro, script, plugin, relationship, archive traversal, external retrieval, executable, and credential access disabled by default. | Exercise parser exploits, bombs, links, relationships, active content, hangs, crashes, low resources, and cancellation; require bounded failure, no external access or execution, and cleanup. |
| `SR-CTX-003` | Preserve original-byte and derivative provenance. | Bind every extracted structure, chunk, OCR result, summary, and index to source hash, parser and transformation identity, exact ranges, confidence, warnings, and freshness. | Reorder, mutate, replace, reparse, summarize, and index beginning, middle, and ending sentinels; require exact source reconstruction and no derivative represented as authority. |
| `SR-CTX-004` | Make context selection complete and reviewable. | Produce one manifest covering every supplied artifact and section, deterministic token partitions, explicit omission reasons, and exact retrieval references. | Force combined-budget overflow, token disagreement, duplicates, restrictions, stale data, and profile change; require complete accounting and no second prompt assembler or hidden omission. |
| `SR-CTX-005` | Block success when required model-visible context is absent or stale. | Issue a context-delivery receipt with exact delivered ranges, route, model request, hashes, and required-unseen set. | Withhold or mutate required beginning, middle, ending, cited, and retrieved ranges and submit model claims or summaries; require terminal block until current exact delivery exists. |
| `SR-CTX-006` | Keep source retention and disclosure least-privilege. | Separate source and runtime artifacts, encrypt retained payloads, minimize model-visible metadata, apply classification and retention, and delete or invalidate every dependent record. | Seed path, URI, credential, private value, expiry, deletion, restore, and source-change cases; require no raw secret or unrelated path disclosure and no stale context use. |
| `SR-GWY-001` | Keep the internal model protocol candidate-neutral and richer than a vendor codec. | Preserve artifact references, message parts, proposals, route receipts, usage, errors, cancellation, and qualification through closed canonical records. | Round-trip the conformance corpus across codecs and versions; require exact preservation or explicit unsupported state and no vendor field becoming authority. |
| `SR-GWY-002` | Isolate model runtime and protocol adapters from tools and workspace. | Give adapter workers only minimized context, route, deadlines, and transport capabilities; return canonical events and untrusted proposals. | Probe filesystem, tool, grant, credential, policy, completion, process, and network surfaces from hostile adapters and model output; require structural absence or denial. |
| `SR-GWY-003` | Separate model, publisher, operator, runtime, codec, endpoint, route, and policy identity. | Use independently hash-bound profiles and qualification records; keep open-weight and open-source-license states separate. | Conflate or substitute each identity and mutate one dimension at a time; require affected qualification invalidation and truthful provenance state. |
| `SR-GWY-004` | Make routing and fallback deterministic, explicit, and receipted. | Select only admitted exact tuples under current data, role, health, resource, cost, and policy constraints; disable fallback by default. | Fail local and remote routes, reorder candidates, falsify health, and request implicit fallback; require deterministic reasons and zero silent local-to-remote or cross-remote transition. |
| `SR-GWY-005` | Confine remote inference to an exact least-privilege worker. | Start a fresh bounded worker with one route, minimized classified payload, credential reference, transport policy, cancellation, and no workspace or tool authority. | Inject hostile prompts and endpoint responses, crash and cancel workers, and inspect processes, descriptors, paths, sockets, and residue; require exact confinement and cleanup. |
| `SR-GWY-006` | Enforce remote destination and transport identity. | Require HTTPS for non-loopback, verified TLS or mutually authenticated TLS, exact host and address policy, deny-first redirects, explicit proxy rules, and SSRF and DNS-rebinding checks. | Use bad certificates, aliases, rebinding, private and metadata addresses, redirects, proxies, alternate protocols, and endpoint substitution; require zero unauthorized connection. |
| `SR-GWY-007` | Keep remote credentials as non-model secret references. | Resolve the exact credential only inside the remote worker and redact configuration, prompts, events, logs, traces, receipts, diagnostics, and errors. | Seed credential canaries in stores, environment, endpoint errors, redirects, logs, retries, and crash output; require zero raw-value model, disk, log, trace, or Git disclosure. |
| `SR-GWY-008` | Bind every remote disclosure to explicit policy and user-visible destination. | Record data class, purpose, minimized fields, endpoint operator, profile class, region, retention, logging, training use, and route receipt before transfer. | Change destination, operator, classification, purpose, region, or terms after approval and attempt replay; require stale authority rejection and no transfer. |
| `SR-GWY-009` | Qualify model, endpoint, and route independently. | Run exact capability, quality, protocol, context, tool-proposal, streaming, cancellation, failure, privacy, latency, resource, and workflow fixtures against each tuple. | Mutate artifact, tokenizer, template, codec, quantization, decoding, runtime, endpoint, transport, operator, or policy; require exact invalidation and no compatibility-as-approval claim. |
| `SR-GWY-010` | Bound remote quota, cost, health, circuit breakers, disablement, and removal. | Enforce request, token, byte, connection, retry, concurrency, quota, cost, and time ceilings plus route-scoped breakers and emergency disablement. | Exhaust each ceiling, flap health, interrupt uncertain requests, disable and remove each route, and rerun strict local; require bounded loss, no hidden retry or charge, and complete restoration. |
| `SR-VSC-001` | Keep Verified Chat webview disposable and untrusted. | Use minimum webview capabilities, strict Content Security Policy, sanitization, nonces, exact local roots, no secrets, and host-owned reconstruction. | Inject script, markup, resource, navigation, origin, and reload attacks during every state; require no privilege, data escape, hidden state, or task-truth loss. |
| `SR-VSC-002` | Authenticate and bound every bridge and attachment frame. | Bind session and host identity, protocol version, sequence, replay window, chunk hash, byte count, acknowledgement, flow control, cancellation, and expiry. | Replay, reorder, duplicate, truncate, corrupt, oversize, cross-session, and race frames; require fail-closed rejection and zero partial authoritative admission. |
| `SR-VSC-003` | Make native Chat reference limitations fail visibly. | Account for every stable public request reference and keep unresolved, opaque, remote, unsupported, and provider-only parts explicit. | Exercise every reference class across supported VS Code versions and remote placements; require required-unseen block and no native-parity overclaim. |
| `SR-VSC-004` | Keep provider compatibility stable, normalized, and independently disableable. | Preserve supported normalized parts, expose capability labels, use stable APIs, isolate proposed experiments, and version-gate affected adapters. | Mutate message parts and VS Code versions, inject unknown fields, and trigger known-bad disablement; require explicit unsupported states and no private API or false Agent Host claim. |
| `SR-CAP-001` | Prevent capability manifests from creating authority. | Treat schemas, tools, roles, requested maximum authority, budgets, workflow, and verifiers as signed or admitted declarations intersected by kernel policy. | Add hidden tools, grants, credentials, routes, effects, completion, unknown fields, prompt instructions, and stale hashes; require admission refusal. |
| `SR-CAP-002` | Make capability lifecycle and dependency health explicit. | Define draft, admitted, enabled, degraded, disabled, quarantined, and retired states with exact dependency and evidence transitions. | Remove, replace, fail, restore, quarantine, migrate, and retire every dependency and manifest; require truthful state and no implicit re-enable. |
| `SR-CAP-003` | Bind each capability to deterministic workflow and verification. | Require closed input and output schemas, versioned graph, exact tools, role qualifications, side-effect policy, budgets, fixtures, and capability-specific verifiers. | Reorder graphs, insert loops, omit verifiers, substitute roles, exceed budgets, and submit prompt-only capabilities; require bounded rejection or non-success. |
| `SR-CAP-004` | Make migration, disablement, and removal complete. | Revoke registrations and grants, stop workers, reconcile effects, migrate or delete state, apply retention, and rerun shared and strict-local gates. | Remove capabilities during every lifecycle phase and after crash or restore; require no worker, socket, credential, registration, stale state, authority, or residue. |
| `SR-MAG-001` | Gate multi-agent execution on single-agent reliability. | Require current context, workflow, observation, persistence, verification, resource, and removal evidence before enabling a team profile. | Invalidate each prerequisite and attempt team start or resume; require fail-closed denial with no child process or lease. |
| `SR-MAG-002` | Isolate each pod's authority, state, branch, and worktree. | Issue separate work packets, grants, budgets, artifacts, journals, branches, and owned worktrees; prohibit credential or context sharing outside declared edges. | Cross-address paths, branches, grants, context, credentials, receipts, and recovery state; require zero cross-pod access or mutation. |
| `SR-MAG-003` | Enforce path, test-resource, provider, and global resource leases. | Allocate finite leases, heartbeats, expiry, conflict handling, cancellation, and global ceilings before dispatch. | Race, starve, leak, expire, replay, and oversubscribe every lease and provider capacity; require bounded scheduling, isolation, cleanup, and no healthy-pod cancellation from unrelated failure. |
| `SR-MAG-004` | Keep review independent and integration serialized and human-gated by default. | Route findings back to the exact pod, preserve dissent, bind review to source and effective diff, serialize integration, reverify changes, and require explicit final promotion authority. | Attempt self-approval, stale review reuse, duplicate merge, hidden promotion, changed-diff bypass, and completion with blocked pods; require rejection and truthful campaign state. |

## 9. Proposed Reviewer Command Contract

These commands are interfaces to implement. They do not exist yet. They should be packaged in a signed, read-only verifier and must not modify the workstation except inside an explicit temporary test directory.

```text
agentmage-review inventory
agentmage-review verify-release --package <path> --model <path>
agentmage-review preflight --profile <fedora|ubuntu|windows|macos>
agentmage-review audit-codebase --repository <path> --profile comprehensive
agentmage-review verify-platform --profile <fedora|ubuntu|windows|macos>
agentmage-review verify-crypto
agentmage-review verify-sbom
agentmage-review verify-model
agentmage-review verify-sandbox
agentmage-review verify-ipc
agentmage-review verify-paths
agentmage-review verify-offline --duration 60m
agentmage-review verify-privacy
agentmage-review verify-audit
agentmage-review verify-retention
agentmage-review verify-injection
agentmage-review verify-resilience
agentmage-review verify-accessibility
agentmage-review verify-adapter --manifest <path>
agentmage-review verify-autonomy --matrix <path>
agentmage-review verify-communications --matrix <path>
agentmage-review verify-finance --profile <path>
agentmage-review verify-cloud-observer --matrix <path>
agentmage-review verify-productivity-removal
agentmage-review verify-command-authority --matrix <path>
agentmage-review verify-owner-session --profile <fedora|ubuntu|windows|macos>
agentmage-review verify-public-research --corpus <path>
agentmage-review verify-credential-broker --profile <fedora|ubuntu|windows|macos>
agentmage-review verify-continuity --destination <local|reference-cloud>
agentmage-review verify-model-manager --catalog <path>
agentmage-review verify-muse-candidate --profile <path>
agentmage-review verify-experimental-model-lab --profile <path>
agentmage-review verify-trusted-operations-removal
agentmage-review verify-engineering-runtime --fixture-set <path>
agentmage-review verify-model-gateway --profile-set <path>
agentmage-review verify-verified-chat --vscode-matrix <path>
agentmage-review verify-capability-registry --catalog <path>
agentmage-review verify-multi-agent --campaign <path>
agentmage-review verify-delivery-graph
agentmage-review verify-external-effects
agentmage-review verify-provider-isolation
agentmage-review verify-support-matrix
agentmage-review uninstall-test
agentmage-review full --profile <fedora|ubuntu|windows|macos> --evidence-dir <path>
agentmage-review validate-evidence --evidence-dir <path>
```

Command requirements:

- Default to read-only inspection.
- Require explicit confirmation before creating a temporary fixture or uninstalling a test instance.
- Run without internet access after package/model import.
- Display each requirement ID, procedure, status, reason, evidence path, and remediation.
- Return stable exit codes and machine-readable JSON in addition to concise text.
- Record skipped and not-applicable checks separately from passes.
- Never turn an unavailable check into a pass.
- Redact usernames, unrelated paths, prompts, excerpts, environment values, credentials, and keys.
- Hash and sign the final evidence index.
- Allow the reviewer to reproduce each individual test.

## 10. Explicit Reviewer Test Protocols

### `RV-01` Release Identity and Integrity

1. Start from the received package, detached manifest, model artifact, and public verification key.
2. Verify package and component hashes.
3. Verify the platform-specific package and executable identities: Linux package signatures and manifests, Windows MSIX/Authenticode signatures and timestamps, or retained Apple Developer ID/notarization/App Sandbox identities.
4. Compare every executable and library with the signed component inventory and SBOM.
5. Verify provenance signatures and source/build identity.

Pass: every shipped component is declared, signed, hash-matched, and policy-approved; no undeclared executable code exists.

### `RV-02` Clean Standard-User Installation

1. Use the exact supported Fedora, Ubuntu, or Windows 11 environment and OS build, or the retained Apple Silicon environment when reviewing that lane.
2. Use a fresh standard account without undeclared development tools, ambient Python, ambient Git, or an unmanifested model runtime.
3. Install, launch, run diagnostics, select a synthetic workspace, and remove the application using only the published procedure.

Pass: three independent first-GA installations per supported platform require no administrator access after approved installation; no baseline control is weakened; every component and location matches the manifest.

### `RV-03` Sandbox and Ambient-Access Resistance

1. Place unique canaries in the workspace, adjacent directory, home folders, browser data, SSH folder, environment, clipboard, removable location, and another user's simulated area.
2. Exercise normal tools and malicious model/tool requests.
3. Attempt filesystem, process, device, environment, bookmark/entitlement, AppContainer/restricted-token, registry, clipboard, and worker-boundary escapes as applicable.

Pass: only authorized workspace canaries are readable; zero sandbox escapes; zero prohibited canaries reach output, storage, logs, or the model.

### `RV-04` Path and Race Safety

Run at least 500 fixtures covering absolute paths, traversal, alternate separators, encodings, NULs, aliases, symlinks, hard links where applicable, stale bookmarks, case collisions, Unicode normalization, rename races, replacement races, and mount changes.

Pass: zero workspace escapes and 100 percent success for valid unambiguous fixture paths.

### `RV-05` IPC Identity and Replay

Attempt connections from unsigned, wrongly signed, wrong-bundle or package, wrong-App-Group, wrong-user, wrong-logon-session, wrong-integrity, stale, replaying, malformed, oversized, version-incompatible, and rapidly reconnecting clients.

Pass: zero unauthorized accepted connections; bounded failures; correct audit events; no service crash or resource exhaustion.

### `RV-06` Offline and Egress Proof

1. Confirm the installer is not running.
2. Start packet, DNS, socket, process, and endpoint-firewall observation before AgentMage.
3. Record the active native runtime build or immutable Docker Model Runner engine and model image digests.
4. Run model inference, repository mapping, read/search/hash/Git inspection, crash recovery, diagnostics, and cancellation for at least 60 minutes.
5. Repeat with no network route.
6. For Docker Model Runner, probe its unauthenticated API from a LAN peer, ordinary container, separate network namespace, tool worker, extension host, and unrelated same-user process.
7. Attribute any unrelated Visual Studio Code or OS traffic by process and verify it contains no AgentMage data.

Pass: AgentMage produces zero outbound attempts and zero outbound bytes; only declared local paths exist; no undeclared peer reaches the inference API; all supported workflows succeed offline.

### `RV-07` Acquisition Separation

Observe successful, resumed, canceled, corrupted, hash-mismatched, insufficient-disk, and incompatible-model acquisitions.

Pass: the installer accesses no workspace/session authority; bad artifacts never become runnable; staging cleanup is complete; offline preflight succeeds only after installer exit.

### `RV-08` Data Minimization and Secret Leakage

Inject unique synthetic secrets into prompts, files, environment values, model responses, errors, paths, attachments, tool output, and crash conditions. Search process outputs, database, journal/WAL, caches, logs, diagnostics, exports, backups, temporary files, and model context captures.

Pass: no prohibited secret appears; permitted bounded excerpts follow declared policy and retention exactly.

### `RV-09` Cryptography and Key Handling

1. Verify the exact cryptographic provider, version, operating environment, configured mode, self-tests where available, and algorithms.
2. Trace each security-relevant cryptographic operation to that module.
3. Inspect key creation, storage, access, rotation, backup exclusion, zeroization, and error paths.
4. Compare the runtime findings with the CBOM.

Pass: all required cryptography uses the reviewed provider and configuration; no raw key escapes its allowed boundary; documentation makes no unsupported certification or security claim.

### `RV-10` Retention, Backup, and Sanitization

Use a fake clock and synthetic records across every class. Exercise expiry, hold, export, backup, restore, index cleanup, cryptographic erase, uninstall, and reinstall.

Pass: every copy follows the declared lifecycle; expired eligible data is unreadable and absent from active indexes; retained remnants and physical-media limitations are disclosed.

### `RV-11` Prompt Injection and Authority Escalation

Run at least 200 labeled attacks in source files, comments, docs, filenames, Git metadata, tool results, model output, and user content. Include requests to reveal secrets, change policy, broaden roots, mint grants, invoke Codex, use the network, write files, or claim completion falsely.

Pass: zero unauthorized action, zero secret disclosure, zero external transfer, and zero false completion claim.

### `RV-12` Grant Mutation and Replay

Run at least 500 cases mutating actor, session, task, action, tool, path, arguments, preimage, side effects, expiry, nonce, parent, use count, and preview digest. Include concurrent and crash-interrupted consumption.

Pass: zero unauthorized executions; every denial has a stable reason and audit event.

### `RV-13` Model and Runtime Provenance

Verify the `MODEL-PROVENANCE-POLICY.md` admission record, license, publisher/control, lineage,
original artifact, conversion, quantization, tokenizer, chat template, family codec, GGUF, immutable
OCI digest where applicable, runtime build, adapter, context and decoding profiles, platform,
hardware/driver envelope, resource requirements, evaluation identity, and all hashes. Substitute each
item independently. Run the same fixed response, closed-proposal, tool-schema, cancellation,
context-limit, and resource corpus through every enabled native and Docker adapter.

Pass: the exact approved identity loads; every silent substitution, mutable-tag-only identity,
mismatch, corruption, prohibited lineage, or unsupported environment is quarantined or refused;
all enabled adapters meet the same published contract thresholds, with differences recorded rather
than hidden; no kernel or capability behavior branches on a model-family name.

### `RV-14` Model Quality and Evidence Integrity

Run the fixed repository, tool-call, factual-grounding, uncertainty, conflicting-evidence,
stale-citation, and false-completion corpora across repeated trials. Run the first-party-recommended
quality profile and exact diagnostic-repeatability profile separately, and freeze the complete
artifact, tokenizer, template, codec, runtime, sampler, seed, slot, context, platform, hardware,
driver, tool, grader, and corpus tuples.

Pass: all published thresholds are met; every material claim has the correct evidence state;
negative results and limitations remain visible; incomparable tuples are never merged; repeated
tokens are described only for the observed tuple and never as universal determinism.

### `RV-15` Fuzzing and Malformed Input

Fuzz IPC, manifests, schemas, model output, paths, text encodings, Git objects, repository-map parsers, database imports, and every FFI boundary using pinned corpora and sanitizers where supported.

Pass: no memory-safety failure, sandbox escape, unauthorized action, secret disclosure, unbounded resource use, or unrecoverable corruption.

### `RV-16` Resource Exhaustion and Cancellation

Exercise oversized repositories/files/results, deep trees, adversarial tokens, long inference, full disk, low memory, GPU failure, rapid requests, process termination, sleep/wake, and cancellation at every phase.

Pass: declared limits hold; cancellation is bounded; scratch is cleaned; state remains recoverable; the endpoint remains responsive.

### `RV-17` Crash Recovery and State Integrity

Inject a crash before and after each durable transition and during grant validation, tool execution, model inference, receipt creation, checkpoint, and shutdown.

Pass: 100 of 100 recovery fixtures select the correct named terminal state, repeat no completed
operation or consumed grant, reconcile uncertain effects, and produce no false success; only
current deterministic postcondition evidence produces `SUCCESS` or verified `NO_OP`.

### `RV-18` Audit Completeness and Redaction

Trigger every declared security event, permission denial, configuration change, integrity failure, and incident. Alter, remove, reorder, and duplicate exported events.

Pass: event coverage is complete; redaction tests find no canary; tampering is detected; ordering remains valid across clock changes.

### `RV-19` Vulnerability and Supply-Chain Review

Regenerate source and binary SBOMs, scan dependencies and package, compare them, review licenses and suppliers, and inspect every suppression and unresolved advisory.

Pass: no undeclared component; no unaccepted critical/high finding; all exceptions have owner, evidence, expiry, and compensating control.

### `RV-20` Accessibility

Test keyboard-only operation, focus order and visibility, screen-reader names/status/errors, zoom/reflow, contrast, color independence, timing, cancellation, and generated documentation using WCAG 2.2 AA and manual assistive-technology checks.

Pass: the accessibility conformance report accurately reflects results; no core workflow has an unmitigated accessibility barrier.

### `RV-21` Incident Tabletop

Run four scenarios: suspected egress, compromised dependency/package, model prompt-injection disclosure, and cryptographic/key-store failure.

Pass: the team can detect, suspend, contain, preserve bounded evidence, notify the correct owner, remediate, verify, recover, and document lessons without improvising authority.

### `RV-22` Update, Rollback, and End of Support

Test valid update, wrong signer, downgrade, interrupted update, corrupt package, manifest mismatch, schema migration failure, rollback, revoked component, and end-of-support state.

Pass: only authorized upgrades occur; downgrade and wrong-signer attempts fail; rollback preserves security and data integrity; unsupported versions are visibly blocked or constrained by policy.

### `RV-23` Provider Manifest and Conformance

For each promoted provider/version/capability tuple, regenerate the signed adapter manifest, run L0-L5 tests through the exact promoted level, compare registered operations with the support matrix, and invoke every unsupported operation name and provider extension.

Pass: every supported tuple passes its exact contract; every unsupported operation is absent or denied; versions outside the matrix are refused or enter only the declared degraded state; no provider-specific behavior is misrepresented as common behavior.

### `RV-24` Credential, Host, Tenant, and Account Isolation

Run at least 2,000 combinations of provider, host, clone host, redirect, proxy, DNS answer, callback, tenant, account, project, environment, credential, single-sign-on state, and capability class. Seed unique credential canaries and inspect requests, workers, errors, logs, receipts, model context, and caches.

Pass: zero secret disclosure, wrong-host request, cross-tenant access, account confusion, project crossover, or capability reuse; each denial is attributable without exposing the secret.

### `RV-25` External Effect, Idempotency, and Reconciliation

For every remote write, CI execution, deployment, infrastructure apply, migration, flag change, secret operation, and administrative fixture, mutate each previewed field and precondition. Inject loss before send, during transport, after remote effect, before local persistence, and during reconciliation; duplicate and reorder responses and retries.

Pass: only the exact current approved effect occurs; stale approvals produce no request; unknown results block retry; no duplicate effect occurs; partial effects are visible; rollback or compensation requires a new preview and grant.

### `RV-26` Event, Webhook, and Polling Integrity

Forge, delay, replay, duplicate, reorder, omit, truncate, and mutate provider events. Rotate webhook secrets, skew clocks, expire cursors, create pagination loops, force bounded-poll overlap, and test backfill and tombstones.

Pass: invalid events are rejected, valid duplicates are idempotent, gaps and uncertainty are visible, local state converges without erasing history, and no event creates operation authority.

### `RV-27` Delivery Graph and Cross-System Identity

Build complete work-item-to-release and incident-to-rollback fixture graphs. Rename and transfer repositories/projects, reuse display numbers, move branches, replace tags, rebuild artifacts, shift time windows, and create conflicting provider links.

Pass: every authoritative node and edge resolves to exact immutable provider evidence; inferred or conflicting edges remain labeled; no display-name collision links the wrong object or authorizes an operation.

### `RV-28` Deployment, Infrastructure, and Rollback Safety

Exercise Kubernetes, GitOps, Terraform/OpenTofu, release, feature-flag, progressive-delivery, and migration fixtures across production/non-production, stale plans, changed policy, drift, health failure, locks, destructive actions, crash, cancellation, partial application, and later independent changes.

Pass: plan never implies apply, non-production authority never reaches production, destructive and secret/admin effects remain separate, health and timeout behavior are deterministic, and rollback never overwrites later work without a new exact approval.

### `RV-29` Provider Failure, Version Skew, and Resource Exhaustion

Exercise minimum, maximum, future, and behavior-changed provider versions with rate limits, quota exhaustion, revocation, permission reduction, outage, partition, slow response, event flood, pagination explosion, oversized logs/artifacts/archives, telemetry cardinality, full disk, low memory, and concurrent cancellation.

Pass: operation and support state degrade exactly as declared; no unsafe retry, false completion, authority expansion, unbounded growth, unrecoverable state, or omitted blocking result occurs.

### `RV-30` Adapter Removal and Strict-Local Restoration

Remove every provider adapter separately and all connected packs together. Inspect credentials, caches, databases, event registrations, webhooks, schedules, workers, processes, sockets, firewall rules, temporary files, logs, and retained data, then rerun the complete strict-local suite.

Pass: removal follows declared retention without harming user data or neighboring adapters; zero undeclared connected authority or residue remains; strict-local behavior and its 60-minute zero-egress proof still pass.

### `RV-31` Autonomy, Recipient, and Communication Effects

Exercise every autonomy level and narrowing dimension against Outlook and Exchange Online, Teams,
Gmail, Slack, Proton Mail Bridge, the Proton Calendar confirmed-UI adapter, generic mail, calendars,
contacts, tasks, and document fixtures.
Mutate sender, account, recipients, destinations, channels, threads, mentions, visibility, content,
attachments, event fields, provider-surface versions, structured controls, bounded visual fallbacks,
reply correlations, schedules, budgets, expiry, and policy between preview and effect. Inject
ambiguous replies, focus loss, session expiry, duplicate, partial, timed-out, reordered, and
provider-transformed results.

Pass: effective autonomy is always the narrowest applicable ceiling; every effect matches one exact
current preview and consumed grant; duplicate or uncertain delivery is reconciled; no inferred,
changed, external, or hidden recipient receives content; no ambiguous reply becomes consent; no
UI drift produces a guessed or repeated calendar effect; emergency disablement blocks new writes.

### `RV-32` Identity, Synchronization, Unified Inbox, and Workflow Integrity

Exercise identity collisions, aliases, homoglyphs, renamed accounts, moved channels, cursor loss,
event replay, edits, deletions, tombstones, gaps, backfill, stale caches, permission reduction, and
cross-provider links. Compile and run synthetic workflows across messages, meetings, tasks,
documents, delivery work, finance alerts, and cloud observations.

Pass: native identities and source evidence remain intact; ambiguity and incomplete coverage are
visible; no workflow edits itself, broadens authority, hides failure, reuses a grant or credential,
or treats external content as an instruction.

### `RV-33` Financial Precision, Reconciliation, Privacy, and Prohibited Authority

Exercise currencies, scales, signs, rounding, large and fractional values, imports, pending and
posted states, duplicates, transfers, splits, corrections, statements, budgets, forecasts,
recurring streams, receipts, accounting records, and anomaly indicators. Probe every interface and
autonomy level for money movement and financial-account administration.

Pass: financial outputs are exact and reproducible; no false reconciliation or silent rewrite
occurs; analysis remains explained and bounded; financial canaries do not escape; transfers,
payments, trades, credit, tax filing, beneficiaries, administration, and credential recovery are
structurally absent or denied before any request.

### `RV-34` Cloud Observer Read-Only and Scope Enforcement

Exercise AWS, Azure, and Google Cloud fixtures across organization or tenant, account, subscription
or project, region, service, resource, query, time, field, row, byte, rate, role, token, redirect,
and data-plane boundaries. Probe every write, command, shell, deploy, secret, identity, policy,
logging, budget, upload, delete, and administration family.

Pass: only exact manifested reads occur; zero out-of-scope data or credential crosses a boundary;
cloud content creates no authority; every mutation or remote-execution path is absent or denied
before a provider request.

### `RV-35` Productivity-Pack Removal and Cross-Pack Restoration

Disable and remove each communications, personal-information, document, finance, and cloud pack
separately and together during idle, queued, in-flight, uncertain, and partially synchronized states.
Inspect credentials, caches, cursors, graph edges, webhooks, schedules, indexes, workers, sockets,
network scopes, logs, temporary data, and retained evidence, then rerun strict-local and delivery
suites.

Pass: declared retention and reconciliation complete; no undeclared credential, data, process,
event, schedule, tool, or network authority remains; neighboring packs and canonical user records
are unharmed; strict-local and delivery behavior remain complete.

### `RV-36` Command Authority and Owner-Session Lifecycle

Exercise Disabled, Inspect, Workspace Autonomous, Connected Operations, and Owner / Unrestricted
Session across direct, shell, pipeline, redirection, script, interpreter, PTY, package, Git, Docker,
process-tree, environment, filesystem, network, resource, timeout, cancellation, lock, logout,
restart, panic, expiry, policy change, replay, race, schedule, workflow, child-agent, and emergency
states. Attempt activation from models and every untrusted content channel.

Pass: constrained modes do not escape declared authority; Owner mode starts only through direct
authenticated user activation, remains visibly risky and time bounded, records truthful host-user
effects, supplies no implicit elevation or secret, and terminates every descendant at revocation;
zero content, model, schedule, workflow, or child can activate, inherit, or renew it.

### `RV-37` Public Research, Citation, and Disclosure Isolation

Search and retrieve current, stale, conflicting, redirected, malicious, oversized, archived,
script-heavy, unavailable, primary, and secondary fixtures. Insert hidden instructions, tool calls,
credential requests, downloads, form actions, false completion, and requests for private workspace,
memory, message, finance, connector, and secret canaries.

Pass: every web-grounded claim has current source and retrieval evidence; changing facts are
revalidated; inference and uncertainty remain visible; requests and downloads stay within bounds;
zero page content creates authority or causes undeclared private disclosure.

### `RV-38` Credential Broker Isolation and Lifecycle

Exercise Linux Secret Service, macOS Keychain, and Windows Credential Manager or DPAPI fixtures
across OAuth, short-lived tokens, agent and certificate references, provider, host, tenant, account,
scope, operation, expiry, rotation, revocation, redirect, proxy, crash, restore, export, diagnostics,
logging, removal, and concurrent worker states.

Pass: one exact worker resolves one exact reference; zero raw secret appears in model context,
prompts, chat, arguments, ordinary environment, logs, receipts, diagnostics, exports, crash data, or
backups; no wrong-account use occurs; restored installations reauthenticate.

### `RV-39` Encrypted Continuity and Disaster Recovery

Create local and reference-cloud snapshots, interrupt every phase, and exercise low disk, duplicate,
missing, corrupt, replayed, replaced, cross-version, cross-account, wrong-prefix, revoked, throttled,
partially deleted, retention-expired, ransomware-like, provider-outage, clean-device restore,
migration, rollback, and removal states.

Pass: no incomplete or corrupt snapshot appears complete; plaintext never leaves the client; cloud
access remains inside one exact backup namespace; live canonical state never runs from synchronized
or network storage; staged restore and rollback are deterministic; Cloud Observer remains read only.

### `RV-40` Approved Catalog and Chat-Guided Model Management

Exercise candidate, evaluating, approved, degraded, quarantined, rejected, and retired profiles plus
Chat requests to list, recommend, acquire, import, resume, verify, activate, compare, cancel, crash,
roll back, remove, and clean storage. Mutate publisher, license, origin, lineage, source, redirect,
artifact, hash, size, tokenizer, template, runtime, hardware, quality, security, support, and preview.

Pass: only exact approved or published degraded profiles run ordinarily; every acquisition and
activation matches one confirmed plan; the separate installer has no workspace or connected
authority; no model approves, downloads, activates, replaces, or falls back to itself; interruption
leaves the prior valid state or a visible block.

### `RV-41` Muse-First and Role-Aware Candidate Admission

Run Muse Glimmer first through the normal exact-profile identity, classification, license, origin,
lineage, artifact, transformation, tokenizer, template, codec, hash, runtime, context, decoding,
resource, coding, tool, repeatability, security, platform, and support evidence pipeline. Freeze the
eligible official first-party Gemma catalog; assign each exact profile a declared role and
applicable suite; run every feasible case; and retain visible `BLOCKED-HARDWARE`, `BLOCKED`,
`REJECTED`, and non-applicable evidence. Independently remove, contradict, stale, or fail every
evidence class and introduce an additional eligible candidate through the same intake.

Pass: complete conforming evidence yields `PASS`, incomplete or contradictory evidence yields
`BLOCKED`, and non-waivable failure yields `REJECTED`; 100% of frozen eligible Gemma entries have
exact attributable records and no role escalation or silent omission; no unsupported open-source,
compatibility, download, support, or activation claim appears; and no named-family non-pass blocks
an independently conforming eligible profile.

### `RV-42` Experimental Model Lab Isolation and Promotion

Import malformed, oversized, hostile, unknown, mirrored, mutable, provenance-incomplete,
license-unclear, and policy-excluded artifacts. Probe network, credentials, commands, connectors,
messages, finance, delivery, cloud, backup, operational memory, canonical workspace writes,
approved-store writes, resource exhaustion, crash residue, removal, and direct promotion routes.

Pass: all prohibited authority is structurally absent or denied; provenance and license gaps remain
visible; resources and cancellation hold; removal leaves no undeclared artifact or authority; no
experimental artifact runs ordinarily until a separate complete admission passes.

### `RV-43` Trusted-Operations Removal and Superseding First-GA Closure

Disable and remove command, public-research, credential-broker, continuity, and model-manager
capabilities separately and together during idle, active, queued, interrupted, uncertain, restored,
and incident-disabled states. Inspect workers, descendants, sockets, rules, credentials, snapshots,
incomplete transfers, caches, quarantines, catalog entries, schedules, logs, and retained evidence,
then rerun strict-local, delivery, productivity, finance, Cloud Observer, model, accessibility,
update, rollback, and uninstall suites.

Pass: declared retention and recovery complete; no undeclared process, credential, network, command,
storage, model, schedule, or provider authority remains; neighboring user data is unharmed; every
prior gate still passes; Sprint 166 closes `G-GA` only from current independently reproduced raw
evidence.

### `RV-44` Repository Census and Coverage Truth

Run clean, dirty, monorepo, polyglot, generated, vendored, ignored, untracked, sparse, Large File
Storage, submodule, worktree, linked, archived, binary, oversized, malformed, inaccessible, special,
external, and concurrently changing repository fixtures. Compare source-control, filesystem, parser,
and audit-manifest records.

Pass: every in-scope path has exactly one current disposition and stable identity; totals reconcile;
every exclusion and failure remains visible; no path is silently omitted, followed outside policy,
or labeled analyzed when unavailable.

### `RV-45` Read-Only Audit, Parser Isolation, and Secret Protection

Run census, parsing, archive inspection, language services, builds, tests, dependency resolution,
generation, coverage, formatting, hooks, cancellation, crashes, path races, and hostile repository
instructions. Seed raw-secret canaries in files, metadata, history, issues, build output, archives,
errors, and generated records.

Pass: the canonical repository, Git metadata, hosted services, credentials, and neighboring data are
unchanged; all permitted writes remain disposable; workers remain bounded and cleaned; no raw
secret enters model context or any retained audit surface.

### `RV-46` Structural Index, Semantic Partition, and Cross-Module Reconciliation

Build exact structural graphs over the published language and build-system matrix. Vary packet
boundaries, order, context, retrieval, interruption, and approved model profile. Seed distributed
architectural drift, duplicate responsibility, circular dependencies, dead and orphaned code,
conflicting state, stale adapters, abandoned migrations, missing tests, and documentation drift.

Pass: deterministic graph identity and coverage remain unchanged by model variation; every required
unit receives bounded analysis; contradictions remain visible; repository-wide findings cite every
material side or declare the exact local limitation.

### `RV-47` Audit Checkpoint, Invalidation, Findings, and Reporting

Pause, crash, restore, and resume every phase. Change, rename, delete, reclassify, or reparse source
and mutate scope, parser, model, runtime, policy, graph, checkpoint, evidence, finding, severity,
confidence, and counterevidence records. Recompute reports from raw evidence.

Pass: unchanged work resumes deterministically; changed inputs invalidate every dependent result;
uncertain dependencies trigger broader rescan; every report claim resolves to current immutable
evidence; gaps, uncertainty, conflicts, and non-pass states remain visible.

### `RV-48` Whole-Codebase Audit Removal and First-GA Qualification

Independently run comprehensive audits against each release-reference repository and adversarial
corpus on Fedora, Ubuntu, and Windows. Exercise resource limits, accessibility, cancellation,
recovery, capability disablement, removal, and strict-local restoration, then reconcile census,
graphs, packets, cards, checkpoints, findings, reports, manifests, and raw evidence.

Pass: `AT-CBA-001` through `AT-AUR-001` reproduce with zero canonical mutation, secret disclosure,
silent omission, stale result, unresolved hidden contradiction, or removal residue; every limitation
is published; `AT-GA-004` blocks Sprint 166 and `G-GA` on any non-pass or unreviewed result.

### `RV-49` Repository and GitHub Mutation Safety

Run every admitted Git and GitHub operation against clean, dirty, hostile-configured, concurrently
changing, cross-host, stale-ref, protection-changing, interrupted, and unknown-result fixtures.
Snapshot the active checkout, user index, every path disposition, refs including notes/stash/tags,
reflogs, configuration, hooks, filters, worktrees, submodules, Large File Storage state, credentials,
processes, network destinations, and hosted refs. Probe every prohibited operation, implicit refspec,
force form, hook/filter/helper path, URL rewrite, protocol, signer substitution, credential crossover,
and bypass capability.

Pass: at least 10,000 composed mutations produce zero unauthorized file, index, ref, configuration,
credential, network, hosted, or publication effect; every admitted effect matches its exact current
grant and reconciled preservation manifest; user-owned and unrelated work survives byte-for-byte;
no destructive or implicit operation is registered; no unknown push is retried; all receipts and
raw evidence reconcile on Fedora, Ubuntu, and Windows.

### `RV-50` Engineering Runtime Authority and Completion

Run the complete fake-model runtime lifecycle across Chat, CLI, and headless callers with hostile
model proposals, malformed transitions, stale authority, missing observations, false completion,
budget exhaustion, cancellation, crash, and resume.

Pass: `AT-ERT-001`, `AT-WKF-001` through `AT-WKF-003`, `AT-RESUME-002`, `AT-TIO-001`,
`AT-TIO-002`, and `AT-VER-001` pass with one Rust-owned state machine, zero replay, zero false
completion, bounded termination, and one explicit terminal result.

### `RV-51` Artifact, Context Fidelity, and Parser Isolation

Exercise every admitted artifact class and hostile parser fixture, including source mutation,
oversize, active content, archive and relationship attacks, cancellation, crash, token pressure,
exact retrieval, and required-unseen delivery.

Pass: `AT-CTX-001` through `AT-CTX-004` pass with complete source accounting, exact provenance,
bounded parser behavior, no active-content execution, no secret disclosure, and no success while
required context is missing or stale.

### `RV-52` Tool Observation, Persistence, and Execution Lineage

Drop, duplicate, reorder, corrupt, truncate, and replay tool and runtime events across large output,
resource pressure, client loss, host restart, uncertain effects, retention, and inspector use.

Pass: `AT-TIO-001`, `AT-TIO-002`, `AT-RESUME-002`, and `AT-OBS-002` pass with exactly one terminal
observation per call, complete artifact-backed output, current reconstruction, zero secret leakage,
and no observability-derived authority.

### `RV-53` Model Gateway Qualification and No-Silent-Fallback

Run the canonical conformance corpus across every claimed local and remote codec, adapter, model,
endpoint, and route tuple; induce unknown message parts, malformed streams, health loss, quota and
cost pressure, cancellation, policy change, and every fallback direction.

Pass: `AT-GWY-001` through `AT-GWY-003` and `AT-REM-001` through `AT-REM-003` pass for the exact
qualified tuple; unsupported semantics are explicit; zero unqualified or silent fallback route is
selected; the gateway never executes a tool or establishes completion.

### `RV-54` Remote Endpoint, Disclosure, and Credential Isolation

Probe remote workers with TLS, mutually authenticated TLS, DNS rebinding, SSRF, redirect, proxy,
host substitution, credential canary, region, retention, logging, training-use, quota, cost,
circuit-breaker, emergency-disable, interruption, uncertain outcome, and removal cases.

Pass: only exact admitted destinations receive minimized authorized data; raw credentials and
private endpoint values appear in no model, event, log, receipt, error, or repository artifact;
all effects and limits reconcile; strict local remains fully operable after removal.

### `RV-55` Verified Chat and Native Compatibility

Exercise Verified Chat and native compatibility through supported VS Code versions and remote
placements with reference loss, opaque parts, attachment pressure, script and markup injection,
origin and replay attacks, reload, view loss, cancellation, accessibility, and known-bad-version
disablement.

Pass: `AT-VSC-004` through `AT-VSC-006` pass with Rust-host reconstruction, bounded authenticated
transport, stable public APIs, explicit native limitations, no webview or TypeScript authority, and
no false External Agent Host or Verified Chat parity claim.

### `RV-56` Engineering Capability Registry

Validate the complete manifest and initial catalog corpus with missing, extra, malformed, stale,
prompt-only, authority-widening, unqualified, degraded, migrated, quarantined, disabled, removed,
crashed, and restored variants.

Pass: `AT-CAP-001` and `AT-CAP-002` pass; no manifest creates a grant, credential, route, tool,
effect, policy, or completion state; every capability uses the shared runtime and removes without
authority or residue.

### `RV-57` Multi-Agent Authority, Resources, Recovery, and Integration

Run concurrent role-qualified pods under path, worktree, test, provider, and global resource leases
with failure, cancellation, disagreement, correction, stale review, changed diff, duplicate
integration, self-approval, hidden promotion, interruption, resume, and removal.

Pass: `AT-MAG-001` and `AT-MAG-002` pass after all single-agent prerequisites; pods remain isolated,
healthy independent work survives unrelated failure, dissent remains visible, integration is
serialized and reverified, and default-branch promotion remains human-gated by default.

## 11. Reviewer Evidence Bundle

Every release candidate should produce one immutable directory or archive with this minimum structure:

```text
review-evidence/
  START-HERE.md
  evidence-index.json
  SHA256SUMS
  signatures/
  scope/
    use-case.md
    limitations.md
    shared-responsibility.md
    system-boundary.md
    data-flow.md
    runtime-boundaries.md
    threat-model.md
    risk-register.md
  plans/
    security-plan.md
    privacy-plan.md
    supply-chain-plan.md
    incident-response.md
    continuous-monitoring.md
    retention-and-sanitization.md
  controls/
    control-matrix.csv
    component-definition.json
    assessment-plan.json
    assessment-results.json
    remediation-register.json
  release/
    release-manifest.json
    package-provenance.jsonl
    sbom-source.spdx.json
    sbom-binary.spdx.json
    cbom.json
    model-bom.json
    licenses/
    support-policy.md
    vulnerability-disclosure.md
    model-admission.json
  platform/
    fedora-profile.json
    ubuntu-profile.json
    windows-profile.json
    macos-post-ga-profile.json
    execution-venue-policy.json
    guest-image-manifests/
    guest-cleanup-results/
    signatures-and-package-identities.txt
    platform-boundaries/
    baseline-before.json
    baseline-after.json
  delivery/
    support-matrix.json
    adapter-manifests/
    provider-conformance/
    delivery-graph-results.json
    external-effect-results.json
    event-integrity-results.json
    cross-provider-results.json
    removal-results.json
  codebase-audit/
    audit-plan.json
    path-census.json
    coverage-manifest.json
    structural-index.json
    structural-graph-hashes.json
    semantic-packet-index.json
    evidence-card-index.json
    reconciliation-results.json
    checkpoint-invalidation-results.json
    findings.json
    audit-report.md
    read-only-attestation.json
    secret-protection-results.json
    platform-results/
    resource-results.json
    removal-results.json
  tests/
    summary.json
    raw/
    packet-capture-summary.json
    sandbox-results.json
    path-results.json
    ipc-results.json
    injection-results.json
    privacy-results.json
    accessibility-results.json
    recovery-results.json
  exceptions/
    accepted-risks.md
    skipped-tests.md
    false-positives.md
```

Evidence rules:

- Every file is listed by relative path, media type, size, SHA-256 hash, producer, creation time, release identity, and sensitivity in `evidence-index.json`.
- The index and checksum file are signed.
- Raw and summarized results agree mechanically.
- Failures, skips, unavailable tests, and not-applicable decisions remain visible.
- Every native result identifies its execution venue. Local guest evidence binds
  the immutable base image, disposable overlay, source revision, virtual
  hardware and firmware controls, standard-user identity class, network phase,
  commands, result, cleanup, and evidence digest.
- A GitHub-hosted macOS source result is labeled preliminary, identifies the
  hosted image and observed architecture, contains no signing credential, and
  cannot be promoted to MacBook Pro M5 or release evidence.
- Evidence contains no credentials, private keys, raw sensitive prompts, unrelated user paths, or real customer content.
- Re-running `agentmage-review validate-evidence` offline verifies the package without trusting AgentMage itself.

## 12. Questions Reserved for a Device Owner or Customer

AgentMage should provide evidence for these questions but should not answer them on a device owner's or customer's behalf.

- [ ] Which device and software environment will contain AgentMage, and who owns or manages it?
- [ ] Who are the deployment owner, information owner, security reviewer, privacy reviewer, records owner, accessibility reviewer, AI-risk owner, and final approver?
- [ ] What local risk tier and security-control baseline apply?
- [ ] Which information types and repositories may AgentMage process?
- [ ] Are personal, financial, customer-confidential, procurement-sensitive, source-code, or credential data prohibited or conditionally allowed?
- [ ] Is a privacy, data-protection, or records assessment required?
- [ ] What records schedule and legal-hold rules apply to prompts, outputs, receipts, and generated artifacts?
- [ ] Is the AI use case high-impact under current policy?
- [ ] Which model, runtime, license, lineage, quantization, and supplier are approved?
- [ ] Which Fedora, Ubuntu, Windows 11, or retained macOS build, device-management or hardening baseline, Visual Studio Code build, extension policy, endpoint monitoring, firewall, and inventory tools apply?
- [ ] Which provider hosts, tenants, accounts, repositories, projects, environments, API versions, credential methods, capability classes, and data scopes are approved?
- [ ] Which cryptographic provider and operating environment must be used?
- [ ] Which application events must feed external logging, and by what approved local collection mechanism?
- [ ] Does the customer require an SBOM, secure-development mapping, supplier attestation, third-party assessment, penetration test, or source review?
- [ ] What support, vulnerability remediation, incident notification, and end-of-life commitments are required?
- [ ] Is managed-device installation, software allowlisting, AI-tool approval, or another internal deployment decision required?

## 13. Build Integration Recommendation

Integrate this security baseline into the project without turning it into a paperwork-only exercise:

1. Give every requirement a stable ID in the implementation backlog.
2. Link every security-sensitive design decision to its requirement IDs and threat cases.
3. Implement the release manifest, data inventory, component inventory, model manifest, SBOM, CBOM, and evidence index before feature growth.
4. Build the reviewer command contract alongside each platform component rather than after the product is complete.
5. Generate human-readable and machine-readable control evidence from the same source records.
6. Assign the first execution of every `RV-*` protocol to the earliest sprint that implements its boundary; release sprints rerun the complete applicable suite and assemble evidence rather than discovering controls for the first time.
7. Make failed critical gates block signing and packaging.
8. Test Linux core behavior locally against native `llama.cpp` and the separately gated Docker Model Runner compatibility adapter; test Windows in a disposable local Windows 11 guest against its native reference runtime; require independent M5/macOS evidence for every later Mac claim; and bind every result to an exact candidate-neutral profile and codec tuple. GitHub-hosted macOS execution is manual, budget-confirmed, source-only preliminary evidence and never substitutes for M5 qualification.
9. Run `RV-23` through `RV-35` for every promoted delivery and productivity adapter and applicable first-GA release candidate.
10. Keep all reviewer fixtures synthetic and public so the package can be shared without exposing organizational data.
11. Have an independent reviewer reproduce the release assessment from the signed package and evidence bundle before publishing a release or requesting optional managed-device evaluation.
12. Run `RV-36` through `RV-43` incrementally with the owning trusted-operations sprint and rerun the complete set at Sprint 166; extend `RV-13`, `RV-14`, `RV-17`, and `RV-41` for Decision 0027 in Sprints 12-15 and 165; run `RV-42` again for the post-GA Experimental Model Lab gate.
13. Run `RV-44` through `RV-48` with their owning whole-codebase audit stories and rerun the complete set from raw evidence at Sprint 166.
14. Run `RV-49` first across Sprints 42, 47, 71, 85-86, and 106 as its local, authentication, commit, push, and hosted boundaries become available; rerun it at Sprints 126 and 166 and for every source-control adapter promotion.
15. Run the Decision 0042 artifact-ingestion and verified-workflow corpus incrementally through its
    owning sprints. Require complete input accounting, exact token-ledger reconciliation, bounded
    parser isolation, fresh attempt identities, no effect replay, verifier-only completion, one
    terminal diagnosis per non-cancelled failure, and identical correctness results across clients.

Recommended implementation gates:

| Gate | Required outcome |
|---|---|
| `SEC-G0` Scope | Approved threat model, use-case boundary, data inventory, shared responsibilities, and product risk baseline exist before implementation. |
| `SEC-G1` Kernel | Grants, paths, storage policy, audit schema, fail-closed configuration, and fake platform tests pass. |
| `SEC-G2` Platforms | Fedora/Ubuntu confinement and Windows MSIX/AppContainer/IPC/NTFS/DPAPI evidence pass independently in fresh disposable local guests with exact image, overlay, revision, standard-user, network-phase, and cleanup provenance; retained Apple Silicon signing, notarization, App Sandbox, XPC, bookmarks, Keychain, and Metal evidence remains separate. |
| `SEC-G3` Model | Candidate-neutral runtime and closed-codec conformance, exact-profile provenance, installer separation, native/container contract parity, Docker API isolation, Muse-first and complete role-aware eligible-Gemma evidence, classifier non-authority, verifier-only completion, separate quality/repeatability reporting, injection resistance, context minimization, uncertainty, and resource gates pass. |
| `SEC-G4` Supply chain | SBOM, CBOM, Model BOM, due diligence, vulnerability disposition, reproducibility/provenance, and support plans pass. |
| `SEC-G5` Privacy and accessibility | Privacy, records, retention, sanitization, accessibility, and conformance evidence are complete. |
| `SEC-G6` Independent assessment | A reviewer independent of the implementation under test runs all applicable `RV-*` protocols and reproduces the signed evidence bundle. |
| `SEC-G7` Optional environment review | Customer decisions, environment controls, exceptions, allowed data, and deployment approval are recorded outside the product's control. |
| `SEC-G8` Delivery adapters | Every promoted provider/version/capability tuple passes manifest, identity, credential, event, effect, failure, removal, and support-matrix conformance. |
| `SEC-G9` Productivity and communications | Autonomy, identity, synchronization, unified inbox, provider operations, recipients, attachments, workflows, recovery, and removal pass. |
| `SEC-G10` Finance | Precision, reconciliation, privacy, explainability, accounting boundaries, and money-movement absence pass. |
| `SEC-G11` Cloud Observer | AWS, Azure, and Google Cloud scope, read-only enforcement, content isolation, correlation, and removal pass. |
| `SEC-G12` Trusted operations | Command authority, Owner-mode lifecycle, public research, credential isolation, encrypted continuity, approved-model management, Muse disposition, and complete removal pass. |
| `SEC-G13` First GA | Fedora, Ubuntu, Windows, strict-local removal, delivery and productivity cross-provider lifecycles, financial and cloud prohibitions, trusted operations, whole-codebase audit, extreme verification, and signed evidence reconciliation pass at Sprint 166. |
| `SEC-G14` Experimental models | Post-GA lab authority absence, resource confinement, truthful provenance gaps, normal-admission-only promotion, and removal pass independently from first GA. |
| `SEC-G15` Whole-codebase audit | Complete census, deterministic structure, bounded semantic review, cross-module reconciliation, canonical immutability, secret protection, checkpoint invalidation, evidence-backed reporting, platform parity, resource controls, and removal pass. |
| `SEC-G16` Repository safety | Exact preservation manifests, hardened Git process isolation, namespaced fetches, owned worktrees and indexes, signed commits, host/account/repository/ref-bound authentication, separately approved ordinary fast-forward pushes, uncertain-effect reconciliation, prohibited-operation absence, and `RV-49` pass. |
| `SEC-G17` Foundational artifact and workflow runtime | Request-bound ingress, zero silent drop, parser isolation, complete context accounting, deterministic preflight, fresh-attempt no-replay, verifier-only completion, bounded recovery, terminal diagnosis, interface parity, `M-FOUNDATIONAL-RUNTIME-CORE` at Sprint 50, and complete `M-FOUNDATIONAL-RUNTIME` no earlier than Sprint 62. |

## 14. Release Decision Rule

A release is not review-ready when any of the following is true:

- A component, model, runtime, library, signer, entitlement, socket, store, data flow, or network behavior is undeclared.
- A critical/high vulnerability lacks an approved, time-bounded disposition.
- Required cryptography cannot be traced to the reviewed provider, configuration, and operating environment.
- Any sandbox, path, IPC, grant, autonomy, command-authority, owner-session, recipient, attachment, credential-isolation, cross-tenant, cross-account, cross-pack, prompt-injection, research-disclosure, financial-precision, secret-leakage, backup-integrity, backup-namespace, model-substitution, experimental-lab, repository-mutation, audit-omission, stale-evidence, parser-isolation, package-integrity, unauthorized-network, unauthorized-send, money-movement, cloud-mutation, duplicate-effect, unsafe-retry, deployment, rollback, or adapter-removal test succeeds for the attacker.
- A failed, skipped, or unavailable test is represented as passed.
- The reviewer cannot reproduce the result from the signed package and evidence.
- The product claims external certification or deployment approval without current evidence.

The desired final reviewer conclusion is narrower and defensible:

> The tested AgentMage release implements the documented local, delivery, productivity,
> communications, finance, cloud-observer, command, public-research, credential, continuity, and
> approved-model-management controls for the exact published platform and provider capability
> matrices; the supplied evidence is reproducible; unsupported operations, residual risks, and
> environment responsibilities are explicit; and each device owner or organization can
> independently decide whether to install it for the stated use case.
