Commit: da13234ff9c3c75d6672db108a8713130c5cccd7
Scope: engine-readme-dual-licence

Reviewer: independent Claude Opus 5.5 session, extra-high effort, read-only, pinned checkout
`/lane/review-checkout`. Base `3a84d30f86006488e7a36a74c4fa4cdd412e7e40`. Row T-301: non-closing owner
README and licence maintenance, with T-317/T-318 as planning only. Tier: core. No GPU, model, candidate,
hidden or held-out test was used.

## Methods

I read the owner orders of 2026-11-16 14:20 and 14:25 in `/lane/control/brief.md`, then the complete
non-mechanical diff. That covers README, NOTICE, LICENSE*, CONTRIBUTING, THIRD-PARTY-NOTICES, Decisions
0210/0211, the evidence record, the TASKS/PRD/plan edits, the guide, both new checkers with their tests,
and the changes to `validate_docs.py`, `status_model.py`, the fast gate, `supply_chain.py`,
`build_contract.py` and `kernel_contract_package.py`. I verified the 177 Rust/TS/Swift diffs by bytes,
not by reading them. I also read both capsule verifier scripts in full and reproduced escape attempts
against the private-path checker in scratch repositories under `/tmp`.

## Commands and results

1. `git rev-parse HEAD` gives `da13234ff9c3c75d6672db108a8713130c5cccd7`. `git status --porcelain` is
   empty. `git diff --check 3a84d30f…` exits 0. The diff has 228 files: 13 added, 215 modified.
2. Requested checks:
   - `python3 scripts/license_policy.py`: `LICENCE POLICY: PASS`.
   - `python3 scripts/private_host_paths.py --base 3a84d30f…`: `PRIVATE PATH CHECK: PASS`.
   - `python3 scripts/supply_chain.py`: `supply-chain artifacts validated`.
   - `local_agent_fast_gate.py --base 3a84d30f… --plan` exits 0 and lists 228 paths. The plan includes
     `private_host_paths.py --base <resolved base>`, `license_policy.py`, `supply_chain.py`, review lint,
     markdownlint of the 14 changed Markdown files, Python lint, 14 unittest modules, and cargo
     fmt/clippy/test for all 11 workspace packages. The plan only lists these commands; I did not run it as
     a gate.
   - `licence_metadata_only` admits the six metadata files only when the change is the exact licence-field
     substitution. Its negative control rejects a changed npm script.
3. `bash /tools/build-slot python3 -B -m unittest` over the ten requested modules: 107 tests, 4 failures.
   - Inherited: `test_canonical_build_contract_passes` fails with the same three diagnostics (workspace
     member set, platforms/linux graph, Cargo.lock closure).
   - Inherited: `test_source_member_allowlist_matches_contract_crate` fails with the same four omissions
     (`endpoint_model.rs`, `runtime_endpoint_run.rs`, `runtime_model_binding.rs`,
     `tests/runtime_approval_selection.rs`).
   - Environmental: `test_canonical_documentation_passes_every_local_integrity_check` reports a "link leaves
     repository" error for `artifacts/sprints/sprint-64/…`, and
     `test_private_and_generated_local_artifacts_are_ignored` gets `node_modules/package/index.js is beyond
     a symbolic link`. Both paths are read-only symlinks the review checkout makes into the builder's
     repository.
   - To isolate the environment, I made a local `git clone --shared` of the pinned commit in `/tmp`, with a
     real copy of the ignored sprint-64 directory. There `tests.test_documentation_controls`,
     `tests.test_story_0_2_acceptance`, `tests.test_private_host_paths` and `tests.test_license_policy`
     pass (28 tests OK). `validate_docs.py` reports `Validated 1066 Markdown file(s) and all policy
     invariants`. The builder's log says 1,067; the difference is consistent with one extra untracked
     Markdown file in the builder's tree.
   - So only the two declared inherited failures remain, and they are still strict failures.
