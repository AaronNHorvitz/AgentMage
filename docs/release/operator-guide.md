# AgentMage v0.1 Operator Guide

## Document Status

This is a pre-release operating guide. No supported v0.1 package or production trust root is
available. Commands below are maintainer and candidate-validation interfaces; they are not an
installation recommendation.

## Installation

AgentMage targets an ordinary local user with the VS Code extension and a separately verified host
package. Fedora and Ubuntu package-candidate mechanics exist for RPM and DEB. macOS has a platform
contract scaffold, and Windows has a native identity increment; neither is a supported package.

Do not install an artifact unless all of these are available for the exact release:

1. A release manifest naming the version, sequence, platform, architecture, configuration, complete
   payload set, and SHA-256 identities.
2. A detached signature and separately obtained trusted public key whose fingerprint matches the
   published release record.
3. Platform-native package verification and a supported-platform statement.
4. Current clean-build, installation, offline, accessibility, recovery, and removal evidence.
5. Release notes that list every limitation and unresolved risk.

Candidate bundles are build outputs, not releases. Candidate verification intentionally refuses to
claim signed-release status.

## First Run

The extension registers AgentMage in native VS Code Chat and launches only the fixed installed host
through a one-use authenticated local channel. Package, platform, peer, challenge, or protocol
failure leaves an inert provider with no selectable model.

Current bounded commands are:

- `models` for exact local profile state;
- `doctor` for content-free local component status;
- `export diagnostics` for an explicitly reviewed local diagnostic file;
- `read <workspace-relative-path>` for one approved bounded local read; and
- `handoff` for review and local rendering of a packet that the user may transfer manually.

The production host does not yet load a signed profile catalog, route prompts to a model, or compose
a current handoff draft. A first run is therefore expected to show unavailable components and no
ordinary model profile. That is a truthful refusal, not a setup success.

## Model Installation and Selection

Model family names and mutable tags are never sufficient. A model requires an exact artifact,
license, provenance, codec, tokenizer, template, context, decoding, runtime, hardware, policy,
quality, support, and lifecycle tuple.

The acquisition design separates network-enabled acquisition from strict-local operation. Import,
download, scan, quarantine, self-test, activation, rollback, and removal are distinct stages.
Activation requires current verification and an explicit user decision. AgentMage must never:

- activate during download or import;
- substitute another profile after selection;
- treat a family name as approval;
- launch quarantined, stale, incompatible, unsupported, or failed material; or
- retain acquisition authority during the offline operating phase.

The current production VS Code path cannot install or activate a model. Existing model lifecycle
code and fixtures are development evidence only.

## Diagnostics

`doctor` reports typed states such as Healthy, Degraded, Blocked, Unavailable, Quarantined, and
Unsupported. It reports component identities where available, but it does not probe arbitrary
paths, contact external services, or turn missing evidence into Healthy.

`export diagnostics` requires a user-selected private local destination, an exact preview, and one
confirmation. Exports are content-free and redact credentials and prompts. Do not attach raw
prompts, workspace files, credentials, private keys, broad environment dumps, unrelated paths, or
unreviewed archives to a support request.

## Permissions and Approvals

The model and Chat text are never authority. AgentMage derives authority only through typed kernel
grants bound to exact actor, session, task, action, tool, operation, workspace, object, policy,
configuration, expiry, and confirmation identities.

An approval applies only to the exact displayed preview. Cancellation, expiry, replay, changed
content, changed policy, changed source identity, or changed target invalidates it. Display links do
not grant filesystem authority. A denial should not be bypassed by changing prompt wording.

The v0.1 scope is read-only. It does not provide repository writes, shell execution, Git mutation,
GitHub publication, email or chat actions, browser actions, calendar changes, cloud changes, or
automatic Codex transfer.

## Evidence and Receipts

Material claims use four states: Observed, Derived, Inferred, and Unknown/Blocked. A model statement
is not Observed merely because it is confident. Receipts bind exact local operations and outcomes;
failed, cancelled, timed-out, uncertain, or denied work cannot be represented as success.

Evidence reports identify their source revision, environment, exact command inventory, exit codes,
output hashes, source hashes, limitations, and blockers. Generated summaries must be reproducible
from raw records. A later source change makes prior evidence historical unless the report explicitly
binds that later revision.

