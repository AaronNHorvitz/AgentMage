# Expanded Rust Roadmap: Source and Evidence Reconciliation

## Baseline and authority

Decision 0081 and the owner's explicit restart supersede the prior narrow implementation
assignment. Base commit: `e06d8f34c5e4c4165a55bd482051c61e3c057627`, branch
`demo/fedora-local-docs`; all 93 prior local commits remain intact. The operator supplied
12 modified documents and three new roadmap/amendment/decision documents, not runtime code.
Their exact changes were read before this implementation increment. Privacy edits retain
historical meaning but do not retain the edited files' previous hashes.

The implementation has no source differences in `kernel`, `capabilities`, `platforms`,
`shells`, `Cargo.toml` or `Cargo.lock` between the native campaign pin
`ad28b5ca7862a483bb4b9bf74c199d5f994ae43f` and the restart base. The following files
were rehashed before new source edits; all matched the previous handoff:

| Identity | SHA-256 |
|---|---|
| Actual CLI binary | `7e1185bf2197a30cbf7665c18c8335c6d07a87ad5260c22581fb5115f672e42a` |
| Ordinary host binary | `935c0404504a3e946a3a2ebe9d15ea7b85ef43a7ee12e2635a60732dcc367799` |
| Native read-only worker binary | `7cbcf459492012491a27607225ddc5c778c589b81956a462441d3ed4c9dec077` |
| Retained Muse campaign13 summary | `62a943cb551be20995a1add1fdbdbaa44d0365883d466a200f57ea891d33a1fd` |

The eight Muse cases remain bounded development evidence at their exact source/model/runtime/
codec/profile, not qualification of this next source revision. GPT-OSS remains separately
not qualified on its tested profile; no unchanged failed campaign is scheduled. The original
rejections, fixes, failures, raw streams and receipts are preserved. See the
[campaign results](coding-harness-campaign13-results-2026-09-23.md), not the superseded
September 22 first-call failure disposition, for the later Muse development result.

## Reuse and exact gaps

| Existing owner | Reused behavior | New acceptance still needed |
|---|---|---|
| CLI, ordinary host, private IPC, shared coordinator | Actual development launch, live events, exact approvals, cancellation and follow-ups | Production activation and broader client/host contracts remain gated |
| Native coding tools and Linux effect owner | Confined reads, exact patches/create, registered command/validation, conflict-aware rollback | New research effects require explicit independent network authority |
| Canonical journal/artifacts/checkpoints | Durable coding resume, drift refusal, verified full outputs, source-backed context | Research records must enter these owners, not a second database |
| Exact Muse/GPT-OSS codecs and native driver | Actual served 32K preflight, family tools/reasoning, bounded output and cleanup | Cross-host inference ownership was absent in the Rust driver |
| `public_research` and `web_research_safety` | Inert bounded request, citation and disclosure validation | No real search-provider transport, mediated DNS/redirect worker or research-to-patch composition exists at the restart base |
| Existing network policy and grants | Closed `NetworkAccess` operation and strict-local refusal | A research policy object must not itself mint an effect permit |

The `later-network` example is still `deny-all`, has no allowed endpoints and disables shell
network access. It is not a configured research provider or an outbound admission artifact.
Account/provider availability and real research qualification must be reported separately from
the implementation work that can proceed locally. No account, credential or paid provider is
obtained by this reconciliation.

The root license and Cargo SPDX metadata already disagree at the restart base. No metadata or
license text is changed here; the owner's separate licensing disposition is still required.

## Explicit freshness disposition

A read-only scan of named SHA-256 bindings in JSON under `artifacts`, `docs/verification`,
`architecture`, `release` and `supply-chain` found 91 distinct bindings to the changed Markdown
files. Comparing each stored digest both to the restart base and the edited file distinguishes
**five newly stale bindings** from **86 already stale bindings**. This is a direct-binding
inventory, not a transitive acceptance audit or a claim that other evidence is fresh.

| Newly stale artifact | Changed bound input | Disposition |
|---|---|---|
| `architecture/schema-evolution-and-rollback.json` | `ENGINEERING-RUNTIME.md` | Historical binding; revalidate applicable schema contract in the batched pass |
| `artifacts/sprints/sprint-1/story-1.2/contract-boundary-report.json` | `TASKS.md` | Current boundary check requires renewal after the source batch |
| `artifacts/sprints/sprint-48/local-evidence-report.json` | `README.md` | Prior collector remains historical, not current amended-document acceptance |
| `artifacts/sprints/sprint-9/story-9.1/linux-native-inference-boundary.json` | `IMPLEMENTATION-PLAN.md` | Previous boundary receipt is not silently rebound; applicable checks need new results |
| `docs/verification/remaining-plan-blocker-audit.json` | Decision 0061 | Previous narrow-scope audit is superseded for sequencing, not erased or treated as new proof |

The scan recognizes path-keyed hashes and explicit path/hash records; unnamed digests and
transitive dependants are not asserted covered. The 86 previously stale bindings are not newly
introduced failures and must not be silently reaccepted. Edited historical privacy/handoff
documents are not test receipts even where no direct binding was found. Native campaign
artifacts remain unmodified at their original pin. Any new Cargo-member source edit also
requires the one cumulative SBOM/evidence pass after the work unit, never a per-file cascade.

## Open gates

AMR package rows remain open. Existing model admission, external independent review, human
acceptance, production transport, supported-platform and release gates are not promoted.
The prior pinned independent-review package remains an outstanding historical candidate;
the new source increment will require a new pinned package, not self-approval or an automated
review-pin update presented as independent review.