4. Capsule `readme-dual-licence-da13234ff9c3-r2`:
   - `manifest.json` has SHA-256 `e989fbec…f55aa`. Its 154 listed files all match their hashes, with none
     missing and none unlisted apart from the manifest itself.
   - Both verifiers only read, hash and run `git show`. `verify-baseline-drift.py` runs only the base
     revision's own `build_contract.py` and `kernel_contract_package.py`, and first asserts each is
     byte-equal to its Git blob.
   - `verify-preservation.py` returns PASS: 177 SPDX-only source files, 49 historical files, 507 preserved
     ledger lines plus T-317/T-318, and 540 external records unchanged. The inventory original SHA-256 is
     `71ea8e69…`. The scoreboard, the status model and Decision 0051 are unchanged, and the retained review
     is identical.
   - `verify-baseline-drift.py` returns PASS with the same 3 + 4 failures as at the base.
   - Raw logs: `focused-tests.log` has 84 tests and 3 failures. `documentation-tests.log` has 26 OK.
     `filename-diagnostic-test.log` has 1 OK. `supply-chain.log` validated. `build-contract.log` has the 3
     diagnostics. `final-markdown.log` has 0 issues. `final-python-lint.json` is PASS. `format-cancel.json`
     shows the formatter was cancelled while queued and never started. These agree with the evidence text.
5. Markdownlint and the retained review:
   - The pinned node with `markdownlint-cli2` over the 14 listed files: 0 issues.
   - `cmp docs/reviews/3a84d30f8600-inline-schema-31b-development-completion.txt
     /lane/reviews/3a84d30f8600-inline-schema-31b-development-completion.md` shows the files are identical.
     SHA-256 `16508cd9…`; it ends `Verdict: PASS`.

### Independent checks beyond the requested commands

- **README**
  - The owner line is on line 3.
  - The independence paragraph is byte-identical to the base line, a whole paragraph in both versions.
  - 2026-10-25 and 2026-11-02 are given as two separate facts, with "standalone assistant, from August
    2026". The only addition is "alongside the measured local-coding priority", which Decision 0153 itself
    records as its coding directive. No further reason is given.
  - The host is described only as "a desktop host application". The link text reads "historical planning
    list".
  - Scoreboard figures match the 2026-11-16 rows exactly: A4B 19/30, E4B 17/30, 31B 17/30 strict ungraded
    proposals, task passes unavailable, held-out not run, three profiles (90 attempts) remaining. The
    status lines (scaffolded, no enabled models, no platforms) match the unchanged status model.
- **Names.** A scan of added lines finds no other agent product, sibling project, employer or client. "Muse"
  appears only in a pre-existing artifact filename listed in the exception file. "OpenAI-style" describes
  protocol compatibility and is not used as branding.
- **Inventory.** It differs only by a top block: an HTML `markdownlint-disable MD041` comment plus the
  banner. Everything after the first blank line equals the base, SHA-256 `71ea8e69…`. Decision 0210
  explains the replacement and preserves the text, as §35C requires.
- **Licence texts.**
  - LICENSE-MIT is the standard MIT text.
  - LICENSE-APACHE is word-identical to the canonical Apache 2.0 text: one line wraps differently and the
    appendix placeholder holds the owner's copyright. I compared it with the canonical-hash copy
    (`cfc7749b…`) already present under `node_modules`; nothing was downloaded.
  - LICENSE and `kernel/contracts/LICENSE` are identical. Each contains both complete texts.
- **Manifests and headers.**
  - The workspace manifest says `MIT OR Apache-2.0`, all 11 member crates inherit it, and
    `experimental/model-lab` says it explicitly.
  - Both npm package files and their two lock entries agree.
  - Both spec templates say `(MIT OR Apache-2.0)`.
  - No first-party `SPDX-License-Identifier: Apache-2.0` header remains. Every changed .rs/.ts/.swift file
    equals its base version with only the header replaced.
- **Validators.** The old Business Source checks were replaced, not dropped:
  - `validate_docs.py` now calls `license_policy.check` and pins the independence paragraph's hash.
  - The Business Source license pattern is kept for the historical inventory.
  - Negative controls cover truncated texts, manifest drift, an inherited-crate override and a missing
    sign-off or history marker.
- **Dependencies.** The THIRD-PARTY-NOTICES expression table matches all 540 registry records exactly. The
  14 first-party components use the dual expression. elkjs EPL-2.0 and khroma NOASSERTION (both dev-only)
  are reported as findings, not reworded. I found no incompatible production Cargo licence.
- **History and contributions.** README and NOTICE give the two historical periods and claim nothing
  beyond the owner's wording. Decision 0051 is pinned unchanged and superseded by 0211. CONTRIBUTING
  states both licences, `Signed-off-by:` and the name boundary.
- **Private paths.**
  - Exactly 49 tracked files contain the username or a private home path. All 49 are listed, byte-equal to
    the base, and were first added between 2026-08-17 and 2026-09-23.
  - Case-insensitive matches outside them are only the public repository URL.
  - The capsule's tests of staged-hidden edits, filename redaction, symlink targets and tampering with the
    exception file pass.
