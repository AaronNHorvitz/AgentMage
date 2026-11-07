# Original-checkout diff and exact apply approval — 2026-11-07

T-303 and T-306 remain open. This batch extends the existing manual Linux coding
repository owner, versioned local HTTP API and terminal client. It does not add
another agent loop or compose that manual owner into the native coordinator.
[Decision 0156](../decisions/0156-original-checkout-apply.md) records the boundary;
the [guide](../guides/coding-original-apply.md) describes the commands and recovery.

## Behavior

Successful native worktree creation retains both held roots. Only committed
native worktree edits establish transferable source lineage: the first preimage
and most recent postimage of at most sixteen paths. Repeated edits must match
the preceding postimage. An unsuccessful/uncertain write dispatch removes those
paths' transfer eligibility. The public owner has no synthetic creation path.

Explicit read consent prepares a complete, inert original-checkout diff from
selected eligible paths. The original must still match the first preimage and
the worktree the last committed postimage. A separate exact approval consumes a
new kernel write grant through the existing shadow-change and atomic-write
owners. Both roots, creation provenance, the complete diff and expiry are bound.
The existing writer checks all original preimages before replacement and
reconciles partial failures. No new patch applier or Git write command is added.

Denial writes nothing; a foreign digest preserves the preview. Matching expiry,
clock rollback, drift and dispatch consume the preview. Later worktree write
dispatch invalidates pending original consent; original transfer dispatch
invalidates an older pending worktree edit. A committed transfer consumes its
selected lineage. Restart restores no ownership, lineage or pending approval.
Before write dispatch the process records uncertainty; a lost reply requires
manual inspection. Results retain native transaction receipts but remain
process-only, without a durable coding-run claim. `tested` stays false.

The API accepts no original root, worktree root, patch, answer or creation receipt
from its preparation body. The terminal verifies the selected roots, displays
complete escaped consent and requires separate exact controlling-terminal
answers. It sends a consequential decision once. All routes retain control-token,
loopback/origin/framing and strict JSON requirements. Symlinks, hardlinks, mount
crossings, metadata, new files and deletion retain existing refusals.

The prior [independent context review](../reviews/d6cc0a26b55a-local-coding-context-retrieval.txt)
is retained unchanged. Its low L1 is fixed here: the context owner decodes the
native repository JSON envelope before scoring its content, so newline/tab and
Unicode escapes cannot hide line-initial words. Original excerpt/provenance
bytes, authority tiers, exact token accounting and native admission are unchanged.
A regression uses the production JSON envelope. Low L2 (conflicting identity
reported as a resource failure) remains backlog; duplicate identities still
refuse. This is not a measured coding-quality improvement.

## Verification

Initial reserved native component observation: all five then-present methods
passed (294 other methods explicitly filtered). A real-file workflow with
synthetic creation provenance performed four separately approved worktree edits,
two original transfers and three transferred file versions, preserving unrelated
human and index sentinels. It took 326,342 microseconds in that observation.
This is component latency, not a model run, GPU time, native Git creation or a
coding score. Four further methods were added before final full verification.

The initial reserved worktree API/terminal target passed all twelve then-present
methods with no skips or filters; a thirteenth preparation-consent method was
added before final full verification. The synthetic terminal peer includes complete original-apply
preview display, exact consent, incorrect answers and root replacement during
consent. Real API process tests cover control credentials, read consent, exact
objects/unknown fields, no ownership adoption and restart. Native Git creation
still reports its unchanged executable-ownership refusal. Every synthetic peer
and synthetic creation identity remains test-only.

The final reserved fast gate selected the complete host, local API and Linux
all-target suites, with no test filters or failure allowlist. Formatting and
Clippy with warnings denied passed. Tests recorded **888 passed, 72 failed,
62 existing ignored and 0 filtered**. All seventy-two failure identities exactly
match the retained host/Linux baseline logs: fifty-three host and nineteen Linux
failures, with no new identity. The complete gate is **FAIL**, not PASS; the
unchanged executable-trust blocker remains deferred under T-301.1.

All nine original-apply component methods, all six context retrieval methods,
and all thirteen worktree API/terminal methods pass in that full run. The local
API's complete ninety-four contract methods pass with no skips or filters. The
nine new native methods cover cumulative edits and separate transfers, preservation
of unrelated human/index bytes, refusal without creation/edit provenance,
duplicate/reserved paths, source and root substitutions, creation-receipt drift,
wrong digests, denial, expiry, clock rollback, symlinks/hardlinks, multi-file
preimage refusal before any write, preview invalidation in both directions,
broken/uncertain edit lineage, the cumulative sixteen-path bound and restart.
The extra terminal method verifies the complete read disclosure for both roots
and refuses substitution of either root while consent is pending.

The source freeze binds all ten changed Rust files across the final gate.
The publication audit checks the exact failure set, unchanged old ledger and
checkboxes, byte-identical prior review, source/evaluation/dependency/status
boundaries and public hygiene. The complete tracked/current source-mirror policy
validator passes all 781 Markdown files and current policy invariants; ignored
build caches are excluded, not policy inputs. This is not the T-310 long gate.
Changed Markdown, review-format and whitespace checks pass. Shared reservation
waits were honored. Initial logs remain retained; no failed result is removed or
relabelled.

Raw commands, complete output, source hashes and the publication audit are kept
privately under `<lane-state>/tmp/t306-original-apply/`. They are builder evidence,
not an independent verdict. The exact commit requires independent core review.

## Open obligations

Actual Git worktree creation and positive confined execution still hit T-301.1's
unchanged native executable-trust blocker. It is deferred, not bypassed. The
native coordinator requires exact admitted model and rendered-token facts;
the configured chat endpoint does not establish them. Connecting the manual
worktree/edit/test/apply owner to that coordinator, durable HTTP recovery,
approved branch commit/push, MCP and measured parent acceptance remain open.

No model/GPU run, scoreboard entry, held-out execution, hidden-test change,
threshold change, dependency/lockfile change, supply-chain regeneration,
status-model change, push, release or owner-ready declaration occurs. The frozen
development campaign is still required for measured parent acceptance; held-out
tasks run only for row-closing scoreboards. No row or sub-row is added or closed.
