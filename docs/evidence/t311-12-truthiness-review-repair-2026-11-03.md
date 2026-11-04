# T-311.12 predicate truthiness review repair

Status: builder verification passed; independent re-review required; T-311.12 and parent T-311 stay open.
Implementation base: `694808d7`; fixes medium M1 in the
[independent first review](../reviews/9563c0a80d55-local-coding-t311-12-failure-causes.txt)
of `9563c0a80d5599ac0b8959558664f314f91b31b9`.

## Correction

The initial batch carried integer truthiness for verification report predicates
only. Its claim did not apply to dev-015 and dev-029: their independent adapters
still imposed exact Boolean results even though the frozen tests assert
truthiness. This repair carries finite JSON scalars for dev-015 and string-keyed
maps of bounded JSON values for dev-029, including nulls and empty maps. Original
assertions decide their results. Reports now explicitly label exact Boolean result
types as ungraded for both tasks. The adapter version advances to v7.

Reserved policy and exception tags retain their exact, sticky semantics. A policy
tag remains protocol failure for these two tasks. Wire bounds, counts and causes
are unchanged; unsupported wire shapes remain protocol failures. The worker
still fails unknown result types. No hidden bytes, task statements, source pins,
method selections, historical outcome profiles or thresholds change. New tests
replace the adapter's mistaken exact-Boolean expectations, preserving the actual
frozen assertions and adding independent/legacy comparisons for accepted and
rejected truthiness observations.

Review low L1 remains backlog: observations exceeding the frame or integer-digit
encoding limit currently fail as protocol with candidate exit 1. The guide now
states that limitation. It affects cause precision; each attempt still fails and
remains counted. Generated scoring stays refused. Twenty independent adapters,
aggregate quotas, protected held-out inputs, covered-clause admission and the
current-harness real-model baseline remain open. No GPU, model, held-out run,
harness tuning, download, long acceptance gate, supply-chain regeneration,
release or owner-readiness claim.

## Verification

All heavy checks used the shared build reservation. The initial 67-test oracle
suite passed in 29.753 seconds. The final scoped gate repeats those tests after
the final source edit. Times below exclude the outer reservation wait.

| Check | Result | Seconds |
| --- | --- | --- |
| Scoped fast gate | PASS; 67 oracle tests | 31.21628 |
| Existing evaluator unit suites | PASS; 58 tests | 19.71305 |
| Independent CLI verifier | PASS; ten pairs, 13 methods, 149 reference calls | 5.050283 |
| Unchanged CLI verifier | PASS; 30 pairs, 89 reference tests, no skips | 43.924432 |
| Persistent confined probes | PASS; 14 probes and nine legacy contrasts | 3.960425 |
| Documentation policy validator | PASS; 710 Markdown files and policy invariants | 133.405204 |

The independent controls retain twenty invocations: ten failing bases and ten
passing references. Exact historical profiles and all failed-attempt counts are
unchanged. These are controls, with no model attempts or score.

| Persistent probe | Observed result |
| --- | --- |
| Reference scalar and map predicates converted to integers | Both graders pass both tasks |
| Reference false scalar and map values converted to null | Both graders pass both tasks |
| Scalar string and float truthiness; map list truthiness | Both graders pass all three |
| Inverted scalar and map predicates | Original assertions fail in both graders; independent cause is test_failure |
| Policy tag on scalar and map tasks | Both refuse as protocol, counted failures |
| Oversized scalar/map result and unencodable helper integer | All fail as protocol with candidate exit 1; low L1 reproduced and documented |

The nine legacy contrasts compare identical candidate bytes. Diagnostic probes
patch only historical admission in a private host process; production CLI and
`grade` continue to reject modified snapshots. Unit tests also check scalar type
preservation, empty maps, nested JSON map observations, depth and finite-number
bounds, malformed frames, exact sticky reserved tags, startup, isolation,
interruption, output limits, cleanup and failed-attempt accounting. None edits
an original hidden method or weakens a threshold.

All thirty frozen base/reference source pins and hidden-test hashes were audited.
The six legacy evaluator source/test modules are byte-identical to the base.
Collection v9 remains
`0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570`.
All 103 retained report/log bindings were audited (20 independent controls,
60 legacy controls, 14 independent probes, nine legacy contrasts). Independent
reports match their oracle-owned counts and final source digests. The first
independent review is retained unchanged. Raw logs and diagnostic source remain
private; artifact labels below are relative to the private batch record.

## Source and artifact bindings

| Source | SHA-256 |
| --- | --- |
| `scripts/coding_eval_oracle.py` | `b9f422e93d38d90673ede9a047174a4c782098a53d6c3db382f99785e132946e` |
| `scripts/coding_eval_oracle_worker.py` | `0d873422c537cfb339a71269f280fc7654007b2704d7250ce6f48658d891174c` |
| `tests/test_coding_eval_oracle.py` | `0d8156709a4f572e75616feadd8ee1991c855fd025038488019e9fa948524bf1` |
| `tests/test_coding_eval_oracle_worker.py` | `626e7cb298fecf7dee298098cf57db9fb5cb1bf4bfe7d156eedbd76fdf8004f2` |

| Artifact | SHA-256 |
| --- | --- |
| `initial-units.log` | `6fdcb057550d8515e55ff5c14ec5148490becbfde2428085b87388239fe85156` |
| `gate.log` | `dc6f8234d5af35ff0753dcf20b7c8a75066cee0ed8cc623bba808f51e4243014` |
| `legacy-units.log` | `6308a9bd5d208d83ca7ae0a353dc61170ae87aacfde6c93377fc11e48eeca32f` |
| `controls/summary.json` | `30b8922f2913c709fcd79eaa1bed9684cb0ba986d791aa12d61e8a525b95ffdd` |
| `full-controls/summary.json` | `db58596a83fabb200b28e4eac2f56f12f43c83df8fc4c318cb09d48c7b928e8e` |
| `probes/summary.json` | `4e300a7ee284379c422e55f359e63e49ecb4aabd625264628f1dcb05fe44cfd1` |
| `probes.py` | `4415dd23015f4aeb321db21e7e867e73b76e62773dfed97c21537c550c1e62bb` |
| `policy.log` | `94c3d52b03ef7548e1f44f14182ddb85310e7bd694f41561a63e8ea6ffd7c4c3` |
| `checks.json` | `ad02c22cc86967406691df1ce1770756c4b8a6f203f9b7d74324e471b4d09d0e` |
| `audit.json` | `a3052cd8e7e6549e7ba41edeadb9730f6b3e2948198fcf8bdfa3eb14adde9c10` |

The first final Markdown check caught a duplicate trailing blank line in this
evidence file; it was removed and the failed lint log is retained privately.

Only queue and evidence text changed after these checks. Final Markdown,
whitespace, added-text hygiene and source-digest checks follow before commit.
Builder verification does not close T-311.12; exact-commit independent re-review
is required. Parent T-311 and its generated-scoring prerequisites stay open.
