# T-306: Retained worktree Git creation identity

Scope: T-303/T-306, core, non-closing increment. Both parents remain open;
no row or sub-row is added or closed. Base is immutable commit
`55b674a02340c6db4d6c83e6cfc853147f7bc906`, still under independent review
when this work began. No newer verdict was available at that point.

The existing native creation path now preserves its original verified plan
across dispatch and correlates both terminal receipts before attaching Git
creation provenance to the live owner. It retains the exact created task branch,
immutable base, plan digest, repository receipt digest and canonical authority
receipt digest. Failure to establish these correlations refuses attachment after
dispatch; such a refusal does not establish absence of native effects.

`created_git_identity()` returns a borrowed, nonserializable view only after
revalidating the original checkout, worktree and management root. The owner must
retain its matching creation receipt. The same provenance survives runtime
handoff and exclusive owner return. Synthetic creation fixtures retain `None`.
Reopening the authority store retains actual receipt history without restoring
Git provenance or execution ownership.

Existing original-apply inspection and exact diff previews include `git_creation`
when available. This is historical creation information: `current_git_state_verified`,
`commit_authority` and `publication_authority` are all false. It never establishes
current branch/HEAD, task success, test success or approval for Git effects.
A future commit operation still needs fresh native observations and the existing
complete typed review packet, preservation, signer and exact approval contracts.
No serialized value can adopt a checkout or construct this typed view.

Five new tests use the existing synthetic executor through real held directories
and canonical durable authority. They verify exact plan/receipt correlation;
missing specialized receipts plus fourteen independent substitutions; same-owner
handoff, busy return, final return and receipt-only reopen; and refusal of missing
provenance, a changed creation receipt and each replaced root. A real native
edit and separately approved original transfer bind this historical metadata into the exact preview digest while
preserving human index and untracked sentinels. These tests assert
no native branch or Git effect. The existing native creation test additionally
checks exact retained branch/base and receipt identities on its supported-host
path, and no Git provenance on the lane's existing trust refusal. Existing test
assertions remain intact; no ignore, threshold or admission rule is changed.

The complete affected fast gate remains **FAIL**: **1,267 passed, 72 failed,
64 ignored**, zero measured or filtered, across **26 unfiltered targets**.
All 72 failing identities exactly match the unchanged
[recorded host/Linux baseline](t301-baseline-failures-2026-10-25.md), and existing ignore identities are preserved. No new failure is
observed. All five new tests and six named existing regressions pass; the
comparison is not a passing gate or parent acceptance.

Formatting and Clippy with warnings denied pass. Nine Rust file hashes remain
unchanged throughout verification. The run took 915.60 seconds including shared
reservations: formatting 644.34 seconds (mostly waiting), Clippy 12.02 seconds,
and tests 257.62 seconds including compilation and reservations. The first full
run required no source repair. Raw full-gate log SHA-256:
`5c412dee3c7ce13fc2dfab2a33d9e03fd78df294a16b716b0d1ed1ce1b109b2f`.
Raw logs, source freeze, commands and exact target comparison are retained
privately for independent reproduction. Timings and log hashes naturally differ
on reproduction. Final prose, added-line hygiene, whitespace, unchanged ledger
and existing-test preservation checks pass.

A separate diagnostic reruns only the native creation test with output capture
disabled to establish which conditional path was exercised. It reports the exact
system Git artifact trust refusal: **1 passed, 400 filtered**, no failure or
ignore, and no native creation measured. This filtered diagnostic does not
replace the complete unfiltered gate above. It took 616.58 seconds including
shared reservation wait and 10.65 seconds compilation; the test took 0.12 seconds.
Its raw log SHA-256 is
`9ffea94237e13864ed763551b168e3ca2fe5e139991f0f7356fc22ebca63ae23`.

Native executable trust remains an owner-host blocker. No real model or GPU is
used; the development scoreboard rerun, approved Git execution, durable recovery
and parent acceptance remain open. No frozen/hidden/held-out task, scorer,
threshold, model prompt, dependency, supply-chain output, status record or
scoreboard is changed. The long acceptance gate is not run for this increment.
