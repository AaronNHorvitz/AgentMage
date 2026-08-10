# AgentMage Security Review and Verification Guide

| Field | Value |
|---|---|
| Document status | Public product-security planning baseline; no external certification claim |
| Source review date | 2026-08-10 |
| Product ownership | Independently developed by Aaron N. Horvitz on personal time and personally controlled equipment |
| Intended product boundary | Local, single-user desktop software |
| Primary validation target | Apple Silicon macOS on a personally controlled MacBook Pro |
| Linux release references | Fedora and Ubuntu with native `llama.cpp`; Docker Model Runner is a separately gated compatibility adapter |
| Deferred platform | Windows 11 |
| Initial release scope | Read-only local evidence assistant in native Visual Studio Code Chat |

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

This guide is interpreted with [SECURITY.md](SECURITY.md), [MODEL-PROVENANCE-POLICY.md](MODEL-PROVENANCE-POLICY.md), and [RUNTIME-BOUNDARIES.md](RUNTIME-BOUNDARIES.md). Those documents define public vulnerability handling, model admission, and the process/socket/data-flow boundary; a release cannot substitute looser behavior for any of them.

## 2. Recommended Review Position

AgentMage should be presented as independently developed, locally installed desktop software with a bounded Visual Studio Code integration, not as a cloud service or an extension of any employer's systems or intellectual property.

Recommended initial product-validation boundary:

- One personally controlled Apple Silicon Mac.
- One standard, non-administrator user.
- One stable Visual Studio Code build.
- One allowlisted AgentMage extension and signed local host package.
- One user-selected, read-only workspace at a time.
- One manifest-pinned local model and runtime.
- Public, synthetic, or user-owned non-sensitive data only.
- No cloud inference, hosted account, telemetry, analytics, crash upload, remote tool, connector, browser, shell, write operation, autonomous agent, or automatic model switch.
- No sensitive or regulated data until a separately tested profile explicitly supports it.

The engineering target is a conservative, defense-in-depth desktop security posture with least privilege, deny-by-default authority, local data minimization, reproducible builds, transparent supply-chain records, adversarial testing, and independent verification.

## 3. Reviewer Quick Path

A reviewer should be able to complete the initial assessment in this order:

- [ ] Confirm the proposed use case and allowed information types.
- [ ] Confirm who owns or manages the target device and whether installation is permitted.
- [ ] Confirm that normal v0.1 operation has no cloud or hosted service dependency.
- [ ] Confirm the macOS, Visual Studio Code, extension, model, runtime, and cryptographic module versions.
- [ ] Confirm the model admission record, immutable artifact identity, runtime adapter, and fallback state against the model-provenance policy.
- [ ] Review the architecture diagram, data-flow diagram, threat model, and shared-responsibility matrix.
- [ ] Validate signatures, notarization, hashes, SBOM, model manifest, and release provenance.
- [ ] Run the automated reviewer suite and retain its signed evidence bundle.
- [ ] Independently observe the offline, sandbox, path, IPC, logging, deletion, and prompt-injection tests.
- [ ] Record privacy, retention, accessibility, supply-chain, and AI-risk decisions.
- [ ] Record open findings, owners, deadlines, and compensating controls in the remediation register, or reject the release if a release-blocking gate fails.

No reviewer should need internet access to run the product tests after the approved package and model have been imported.

## 4. Applicability Decisions

