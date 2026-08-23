# AgentMage Security Policy

## Supported Versions

AgentMage is currently in design and planning. No production binary is supported yet.

Beginning with the first public binary release, the latest patch release of each supported minor version receives security fixes until the support end recorded in its signed release manifest. Unless a release states otherwise, a superseded pre-1.0 minor version remains supported for 90 days after its successor is published. Unsupported versions receive no remediation commitment and should not process new work.

## Reporting a Vulnerability

Do not disclose a suspected vulnerability in a public issue, discussion, pull request, log, or example repository.

Use GitHub private vulnerability reporting for this repository when that feature is available. If it is unavailable, open a public issue containing only a request for a private security-reporting channel. Do not include exploit details, credentials, private data, model prompts, workspace contents, or customer information in that issue.

A useful private report includes:

- Affected AgentMage version, commit, package hash, platform, and runtime adapter.
- Reproduction steps using synthetic data.
- Expected and observed behavior.
- Security impact and prerequisites.
- Relevant logs or receipts after removing secrets and unrelated paths.
- Whether the issue is already public or actively exploited.

Receipt of a report is not authorization to inspect systems, access data, exceed granted scope, or disrupt service.

## Triage and Disclosure

The maintainer will acknowledge a complete private report as capacity permits, establish an initial severity and affected-version disposition, and keep the reporter informed at material milestones. Published release notes will credit reporters who request attribution and will avoid exploit-enabling detail until users have a reasonable opportunity to update.

Severity considers authority expansion, confidentiality, integrity, availability, exploitability, affected defaults, required user interaction, and whether the issue crosses the workspace, process, model, network, or release boundary. A failed test, stale dependency, or uncertain result remains visible and cannot be closed as a false positive without evidence.

## Remediation and Patch Delivery

AgentMage performs no automatic update check during normal operation. A security fix is delivered as a new signed release with:

- A new immutable version and release manifest.
- Package signatures, checksums, provenance, and updated software, cryptographic, and model bills of materials.
- Re-run security and regression evidence for every affected boundary.
- A plain-language advisory identifying affected and fixed versions, mitigation, rollback, and support status.
- Manual installation or replacement instructions that work without granting AgentMage runtime network authority.

Users learn about fixes through the repository's security advisories and signed release notes. AgentMage does not phone home, push a remote configuration, or silently replace a model, runtime, or package.

## Emergency Disablement

Every supported release provides a local, user-controlled disable path that prevents AgentMage model loading and tool execution while preserving only authorized diagnostic and incident evidence. An emergency advisory may instruct users to disable, uninstall, or replace a release. There is no remote kill switch.

A compromised signer, release package, dependency, model, runtime, or update channel blocks publication and may revoke the affected artifact from the approved catalog. Recovery requires a new independently verified release identity.

## Incident Handling

The product incident runbook covers detection, suspension, containment, bounded evidence preservation, remediation, recovery, notification, and the interface to the device owner's own response process. Incident evidence follows the same classification, minimization, encryption, access, and retention rules as normal product data; an incident does not create unlimited collection authority.

Release gates include tabletop exercises for suspected egress, a compromised dependency or package, prompt-injection disclosure, hostile public research, Owner-mode misuse, surviving command descendants, key-store failure, provider credential confusion, backup corruption or wrong-account restore, model artifact substitution, read-only audit mutation, secret-bearing repository content, stale audit evidence, duplicate external effect, unsafe deployment, and adapter removal.

## Scope

This policy covers AgentMage source, distributed packages, release infrastructure, supported runtime adapters, trusted-operations and whole-codebase audit workers, continuity objects, credential references, and approved model artifacts. It does not make a security guarantee for modified builds, unsupported platforms, unapproved models, exposed raw inference endpoints, disabled platform protections, commands a user deliberately authorizes in Owner / Unrestricted Session, or information the user independently transfers to another product.

See [SECURITY-REVIEW.md](./SECURITY-REVIEW.md) for normative security requirements and reviewer protocols, [MODEL-PROVENANCE-POLICY.md](./MODEL-PROVENANCE-POLICY.md) for model admission, [TRUSTED-OPERATIONS.md](./TRUSTED-OPERATIONS.md) for command, research, credential, continuity, and model-management boundaries, [CODEBASE-AUDIT.md](./CODEBASE-AUDIT.md) for comprehensive repository-audit boundaries, and [RUNTIME-BOUNDARIES.md](./RUNTIME-BOUNDARIES.md) for processes, privileges, sockets, and data flows.
The vulnerability process covers the Engineering Runtime, Model Gateway, Verified Chat and native
VS Code compatibility adapters, Engineering Capability Registry, multi-agent composition,
local/private/managed endpoint profiles, protocol codecs, remote-inference workers, routing and
fallback policy, context-delivery receipts, tool observations, deterministic verification, and
their schemas and update/removal paths. Reports involving secret disclosure, private endpoint
exposure, route substitution, silent fallback, SSRF, DNS rebinding, IPC spoofing, authority
aggregation, self-approval, false completion, artifact/context omission, or evidence forgery are
security reports even when no model weight or provider account is bundled.
