# T-303/T-306: complete original-owner review deadline

Status: independent core review required.
T-303/T-306 remain open without new rows or sub-rows. Parent commit
`5d47bea006c79c25c64e1fd54f964d14e965f41d` remains immutable. Its independent
review returned PASS during this batch, without high or medium findings; the
report is retained exactly, alongside the traversal-budget PASS. Their carried
lows remain same-row backlog: privileged device-node substitution, deadline
test robustness, a stale execution comment and native forwarding coverage. The
new tests also use short real-time budgets (100/250 ms); severe scheduling
contention can cause a false failure before their intended stop point. This
limitation cannot produce a false pass and remains same-row backlog.

The original-owner candidate checker now receives one absolute monotonic
120-second deadline covering complete precheck, source capture, installed checker
and postcheck. All four complete Git observation samples retain the same deadline.
Preservation collections also retain their shorter 30-second cap. Linked
inventory and immutable-base reads forward the review deadline to the confined
inspection worker, which retains its shorter 15-second cap. Ordinary inventory
and separately approved inspection limits remain unchanged.

Expiry refuses before later observations and before releasing a checked candidate.
The checker receives the exact deadline alongside the unchanged passive caller
cancellation scope; a late success refuses, while rejection/unavailable still stop
immediately. Partial inventory and immutable-base maps never escape after expiry.
No manifest input, hash binding, packet/source/validation check or permission
boundary is removed.

The budget is cooperative. Bounded source capture is checked at its boundaries,
without deadline polling inside each source read. Individual filesystem/admission
calls and a blocking checker are not forcibly interrupted; mandatory worker
cleanup can outlast expiry.
The host must honor the deadline inside its installed checker. No hard return-time
guarantee or actual independent checker implementation is claimed.

## Validation

Five new regression methods cover shared
deadline/cancellation forwarding, all four Git boundaries, expired preflight,
a worker thread observing checker expiry, late success/rejection/unavailable,
all seven linked inventory reads including absent symbolic HEAD, and all four
base type/tree/blob reads. Fixtures use real local files and synthetic Git and
checker observations, with actual monotonic expiry. No native Git or local-model
result is claimed.

The complete affected gate remains **FAIL**: **1,460 passed, 72 failed,
64 ignored**, zero measured or filtered, across **26 unfiltered targets**. The
failed and ignored identities exactly match the hash-verified parent baseline
per target. All five new methods pass. Of 69 methods directly in changed sources,
56 pass and 13 have inherited failures. All eight Rust source hashes remain fixed
throughout the full run and documentation finalization. Formatting and Clippy
with warnings denied pass; every pre-test check passes. No timeout occurs.

The run takes **247.59 seconds** including reservations: formatting 1.35 seconds,
Clippy 10.17 seconds and Rust tests 234.54 seconds. Raw log SHA-256:
`6b91f423dc706e1a56cb100d2906303df79ae254725010788c3796704975fa54`.
An initial focused compile exposed a module-visibility error, corrected before
the final source freeze; focused checks do not substitute for this full run.
No mutation campaign is claimed.

The parent review arrived during the full run and is retained afterward without
changing Rust. Separate final light checks cover both retained reports and the
final documentation. Fresh reviewers check all inputs. The private reproduction
bundle is `/lane/state/tmp/t306-review-deadline`: read README, runner, comparison,
parser and light checker first. The runner reserves heavy commands internally;
never outer-wrap it. Use fresh output and an independent target for review. The
comparison requires exact failure/ignore identities, complete target totals, all
new passes and unchanged source hashes. Its exit zero verifies data only; the
full gate remains FAIL.

## Remaining work

Native executable ownership admission remains blocked in this lane. Actual
independent checker/artifact integration, complete original-owner approval,
repository/signer recovery, native/model measurements and approved push remain
open. No product commit route or row closure is introduced. Evaluation inputs,
hidden tests, held-out tasks, prompts, thresholds, dependencies, SBOM, lifecycle
and scoreboard are unchanged. No GPU work, keys, downloads, pushes, release or
owner readiness action occurs.