| Topic | Default position for v0.1 | Reviewer decision |
|---|---|---|
| Local-only architecture | Required after model installation in v0.1. | Confirm that no hosted control plane, remote inference, telemetry, analytics, cloud storage, or silent update check is present. |
| Data sensitivity | Synthetic and non-sensitive user-owned data only by default. | Identify any additional data classes and require a separate threat model and test profile before use. |
| Privacy and retention | Data minimization, local encryption, explicit retention, export, and deletion are required. | Define any customer-specific notice, retention, backup, or deletion rules. |
| AI risk | Applicable because a local generative model is used. | Review intended use, foreseeable misuse, quality limits, human oversight, and required safeguards. |
| Software supply chain | Signed provenance, SBOM, model manifest, licenses, hashes, and vulnerability dispositions are required. | Decide whether additional attestation, source review, or penetration testing is needed. |
| Accessibility | Core workflows and generated guidance target WCAG 2.2 AA. | Review the Accessibility Conformance Report and independent test results. |
| Cryptography | Platform-backed, reviewed cryptographic providers and precise claims are required. | Confirm the provider and operating environment are acceptable for the intended data. |
| Managed-device compatibility | Optional and outside the personal development boundary. | Supply endpoint-management, monitoring, software-allowlist, and installation constraints before testing. |
| Windows 11 | Deferred and unsupported. | Require a separate platform assessment before any Windows release. |

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
| macOS and Visual Studio Code deployment approval | Compatibility evidence | Yes | Yes |
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
| `SR-NET-003` | Bind local services only to the declared local transport and treat loopback as reachability control, not authentication. | Prefer authenticated private IPC. Where Docker Model Runner's unauthenticated HTTP API is used, isolate it behind the kernel, bind only to the approved local address, and deny every undeclared process, user, container, and namespace. | Scan all interfaces and namespaces and attempt LAN, container, cross-user, tool-worker, extension-host, and undeclared-process connections. |
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

### 8.11 Future Windows 11 Gate

Windows support remains absent until all common requirements pass and a separate Windows platform package proves:

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

## 9. Proposed Reviewer Command Contract

These commands are interfaces to implement. They do not exist yet. They should be packaged in a signed, read-only verifier and must not modify the workstation except inside an explicit temporary test directory.

