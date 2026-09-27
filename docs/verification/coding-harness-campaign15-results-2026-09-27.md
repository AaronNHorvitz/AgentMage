# Native Coding Recheck — 2026-09-27

Status: eight-case Muse bounded development campaign passed at the source pin
below; GPT-OSS remains separately unqualified. No production model admission,
independent review, supported-platform or release acceptance is asserted.
Scope: AMR-01 and AMR-02.4 prerequisites. Engineering decision status: Accepted
under owner delegation, 2026-09-20. Authority: Decision 0054.

## Exact scope and retained failure

Campaign 15 used clean commit
`b4417f474a1ab4406de18c3e45a9ca61692a7743` throughout both repetitions.
The actual CLI, host, authenticated private IPC, existing coordinator, native
tools, exact grants and verifier ran in fresh disposable repositories.
No scripted model reply, direct inference probe, smaller model, relaxed schema
or completion override is counted. The earlier
[campaign 14 admission refusal and reproduced correction](native-hybrid-resource-admission-2026-09-27.md)
remain failures of their original attempt, not reclassified successes.

The [prospective four-case/two-repetition protocol](coding-harness-native-campaign-2026-09-23.md)
was retained. Compared with failed campaign 14, the only production Rust change
was the native hybrid-cgroup admission correction;
the exact model, 32K context, generation budget and resource ceilings did not.
The source and binaries stayed frozen throughout both supervised passes.

## Inspected results

| Case | Repetition | Verifier | Seconds | Full verified artifacts |
|---|---:|---|---:|---:|
| repair | 1 | `SUCCESS` | 415.702746 | 24 |
| new-file | 1 | `SUCCESS` | 414.583845 | 23 |
| multi-file | 1 | `SUCCESS` | 410.458215 | 25 |
| stable | 1 | `NO_OP` | 331.961569 | 13 |
| repair | 2 | `SUCCESS` | 414.448319 | 24 |
| new-file | 2 | `SUCCESS` | 422.513872 | 23 |
| multi-file | 2 | `SUCCESS` | 431.636209 | 25 |
| stable | 2 | `NO_OP` | 328.776836 | 13 |

Each mutation case observed a genuine failed registered validation before its
native edit, then complete passing validation, current diff/status inspection
and verifier-backed completion. Stable cases made no workspace change.
Final changed-file hashes were separately cross-checked against their canonical
native mutation receipts. Tests remained unchanged. Ordinary Git diff excludes
untracked creations: the new-file postimage and native creation receipt are
separately bound in the summary, not represented by an empty tracked diff.

Protocol rejections: 0. Tool rejections: 2. Tool failures: 0.
Each new-file case retained one `read_projection_unavailable` refusal before
creating the absent module. These are not silently omitted failed attempts.
The summary lists every observed tool rejection/failure rather than omitting
them from successful case dispositions. Original streams and candidate records
remain retained privately with exact hashes; no raw transcript is published.

The [curated evidence summary](../../artifacts/coding-harness/2026-09-27/muse-campaign15-summary.json)
has SHA-256 `813467ae8b1d29339a7b1cdfba0a1f6d3295378efcf36550ed41e928860b954a`.
The read-only inspector compared original and copied candidate records,
recomputed each collector exactly, verified raw-log hashes, checked source and
binary identities, and matched final file contents to native receipts.
All 170 verified artifact identifiers were unique within their respective cases.
All 50 model responses ended at EOS, with no generation-budget truncation.

Both supervised passes returned numerical exit 0, including their mandatory GPU
postflight checks. After each pass, a query restricted to its exact CLI/host paths
and synthetic campaign roots found no remaining match. The retained outer logs
have SHA-256 values, respectively:

- `ba9d75c8cff6c218b2ceb0c49fad3bff8e60cbe3b5b67d08bb85cd2ee2028d84`
- `6d7cadea045418f0339ff852aa8a674540cb7068731a74b06ef0265d14ca5369`

## Profile, resources and limits

Muse manifest: `8a8ee8efc1738f30d143f94f753e6079808e05dc3127df28a965676d7edfd24b`.
Exact model artifact: `4cc57c0f51040a226e5a72cc47b7613f7772950e460a665f7083de89f183f60e`.
The summary retains complete model/runtime/codec/template/tokenizer/decoding
identities and private diagnostic driver/wrapper hashes. The prepared profiles
remain disabled candidates; a successful development campaign is not activation.

Maximum inspected input: 28148 tokens. Maximum output: 1437 tokens.
Served and effective context stayed 32,768, output reserve 4,096, safety margin
256 and one model slot. Exact stop outcomes are retained for every case.
All resource guards passed. The unchanged scope used four model threads,
200% CPU, low priority, MemoryHigh 5 GiB, MemoryMax 6 GiB and swap max 512 MiB.
Each complete pass acquired the shared heavy reservation before its exclusive
queued GPU reservation, including startup, testing and cleanup. Each outer
deadline was 2,700 seconds; the inner 2,500-second alarm reserved cleanup time.

Peak sampled total GPU memory: 18532 MiB, below the 22,528 MiB guard.
The largest observed scope memory peak was 5,369,806,848 bytes, below 6 GiB.
Reported scope memory peaks are cumulative within each four-case scope, not
independent per-case peaks. Timings include startup/integrity checks and shared
machine conditions; they are not uncontended inference benchmarks.
A read-only host snapshot observed RTX 4090, NVIDIA 615.71.09, Fedora 44
(Kinoite 44.20260920.0) and Linux 7.2.5-200.fc44.x86_64. It is not continuous
hardware certification or support for another tuple.

## Freshness and remaining gates

This record describes its exact tested revision, not every later main commit.
The single correction-unit SBOM/evidence renewal returned numerical exit 0.
Supply-chain validation, the contract boundary, and the applicable dependency,
configuration and security-map builders/tests passed. Inspection of all twelve
generated files found only hash changes in JSON and timing changes in the raw
contract log; no binding, dependency, license or acceptance claim was changed.
The retained renewal log has SHA-256
`196ce9acf5896604edd66b901b9b754cc24efe9eca83aff302f59bbd8475e7ce`.

A scoped scan of 429 named whole-file bindings found 23 current, three newly
stale Sprint 82 supply-chain bindings, and 403 already-stale/historical bindings.
Sprint 82 reads committed blobs: its bounded `BLOCKED` report must be renewed
against the commit containing this SBOM as the remainder of this same pass,
without another SBOM write. The scan is not a transitive, unnamed-digest,
line-span or global currentness acceptance. Historical Sprint 16 local and
installed reports also remain ineligible as current evidence pending the
separately identified freshness-validator correction.
Historical installed/native/platform reports are not reaccepted by these cases.
The [earlier GPT-OSS disposition](coding-harness-campaign13-results-2026-09-23.md)
remains separate: malformed duplicate Harmony channels and an unregistered
hybrid alias were rejected, with EOS below the existing output budget. No
unchanged failed campaign was rerun and no general incapability is inferred.

Command/Git/validation live cancellation, crash-safe worker ownership, connected
research and broader admission remain open. A pinned package is prepared for
fresh independent review, not approved by this implementing session. Human
acceptance, Task 50.2.4.7, M-HARNESS-DAILY, platform and release gates remain open.
No AMR checkbox or licensing state changes with this record.
