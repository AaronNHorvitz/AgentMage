# Story 5.3 Adversarial Workflow Campaign Evidence

This record covers repository-controlled synthetic attacks for Sub-task 5.3.3.1. It executes no
model inference, runtime effect, network access, or native-platform campaign. `RV-52`, Story,
Sprint, and release completion remain unclaimed.

## Attack matrix

The campaign re-executes the actual workflow boundaries against eight required attack families:

| Attack | Boundary behavior |
|---|---|
| Missing or stale preflight | Definition and retry admission refuse before dispatch. |
| Malformed call | Bounded lossless normalization refuses malformed structure and cannot invent values. |
| Approval bypass | Required approval absence or mutation refuses identity and dispatch admission. |
| Grant reuse | The synchronized identity and prior-use ledgers reject every replay. |
| Retry loop | Unsafe retry and repeated-state policies terminate at their exact bounds. |
| False completion | Exit zero, persuasive stdout, missing results, and tampering cannot create success. |
| Uncertain effect | Uncertainty remains sticky until separately approved safe reconciliation. |
| Contradictory evidence | Output, state, evidence, invariant, and prohibited-effect conflicts fail closed. |

## Verification scope

The retained raw log runs the workflow definition, identity, retry, verifier, repair, and
repeated-state suites plus strict Clippy. Thirteen named boundary markers cover all eight attack
families. Every denied candidate admits zero dispatches and zero false success; the harness uses
synthetic identities and content only.
