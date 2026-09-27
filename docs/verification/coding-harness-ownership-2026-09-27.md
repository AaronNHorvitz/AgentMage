# Coding developer harness ownership observations

Date: 2026-09-27. Base commit: `42c1a3b08692ea8e14af6bd8ee5edadfac801c21`.
The [companion record](coding-harness-ownership-2026-09-27.json) binds the exact
source, tests, authorities and retained log hashes. This verifies developer-wrapper
components under [Decision 0089](../decisions/0089-development-harness-process-ownership.md).
It does not close a roadmap, independent-review, human or release gate.

## Reproduced defect and correction

A rejected start removed a competing run record despite launching no child. The
private reproduction mocked preflight and binary observation; its child launch
count was zero and `competing_record_preserved` was false. The old stop path also
used a bare PID after checking only executable and loose root argument identity.

The existing wrapper now exclusively reserves a versioned private record before
launch. Held descriptors, bounded reads, closed fields, file metadata, link count
and expected bytes constrain publication and cleanup. A different or uncertain
record is preserved. Cleanup attempts the owned child, streams, reservation and
resource guard; failure cannot produce a successful terminal result file.

The child record binds Linux boot ID, UID and kernel start ticks. Cancellation
pins a process descriptor before checking that identity, executable, each root
argument and the current record. Signals use the descriptor without a PID-only
fallback. Native key readiness remains required. Legacy records and stale
reservations are not adopted or silently removed. Diagnosis distinguishes them
from a root ready for a new invocation.

These are cooperative private-state checks. They do not establish protection from
an adversarial process sharing the same UID or replace native Rust admission.
No native source, model profile, schema, confinement rule or authority changed.

## Verification

All test commands ran through the configured shared build reservation.

| Check | Result | Meaning |
| --- | --- | --- |
| New ownership suite | 20 passed | Record contention, launch/publication failure, cleanup replacement/mutation, malformed records, exact identity, cancellation and lifecycle diagnostics |
| Existing harness, model-evidence and coding corpus tests | 23 passed | Existing wrapper, retained model evidence rules and corpus expectations preserved |
| Context registration and work selection tests | 33 passed | Existing planning/governance checks preserved |
| Task graph, context registration, status and supply chain | Passed | Current validators; supply-chain check only, no workspace source or SBOM change |
| Changed guide and decision Markdown; frozen source hashes | Passed | Exact tested source retained |

The ownership suite includes a real Linux child created and reaped solely by the
test. Its installed SIGINT handler returned the expected exit code 73 when
`stop` signalled through a real process descriptor. The test used synthetic root
arguments and a synthetic key file. It did not launch the AgentMage CLI, host,
native tools or a model, and provides no workflow qualification.

The first focused run had 17 passes and one failure: a test mocked every path
resolution as the executable, unintentionally changing its expected roots. The
mock was narrowed to the process executable. A subsequent 41-test run passed;
after adding bounded reader-contention and diagnostic-state coverage, the final
43-test run passed. The failed log and earlier reproduction remain retained.

The direct binding inventory found three earlier research component records bound
to the previous wrapper bytes. Those reports retain their original tested source
pins and complete input sets; they are historical for this changed input. This
record does not assert whole-project or transitive evidence freshness.

Reproduce the wrapper and evidence checks with:

```sh
bash /tools/build-slot python3 -m unittest \
  tests.test_coding_harness_ownership tests.test_coding_harness \
  tests.test_coding_harness_model_evidence tests.test_story_48_coding_harness_corpus
```

## Separate acceptance results

Scripted AgentMage workflow: still blocked by the lane's native executable trust
and unavailable user-systemd bus, as recorded in the
[restart reconciliation](lane-reconciliation-2026-09-27.md). That launch was not
repeated or presented as successful. Real-model qualification was not run; earlier
Muse observations retain their original source/profile scope. Manual user testing,
independent acceptance and release approval remain open. No GPU/model process or
host installation was started. Launch instructions and these limits remain in
[local testing](../LOCAL-TESTING.md).
