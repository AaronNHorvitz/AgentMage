# T-311.18: baseline failure-attribution repair

The [independent core review](../reviews/4ee14c9b7de7-local-coding-t311-18-baseline.txt)
of exact commit `4ee14c9b7de7ae612eb3e516b3d32fe32d0deab0` returned
**Verdict: FINDINGS**, with one medium finding M1. The numbers reproduced, but
the scoreboard's `chat-failed` label hid observed context and output limits.
This repairs that finding in the same expanded baseline row. T-311.18 and parent
T-311 remain open for exact-commit independent re-review. No new row is added.

## Observed result

The unchanged retained records classify as follows:

| Model | Split | prompt-exceeds-context | context-exhausted | output-cap | stopped |
| --- | --- | --- | --- | --- | --- |
| muse | dev | 10 | 9 | 7 | 4 |
| muse | held | 5 | 5 | 0 | 0 |
| gpt-oss | dev | 10 | 12 | 6 | 2 |
| gpt-oss | held | 5 | 5 | 0 | 0 |

The 8K context / 4,096 output budget blocked 36/40 Muse and 38/40 GPT-OSS attempts,
including every held-out attempt. Ten development prompts per model were refused
before generation. All four Muse generations ending below limits passed; the two
GPT-OSS generations ending below limits were API-invalid. These scores mainly
measure integration and budgets, not relative model quality. Later scoreboards
must identify a changed context or output budget as a separate variable.

The [scoreboard](coding-scoreboard.md) now states these limitations first,
publishes split counts and a class for each development task, and keeps all
original scores, times, tokens, API terminal counts and aggregate hashes.
No held-out identity or content is added to public evidence.

## Reproduction and bindings

Use the [retained-log command](../guides/coding-baseline.md#retained-log-termination-report).
The new standalone reporter `scripts/coding_baseline_termination.py` verifies
the original manifest digest, every consumed file against that manifest,
resource/aggregate and resource/launcher bindings, complete unique split
coverage, timing-record equality, and each original server-log slice's digest.
It checks context using the observed server properties and output budget using
the bound request. It exports development identifiers and held-out counts only.
Unknown, duplicated, conflicting or changed observations refuse.

It reads retained data only: no model requests, GPU, task execution or hidden
tests. The frozen collector, prompts, task definitions, hidden tests, policies,
thresholds, original resource reports and score aggregates are unchanged.
In particular, this avoids changing the collector's source identity required
to rederive the original aggregates. No source/evidence renewal cascade occurs.

| Retained item | SHA-256 |
| --- | --- |
| Original independent review, copied without edits | `38016ae6123f7db2b26840dd3f50293491365f78d41f4a6e428d2bd27afe768e` |
| Original campaign manifest | `932463662e05b984a28fd3b329c24df898839d10a749e08b28fc8574725ba9f1` |
| Supplemental termination report | `e83f6f3a7cc422772abcf79b6a7311c35ae2e31b9e2b29a91a6e09802a98b198` |

## Verification

Eight synthetic reporter tests pass: four termination classes, absent/duplicate/
conflicting logs, complete split accounting, held-out output privacy, changed
manifest/files, slice tampering, changed request budgets, missing batches and
external/symlink path refusal. No frozen assertions are imported by these tests.
Running the reporter over the complete retained campaign succeeds and reproduces
all 80 classifications, with 60 development rows and held-out aggregates only.

The first reporter probe refused because observed server properties nest their
defaults and do not expose the request's output cap at the top level. The final
implementation checks context there and the output cap in each bound request.
That refused probe remains retained; no baseline record or scored run was rerun.

The scoped reserved fast gate passes: eight tests (0.061 seconds), Python
syntax/whitespace checks, five Markdown files, unchanged review-format checks
and Git whitespace checks. Final reporting, privacy and preservation checks
confirm the same supplemental digest, all 60 development classifications,
unchanged original timing/token/cause columns, unchanged scores and original
aggregates, and an unchanged preserved ledger. Relative Markdown links resolve.
No original suite source, collector, hidden test or frozen metadata changed.
After documentation-only edits, all five Markdown files pass lint again. A
redundant second fast gate was cancelled while still waiting for the shared
reservation, before it started any checks; the successful first gate remains
retained. No source changed after that successful gate.

An initial hygiene probe scanned the entire inherited ledger and stopped on
pre-existing host paths. The final addition-only scan passes; the inherited
privacy backlog remains disclosed rather than being described as repaired.

## Low findings and limits

- L1: deeply nested JSON can crash the frozen collector with `RecursionError`.
  No baseline attempt did so. Keep its identity intact for replay; refuse this
  as `edit-refused` with a synthetic regression before reusing the parser in
  T-303/T-306. This is backlog, not a reason for a new evaluation row or rerun.
- L2: disclose the dev-004 SIGINT timing race alongside dev-003 in the suite guide
  and each development timing row. The reviewer retained the failed verifier
  attempt and its passing unchanged repeat. Base/reference discrimination held;
  the exact FAIL/ERROR profile temporarily differed. Both model dev-004 attempts
  hit the output cap. No hidden test, threshold or frozen race metadata changes.
- L3: the guide and scoreboard now describe the mixed configuration: 8K context
  from the product chat value, 4,096 output from the development lab. The product
  chat configuration itself uses 2,048 output; the baseline was not that profile.
- L4: clearer API context-overflow and length-stop diagnostic codes remain
  hardening backlog. Reporting now uses backend evidence independently of those
  broad codes. No transport or credential boundary changes.
- The review also records inherited host-privacy issues outside this batch.
  They remain repository hygiene backlog for hardening; this batch adds none.

No native agent-loop, model admission, independent PASS, release or owner-ready
claim follows from this reporting repair. The next builder work after independent
baseline acceptance remains T-303/T-306, then measured T-312/T-313 work.
