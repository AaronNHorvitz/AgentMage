# T-311.1 development evaluator pilot — 2026-11-03

Status: builder implementation; independent core review pending. T-311.1 and
parent T-311 remain open. This is an executable evaluator pilot, not the complete
30-task suite, a local-model coding baseline or a scoreboard entry.

## Scope and frozen inputs

The [guide](../guides/coding-evaluation.md) describes three real tasks from this
repository's own history. The [runner](../../scripts/coding_eval.py) pins each
statement, base, source projection, editable path, historical reference source,
hidden regression module and exact test count. All tasks are development tasks;
no held-out set has been selected, read, tuned against or scored. No existing
agent harness, model prompt, hidden test or task threshold changed.

The editing projection withholds Git history, tests and answers. Scoring refuses
changes outside one source file, additions/deletions, aliases, hard links, special
files, extra directories and oversize inputs. Held directory/file descriptors
bind a copied candidate before grading. A fresh candidate-only repository in the
grader supports diagnostics that read Git; it contains one commit and no original
history, answers, remotes or hooks. It is read-only during testing.

The worker has isolated network/PID namespaces, read-only runtime/code/tests,
temporary scratch and a temporary writable result directory. The parent keeps
raw logs outside that mount, stopping the worker on timeout or either stream
exceeding 2 MiB. Exact counts are required; skips, missing/malformed results,
changed hashes and unavailable confinement cannot pass. No product permission,
admission, executable trust, sandbox or release boundary changed.

## Evaluator discrimination

One reserved command scores every frozen base and reference with the same hidden
tests. References validate the scorer; they never count as agent/model attempts.

```sh
bash /tools/build-slot python3 scripts/coding_eval.py verify --output <new-private-results>
```

Final observed aggregate: **PASS**. All six cases retain raw stdout/stderr,
exit status, exact counts, wall time and input/log hashes. Every case has zero
skips/expected failures; model attempts are zero, GPU minutes zero and tokens
unavailable because this command runs no model.

| Task | Candidate | Tests | Failures / errors | Result | Wall seconds |
| --- | --- | ---: | --- | --- | ---: |
| dev-001 | base | 12 | 0 / 3 | FAIL | 0.322808 |
| dev-001 | reference | 12 | 0 / 0 | PASS | 0.313255 |
| dev-002 | base | 3 | 0 / 19 | FAIL | 0.278779 |
| dev-002 | reference | 3 | 0 / 0 | PASS | 0.395918 |
| dev-003 | base | 23 | 2 / 3 | FAIL | 2.375230 |
| dev-003 | reference | 23 | 0 / 0 | PASS | 2.356219 |

Initial reserved validation **failed**, and is retained. dev-002 and dev-003
references passed, but dev-001's reference had one Git-dependent diagnostic error:
the original grading projection lacked repository metadata. The correction adds
a fresh candidate-only Git repository before its read-only mount. It does not
alter any test, assertion, task pin, expected count or score threshold.

| Initial task | Candidate | Tests | Failures / errors | Result | Wall seconds |
| --- | --- | ---: | --- | --- | ---: |
| dev-001 | base | 12 | 0 / 3 | FAIL | 0.182038 |
| dev-001 | reference | 12 | 0 / 1 | FAIL | 0.180106 |
| dev-002 | base | 3 | 0 / 19 | FAIL | 0.143689 |
| dev-002 | reference | 3 | 0 / 0 | PASS | 0.260474 |
| dev-003 | base | 23 | 2 / 3 | FAIL | 2.279931 |
| dev-003 | reference | 23 | 0 / 0 | PASS | 2.309771 |

A second reserved run stopped with exit 2 after dev-001 base/reference and
dev-002 test counts were recorded, before dev-002's final report. Its generic
operator diagnostic did not retain an exception traceback. The targeted dev-002
base repeat finalized normally without changing the evaluator. A fresh full
rerun with private traceback capture passed, followed by another complete PASS
through the documented CLI. The incomplete run remains a failure; its cause is
unresolved for independent review. No result from it is counted as an aggregate
PASS. Both successful full runs and incomplete artifacts remain retained.

An earlier actual boundary fixture also failed because bubblewrap requires an
explicit user-namespace flag before nested user namespaces are disabled. Adding
that flag corrected launch without relaxing confinement. The failed gate log is
retained. The initial source commit `4e886ebb` recorded measurements as pending
before the reserved results were inspected; its exact evaluator hash matches
all six initial case reports.

## Actual current-harness readiness attempt

The existing compiled coding CLI was requested with dev-001's real objective and
its historical source projection, through both build and GPU reservations. It
used the existing named local candidate selection, the existing development
scenario, closed standard input and no approval grant. It was not a substitute
direct-model loop. Binary identities and the exact private command are retained.

| Observation | Actual result |
| --- | --- |
| Reservation | Build slot, then queued GPU slot; 120-second outer ceiling; wrapper exit 0 means recording/cleanup completed, not coding success |
| Native CLI exit | 5 |
| Native CLI wall time | 0.020586 s; excludes queuing, projection preparation, binary hashing and reservation overhead |
| Standard output | Empty |
| Standard error | `Error: linux.repository.git_artifact.invalid`, followed by `linux.development.launch-envelope.failed` |
| Workspace change after invocation | None in Git status |
| Local model inference | Not observed; refused before inference |
| Tokens / GPU minutes | Unavailable; reservation duration was not measured |
| Coding baseline / model score | Unavailable / null; no first-attempt or retry score |

