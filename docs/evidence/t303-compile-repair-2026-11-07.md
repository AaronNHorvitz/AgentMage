# Compilation feedback and separately approved repair

T-303 with related T-306; core, non-closing implementation batch, 2026-11-07 UTC.
Base: `d671b595e4326a1f22a400ffcf7b949969e0c964`.
Both parents remain open pending their complete workflow and independent review.
The preceding [consent re-review](../reviews/d671b595e432-local-coding-http-consent-repair.txt)
is retained byte-for-byte with its exact-commit **Verdict: PASS**; it accepts that
repair only.

## Behavior and authority

The existing rerun planner admitted `compile_failed` in its status check but also
required a nonempty failed-test-name set. The strict normalizer correctly emits
zero tests and no failed names for compilation, making that path unreachable.
The planner now accepts a complete unsuccessful compilation receipt without
inventing tests. The rerun verifier applies the same closed failure eligibility
and verifies the prior receipt's template identity. Cancelled, signalled,
truncated, secret-detected, malformed and successful results cannot select it.

The existing Rust text adapter also recognizes a deliberately narrow Cargo
compilation failure: empty stdout, exit 101, complete bounded UTF-8 stderr with a
separate rustc error diagnostic and one final Cargo compilation summary. Package,
target, positive error count and optional warning-count grammar are checked.
Mixed test/compiler output, multiple summaries, trailing parallel-build notices,
other Cargo failures, ANSI controls, CR and incomplete diagnostics refuse. This
is selected explicitly as `rust_libtest_text_v1`, never inferred as a parser.

That receipt can now flow through the existing repository owner's failure
preview, separately consented configured-model proposal, reviewed native write
and exact rerun selection. No new route, executor, coordinator or authority store
is introduced. The failed receipt still binds both diagnostic channels and the
same immutable test definition. Each model disclosure, edit and test execution
requires its existing separate consent. The three-attempt ceiling, positive-test
pass minimum, unknown-effect refusal and single-use model selection are unchanged.
An empty failed-name set is compilation evidence, never an executed-test count.

## Verification

The final touched-crate source checks select the host, kernel engine, local API
and Linux adapter and every unit, integration and binary test target, with locked
offline dependencies, two build jobs and four test threads. No filter, failure
allowlist or new ignore is used. Rust commands use the shared build reservation.

| Check | Observed result |
| --- | --- |
| Rust formatting | exit 0, 1373.19 s including reservation wait |
| Clippy, all selected targets, warnings denied | exit 0, 2.18 s including reservation wait |
| Complete selected source tests | exit 101, 221.24 s; 2410 passed, 72 failed, 70 existing ignored, 0 filtered |
| Failure-set comparison | All 72 identities exactly match the retained T-301 host/Linux baseline; no added or missing failure |

The overall fast gate remains **FAIL**. Root-owned executable trust remains
unavailable in this lane; this is not a native-host pass or resolution of T-301.1.
All three new tests pass:

- `compiler_failure_is_retryable_without_inventing_executed_tests`: both strict
  JSON and Cargo text failures retain zero counts; exact rerun plans verify;
  contradictory counts, success exits, truncation, secrets and cancellation refuse.
- `cargo_compiler_adapter_refuses_ambiguous_partial_or_noncompiler_failures`:
  supported Cargo target and count forms, missing/partial/mixed diagnostics,
  invalid targets, controls, exit contradictions and truncation.
- `compilation_diagnostics_reach_one_repair_then_fresh_edit_and_exact_rerun`:
  both protocols use real held files and canonical encrypted authority with
  **synthetic** creation provenance and command outcomes. One exact failure
  disclosure is consumed, a denied diff preserves bytes, a fresh approved native
  write changes them, and a separately approved exact second attempt passes.
  This is an owner-component test, not native Git creation, model inference or a
  project-test child running inside the agent sandbox.

The first source run found the 72 baseline failures plus one old assertion that
compilation must forbid rerun preparation. Its case now expects an inert retry
preview while retaining every unsuccessful-status, no-pass, minimum-count and
no-extra-dispatch check. No task, hidden test, acceptance threshold or failure case
was removed. The initial log is retained; the complete source gate was repeated
on the final frozen source.

## Actual Cargo observation

A separate disposable, dependency-free Rust crate was compiled using the existing
toolchain under the same reservation. Its type error produced exit 101; correcting
the function produced exit 0 with one passing libtest test. These are standalone
Cargo commands, **not native agent commands, coding scores or model repairs**.

| Command observation | Elapsed ms excluding reservation wait | Public normalizer result |
| --- | --- | --- |
| `cargo test --offline --lib -- --test-threads=1`, type error | 86 | `compile_failed`, zero passed/failed/skipped tests, empty failed names; inert exact rerun plan verifies |
| Same command after source correction | 82 | `passed`, one passing test |

The exact observed stdout/stderr bytes are fed through the public normalizer with
explicitly synthetic command/authority metadata. This does not claim native runner
receipts or confinement. Original channel hashes match the normalized records.
The private normalization probe initially failed to compile because a copied test
helper used a crate-local module path. Its corrected external module import and
successful rerun are retained alongside the original failure. Product source was
not changed for that probe repair.

Raw commands, stdout/stderr, initial/final gates, normalization records and source
bindings are retained in lane-private scratch for review. The three-source freeze
manifest SHA-256 is `5e7a707255bfa969bb2cd37c4c14393cc6046111a6d3ab6c6e9a522d96403b1b`. The successful normalization record SHA-256 is
`cbb10c8fed2cf575a9ca5795bf76a60ffd76443c81b38ab078d2bf54e07264ce`. Final documentation, immutable-review, whitespace, scope, ledger,
privacy and unchanged source-policy checks accompany the private review request.
No subsequent source change is covered without a new source gate.

## Open scope

The new eligibility reaches existing manual API/CLI repair owners; the native
coordinator still uses its separate exact evaluation profiles and fixed scenario.
Configured-endpoint and manual strict-edit/test composition through that coordinator
remains open. No positive native failure-to-repair campaign, durable HTTP recovery,
original-checkout apply, approved commit/push or MCP acceptance is claimed.
Unsupported honest-runner shapes remain backlog, including parallel Cargo failure
notices and custom ignored-test reasons. Independent core review of this batch is
required before acceptance. No parent closes here.

No model request, GPU use, hidden-test read/edit, held-out run or new model score
occurred. No evaluation input, schema, fixture, dependency, supply-chain output,
product status, checklist state, permission boundary or release gate changed.
Historical evidence remains historical. No release or owner-ready claim is made.
