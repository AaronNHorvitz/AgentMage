# T-301: Second review repairs, 2026-10-25

The [second independent review](../reviews/4fda20f7d9f3-local-agent-t301-review-repair.txt)
of `4fda20f7d9f335cfe859cb3e01206573ea5e3e08` is retained byte-for-byte.
Its verdict is INCONCLUSIVE: no high or medium finding remains, but the reviewer
could not reproduce Markdown lint because its mount lacked the Node runtime.
The report's SHA-256 is
`b889e731fdf921898501d29ad20a57e8983457f6dc2382ac6c39463a191c36bb`.
This batch prepares review attempt 3. It does not close T-301 or T-301.1.

## Repairs

- L4: retained text reports accept exactly the review process's final formats:
  PASS, FINDINGS, or INCONCLUSIVE followed by a non-empty reason line. Tests
  accept these endings without modifying bytes and reject FAIL, BLOCKED,
  missing reasons and extra lines after the reason.
- L5: the supervisor observes child exit with `waitid` and `WNOWAIT`, keeping
  the leader unreaped until TERM and KILL have both been sent to its group.
  This reserves the group ID while cleanup signals are sent. Only then does
  `wait` reap the child. Regression tests exercise both a normal nonzero exit
  and termination by signal; at each group signal they verify that the leader
  remains waitable and has not been reaped. The existing descendant cleanup,
  interruption and reservation-release tests remain unchanged.
- Reviewer tooling: the launcher mounts the same private home for both sessions,
  while exposing only selected state subdirectories to the reviewer. The existing
  Node binary and its license were copied into that shared home, outside Git;
  the launcher and mounts were not changed. The private review request gives
  the PATH setup and checksums. No download or dependency change was needed.
  The next reviewer must independently confirm access; the builder has not
  launched or impersonated that session.

## Verification and remaining scope

The 23-test module passed in 2.33 seconds before the documentation changes.
The full mission-scope fast gate against
`ecf33af35782bf2f76fbfe7332f6348d31ba4277` then passed all six commands in
about four seconds, including both retained report checks, unchanged Markdown
lint rules over seven files (zero issues) and all 23 tests in 2.34 seconds.
Raw command output stays in private lane state. Delta/new-file privacy checks
and the byte comparison of the entire preserved old ledger passed. A tooling-only
pass does not make native Rust tests pass.
The unchanged document/link/policy validator also passed for 678 Markdown files;
it scans ignored build artifacts and took about three minutes in this checkout.
The final evidence text was checked again with the unchanged Markdown linter.

T-301.1 still requires root-mapped trusted executable paths and ancestors or the
unchanged checks on the native owner host. No failed test was skipped, filtered,
allowlisted or relabeled. The inherited workspace measurements are unchanged;
no Rust source or native trust boundary changed in this batch. The row cannot
close without its required independent PASS and resolution of the native blocker.

L2 automatic Python importer/caller selection and the earlier low findings remain
backlog for T-309. After three non-PASS reviews, preserve T-301 open, retain its
remaining findings as sub-rows and proceed only with dependency-ready work as
Decision 0151 requires. T-302 has not started.

No GPU, model, external provider, UI, installation, old long documentation gate,
public release or owner-test readiness was exercised. Runtime status, dependency
locks, supply-chain outputs and historical evidence bindings are unchanged.
