# T-303/T-306: Original-owner completion readiness contract

Date: 2026-11-09. Base:
`d0d7dc0f643e264036c5f1744ed2b2010dd5afaf`.
Scope: same-row core increment; neither parent closes.

The original combined coding owner now issues a private single-use readiness
token only after its original typed validation/source inspection succeeds.
The token binds the complete sealed request and exact turn and retains the
same original owner's post-generation check. The completion framer requires
and consumes this token, refuses a substituted request or turn before preparing
transport, and installs the second check before returning its prepared turn.
There is no production constructor outside the checked owner, serialization,
cloning, wire field, new executor, grant or ownership restoration. The existing
post-generation check still runs after model accounting and before independent
verification. Both continuous controls remain binding.

This addresses low L2 from the retained
[source-check independent review](../reviews/8aa5d391f0d8-local-coding-completion-source-check.txt).
Explicit test-only token issuance lets the synthetic completion grammar/socket
fixtures deliberately skip native readiness. Those fixtures exercise the same
framer; they do not establish native validation or weaken production. The
post-generation refusal fixture still uses the exact original owner guard.
It now supplies that callback through explicit test-only token issuance to the
shared production framer, which must install it itself. There is no fixture
override of the prepared turn's check. An invocation counter must equal one
for both original missing-validation and stopped-control refusals and stay one
after repeated terminal advance. Removing the framer's installation would fail
the invocation assertion. This addresses the incoming post-generation review's
callback-installation and invocation-count gaps; positive original-owner issuance
and other isolated host phase/owner refusal branches remain unobserved.
The new substitution regression uses that explicit synthetic issuance, including
changed request contents retaining the old digest, and observes no connection.
Positive original-owner token issuance still needs the native workflow.

Low L3 is addressed in the same row. The held-file completion refusal matrix
now asserts the exact error code for every case. It stages pending edits and
original-checkout applies through the existing public preview methods. For the
apply case an actual separately approved worktree file edit creates transferable
lineage before sealed validation capture; the later apply remains unapproved.
Refusal preserves the original file, pending worktree bytes, sentinel and
attempt count and dispatches no additional command. Creation provenance and
process results remain synthetic, as in the existing component fixture.

A separate binding component test uses actual held files and sealed capture
with an inert synthetic command record. It independently substitutes only the
command receipt identity, then only the sealed projection identity, while
asserting all other binding conjuncts still pass. Neither substitution satisfies
the completion binding predicate. This tests those equality clauses in isolation;
it is not command receipt authentication, native execution or independent task
grading.

## Verification

The repaired complete affected gate remains **FAIL**: 26 unfiltered Rust
targets, 1,219 passed, exactly 72 inherited failures and none new, 64 existing
ignored, zero measured and zero filtered. All eight named methods pass.
Formatting, Clippy, prose, retained-review and whitespace checks pass. The
exact failure/ignore identity comparison passes; it cannot make the gate green.

| Check | Exit | Seconds including reservation |
| --- | --- | --- |
| Formatting | 0 | 797.34 |
| Clippy, all affected targets | 0 | 13.62 |
| Tests, all affected targets, no fail-fast | 101 | 254.39 |
| Complete affected gate | 1 | 1067.40 |

Five Rust source hashes remain fixed through the repaired run. The raw log
SHA-256 is `ba2e42655f126690ea4de79361476956333478eef91041d4ee6a3b9a53f26a4d`.
Each Cargo command takes its own shared reservation, with two build jobs and
four test threads. Formatting spent most of its elapsed time queued for a slot;
this wait remains included. Only the retained original Node executable and
license supplement the tool path. Final prose and integrity checks run
separately after recording these results. The private reproduction bundle keeps
the raw logs, plans, failed source snapshots, final source freeze, exact baseline
comparison and helper inputs. The test fixtures are components, not a positive
native/model workflow or coding score.

Earlier attempts are retained separately. A preliminary `completion_` filtered
run compiled successfully and executed the host library: 12 passed, one inherited
`InvalidManifest` failure and 684 filtered. Cargo stopped after that failure, so
the Linux test target did not execute. This run predates the incoming review's
callback-counter repair and is not the final source gate.

The initial full gate found a new compile error: the callback-counter test lacked
its explicit `AtomicUsize` import. Formatting passed; Clippy and tests exited 101
without executing a Rust test target. The repair adds only that import and
preserves every assertion. Both the failed log and its five-source snapshot are
retained alongside the full repaired rerun. This compile error is not classified
as an inherited failure.

The [post-generation review](../reviews/d0d7dc0f643e-local-coding-completion-recheck.txt)
arrived during this batch with Verdict: PASS for the exact base commit and no
high/medium finding. It is retained byte-for-byte. No newer verdict was present
at session start. Its two low callback test observations are addressed as
described above; other isolated host refusal branches remain same-row backlog.
The unchanged T-301.1 host-trust blocker prevents positive native/model execution and the requested development-score rerun in this lane.
No score is inferred. Source observations stay non-atomic; the token cannot
prove future filesystem state or accept a task. Both completion verifiers and
verification flags remain unchanged. Independent completion acceptance,
native/model measurements, durable recovery, approved Git and parent acceptance
remain open. No hidden/held-out/frozen task, scorer, threshold, dependency,
supply-chain output, status model, scoreboard or release change. Independent
core review is required.
