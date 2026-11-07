# Original-read consent and full-size source retrieval

This is an in-row T-303/T-306 repair on base
`51e166c107ff7b6ab90fb8412421f7c98c140968`. Both parents remain open; no new
row or sub-row is added. The separate read-only
[original-apply review](../reviews/51e166c107ff-local-coding-original-apply.txt)
has **Verdict: PASS** for that exact base, with three low findings. Its report is
retained byte for byte. This implementation awaits independent core review.

## Changes

The original-apply read confirmation now sends both displayed held-root digests.
Both are required in the closed API request and passed to the existing native
repository owner. Before filesystem revalidation, source selection, reading or
preview replacement, that owner compares them to its held original/worktree
pair. Either mismatch refuses `code.apply.read-scope`. The digests cannot adopt
roots, paths or creation receipts. A separately approved new creation during a
terminal prompt cannot silently retarget the older read consent. The terminal
retains its before/after filesystem checks and sends no automatic retry.
Preparation remains inert; writing still requires the unchanged separate exact
approval and native transaction. This addresses review low L3.

Task-based native context ranking decodes the complete JSON envelope before its
existing 64 KiB source-word scan. The old guard incorrectly capped the encoded
envelope at the raw-source bound. An admitted 64 KiB source expands beyond that
when tabs, line breaks or control bytes are escaped. Original context bytes,
source identities, hashes, omissions, exact rendered-token binding and model
admission are unchanged. This addresses the remaining low L1; the previous
batch's small-source repair did not cover the full admitted range.

The existing edit-lineage restrictions remain deliberate under Decision 0156.
The owned-edit guide now documents `code.edit.lineage-limit` (sixteen distinct
untransferred paths), `code.edit.lineage` (a changed prior postimage), and exact
recovery options, including their effect when no transfer is intended. Nothing
automatically overwrites intervening human edits. This addresses low L2 by
using the review's documentation option, without changing authority or lineage.

## Verification

All seven changed Rust inputs were formatted and frozen before verification.
The full unfiltered per-change gate completed with **891 passed, 72 failed,
62 existing ignored and zero filtered**. The complete `(test binary, target,
method)` failure set is identical to the prior base run: 53 host and 19 Linux
executable-trust failures. No failure is excluded or relabelled a pass. Formatting
and Clippy with warnings denied pass. All 94 API tests pass, including the
thirteen worktree contracts. The new full-size retrieval and both native
read-consent regressions pass within that complete run.

The regressions exercise:

- Five exactly 65,536-byte source fixtures with expanded native-shaped JSON
  envelopes, complete source/provenance equality except allocation identity,
  and terms just inside/outside the unchanged decoded scan boundary.
- Wrong, empty, exchanged and one-sided stale root identities while an original
  root is unavailable and source selection is ineligible. The read-scope refusal
  must precede those other checks, preserve a prior preview and receipts, and
  leave both sets of source bytes unchanged.
- Synthetic replacement of the held created pair between inspection and consent;
  older identities refuse, while the fresh displayed pair prepares an inert diff.
- Closed API decoding of missing, duplicated, null, numeric, array and object
  identity fields; control authentication; exact terminal wire identities and
  unchanged local root-substitution refusals.

A separate Python control confirms that all five full-size fixtures distinguish
the old encoded fallback from decoded matching. It is an analytic control, not
Rust execution or a model measurement. The complete source gate is the Rust
correctness result. Its Rust format, Clippy and test stages took 378.80 s,
499.10 s and 117.68 s respectively, including shared-reservation waits. Clippy's
reported active build time was 7.67 s. Queue time is not inference latency.
The final evidence text is completed after that source run and receives separate
final documentation/policy checks; the frozen Rust bytes remain unchanged.

Focused component runs also pass: eleven original-apply methods (294 deliberately
filtered) and seven retrieval methods (579 deliberately filtered). The complete
gate above has no filters. The existing real-file component workflow performs
four approved worktree edits and two separately approved original transfers
across three transferred versions in 464074 µs, with **synthetic creation
provenance**. The five full-size retrieval fixtures plus two scan-boundary
controls complete in 44780 µs; the largest encoded envelope is
393,246 bytes. These are component timings, not native Git creation,
model latency, GPU time, tokens or coding scores.

Raw logs and the source freeze are retained privately for the reviewer. Their
SHA-256 bindings are:

| Artifact | SHA-256 |
| --- | --- |
| Complete source gate | `5470e3719cd5d3d81377d11903d922b248d8feb45e65d79753b0451646c48ea9` |
| Seven-file Rust source freeze | `746e8186cba832c9b79f8ea7d932dfb5abd5612f613a43a229944cff6886ce5b` |
| Original-apply component run | `cea8153ae56dcd9a073c854d5d183848e4961ba9a37e65a141167efb368b5ee7` |
| Retrieval component run | `4a4cf2b4e24c0b2b6270e5fcd1f1adb989f9ab71f0d7b30548b978f51777af32` |
| Python encoded-fallback control | `7ea06fa6eb400ff1db217fa3708102ccf05d493bc25e8637025f84d05e78398e` |

## Scope still open

No native Git creation, positive native coding workflow, real-model inference,
GPU run or fresh coding score is established here. Synthetic creation provenance
and terminal peers are test-only. Held-out tasks, hidden tests, frozen evaluation
inputs, thresholds, dependencies, SBOM and status model are unchanged.
The current native host requires an ordinary marked Git directory; the manual
repository owner creates linked worktrees. Completing that composition requires
explicitly preserving metadata confinement, held creation provenance, exact
model admission and rendered-token facts. No boundary is relaxed here.
Durable HTTP recovery, coordinator-connected worktree/edit/test/apply, approved
branch commit/push, MCP, development measurements and parent acceptance remain
open. T-301.1 remains deferred and the native suite remains failed until its
actual executable-trust result changes. There is no release or owner-ready claim.
