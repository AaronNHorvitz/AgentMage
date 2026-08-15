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
| Automatic or scheduled delivery | No client, endpoint, routing, schedule, or delivery capability | Whole-product installed-process probe |
| Credential, browser, editor, or clipboard capture | Contracts contain no such fields or adapters | Native interface and operating-system campaign |
| Hidden telemetry or fallback network | Release corpus requires local denial on every named surface | Packaged binary and runtime network observation |
| Incomplete disclosure or failed redaction | Exact preview and receipt validation fail closed | Native accessible review workflow |
| Packet mutation or destination escape | Exact byte/hash verification and controlled create planning | Integrated write and collision recovery campaign |
| Prompt injection in returned content | Suspicious artifacts are quarantined and cannot grant authority | Live-model and adversarial corpus campaign |
| Secret or oversized return content | Secret scan and bounded per-file/aggregate limits reject import | Privacy review and packaged retention evidence |
| Stale citations or repository state | Fresh local workspace, model, policy, permission, and citation facts | Coordinator-level interruption and recovery evidence |
| Imported approval, completion, or tool authority | Closed grammar and authority-free receipt reject those claims | Whole-product prohibited-capability scan |
| Recursive or duplicate application | Imported work is proposal-only; application is absent here | Durable idempotency and normal-flow integration |
| Misleading release claim | Machine-readable gate fixes release, signing, and platform fields false | Independent gate decision and signed packages |

## Residual Risk

The contracts have not yet been composed into a native product workflow. There is no live-model
campaign, supported-platform installed evidence, durable round-trip recovery, lifecycle or
accessibility campaign, independent review, signed package, or completed manual fuzzing. Those
gaps block `G-V0.5`; they are not accepted risks for a release.