```text
agentmage-review inventory
agentmage-review verify-release --package <path> --model <path>
agentmage-review preflight --profile macos
agentmage-review verify-platform --profile macos
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
agentmage-review uninstall-test
agentmage-review full --profile macos --evidence-dir <path>
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
3. Verify Developer ID signatures, designated requirements, notarization, stapling, Hardened Runtime, App Sandbox, App Group, and entitlements.
4. Compare every executable and library with the signed component inventory and SBOM.
5. Verify provenance signatures and source/build identity.

Pass: every shipped component is declared, signed, hash-matched, and policy-approved; no undeclared executable code exists.

### `RV-02` Clean Standard-User Installation

1. Use the exact supported Mac model and OS build with the selected baseline.
2. Use a fresh standard account without Homebrew, Rosetta, Xcode tools, Docker Desktop, ambient Python, or ambient Git.
3. Install, launch, run diagnostics, select a synthetic workspace, and remove the application using only the published procedure.

Pass: no administrator access is required after approved installation; no baseline control is weakened; every component and location matches the manifest.

### `RV-03` Sandbox and Ambient-Access Resistance

1. Place unique canaries in the workspace, adjacent directory, home folders, browser data, SSH folder, environment, clipboard, removable location, and another user's simulated area.
2. Exercise normal tools and malicious model/tool requests.
3. Attempt filesystem, process, device, environment, bookmark, and entitlement escapes.

Pass: only authorized workspace canaries are readable; zero sandbox escapes; zero prohibited canaries reach output, storage, logs, or the model.

### `RV-04` Path and Race Safety

Run at least 500 fixtures covering absolute paths, traversal, alternate separators, encodings, NULs, aliases, symlinks, hard links where applicable, stale bookmarks, case collisions, Unicode normalization, rename races, replacement races, and mount changes.

Pass: zero workspace escapes and 100 percent success for valid unambiguous fixture paths.

### `RV-05` IPC Identity and Replay

Attempt connections from unsigned, wrongly signed, wrong-bundle, wrong-App-Group, wrong-user, stale, replaying, malformed, oversized, version-incompatible, and rapidly reconnecting clients.

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

Verify the `MODEL-PROVENANCE-POLICY.md` admission record, license, publisher/control, lineage, original artifact, conversion, quantization, tokenizer, chat template, GGUF, immutable OCI digest where applicable, runtime build, adapter, platform, resource requirements, and all hashes. Substitute each item independently. Run the same fixed response, tool-schema, cancellation, context-limit, and resource corpus through every enabled native and Docker adapter.

Pass: the exact approved identity loads; every silent substitution, mutable-tag-only identity, mismatch, corruption, prohibited lineage, or unsupported environment is quarantined or refused; all enabled adapters meet the same published contract thresholds, with differences recorded rather than hidden.

### `RV-14` Model Quality and Evidence Integrity

Run the fixed repository, tool-call, factual-grounding, uncertainty, conflicting-evidence, stale-citation, and false-completion corpora across repeated trials.

Pass: all published thresholds are met; every material claim has the correct evidence state; negative results and limitations remain visible.

### `RV-15` Fuzzing and Malformed Input

Fuzz IPC, manifests, schemas, model output, paths, text encodings, Git objects, repository-map parsers, database imports, and every FFI boundary using pinned corpora and sanitizers where supported.

Pass: no memory-safety failure, sandbox escape, unauthorized action, secret disclosure, unbounded resource use, or unrecoverable corruption.

### `RV-16` Resource Exhaustion and Cancellation

Exercise oversized repositories/files/results, deep trees, adversarial tokens, long inference, full disk, low memory, GPU failure, rapid requests, process termination, sleep/wake, and cancellation at every phase.

Pass: declared limits hold; cancellation is bounded; scratch is cleaned; state remains recoverable; the endpoint remains responsive.

### `RV-17` Crash Recovery and State Integrity

Inject a crash before and after each durable transition and during grant validation, tool execution, model inference, receipt creation, checkpoint, and shutdown.

Pass: 100 of 100 recovery fixtures select the correct safe state, repeat no completed operation, and produce no false success.

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
    macos-profile.json
    signatures.txt
    notarization.txt
    entitlements/
    baseline-before.json
    baseline-after.json
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
- [ ] Which macOS build, device-management or hardening baseline, Visual Studio Code build, extension policy, endpoint monitoring, firewall, and inventory tools apply?
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
8. Test Linux core behavior continuously against native `llama.cpp` and the separately gated Docker Model Runner compatibility adapter, but require independent M5/macOS evidence for every Mac deployment claim.
9. Keep all reviewer fixtures synthetic and public so the package can be shared without exposing organizational data.
10. Have an independent reviewer reproduce the release assessment from the signed package and evidence bundle before publishing a release or requesting optional managed-device evaluation.

Recommended implementation gates:

| Gate | Required outcome |
|---|---|
| `SEC-G0` Scope | Approved threat model, use-case boundary, data inventory, shared responsibilities, and product risk baseline exist before implementation. |
| `SEC-G1` Kernel | Grants, paths, storage policy, audit schema, fail-closed configuration, and fake platform tests pass. |
| `SEC-G2` macOS | Signing, notarization, App Sandbox, XPC, bookmarks, Keychain/crypto provider, selected endpoint-baseline compatibility, and offline proof pass on the M5 reference. |
| `SEC-G3` Model | Installer separation, model/runtime provenance, native/container contract parity, Docker API isolation, injection resistance, context minimization, quality, uncertainty, and resource gates pass. |
| `SEC-G4` Supply chain | SBOM, CBOM, Model BOM, due diligence, vulnerability disposition, reproducibility/provenance, and support plans pass. |
| `SEC-G5` Privacy and accessibility | Privacy, records, retention, sanitization, accessibility, and conformance evidence are complete. |
| `SEC-G6` Independent assessment | A reviewer independent of the implementation under test runs all applicable `RV-*` protocols and reproduces the signed evidence bundle. |
| `SEC-G7` Optional environment review | Customer decisions, environment controls, exceptions, allowed data, and deployment approval are recorded outside the product's control. |

## 14. Release Decision Rule

A release is not review-ready when any of the following is true:

- A component, model, runtime, library, signer, entitlement, socket, store, data flow, or network behavior is undeclared.
- A critical/high vulnerability lacks an approved, time-bounded disposition.
- Required cryptography cannot be traced to the reviewed provider, configuration, and operating environment.
- Any sandbox, path, IPC, grant, prompt-injection, secret-leakage, package-integrity, or outbound-network test succeeds for the attacker.
- A failed, skipped, or unavailable test is represented as passed.
- The reviewer cannot reproduce the result from the signed package and evidence.
- The product claims external certification or deployment approval without current evidence.

The desired final reviewer conclusion is narrower and defensible:

> The tested AgentMage release implements the documented local, read-only product controls; the supplied evidence is reproducible; residual risks and environment responsibilities are explicit; and each device owner or organization can independently decide whether to install it for the stated use case.
