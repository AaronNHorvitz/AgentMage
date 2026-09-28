# Decision 0094: Bound and Cancel Development Confirmation Input

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-27 |
| Authority | Decisions 0054, 0061 and 0081; current owner restart |
| Scope | AMR-01 development CLI session and operation confirmation input |

## Finding

The development CLI reads both session preauthorization and operation approval into
an unbounded `String` with blocking `read_line`. The signal owner from Decision 0093
sets a cancellation flag, but these reads do not observe it while waiting. The
existing synthetic approval-delay option is bounded to ten seconds; its sleep also
does not poll that flag. These are source findings, not native workflow results.

## Decision

Keep the existing development CLI, protected challenge presentation, approval port,
signal owner and canonical runtime cancellation path. Move confirmation input to
the existing Linux development boundary. Expose only the two closed confirmation
kinds and a content-free outcome; do not expose a caller-selected descriptor, word,
command or permission grant.

Read at most 256 bytes, including a required newline, from the development CLI's
stdin. Preserve the exact case-sensitive `yes` and `preauthorize` words with the
existing surrounding-whitespace treatment. Empty input, a different word and EOF
before the newline never confirm. Invalid UTF-8, excessive length, invalid input
ownership or an I/O error fail closed without returning input content. Do not accept
an approving prefix, drain an unbounded suffix or consume the following prompt's
bytes. The development CLI remains the sole stdin reader; hold its existing standard
input lock while reading and use bounded direct reads without a background thread.
Reject a write-only descriptor before polling: its peer may remain open without
ever making the descriptor readable. Inspect access flags without changing them.
Preauthorization input failures retain the existing presentation-failure exit class
while exposing the new content-free input diagnostic.

Poll descriptor readiness with bounded waits, checking the same cancellation flag
before waiting, reading or accepting a complete answer. Do not set nonblocking
flags on an inherited open-file description shared with the caller, change terminal
settings, or open a different terminal. Human input may remain pending; cancellation
and memory bounds do not impose a new approval lifetime. Existing exact challenge
and grant expiry checks remain authoritative. Kernel-stalled reads retain the
cooperative-bound limitation; this is not a hard-real-time claim.

Enable only the existing pinned Rust I/O library's readiness feature in the Linux
adapter if required. Retain its exact package/version and complete default and
connected feature inventories. No new dependency, network capability or runtime
activation is introduced. Account for every changed manifest and inventory in the
normal full SBOM and evidence bindings; do not narrow those bindings.

Before runtime preparation, cancelled preauthorization uses the existing startup
cancellation and direct-child cleanup path. During a protected operation prompt,
leave the shared cancellation flag set. The interactive driver already polls it
after the approval port returns and before sending any approval response; preserve
that ordering so the canonical cancellation request takes precedence. A local
non-allowing return must not become a transmitted denial or fabricated terminal
result when cancellation is pending. Make the existing bounded approval delay
cancellation-aware through the same flag, with no second signal owner.

## Verification

Use actual pipes to cover complete and partial answers, EOF, exact size limits,
overlong approving prefixes, invalid UTF-8, multiple lines, open idle writers,
cancellation and unchanged descriptor flags. Exercise the real protected CLI driver
to show a pending cancellation sends one cancel and no approval advance. Test the
preauthorization and operation-confirmation consumers and the existing delay.

Keep component fixtures separate from an actual native host/model workflow. Retain
all failures, full source identities and the missing native prerequisites. Batch
both confirmation paths and delay handling before one applicable evidence pass.
Independent, human-only, model and release gates remain open.
