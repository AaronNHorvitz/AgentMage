# Private-path checker review repair — 2026-11-16

Repair of medium M1 from the [independent core review](../reviews/da13234ff9c3-engine-readme-dual-licence.txt)
of `da13234ff9c3c75d6672db108a8713130c5cccd7`. Independent re-review is
required; the builder does not close a row or declare the finding accepted.

The checker now searches case-insensitive username substrings inside longer
identifiers, including host suffixes, directory suffixes and leading digits.
The username remains represented by its digest and byte length; no private
identity is introduced into public source. It inspects raw bytes and decoded
JSON escapes, URL encoding and UTF-16/32 text. Decoding has a fixed bound and
refuses deeper encoding rather than silently accepting an uninspected spelling.
Only the literal public repository URL is exempt, with boundaries preventing
an extended URL or hostname from borrowing that exemption. Diagnostics continue
to omit matched values and private filenames.

The policy gains the username byte length and its checker pin advances. All 49
exception names and historical blob hashes remain identical; the baseline commit
and username digest are unchanged. Every historical file stays byte-identical.
The checks of staged content, working content, filenames, symlink targets and
exception-policy tampering remain enforced. No evidence input set is narrowed.

Negative controls exercise case variants, username substrings, encoded home
paths, UTF-16/32, staged leaks hidden by clean working bytes, private filenames
and attempts to extend the exact public URL exemption. The 13 focused tests
and five licence tests pass. The final minutes-scale gate passes, including
those tests, licence checks, review hygiene, Markdown/Python lint and the
broadened path check. Its complete log and
preservation hashes are retained for independent review. Full Rust and long
acceptance gates are not rerun; the inherited failures remain disclosed.

The first combined gate failed on the old staged policy and encoded illustrative
paths. The new policy was staged; the checker recognises a typographic ellipsis
as the same placeholder as three dots, with a negative control proving it cannot
hide the username. The synthetic test constructs its refused path at runtime.
The original review remains byte-identical. The final reserved run reports
18 tests OK in 0.645 seconds; wrapper/queue time is additional.

The original review is retained byte for byte. Low L1 remains T-309 backlog;
the guide now states that coverage is the selected range's end state rather
than every intermediate commit. Low L2 is disclosed in an addendum to the
original licence evidence: regeneration caught up pre-existing supply-chain
drift. Low L3 (package notices) remains T-309/package acceptance work. No package
is distributed and no licence or supply-chain output is regenerated here.

MEASURE FIRST remains active. Three failed snapshot preparations produced no
inference request and no task result. Reserved preparation work was 18.443 plus
44.628 seconds, separate from completed-task scores. Settings were revoked,
API/model owners reaped, keys/sockets absent and tokens retired. Their logs and
empty own-process scans are preserved privately. The file-only approach is
stopped. A complete source archive with read-only Git access passed all 30 native
development-context comparisons before any further GPU request. The scoreboard
and all ledger table rows are unchanged; three profiles/90 attempts remain.