## Repository Map

The repository map is bounded, read-only, Git-aware, parser-versioned, and source-resolvable. It
reports discovered, parsed, searched, skipped, excluded, unsupported, failed, and truncated files
and budgets. Unsupported languages use visible lexical fallback or Unknown/Blocked state.

The map must not be treated as complete when coverage reports an omission. Every cited structural
fact should resolve to the exact revision, workspace-relative file, range, content hash, parser, and
grammar identity. Mapping does not execute repository instructions or authorize a tool.

The complete repository-map workflow is not yet connected to the production native Chat route.

## Privacy and Local Data

Normal strict-local operation is designed for no external network path. Local state must reside in a
private, non-synchronized, supported filesystem root. Credentials belong in the operating system's
secret service, not configuration, prompts, logs, command-line arguments, environment variables,
evidence, or repository files.

AgentMage minimizes retained content. Content-free identities and hashes are preferred where they
can establish the required fact. Diagnostic and evidence export is explicit, local, previewed, and
user-managed. Deleting an export does not imply cryptographic erasure of unrelated copies or backup
media.

## Offline Proof

Offline proof is more than setting a client flag. The accepted boundary requires acquisition
authority to be removed, exact process and socket inventories, no undeclared listener or connection,
declared private local inference topology, and a complete one-way evidence lifecycle.

Current static checks include:

```bash
npm run strict-local-source:check
npm run hostile-network:check
npm run effect-boundary:check
```

These checks do not replace a retained live zero-egress observation on every supported platform and
workflow. The absence of captured packets is not a pass if process, namespace, socket, and routing
coverage is incomplete.

## Recovery and Backup

Durable operational state uses an encrypted, single-writer store with versioned migrations,
retention events, hash-bound checkpoints, encrypted backup, explicit restore, and exact restart
reconciliation. Recovery must never replay an uncertain or already completed effect.

Before relying on recovery, verify that the exact release has passed interruption tests at every
state transition and that the backup destination is private, local, nonsynchronized, and supported.
Do not overwrite the only known-good backup during migration. A failed restore must leave the prior
state intact and visibly blocked.

The production native Chat workflow has not completed end-to-end restart and restore acceptance.

## Local Handoff

`handoff` is a local rendering boundary, not an integration with Codex. The review shows exact
included content, hashes, inferences, exclusions, unresolved questions, sensitivity, redactions,
destination class, and size. Permitted user-provided or non-public content requires explicit
acknowledgement. Changed source or policy requires regeneration.

AgentMage cannot invoke Codex, activate or populate another tab, write the clipboard, open a URI,
deliver to a runtime, call a network, or submit the packet. External handling begins only when the
user manually transfers selected content.

## Troubleshooting

Use the narrowest content-free observation first:

1. Run `doctor` and record typed states and reason codes, not private content.
2. Confirm exactly one local file workspace is selected and no remote VS Code workspace is active.
3. Confirm the installed package, extension, platform, and model identities match the release
   manifest; never fix identity drift by disabling verification.
4. Confirm the private state root, ownership, mode, filesystem, secret service, required sandbox,
   and local runtime prerequisites.
5. Reproduce with the same exact configuration and source identities.
6. Export diagnostics only after reviewing every included field and redaction.
7. Stop when the state is Unknown/Blocked; do not broaden permissions or attach private archives.

Common expected pre-release results include an empty model picker, unavailable production model
route, unavailable handoff source, unavailable native control, or blocked platform gate. Those
cannot be repaired by selecting a cloud model or bypassing the host.

## Limitations

No v0.1 release is approved. Production model inference, complete native Chat workflow, signed
catalog integration, canonical handoff composition, complete progress and status indicators,
exact model token counting, installed accessibility evidence, complete Fedora/Ubuntu/macOS clean
installation, production signing, notarization, Windows release packaging, and independent release
review remain open.

The broader roadmap includes writes, semantic indexing, user knowledge and Obsidian, full CLI,
desktop UI, GitHub and delivery systems, browsers, communications, calendars, financial workflows,
cloud observation, schedules, and child agents. None is a v0.1 capability.
