# T-306: Review history consumer integrity

Scope: same-row low L1 from the independent review of
`bac3866b9c48511fef1d51e3a2c8e86784faf718`, under Decision 0185.
This core increment builds on immutable
`5cd6332a060b4a64af7670ead09c54f3736229b5`, which is under independent
review at session start. T-303/T-306 remain open; no row or sub-row is added
or closed. Independent review is required for acceptance.

The JSON consumer previously accepted histories with changed original file
classification or ordering, altered net diffs, missing review hooks, and
substituted composite operation and file hashes. The Linux planner already
refused mismatched live proof; this fixes the weaker consumer without changing
native execution or authority.

The consumer now reconstructs the Rust history projection: original file
ordering and unique operation identities, purpose/artifact agreement, leaf file
digests, five-line diff headers, per-path continuity, first preimages and final
postimages, complete net diffs, per-path ordered operation hashes, complete file
hashes, history/intent/plan hashes and ordered review-hook union. Hashing uses
explicit Rust record field order, so incoming JSON property order cannot change
integrity. Existing bounds remain unchanged; complete diff budgets count UTF-8
bytes. Absent-before creation remains distinct from an existing empty file.

The original history test now produces complete synthetic diffs and their real
integrity hashes instead of placeholders. All earlier rejection cases remain.
Four additional tests cover composite and net-diff substitutions, independently
resealed leaf mutations, absence, net-zero paths, reordered JSON keys, Unicode
content and a history whose character count fits but UTF-8 bytes exceed the
existing eight-MiB bound. A mixed once-seen/retry case preserves exact leaf
line endings and trailing-newline presence for a once-seen path, while retries
recompose the net diff exactly as Rust does. Rejection assertions also require history semantic
diagnostics, avoiding a pass caused only by unrelated commit-plan correlation.

The retained mixed edit/create/retry packet emitted by the existing Rust owner
passes the changed schema and semantic consumer. Its native file transactions
are real component operations; Git/process/publication observations are
synthetic. Rechecking this retained packet is consumer conformance, not a fresh
Rust execution, real model task, independent review or measured coding score.

Two queued source snapshots are superseded during inspection: the first
predates preservation of once-seen diff formatting; the second predates guards
and regressions for malformed scalar path/diff values. Their actual outputs and
source-change flags remain retained, and neither is final verification.
The final complete affected gate is **PASS**: **84 schema tests passed**, zero
failures, cancellations, skips or todos. Its four commands pass: complete-range
Git whitespace, untracked whitespace, documentation lint and the complete
schema suite. No Cargo selection is generated because no crate input changes.
Both JavaScript source hashes remain fixed through the final run. The gate
takes 638.99 seconds including reservation; the Node runner takes 1.437731
seconds. The raw gate log SHA-256 is
`e9ae493a7675098206acde277d5399130aea9fb3e1b47ae129fdd0146c542301`.
Final lightweight checks cover the completed evidence and guide. No source is
edited after the final gate starts.

Both earlier queued gates execute with exit zero but their source-freeze checks
are false, so their wrappers exit one and neither qualifies as final evidence.
The private result checker initially expects TAP summary prefixes; Node emits
its spec reporter prefix. The checker is corrected to recognize either exact
prefix while still requiring all 84 passes and zero failures/cancellations/
skips/todos. No product code, test assertion or threshold changes for this
reporter correction. The corrected checker and retained-packet conformance pass.
The obsolete preliminary suite waiter is cancelled after the final complete
gate passes; it never executes and supplies no result. All owned processes
finish, with no model or GPU process started.

No Rust, schema, frozen/hidden/held-out task, scorer, threshold, prompt,
dependency, supply-chain output, lifecycle status or scoreboard changes.
The existing Rust/native failures remain open and are not converted into a
passing runtime gate. Reviewer and artifact authentication, candidate execution,
approved Git, native/model measurement and durable recovery remain open. No
model, GPU process, download, new route, command, permission or trust exception
is involved. The long acceptance gate is not run for this consumer increment.
