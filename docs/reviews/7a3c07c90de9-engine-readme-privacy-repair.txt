Commit: 7a3c07c90de9c808ef24ead1b3c57bb6acf775ce
Scope: engine-readme-privacy-repair

Reviewer: independent Claude Opus 5.5 session, extra-high effort, read-only, pinned checkout
`/lane/review-checkout`. Base `da13234ff9c3c75d6672db108a8713130c5cccd7`. Row T-301: non-closing owner
README/licence maintenance, and the one re-review of medium M1 from
`da13234ff9c3-engine-readme-dual-licence`. Tier: core. No GPU, model, candidate, hidden or held-out test
was used.

## Methods

I read the owner orders of 2026-11-16 14:20 and 14:25, the repair evidence
`docs/evidence/private-path-review-repair-2026-11-16.md`, and the complete diff: 8 files, +389/-12.

- I read `scripts/private_host_paths.py` and `tests/test_private_host_paths.py` in full.
- I read the capsule verifier `verify-preservation.py` in full. It only reads files, hashes them and runs
  `git show`; it imports nothing from the archive.
- I ran escape probes against the repository's own `check()` in scratch Git repositories under `/tmp`
  (synthetic username `fixture-owner`, 13 bytes).
- I called `contains_private()` directly with the real policy digest and length. Every real-username value
  was built in memory and never written to disk or into this file.
- I also scanned all 9,687 tracked files with the new detector.

## Commands and results

1. Pinned commit:
   - `git rev-parse HEAD` gives `7a3c07c90de9c808ef24ead1b3c57bb6acf775ce`.
   - `git status --porcelain` is empty.
   - `git diff --check da13234f…` exits 0.
2. Fast gate:
   - The requested `/lane/state/readme-m1-repair-20261116/bin` does not exist in this environment. I made
     `/tmp/m1rev/bin/node` as a symlink to `/lane/reviews/42612ca9787f-a4b-node`. That node has SHA-256
     `d1de76d8…bd932c`, the same as in earlier reviews, and is v24.15.0.
   - `PATH=/tmp/m1rev/bin:$PATH python3 scripts/local_agent_fast_gate.py --base da13234f… --timeout 3600`
     with `CARGO_TARGET_DIR=/lane/state/review-target` gives `FAST GATE: PASS`.
   - The gate ran: diff check, untracked lint, the private path check (PASS), the licence policy (PASS),
     review lint, markdownlint over 4 files (0 issues), Python lint, and through build-slot
     `tests.test_license_policy` with `tests.test_private_host_paths`: `Ran 18 tests in 0.515s OK`.
   - Wall time was 2m47s, almost all of it waiting for the build-slot reservation.
3. Direct checks:
   - `python3 scripts/private_host_paths.py --base da13234f…`: `PRIVATE PATH CHECK: PASS`.
   - `python3 scripts/license_policy.py`: `LICENCE POLICY: PASS`.
   - `python3 -B …/verify-preservation.py`: `status PASS`, 49 historical exceptions, 509 ledger table lines,
     30 preflight contexts, review byte-identical.
   - The capsule manifest SHA-256 is `d8e86b2d…13b7`, as stated.
   - All 8 files under the capsule's `public/` are byte-equal (`cmp`) to the pinned checkout.
4. Retained review: `cmp docs/reviews/da13234ff9c3-engine-readme-dual-licence.txt
   /lane/reviews/da13234ff9c3-engine-readme-dual-licence.md` reports identical files. It ends
   `Verdict: FINDINGS`.
5. Policy:
   - The diff of `architecture/private-path-exceptions.json` adds only `"username_bytes": 13`. The
     baseline, digest and 49 `preserved_files` are unchanged; the verifier asserts this against
     `git show` of the base.
   - `sha256sum` of the new JSON is `6c1b07bb…ecbe`, which equals the new `POLICY_SHA256`.
   - The policy digest equals the SHA-256 of the local username, which is 13 bytes and lowercase. So
     comparing lowercased substrings is sound.
