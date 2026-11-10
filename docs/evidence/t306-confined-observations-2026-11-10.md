# T-306: Confine original repository observations

This same-row coding execution prerequisite implements Decision 0188 on immutable
base `544848fb28aacf1d5b03afc243defb5ba25b63fe`, which is under independent core
review when work begins. The earlier creation-history review returns PASS with
no high or medium finding and is retained byte-identically. No new row or
sub-row is added or closed. T-303/T-306 remain open. The immutable base's
mode-binding review arrives before commit with PASS and no new finding; it is
also retained byte-identically. Earlier unrelated lows remain same-row backlog.

Original preservation reads previously launched Git through direct `output()`;
ordinary inventory used a bounded direct child but lacked the worker's host
filesystem and network confinement. Both now use the existing offline read-only
inspection worker, matching linked inventory. A private original-scope request
maps the exact selected metadata below the checkout into `/work`, including
non-default metadata names. Linked requests retain their descriptor-bound
metadata selection. There is no ambient repository discovery or unsandboxed
fallback. Existing executable trust, supervision, descendant cleanup, deadlines,
seccomp and read-only mounts are reused without an exception or limit increase.
Ordinary output now also refuses beyond the worker's existing 4 MiB ceiling.

Five new regression methods exercise exact metadata selection rather than a
misleading `.git`, small and maximum output budgets, complete and failed worker
results, missing refs, pre- and post-dispatch cancellation, malformed output,
root replacement and metadata aliases before or during a read, and linked
pointer changes. They use synthetic worker observations with real scope and
held-root checks. They do not launch native Git or establish native admission.
The existing direct-child timeout regression remains unchanged, with its helper
compiled only in tests. Existing supervisor projection/admission tests still
exercise the real supervisor with inert artifacts and precancellation.

The complete affected fast gate remains **FAIL**: **1,321 Rust passes, 72
inherited failure identities and 64 existing ignores** across **26 unfiltered
targets**, with zero measured or filtered tests. Every failure and ignore
identity matches the preceding complete run exactly; there are five additional
Linux passes. All five new methods, three affected controls, formatting and
Clippy pass. Three Rust source hashes remain unchanged throughout verification.
The exact per-target comparison is data only; it does not make the gate pass.
No failure allowlist, new ignore or threshold reduction is introduced.

The complete run takes **259.31 seconds**, including reservations: formatting
1.30 seconds, Clippy 9.47 and Rust 246.97. No timeout occurs. These are test
verification timings, not model performance. The raw log SHA-256 is
`ee4b180134d2662b59b5672a668063576c2f1e2074226da782cde0ff45c695d0`.
Documentation, privacy, unchanged task-table and byte-identical retained-review
checks pass. All owned checks complete.

No model or GPU process starts, and there is no coding score. The native-tool
ownership blocker remains unchanged. The existing approved worktree mutation
adapter still launches a bounded direct Git child; confining that separate effect path is the next prerequisite. This
batch changes collector reads only. The dormant candidate and local-commit
adapters remain unactivated. Candidate execution, reviewer/artifact provenance,
separately approved local commit/push effects, native/model measurements and
durable recovery stay open.

Frozen, hidden and held-out tasks, scorers, prompts, thresholds, dependencies,
supply-chain outputs, lifecycle and scoreboard are unchanged. No release or
owner readiness is claimed.