- **T-317/T-318.** Both are unchecked "Planning only" rows. They add no second runtime, desktop interface
  or folder requirement, and they keep the order of work and MEASURE FIRST.

## Findings

**M1 (medium): the username check matches only an exact, case-sensitive whole token, so a local user name
inside a longer identifier is not detected.**

- Location: `scripts/private_host_paths.py:24` (`TOKEN = [A-Za-z0-9_-]+`) and `:31-32`, which compare the
  hash of each whole token with the username digest.
- Reproduction (scratch repository, the repository's own `check()`, synthetic username `fixture-owner`):
  - A new file containing `ssh fixture-owner_laptop and Fixture-owner` returns `[]` (PASS).
  - Called directly with the real digest, `contains_private` returns False for `<user>-fedora.local`,
    `/tmp/<user>_cache/file`, `1000<user>`, the capitalised or uppercase name, a JSON-escaped
    `\/var\/home\/…` path, URL-encoded `%2Fvar%2Fhome%2F…` and UTF-16 text.
  - Plain token, home path and e-mail forms are caught.
- Why medium: the owner order (14:20, point 3) asks for a check that "fails when a tracked file … contains
  … the local user name". The evidence (`docs/evidence/readme-and-dual-licence-2026-11-16.md:42`), the
  README and the guide say the local username is checked. `<user>-<machine>` is the usual hostname form,
  and the brief lists hostnames among the values that must fail. No such value is committed at this
  commit, so it is not high.
- Fix:
  - Detect the username as a case-insensitive substring of the raw bytes, also after JSON-unescaping and
    URL-decoding.
  - Exempt only the exact public repository URL `https://github.com/AaronNHorvitz/AgentMage` by an explicit
    literal.
  - Add negative controls in `tests/test_private_host_paths.py` for `<user>-host`, `<user>_dir`, digits
    before the name, mixed case and an escaped home path. Keep the check that diagnostics do not echo the
    value.

**L1 (low): the range check covers the end state, not each commit in the range.**
`scripts/private_host_paths.py:68-69` diffs the base against the index and working tree. In a scratch
repository, a leak committed and then removed inside the checked range returns `[]`, although the public
history would keep it. The guide says "in the entire selected range, including committed …"
(`docs/guides/local-agent-development.md:48`). Fix: also check the blobs each commit in
`git rev-list base..HEAD` adds or changes, or say in the guide that only the end state is checked. Backlog.

**L2 (low): an evidence omission on supply chain.** At base, `scripts/supply_chain.py` fails: provenance,
SBOM and hash manifest have been stale since `1d59bed3` (2026-10-04), which I checked in the scratch clone.
This commit's regeneration therefore also adds the previously unrecorded first-party component
`cargo:agentmage-local-api@0.0.0` with its dependency edges, and refreshes the Cargo.lock and package-file
hashes, though Cargo.lock itself is unchanged. The external records are unchanged, as claimed. However,
`docs/evidence/readme-and-dual-licence-2026-11-16.md:47` ("PASS. The full-tree binding is unchanged in
scope") does not say the base check failed or that non-licence drift was caught up. Fix: state both in the
evidence. Backlog.

**L3 (low): packaging.** Both spec templates install only `LICENSE`, not `NOTICE` or
`THIRD-PARTY-NOTICES.md` (`packaging/linux/agentmage.spec.in:30`, `agentmage-release.spec.in:30`). This
belongs to package acceptance, which the evidence already leaves open. Backlog.

Not findings:
- The MD041 directive above the inventory banner. It is disclosed in Decision 0210 and invisible when
  rendered.
- The Apache line-wrap variant, which is word-identical.
- The `/lane/...` paths inside the retained review, which follows accepted earlier practice and contains no
  home path or username.

## Per-row result

- T-301, README/licence maintenance (non-closing):
  - Licence order: all four points met.
  - README order points 1, 2 and 4: met.
  - README order point 3, the path check: medium M1 is open. The exempt list and preservation are correct,
    but detection of the local user name is incomplete.
  - No row closes here, and none was requested.
- T-317/T-318: planning-only rows, correctly unchecked, with nothing claimed as built.
- Not established here: full Rust/source gate, long acceptance gate, any runtime capability, model
  admission or coding score. The inherited source FAIL (1169/53/19) stands as disclosed.
- This review is not a certification, compliance statement or legal clearance.

Reason: open medium M1. The private-path gate does not detect the local user name inside longer identifiers
or in other letter case, as owner order 14:20 point 3 requires.

Verdict: FINDINGS
