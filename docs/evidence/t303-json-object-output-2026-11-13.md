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

## Frozen build and queue checkpoint

Source commit is `40cad6472560e47314a55ddb1f45e2c4c22b1792`. The newly built
API SHA-256 is
`a8995c1515165d869e9382cab85cd1cd38c5902d3bd3dea8c641bdef50316dc2`.
The frozen input manifest binds 2,283 repository and 16 private inputs:
`79a130de9795f6317b4a4ebfa92777c025bb608e3b12634fe91f1d0a12f32ab3`.
The unchanged helper retains SHA-256
`852cd7f57d414c03d337102752a678f4e8c7c62b89a9485a43f3ebfd2698e9ba`.
Source verification's seven retained artifacts have binding-manifest SHA-256
`14aac92a3e2401214fb6c5ae21da8aa9dc5afb0acfa52fa838e027e9499fa39a`.

The first E4B pair waited at least 467 seconds for the shared build reservation.
The owned queue was checkpointed before any job directory, GPU reservation,
model or task attempt existed; all owned processes exited. Queue logs remain
unchanged and a fresh resume driver is prepared. This is **0/30 attempts and
zero GPU job minutes**, not a new model measurement. The scoreboard records
its pending state explicitly. Full development repeats still precede tuning;
no inference, task grade, retry or held-out result is claimed.

The queue-checkpoint archive retains the exact new binary, helper, source
snapshot, launch policy, queue logs and resume driver. Its read/hash-only
verifier passes all **2,307 files**, including the 2,283 repository and 16
private input bindings. It extracts nothing and executes no archived code.
No credential or weight is included.

| Queue-checkpoint retention | SHA-256 |
| --- | --- |
| `json-object-source-queue-20261113-inputs.tar.gz` | `46c0856bc7de694ec39ba0e5ab541081c24e082ec305fed16af9a36c4810f73d` |
| `json-object-source-queue-20261113-manifest.json` | `74d9cfb6bb7af54ad1db43af30217dd301a7eddffecfae619d7abedad102e693` |

## Independent source review retained — 2026-11-13

The [independent core review](../reviews/de8c204cbc52-json-object-model-output.txt)
returns **Verdict: PASS** for exact commit
`de8c204cbc52c7b090f8039a4bfaf15666854852`, with no high or medium finding.
It reproduces the refusal classification, source and archive bindings, default
behavior and unchanged 53 gate failure identities, and reruns the five focused
tests successfully. Markdown lint was unavailable in the review lane; the
reviewer checked the retained builder logs and did not download tooling.

Low L1 records that preflight size checks use Stream/Text while the eventual
transport honors the configured mode and format. A JSON-object request near
the ceiling can therefore receive an error event after chat headers rather
than a pre-dispatch 400. It fails closed with no upstream I/O; proposals still
refuse before dispatch. Format-aware preflight and a boundary test join T-309
backlog. Explicit mode/format record fields are a non-finding backlog option.
No source, frozen input, threshold or row status changes. The new development
repeat remains required before further tuning; source acceptance alone gives
no task score or native qualification.
