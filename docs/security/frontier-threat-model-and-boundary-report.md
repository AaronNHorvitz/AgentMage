# Frontier Consultation Threat Model and Boundary Report

## Status and Boundary

This is a source-level threat model for the blocked v0.5 candidate. AgentMage has no external-model
client in this capability family. The trusted boundary ends at a locally rendered packet and begins
again only when the user explicitly supplies return files. Manual transport and every external
service remain outside AgentMage's authority, telemetry, credential, and release boundaries.

## Protected Assets

- repository contents, uncommitted work, citations, and exact preimages;
- prompts, packets, return artifacts, local model measurements, and policy decisions;
- credentials, account identities, clipboard contents, browser state, and editor state;
- approval, tool, filesystem, command, Git, completion, and publication authority; and
- receipts, manifests, hashes, quarantine decisions, and retained evidence.

## Trust Boundaries

1. Model output is untrusted data entering deterministic policy and contract validation.
2. Packet rendering is a local no-effect operation; file creation remains a separate controlled
   filesystem transaction.
3. The user manually crosses the external boundary. AgentMage does not call, authenticate to, or
   monitor the selected service.
4. Every returned byte is untrusted. A closed manifest, exact hashes, bounded sizes, quarantine,
   and fresh local facts are required before a proposal may be produced.
5. A proposal has no authority. Existing permission, review, and application controls still apply.

## Threats and Controls

| Threat | Current deterministic control | Remaining release evidence |
|---|---|---|
| Automatic or scheduled delivery | The registered coordinator maps Chat, CLI, model-tool, skill, schedule, injection, clipboard, editor, and network attempts to local denial receipts | Whole-product installed-process probe |
| Credential, browser, editor, or clipboard capture | Contracts contain no such fields or adapters | Native interface and operating-system campaign |
| Hidden telemetry or fallback network | Release corpus requires local denial on every named surface | Packaged binary and runtime network observation |
| Incomplete disclosure or failed redaction | Exact preview and receipt validation fail closed | Native accessible review workflow |
| Packet mutation or destination escape | Exact byte/hash verification and controlled create planning | Integrated write and collision recovery campaign |
| Prompt injection in returned content | Suspicious artifacts are quarantined and cannot grant authority | Live-model and adversarial corpus campaign |
| Secret or oversized return content | Secret scan and bounded per-file/aggregate limits reject import | Privacy review and packaged retention evidence |
| Stale citations or repository state | Fresh local workspace, model, policy, permission, and citation facts plus hash-chained phase recovery | Installed cross-platform interruption evidence |
| Imported approval, completion, or tool authority | Closed grammar, authority-free receipt, and native pre-grant routing reject those claims | Whole-product prohibited-capability scan |
| Recursive or duplicate application | Imported work remains proposal-only; exact replay is idempotent and native-flow tickets retain all application requirements | Installed optional-application campaign |
| Misleading release claim | Machine-readable gate fixes release, signing, and platform fields false | Independent gate decision and signed packages |

## Residual Risk

The contracts are composed into the registered local host workflow with durable content-free
recovery, but there is no live-model campaign, installed whole-product capability scan,
supported-platform installed evidence, lifecycle or accessibility campaign, independent review,
signed package, or completed manual fuzzing. Those gaps block `G-V0.5`; they are not accepted risks
for a release.
