# Decision 0212: Copy original fragments before replacement

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-17 |
| Authority | MEASURE FIRST; T-303/T-306 |
| Scope | Fixed coding-proposal instruction only |

The completed six-profile development repeat has 56 exact-edit refusals in
180 attempts. The read-only diagnosis finds absent original fragments in 25,
overlapping ranges in 11, ambiguous fragments in eight, empty fragments in
five, unchanged edits in five and invisible fragments in two. These are
mechanical refusals, not task grades. The diagnosis selects original-fragment
fidelity as the next generation problem; it does not select a model or prove
an improvement.

The fixed instruction now gives an explicit construction order: choose and
copy a small unique original fragment into `old`, then derive `new` from that
copy, then check every edit against the same original source. It describes
insertion using an existing nonempty anchor, deletion using an empty `new`,
combining changes that share source characters, and omitting unchanged edits.
It explicitly distinguishes JSON escaping from the decoded original text.
The instruction supplies no task-specific solution, new source or authority.
Task and repository content remain untrusted data.

This is a generation hypothesis. The JSON shape, exact unique visible preimage
checker, source selection algorithm, byte ceilings, all-or-nothing refusal,
permissions, native admission and grading controls are unchanged. There is no
normalization, fuzzy matching, output repair or automatic retry. The fixed
instruction retains its previous 1,072-byte overhead, preserving the source
allowance under the existing context ceiling. Development preflight must
confirm all 30 user payloads and excerpts remain identical.

Freeze this change before inference and repeat all 30 development tasks on
all six available profiles in resumable resource jobs. Keep the existing 32K
served context, per-task output formula, inline schema, model configurations,
framing, thinking request and deadlines. Report the prompt as the changed
variable, any excerpt changes and does-not-fit cases, strict ungraded proposals,
refusal causes, tokens and time. No further tuning or capability work precedes
completion of that repeat. Hidden and held-out tests are not used for choosing
this change. Task passes and retries remain unavailable until the original
execution and grading controls are available; they are never reconstructed.
Independent core review is required. T-303/T-306 remain open.