6. Tests: the test diff only adds tests and sets `username_bytes` in `setUp`. No test was removed and no
   assertion was weakened.
7. Initial failure: the capsule's `logs/gate.log` shows the first gate failing as disclosed. It reports
   "staged private-path exception policy changed" and flags the retained review and the test file, then
   prints `Ran 17 tests`, `FAST GATE: FAIL`. `logs/gate-final.log` has `Ran 18 tests in 0.645s` and
   `FAST GATE: PASS`.
8. Names: added lines contain the local username case-insensitively only twice, both inside the exact
   public repository URL. One is in the retained review and one is the `PUBLIC_REPOSITORY_URL` literal.
   Added lines name no other agent product, sibling project, employer or client.

### Escape probes (repository `check()` in scratch repositories; FAIL means detected)

| Probe | Result |
| --- | --- |
| `<user>-fedora.local`, `/tmp/<user>_cache/f`, `1000<user>`, upper, title, mixed case, e-mail | FAIL (all) |
| JSON-escaped home path; `\u`-escaped letters; URL-encoded path; URL-encoded letters | FAIL (all) |
| Double URL encoding; JSON then URL; URL then JSON | FAIL (all) |
| JSON- and URL-encoded home paths of another synthetic user | FAIL (both) |
| UTF-16 with BOM, UTF-16LE and UTF-16BE without BOM, UTF-32 with BOM, UTF-32BE | FAIL (all) |
| Private filename: uppercase, inside a directory component, URL-encoded | FAIL, value redacted |
| Symlink whose target contains the username | FAIL |
| Staged leak with clean working bytes | FAIL ("staged …" diagnostic) |
| `%25` nested 10 levels (past the 8-step bound) | FAIL (refused) |
| `%25` nested 5 levels, benign | PASS |
| `<home>` ellipsis placeholder alone | PASS |
| Ellipsis or three dots followed by the username | FAIL |

No diagnostic contained the synthetic username or the other synthetic user name.

With the real digest and length, `contains_private()` is False only for these:

- the exact public URL in a Markdown link, in angle brackets or in quotes;
- a URL-encoded spelling that decodes to exactly that URL.

It is True for all of these:

- the URL followed by `-private`, `/blob/…`, `.git` or a trailing period;
- the URL in upper or lower case, with `http`, or with a leading `x`;
- the handle on a foreign host or with another repository name;
- a JSON-escaped URL;
- the URL followed on the same line by `ssh <user>-host`;
- `<user>-laptop`, an uppercase username, a home path or an e-mail address.

So the exemption cannot be borrowed by a suffix, prefix, case change or another host.

Not detected:

- a UTF-16LE run at an odd byte offset whose first character is the username's first letter;
- UTF-32 text at byte offsets 1 and 2;
- another user's home path as UTF-16 at an odd offset;
- `<home>//name`, `<home>/./name` and `~name`;
- HTML entities, full-width letters and base64;
- a leak committed at HEAD but hidden by a clean index and a clean working tree;
- a leak added and then removed inside the range (L1, already backlog).

### Whole-tree scan with the new detector

Six tracked files outside the 49 exceptions would now fail if they were changed:

- Five are false positives, all already flagged by the old case-insensitive home pattern:
  - an uppercase `HOME/LANG` environment-variable list after a slash, in four retained reviews;
  - a `Users/[^` regular expression in `scripts/story_0_3_macos_evidence.py`.
- One is real; see B1.

## Findings

M1 is **resolved**. The checker now does what M1 asked:

- it detects the username as a case-insensitive substring inside identifiers;
- it inspects JSON-unescaped, URL-decoded and aligned UTF-16/32 views within a bounded decode that
  refuses rather than accepts when the bound is reached (`scripts/private_host_paths.py:36-63`);
- it exempts only the exact literal public URL, with boundaries (`:26-28`);
- negative controls cover the requested forms;
- diagnostics omit the values;
- the username stays a digest plus a byte length.

