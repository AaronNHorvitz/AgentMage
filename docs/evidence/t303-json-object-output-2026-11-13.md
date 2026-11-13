# JSON-object output requests — 2026-11-13

T-303/T-306 remain open. This increment requests structured output from a
supporting local endpoint; it does not change strict edit acceptance.
Independent core review and a fresh development repeat remain required.

## Measured cause

The preceding six-profile complete-response repeat records 51 strict format
refusals in 180 development attempts, the largest aggregate cause. A read-only
inspection of those 51 recorded answers finds:

| First classification | Answers |
| --- | --- |
| Markdown fence prefix | 27 |
| Other malformed JSON | 21 |
| Duplicate keys in otherwise parseable JSON | 2 |
| Missing edit field in otherwise parseable JSON | 1 |

Fenced answers can have additional defects; this classification makes no claim
that stripping fences would yield valid edits. No answer, frozen task, hidden
test or threshold was changed. Only development answers were inspected.
Raw answers and their exact check/hash records remain in the existing private
sealed archives. The classification records each answer and check digest.

## Implementation

[Decision 0207](../decisions/0207-request-json-object-model-output.md) adds
trusted `response_format: text | json_object` configuration. Text is the
default and preserves the previous upstream request. JSON-object mode encodes
`response_format: {"type":"json_object"}` inside the exact hashed request
body. It is independent of response framing and works without a workspace.
The local API reports it through authenticated settings. Invalid or duplicate
settings refuse without replacing the previous connection; client chat and
proposal bodies cannot choose it. Omitting it restores text on a new choice.

The server may ignore or reject the syntax request. No prompt is changed, no
answer is repaired and no fallback or retry is added. Ordinary chat remains
untrusted text. Coding proposals still pass the unchanged strict parser and
exact-preimage checks. Existing destination consent, credential ownership,
request/response ceilings, deadlines, settings revisions, cancellation and
approval rules remain. This introduces no tool or native executable admission.

## Verification and limits

Three new API socket tests pass. They cover trusted selection/default/reset,
invalid/duplicate settings and client-body refusal without I/O, actual upstream
format encoding in both framings, valid unapplied proposals, fenced or empty
edits and out-of-scope paths, and an unsupported server's status refusal.
Existing complete-mode socket checks now cover both formats, including exact
wire-body hashes and a server ignoring the format request. The codec's exact
request-size boundary test covers both formats, both framings and usage choices.

The complete affected gate finishes all 30 targets: **1,164 passed, 53 failed,
19 ignored**, zero measured or filtered. Every failure identity exactly matches
the retained preceding source gate. **The gate remains failing.** Formatting
and Clippy with warnings denied pass. Direct Markdown lint checks the six
changed Markdown documents with zero issues; the CLI's zero-file summary is
not counted as direct coverage. Compilation and tests use the shared build
reservation. No supply-chain output is regenerated during this source batch.

The next measurement freezes this source and a newly built API binary, keeps
the existing 32K context/output-budget formula, prompts, strict checker and model
profiles, and changes only the explicit output-format request. It uses bounded
two-task GPU jobs with both resource wrappers and releases both reservations
between jobs. This document contains no new model result or improvement claim.
Native executable artifact admission and the missing original grading controls
still block candidate execution, tests, feedback and retries. Do not reconstruct
controls. Task grades remain unavailable, and held-out tasks remain unrun.

The previous E4B measurement's independent PASS is retained separately, with
its low wording note clarified additively. It does not approve this source
increment or close either parent. No new row or completion tick is added.
