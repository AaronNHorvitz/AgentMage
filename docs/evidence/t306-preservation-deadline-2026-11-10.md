# T-303/T-306: one preservation collection deadline

Status: independent core review required. T-303/T-306 remain open without
new rows or sub-rows. Parent commit
`4c8c2dd416b5356fb0c66c6265603203f42d226a` remains under independent review;
its ancestry is preserved. Prior low findings remain same-row backlog.

A full preservation collection previously started a fresh timeout at each Git
read. Local scans checked cancellation and resource limits but no shared time
budget. Now each collection starts one monotonic 30-second observation budget
before executable admission. That absolute deadline reaches every Git read and
local scan. Each inspection worker uses the earlier of the original 15-second
worker limit and the remaining collection deadline. Other inventory and
independently approved inspections retain their original per-worker limit.

Expired collections stop before the next observation, around each bounded file
read chunk and before accepting a final manifest. Late success and optional
missing-reference results refuse. The original cancellation scope remains
unchanged. No partial manifest, retry authority or successful effect receipt is
inferred. All preservation fields, hash inputs, sorted record encodings and
resource limits remain bound as before.

Individual admission or filesystem calls are not preempted. Mandatory supervisor
cleanup may finish after expiry. Multiple collections within an original-owner
review do not share a review deadline, and installed checker time remains the
host's responsibility. This is not a hard total return-time guarantee.

## Validation

Five new regression methods cover unchanged complete manifest bytes and the
same deadline at all 17 worker calls; expiry at the first and final worker with
and without a caller token; expired-before-filesystem refusal; the worker's
minimum deadline calculation; request deadline forwarding; late success and
both missing-ref exit codes; and real local-file expiry after the first 64 KiB
chunk with every local entry point refusing afterward. Fixtures use real local
files and synthetic Git/worker output. No native Git launch or model-quality
result is claimed.

The focused selection passes all 21 methods with 567 filtered in 4.00 seconds
of test execution. These filtered results are not a complete affected gate.
The complete affected gate remains **FAIL**: **1,455 passed, 72 failed,
64 ignored**, zero measured or filtered, over **26 unfiltered targets**. The
failed and ignored identities exactly match the hash-verified parent baseline
per target. All five new methods pass. Of the 44 test methods directly in changed
sources, 31 pass and 13 have inherited failures. All seven Rust source hashes
remain fixed throughout the full run and after documentation finalization.
Formatting and Clippy with warnings denied pass. No timeout occurs.

The initial complete run also records one Markdown lint failure for an extra
blank line in the guide. It is corrected during the run without changing Rust;
the separate final light checks pass afterward. The raw full run remains FAIL,
and its earlier documentation failure is retained. Rust checks are not repeated
for that documentation-only correction.

The full run takes **255.87 seconds**, including reservations: formatting
1.31 seconds, Clippy 9.53 seconds and Rust tests 243.56 seconds. Raw log SHA-256:
`908a7402b8b8bc6ea53d4a5fe3a57ac37f09918ff4d83f50408d41ba2e875fe1`.
No mutation campaign is claimed.

The private reproduction bundle is `/lane/state/tmp/t306-preservation-deadline`.
Read its README, runner, comparison, parser and light checker before execution.
The runner reserves heavy commands internally; never outer-wrap it. Supply a
fresh output directory and independent target for review. The comparison checks
all target counts, exact failed/ignored identities and all new passes; exit zero
means data verification only. Its one documented builder-log exception binds
only the exact raw hash above and the corrected Markdown failure; fresh review
runs must pass all pre-test checks. Source and Cargo commands remain identical.
No gate is relabeled green.

## Remaining work

Native executable admission remains blocked by unmapped system ownership in
the lane. The trust boundary is preserved. Actual independent checker and
artifact integration, complete original-owner commit approval, repository and
signer recovery, native/model measurements and approved push remain open. No
product commit route or row closure is introduced. Evaluation inputs, hidden
tests, held-out tasks, prompts, thresholds, dependencies, SBOM, lifecycle and
scoreboard are unchanged. No GPU work, host keys, downloads, pushes, release or
owner readiness action occurs.