No high or medium finding remains. Low and backlog items:

**L4 (low): misaligned UTF-16/32 is not inspected.**

- Location: `scripts/private_host_paths.py:38-40` decodes UTF-16/32 only from byte offset 0.
- Effect: a UTF-16LE run at an odd offset whose first character begins the username loses that character
  in every view and passes. UTF-32 at offsets 1 and 2 passes entirely. I reproduced both in a scratch
  repository.
- Exposure: this needs a binary container with an embedded wide string, so it is low.
- Fix: add a NUL-stripped view (`data.replace(b"\x00", b"")`), which catches ASCII in UTF-16/32 at any
  alignment, or decode at offsets 1 to 3 as well. Add a negative control. Backlog (T-309).

**L5 (low): the committed end state is not scanned when the index differs.**

- Location: `:101-103,113` checks the index and working tree, but not the blobs in the HEAD tree.
- Reproduction: commit `<user>-host`, then stage and write clean bytes. `check()` returns `[]`.
- The guide (`docs/guides/local-agent-development.md:48-52`) says "end state, including committed".
- Effect: in a clean checkout HEAD equals the index, so the gate is sound as reviewers run it. This is a
  variant of L1.
- Fix: also check `git show HEAD:<name>` for names in `git diff --name-only base HEAD`, or reword the guide.
  Backlog (T-309, with L1).

**L6 (low): heuristic gaps beyond M1.**

- Another user's home path written as `<home>//name`, `<home>/./name` or `~name` is not detected.
- The username written with HTML entities, full-width letters or base64 is not detected.
- None of these was in M1's fix list. Backlog.

**B1 (backlog, out of scope; owner attention): a pre-existing file links to another repository under the
owner's handle.**

- The file is under `docs/architecture/`. Commit `17e337b4` (2026-09-16) added it and `4ad6d700`
  (2026-09-18) last changed it; this commit does not touch it.
- It contains the handle seven times in links to a different repository of the same owner, not the
  AgentMage URL.
- It is not one of the 49 exceptions, so any future edit fails the new gate. That is conservative and
  correct.
- It also names that other project. I cannot tell whether that project is a private sibling. If it is,
  the name in a published file falls under the public-repository rule. That is for the owner, outside
  this commit.
- Correction to my review of `da13234f`: "Case-insensitive matches outside them are only the public
  repository URL" was wrong. The old exact-case token check missed this file, and so did my own count.

**B2 (backlog, note):**

- The five pre-existing false positives above will make the gate fail when those files are next edited.
- The 18.443-second preparation figure cannot be traced in the capsule; the 44.628-second figure matches
  `measurement/failed-preparation-cleanup.json`. Neither is a score, and the scoreboard is unchanged.

Other items checked and not findings:

- Typographic-ellipsis placeholder. It is exempt only as the whole captured component, and a negative
  control shows it cannot hide a username.
- Test strings built at runtime. They are synthetic identities only.
- The `model_started: true` field in the cleanup record. It is consistent with "no inference request",
  and the API and model were reaped.

## Per-row result

- T-301, README/licence maintenance (non-closing):
  - README order point 3 is now met. New or changed files with a private home path or the local user name
    fail; the 49 published files stay byte-identical and are exempted by the explicit list; the
    exception set and the pin were not weakened.
  - L1 wording now matches the end-state behaviour, apart from the HEAD-versus-index case (L5).
  - The L2 addendum accurately states the base supply-chain staleness and the catch-up.
  - L3 stays at T-309 or package acceptance.
  - The original commit, Decision 0051, the inventory and the 49 historical blobs are unchanged.
  - No row closes, and none was requested.
- MEASURE FIRST: the scoreboard and the 509 ledger table lines are unchanged, and nothing that the model
  sees or that edits, tests or retries depend on changed. The order of work is unchanged.
- Not established here: the full Rust/source gate, the long acceptance gate, any runtime capability,
  model admission or coding score.
- This review is not a certification, compliance statement or legal clearance.

Verdict: PASS