The readiness diagnostic reports all four required native executable paths as
untrusted and does not probe an untrusted service manager. This is consistent
with deferred T-301.1. No ownership, permission, activation or admission check was
changed, and no identical refusal was retried. The probe does not qualify later
prerequisites or establish native host acceptance.

The probe records source HEAD `25c580f871653171543932ca1aa14dae384dc138` with
the new evaluator files still uncommitted. It is not a clean-source baseline.
Existing coding implementation files did not change, but the binary build source
was not re-established: the following are observed executable hashes, not a
reproducible-build or exact-commit source claim.

| Observed executable | SHA-256 |
| --- | --- |
| Coding CLI | `4565f74a7a98e5756ec98c8580c16b31666602ad8f8844184a441fbd37253d38` |
| Host | `a380ee28f25f7623a7f9b7358d4cd4ca7990ba09460be15cecdb8e8f50b9e186` |
| Read-only worker | `a790706335fdf087870e253a6fa2ada5d5d2eb74a421e54f359bda69ef0f8696` |

The slot wrapper completed its cleanup check. No owned model process remained;
no model was downloaded, no model inference was observed, and no production
run, completion or coding quality is asserted. Raw logs contain private paths
and are retained privately, with hashes below instead of public transcripts.

## Per-change verification and retained records

The fast gate selects changed Markdown, Python syntax/indentation/whitespace and
`tests.test_coding_eval`. The corrected source gate passes **17 tests**, no failures
or skips, in 0.335 s. These include an actual worker's read-only mounts and blocked
connection to a host listener, candidate/result refusals, timeout/output limits,
frozen definitions and answer-free grading Git metadata. Synthetic unit fixtures
are not counted as real repository or model task outcomes. No Rust crate changed;
this does not resolve deferred T-301.1 or establish a green full Rust gate.

| Artifact | SHA-256 |
| --- | --- |
| `scripts/coding_eval.py` | `6663c4628dad9df7bf81ed296fb367656a979e57a7c5b42245d733721afd6901` |
| `tests/test_coding_eval.py` | `6769d7b4a4e25a8a820aa1be653588003869135875a4311059016090e573c1d8` |
| Initial evaluator summary (private) | `bb0ca62d39ac0d9e7bc5ae4eea9c05f2103587c1ebb19d735d3a5b45917453ad` |
| Native readiness record (private) | `ea8f03aac02ebac811bbac6f67d24a65edd60d6d903b09ddcbb303d40125ca1a` |
| Native stderr (private) | `5579101cf478cccc486cea8aa897284fb6c252c158c45681e41415d34fb03c79` |
| Corrected source gate raw log (private) | `9c11813f9e924dac2b9d7f41d5810388773acbc9574de39ee00219cd448337e9` |
| Incomplete run dev-001 reference report (private) | `a0f40218ab1b4104f1f322f5690b8315335b2bc16328b75430e00bf742da3e31` |
| Incomplete run dev-002 test counts (private) | `a9ecf819356d9af2f37d72707b202481c21d7f19944cd55323564a1b668819a6` |
| Targeted dev-002 repeat report (private) | `70e61da208a83b0f1fa11e84390a31df620f41816c0d126c0d1d2404923d8d92` |
| Final CLI evaluator summary (private) | `e3f7da4809b1f7e9feb550bbfaadb12324de3d5a3761a943bc3f884969e710a3` |
| Earlier corrected full evaluator summary (private) | `0a4ad07ad0c81a51053c7046ffee71e8530970da2ce202a7880f86f8a8291435` |
| Ordinary docs validator log (private) | `978019c7f170d3e55ca1494eb05210b76d551a56bef3bf214389d0749a6bf9d0` |

Task-definition SHA-256 remains
`f950e413f8dc4bf7ff64e323a5b78c2ba018091b2429e8999eb5850de6b1c00a`.
Public source, evidence, guide and queue receive scoped hygiene/format checks;
private raw records retain host-specific paths. No dependency, licence notice,
Cargo lock, status model, old ledger, SBOM or historical binding was changed.
The ordinary reserved documentation validator passed all 693 Markdown files
and policy invariants. The long acceptance gate remains deferred to T-310.

## Remaining work and limits

The scorer imports candidate Python into its test interpreter. Tests are withheld
during editing, but this is not a proof against deliberate test-framework/result
tampering. Independent checking is still needed before a trusted generated-candidate
scoreboard. This pilot evaluates no model-generated candidate.

Full T-311 still needs at least 30 varied real tasks, a sealed held-out collection,
a bounded/resumable current-harness adapter and actual local-model attempts with
first-attempt/retry counts, wall/GPU time, tokens and failure causes. T-311.1 is a
bounded executable scoring pilot and its readiness refusal, not a replacement for
that work. Agent tuning remains deferred. No row closes without an exact-commit
independent core PASS with no high/medium finding. No public release or owner-test
readiness is authorized or claimed.
