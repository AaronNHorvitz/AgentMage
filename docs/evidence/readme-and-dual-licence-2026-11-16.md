# README, historical inventory and dual licence — 2026-11-16

Builder batch under Decisions 0210 and 0211, based on
`3a84d30f86006488e7a36a74c4fa4cdd412e7e40`. Independent **core** review is
required. No row closes, no model is admitted and no release is prepared.
The preceding completed 31B measurement's independent PASS is retained unchanged.

## Result

README describes the engine boundary and recorded limitations, with the owner's
line near the top, the two distinct decision dates, and the original independence
paragraph byte for byte. The host is described only as a desktop host application.
The editor client stays T-310a. T-317/T-318 record later flows and several-agent
work on this runtime, without implementation or a change to MEASURE FIRST.

The historical inventory receives only a banner and a file-local heading-order
lint directive. Every byte below the banner remains unchanged, SHA-256
`71ea8e693134e43a735c4f2dd3ad80dbbbd1533147528ce47dbc4dada51ae4df`.
All 507 original ledger table lines remain unchanged; two unchecked later rows
are appended to the active table. The scoreboard and status-model JSON are
unchanged. No task, hidden test, threshold, prompt or measurement is modified.

Project-owned work is MIT OR Apache-2.0 from this commit forward. Complete standard
texts carry the owner's copyright; existing package LICENSE paths contain both
alternatives. All resolved Cargo licences, the excluded experimental manifest,
root/editor npm entries, package specs, source SPDX headers and generated
first-party supply-chain entries agree. Decision 0051 stays unchanged. README
and NOTICE distinguish the pre-2026-09-07 Apache versions from the subsequent
Business Source versions, whose published licences remain in force.

177 Rust/TypeScript/Swift source files differ only by the SPDX header replacement.
Five Python header replacements are also mechanical; the substantive Python
changes implement the two checks and reconcile the superseded documentation
requirements. Other historical document boundary checks remain enforced, with
negative controls for both the current README and the preserved document rules.

## Checks and limitations

- `python3 scripts/license_policy.py`: PASS. Complete-text hashes, historical
  inventory/Decision 0051, effective manifests, package locks, headers and notices.
- `python3 scripts/private_host_paths.py --base 3a84d30f86006488e7a36a74c4fa4cdd412e7e40`:
  PASS. New/changed working and staged bytes, filenames and symlink targets are
  checked. The explicit 49 historical files are pinned to their baseline blobs;
  edits, deletion and renaming are refused, including staged edits hidden by a
  restored working file. The exception list is itself pinned. Synthetic fixture
  identities are explicitly distinguished from the private username digest.
- Supply-chain build/check: PASS. The full-tree binding is unchanged in scope.
  All 14 generated first-party components use the dual expression; all 540
  external component records remain byte-identical as structured records. No
  third-party licence or dependency version was changed. Historical hash-bound
  evidence is not regenerated or presented as current acceptance.
- Changed Markdown lint, Python syntax/indentation/whitespace, diff whitespace,
  targeted documentation/status invariants and preservation audit: PASS.
- First reserved focused run: 84 tests, 81 passed and 3 failed. The new licence,
  privacy and gate tests passed. One failure was an obsolete negative-control
  stimulus deleting an old runtime name no longer present in README. It now
  removes the required versioned-API boundary instead, preserving the refusal
  assertion. The second reserved run passed all 26 tests in 61.391 seconds:
  Story 0.2, documentation controls and supply-chain tests. It validated 1,067
  Markdown files and all policy invariants. A final targeted filename-diagnostic
  regression passed (one test, 0.028 seconds). The two other failing tests remain
  inherited below; no fresh all-green source/build claim follows.
- Build-contract check: FAIL, with the same three failures reproduced on the
  untouched base inputs: workspace-member set, Linux dependency graph and lock
  package closure. No additional licence error remains. These are inherited
  drift, not a green build-contract claim.
- Contract-package allowlist test: FAIL, with the same four unlisted contract
  files at the untouched base: `runtime_model_binding.rs`, `endpoint_model.rs`,
  `runtime_endpoint_run.rs` and `runtime_approval_selection.rs`. The allowlist
  remains strict; no omission is silently accepted. This is a second inherited
  failing test, separate from the build-contract test.
- Full Rust/source gate and long acceptance gate: NOT RERUN. Rust source changes
  are comment-only; the inherited source disposition remains FAIL (1,169 passed,
  53 failed, 19 ignored). This batch establishes publication/licence checks, not
  a newly passing executable workflow or full acceptance. The fast gate's new
  metadata coverage rejects unrelated manifest changes rather than accepting
  package behavior under a licence-only check. An optional Rust-format
  reservation was cancelled while still queued, before any formatter or build
  started; it is not counted as a passing check.

[Third-party findings](../../THIRD-PARTY-NOTICES.md) retain the unresolved khroma
NOASSERTION declaration, elkjs EPL distribution obligations and the font/icon
asset attribution obligations. A blanket claim that every development dependency
is permissive is not made. Distributable-package notice completeness remains an
acceptance responsibility; no model, runtime or dependency was downloaded.

Raw validation logs, the baseline build-contract comparison and preservation
checker are retained in private lane state for independent review. This batch
uses no GPU and runs no model or candidate. Resume the remaining three frozen
measurement profiles before tuning or new capability work; Git reads and fresh
exact-approved pushes stay at T-306's first permitted capability boundary.
Service actions, sign-in and code-host issue/pull-request tools remain planning.

## Independent review correction — 2026-11-16

The [independent review](../reviews/da13234ff9c3-engine-readme-dual-licence.txt)
returned FINDINGS, including medium M1 in the path checker. Its low L2 also
corrects the supply-chain account above: the base check failed because provenance,
SBOM and the hash manifest had been stale since `1d59bed3`. Regeneration caught
up non-licence drift as well: it added the previously unrecorded first-party
`cargo:agentmage-local-api@0.0.0` component and its edges, and refreshed the
Cargo.lock and package-file hashes. Cargo.lock itself was unchanged. External
records stayed unchanged and full-tree binding scope was preserved. The original
batch's passing regenerated check does not mean the base was current.

Low L3 records that package spec templates install LICENSE without NOTICE or
THIRD-PARTY-NOTICES.md; package notice completeness remains open at T-309 and
package acceptance. The original commit and independent review remain unchanged.
