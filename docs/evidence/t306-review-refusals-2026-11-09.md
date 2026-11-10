# T-306: Refuse incomplete creation review and malformed packets

Same-row repairs of low L1 from the creation-provenance review at
`5cd6332a060b4a64af7670ead09c54f3736229b5` and low L3 from the
history-consumer review at `906dc1e0bd1cce9b0120fc6247e7526d2905ad56`.
The immutable base is `419331de0c7517fcee21200fb7a34dff807b5489`, under
independent review at session start. T-303/T-306 remain open. No row or
sub-row is added or closed, and independent core review remains required.

Before native creation dispatch, the live repository owner marks creation
review incomplete. Only checked successful completion after publication can
clear that dispatch's marker, and only if original write proof remains
available. The existing full-lineage checks still apply separately. An earlier
incomplete creation remains sticky across ordinary edits,
later successful creations and runtime handoff/recovery. Candidate-review
projection refuses with `code.commit.creation-provenance` even when later
existing-file write proof is otherwise valid. Pre-dispatch denials never set
the marker. The conservative restriction lasts for this owner; a fresh service
cannot adopt the old worktree. Existing inspection, edit and separately approved
original-transfer boundaries remain unchanged. This prevents the reported
unpublished-creation relabeling; it does not establish complete Git-base
preimage verification for other paths.

The runtime JSON consumer now checks semantic invariants only after its closed
schema accepts the input shape. Malformed arrays, objects, nulls and scalar
packets return the existing structured schema refusal instead of throwing
during traversal. Schema-invalid records still refuse; schema-valid records
still receive every existing semantic check. The same rule applies to all
runtime record types. No schema or acceptance threshold changes.

The updated native creation regression requires refusal after an unpublished
creation followed by an ordinary edit. A new regression retains that refusal
through a second runtime handoff and a separately published creation, while
checking actual file bytes and preservation of the original checkout. Positive
published-creation coverage remains in the complete suite. The synthetic
creation, process and publication fixtures do not demonstrate native Git or a
model-driven workflow. The new JSON regression checks both single-transaction
and history packets, malformed net files/hooks, commit groups, review findings
and scalar packet roots. No failing assertion is removed or weakened.

The complete affected gate remains **FAIL**: **1,301 Rust passes, 72 inherited
failures and 64 existing ignores** across **26 unfiltered targets**, zero
measured or filtered tests. Each target's failure and ignore identities matches
the retained base exactly, with one additional Linux pass. Only the affected
Linux crate and its host/API dependents are selected; no kernel input changed,
so its twenty targets from the previous broader gate are outside this selection.
No test command filters or skips tests. Exact comparison is data only and does
not make this gate pass or accept either parent.

Both affected Rust creation-review methods pass, including the new later-create
and handoff regression. The complete schema suite passes **86/86**, with zero
failures, cancellations, skips or todos. Formatting and Clippy pass. All six
changed code files retain their verification-start hashes throughout the run.
The final prose receives separate lightweight checks because it is completed
after the suite starts; those checks also pass.

The full gate takes **1,930.98 seconds**, including shared reservations. The
schema step takes 1,171.02 seconds including reservation; its Node body takes
1.351651 seconds. Formatting takes 19.38 seconds, Clippy 467.54 and Rust 272.76,
each including reservation. These are component verification timings, not model
performance. No timeout occurs. The complete raw log SHA-256 is
`d0d12f6fb941d2273db158916471e468bfb43b5567b6fdc0ba526baa622f7b45`.
The retained review bundle includes the existing licensed runtime and exact
commands to reproduce the run and compare failures without treating them as a
passing gate. All owned checks complete.

No model or GPU process starts. Frozen, hidden and
held-out inputs, scorers, prompts, dependencies, supply-chain outputs, lifecycle
status and the scoreboard remain unchanged. Native executable trust, reviewer
and artifact provenance, candidate execution, separately approved Git effects,
native/model measurements and durable recovery remain open.
