# T-303/T-306: Fixed-test evidence correlation repair

Date: 2026-11-09. Base:
`9ac41a62c20998d96709cbd0a658fecfbfc4b973`.
Same-row core repair; neither parent closes and no sub-row is added.

## Finding and change

The [independent review](../reviews/c27e2bee3611-local-coding-fixed-test-checker.txt)
found medium M1 on exact commit
`c27e2bee3611f384bb57be96f15a07a929ec074f`: the original coordinator sorts
aggregate evidence by ID, while the checker compared it in edit/test order.
Native command IDs sort before native edit IDs, so genuine records could never
pass. The earlier fixture's sequential evidence names concealed this defect.
The retained review is byte-for-byte unchanged. The later launch connection
inherits this defect; its API gate did not test host evidence correlation.

The repair requires aggregate evidence to be exactly the set of each result's
single reference, matching unique IDs and every reference field. Equal lengths,
unique aggregate IDs and unique result reference IDs exclude missing, extra and
duplicate evidence. Tool results, receipt IDs, original attempts and rerun plans
still correlate chronologically; their ordering is not relaxed. All command,
receipt, scope, payload and complete-history checks remain in force.

The guide also addresses low L1: display the executable, arguments, test kind,
parser and minimum alongside the criterion digest before approval. Criterion
bytes and startup contracts do not change. This is guidance for embedding
programs, not a claim of a new implemented approval interface.

## Verification

The pre-fix regression run reproduces M1: five tests pass and the two positive
checks fail with native-form sorted evidence (exit 101; 698 filtered). The
unchanged checker and new fixture snapshot are retained. After the repair, all
seven checker methods pass (exit zero; 698 filtered). These focused runs are
component checks. Regression fixtures use native-form payload-derived IDs
sorted as the coordinator sorts them. Positive checks cover one to three attempts, including
earlier failures, sorted and permuted aggregate evidence. Negative checks cover
missing, extra, duplicate and altered references, duplicate result references,
and reordered tool results or receipts. Existing adversarial tests remain.

The final complete affected fast gate remains **FAIL**: 21 unfiltered host/API
targets, 891 passed, exactly 53 inherited host failure identities, 11 existing
ignored and zero measured or filtered. There are no new test failures or ignores.
All 12 checked methods pass, including all seven checker methods. Formatting,
Clippy and all four prose/whitespace/review checks pass. Both Rust source hashes
remain fixed throughout both full runs and match the final files.

| Check | Exit | Seconds including reservation |
| --- | --- | --- |
| Formatting | 0 | 0.95 |
| Clippy, all affected targets | 0 | 0.20 |
| Tests, all affected targets, no fail-fast | 101 | 128.82 |
| Complete affected gate | 1 | 131.50 |

Final raw log SHA-256:
`b09e98e9735a19923828f651d7a521a1fdeefcdaa1e5d0e4c86e584ca3dee011`.
The first full run also has exactly those test counts, but its Markdown check
failed on an added blank line. That prose defect is corrected; both full logs
and the intervening passing light checks are retained. An initial comparison
helper used older API counts; it was corrected against the base commit's
retained API gate before final comparison. No source or test result changed.
The baseline combines the unchanged host targets with the base's 11 API targets.
A matching baseline does not make the failing gate green.

Each Cargo command owns its shared reservation, with two build jobs and four
test threads. Reproduction helpers retain exact commands, per-target counts,
failure and ignore identities, source freezes and raw logs. Final privacy,
review-byte and unchanged-ledger-row checks pass. No new failure allowance,
ignore, assertion weakening or test threshold change is introduced.

These are pure checker fixtures with synthetic command/authority records and a
controlled claim from the original coordinator; records are substituted by the
probe, which deliberately returns an error. They do not execute native tools,
exercise original-owner positive issuance or grade a coding task. Original
source/validation checks, approvals and native trust admission are unchanged.

## Remaining work

Independent core re-review is required. T-303/T-306 remain open. Positive native
and model execution, original-owner positive dispatch, durable recovery,
approved Git and development measurements remain open under the existing
host-trust blocker. Prior same-row lows remain backlog. No hidden/held-out task,
evaluation definition, scorer, threshold, dependency, supply-chain output,
status, scoreboard or release state changes. No model or GPU is used.
