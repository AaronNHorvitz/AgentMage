# T-311.6: Pre-measurement statement-contract repair

Date: 2026-11-03. Tier: core. Base: `57ea7ee6`. Same T-311.6 row,
review attempt 2 pending. Parent T-311 remains open.

## Correction

The [independent review](../reviews/57ea7ee64ec6-local-coding-t311-6-development-collection.txt)
finds medium M1 (dev-021 digest-check scope) and M2 (dev-029 notice and
controls shape). Before any model measurement, `coding-development-v9`
clarifies both contracts and low L1 (dev-022 artifact ID field locations).
The unchanged independent report is retained as plain text; its SHA-256 is
`e11ef7af9bd143138aaee9ac552b5ab42d3d92792c3db4f6ba4b238ad09bb98f`. Low L2 is
acknowledged: dev-024's base projection contains a similar observation helper
in another script, so its result includes finding and adapting existing code.

Only three statements change. A preservation regression compares every field
of all 15 expansion dictionaries with v8 and requires all non-statement fields
to remain equal. Existing tests preserve the 20 previously accepted definitions.
All hidden bytes, selections, counts, pins and thresholds are unchanged.
The collection fingerprint is
`0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570`.
The automatic interface audit checks names only; it cannot establish complete
statement sufficiency for data shapes or conditional requirements.

## Contract details

| Finding | Clarification |
| --- | --- |
| M1 / dev-021 | Only read-arguments-correction checks the retained request digest; read-arguments-denied does not. The denial still refuses completed tools and requires the terminal unresolved code. |
| M2 / dev-029 | Each observed notice uses the top-level type discriminator. The job_state notice has top-level controls containing ordered objects with action values suspend, resume. The job phase remains nested under job.phase. |
| L1 / dev-022 | artifact_created stores its ID at kind.artifact_id; runtime_artifact_verified uses top-level artifact_id. |

## Verification

All execution checks ran under one shared build reservation. Wall times below
exclude the reservation wait; the verifier also reports its summed case timers.

| Check | Observed result | Wall seconds |
| --- | --- | --- |
| Diagnostic replay | Both original reviewer candidates fail once; both one-line corrected candidates pass | 0.875153 sum of four probe timers |
| Scoped fast gate | PASS, eight paths, six commands, 58 unit tests | 20.861339 |
| Full verifier | PASS, 30 failing bases, 30 passing historical fixes, 89 reference tests | 43.588206 |
| Documentation policy validator | PASS, 702 Markdown files and all policy invariants | 122.205681 |

The unit suite itself took 19.097 seconds. The verifier ran 178 test invocations
with 39.833331 seconds summed case timers. Every base has cause test_failure;
every reference has cause none. No skip, expected failure, unexpected success,
input refusal, truncation or infrastructure error occurred. All cases record
zero model attempts, zero GPU minutes and null tokens/model score. The dev-003
race did not occur. No check failed unexpectedly in this repair batch.

The diagnostic replay reuses the separate reviewer's retained candidate1 and
candidate2 bytes for dev-021 and dev-029. Each first candidate still produces
exactly one assertion failure and no error under the frozen method; each second
candidate passes. These are diagnostic replays of reviewed candidates, not new
blind implementations, model attempts or production scores. A private driver
calls the existing sandboxed grade_case directly for these probes; the CLI's
controls-only admission remains unchanged. Every original failure log stays
retained. The evaluator and candidate code are unchanged by the replays.

The source snapshots before and after execution match. Final changes after the
checks are only queue/evidence prose; scoped Markdown, whitespace, source-binding
and added-text privacy checks run before commit. The private handoff locates raw
logs, candidate provenance and audits. All owned jobs exited and released their
reservation. No new evaluator tree remains; historical leftovers are untouched.

| Tested source | SHA-256 |
| --- | --- |
| `scripts/coding_eval.py` | `f4a8a26672a73426845fd7094b6ba6946951a8235c1db03c80552ce35462ada9` |
| `scripts/coding_eval_collection.py` | `2fd431dc75f7b68f87330aff3024a924380daebe7fb71972d583b3ee5715bd81` |
| `scripts/coding_eval_rust.py` | `735390c9e14d151c3a3b24645458f1cb307fa120ba23071fda81a69ebd0a9fda` |
| `tests/test_coding_eval.py` | `7111513acde8f87e3f47ebc71c11274f7c87444b73e607d9a990d00607461487` |
| `tests/test_coding_eval_collection.py` | `5cd6429896f360893e73093a87c3b52ad07e459981e4e88c8ad51676b1b9647d` |
| `tests/test_coding_eval_rust.py` | `cee74a83e2cb3ca23d0420a8e5fb0510aa888748088d5a0a64460d826938cbd9` |

| Private artifact | SHA-256 |
| --- | --- |
| `check.py` | `17c5e347233ef5fb1d959b00a4bdd0c7893ce12091e9b312d7867c9a5adb51f9` |
| `probe-summary.json` | `14676b03a28c727335ac8735916a2908eaaceb50e2033570160438351d7ad887` |
| `fast-gate.log` | `73c94f09a759470689b1608065caa00ef0121bec7a4f5300013fe51ce6b0a034` |
| `verify/summary.json` | `16833bb910a91349da2c9eb02838832402954ed4f35002c64080932e80a63601` |
| `verification-audit.json` | `5bcb986b8e5b56f620162ced66dfe88f74ddac2f9b2e9cd3d907729749c2bf11` |
| `definition-audit.json` | `0e263c67680900784169196b6806f22acf49a44421300daa755efee94e7848f4` |
| `source-before.json` | `c0dfaeb3627d19da9e5b96633812a2b34ad34e8b91c393bdc5aecd6c735c9d4b` |
| `source-after.json` | `c0dfaeb3627d19da9e5b96633812a2b34ad34e8b91c393bdc5aecd6c735c9d4b` |
| `checks.json` | `0db19ec216c58438943d1b21de9111d0dac2d489a3f5622a25717a802b55ff47` |
| `documentation.log` | `ca48649354010a3d8726b885a71d6801ae3f1ede06986f93cf763f7d6c99ca62` |

## Boundaries

This repairs development statements before model measurement. No model/GPU run,
held-out access, generated scoring admission, harness/prompt tuning or product
permission change occurs. The parent still needs protected held-out inputs,
independent generated-result checks, aggregate worker quotas, reporting labels
and a real current-harness baseline. No long acceptance gate, supply-chain
cascade, native qualification, release or owner-test readiness is claimed.
Only the separate reviewer's exact-commit PASS can close this row.
