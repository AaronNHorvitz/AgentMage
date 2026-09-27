# Research draft reader compatibility, 2026-09-27

[Decision 0087](../decisions/0087-research-draft-reader-version-barrier.md)
implements the older-reader barrier identified in the
[retained-draft component](research-retained-report-component-2026-09-27.md).
Verified source commit: `4e0159e42d15ae5e5ff07c84592613a995375626`;
base: `373236da510bc3e402b3aa1c43ec5c3f5d9526c5`.

## Boundary

Canonical store version 20 records changed read semantics without adding tables.
The existing migration owner verifies all version-19 history before atomically
appending the version-20 hash and advancing the version. Earlier migration bytes,
version-18/19 fixtures, table inventory and existing records are preserved.
Failed transitions remain at version 19 and can be retried after correcting the
failure. Current opening still checks the entire current history.

An older reader ceiling refuses version 20 before claiming the exclusive writer.
The component probe invokes that real opening boundary against an encrypted store
containing a retained draft. The refusal preserves encrypted bytes. Current reopen
reconstructs the source-checked report without changing accounting or receipts;
generic artifact reads remain refused. This is not an executed older application
binary or installed update/rollback qualification.

Exact-current restore also refuses an older backup before changing journal
metadata, with a second version check preserved under the writer lock. It creates
no candidate and preserves backup bytes. Automatic conversion of old backups and
migration of host user data are outside this increment.

## Observed correction and checks

The first four focused compatibility tests passed. The storage suite then passed
49 tests, but its Python evidence check failed: test definitions were searched in
the wrong source file. The check now validates and binds the included test module.
A subsequent Python run caught the older exact-count assertion; it was advanced
with the builder's count, preserving exact-result checking.

Adding the old-backup regression found a production failure. The full engine run
passed 1,215 tests and failed that regression because restore configured journal
metadata before rejecting the older version. The pre-writer version check fixes
that ordering while retaining validation under the lock. The failed invocation
and corrected run are both retained by digest.

The final source passed 1,216 engine tests; seven existing native or external cases
remain ignored. All 50 focused storage tests, 13 documentation tests, the host
error-mapping regression and 49 Python builder regressions passed. Workspace
all-targets Clippy with warnings denied, strict-local source and module audits,
formatting, whitespace and frozen-source checks passed. The three changed source
documents passed Markdown lint. Exact inputs and private log identities are in the
[component manifest](research-reader-compatibility-2026-09-27.json).

All heavy checks used the shared build reservation, one Cargo job, single-threaded
tests and locked offline dependencies. No dependency, license or model changed.
Current-version evidence bindings extend the older input sets; historical
migration hashes and fixtures remain intact. The one SBOM regeneration and core
evidence pass completed with 551 Markdown files and 197 Python regressions.
Storage, security and resume builders ran their actual checks and passed 83 Python
regressions. Counts overlap other source checks. All 34 final validators, full
Markdown lint and frozen-input checks passed. The routine Story 11.2 automated pin
renewal remains pending at this evidence checkpoint.

The direct binding inventory excludes transitive, unnamed and line-span bindings.
Apart from the pending automated pin, newly stale inputs occur only in the earlier
report and retained-draft component records. Their original source pins and test
observations remain historical; they are not rebound to this implementation.
Existing builders retain some historical generation dates; current execution is
identified by these source and log hashes. No global evidence-freshness claim follows.

## Acceptance limits

This is synthetic component verification. It does not activate research tools,
qualify the native worker, provide a configured search provider, demonstrate an
installed older binary, or establish independent or human acceptance. No checkbox,
production model, supported platform or release status changes.

The earlier actual CLI invocation refused native ownership and session-bus
prerequisites in this sandbox. It is historical evidence at source `50f0dbb9`,
not a successful workflow on this revision. Current Linux edit, failed-test repair,
denial, cancellation and preservation acceptance remain open; qualified-environment
launch commands and limits are in [local testing](../LOCAL-TESTING.md).

Real-model, manual-user, independent-review and release results remain separate.
