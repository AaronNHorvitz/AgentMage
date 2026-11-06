# T-303/T-306: connected snapshot proposals

This first implementation batch follows the independently closed T-311 baseline.
T-303 and T-306 remain open. It adds a running proposal API, not an executable
agent workflow, filesystem tools, isolated worktree or permission to apply code.
No additional evaluation row or review-finding sub-row is created.

## Connected capability

The existing local HTTP service now exposes
[`POST /v1/code/proposals`](../guides/coding-proposals.md). A client supplies an
authorized source snapshot and task. The repository-map crate selects bounded
parser-backed definitions and lexical excerpts with exact original ranges and
complete file digests. The existing credential-owning transport submits those
messages to the explicitly configured local endpoint. Streamed model text stays
provisional until a strict original-source edit check returns candidate changes.

The result explicitly says `applied: false`, `tested: false` and
`authority: "proposal-only"`. No source path is opened or resolved. No proposed
text is executed. Commands, workspace writes, repository collection, worktree
ownership, approvals, durable interruption/resume/replay, retry feedback,
diff/apply, approved branch commits/pushes, CLI and MCP remain unimplemented in
this mission path. Native authority and sandbox boundaries remain prerequisites
for those effects; these snapshot helpers confer no grant.

Strict parsing refuses duplicate and unknown keys, positional arrays, nested
values, trailing JSON, ambiguity including overlapping matches, invisible or
overlapping edits, stale complete snapshots, unchanged results and excess growth.
Length-limited, malformed and incomplete streams cannot become proposals.
The original frozen baseline collector and its parser remain unchanged; this
product path uses a new native typed parser with nesting-refusal regressions.

The new inference dependency is the existing repository-map workspace crate.
Cargo.lock changes only that workspace edge; no third-party dependency/version,
model, licence or acquisition changes. The new code retains the existing Apache
headers and does not claim that the import licence reconciliation is complete.

## Builder checks before model measurement

Rust commands used the pinned offline toolchain and shared build reservation.
Lightweight documentation and Python checks also ran through the ordinary gate:

| Check | Observed result |
| --- | --- |
| Repository-map `coding_proposals` integration target | PASS: eight methods, zero ignored/filtered, 0.02 seconds |
| Inference `local_chat_api` integration target | PASS: all 20 methods, zero ignored/filtered, 4.27 seconds |
| New measurement-client unit module | PASS: four methods, 0.017 seconds on its first run; ordinary gate repeat also passes |
| Workspace formatting and Clippy, all targets, warnings denied | PASS; Clippy 47.37 seconds including its reservation |
| New/changed Markdown, Python syntax/indentation/whitespace and Git whitespace | PASS in the ordinary gate |

The first focused compile failed because the pinned SHA-256 output type lacks
the requested hexadecimal formatting trait. The digest helper was corrected;
the failed log is retained. No test or threshold changed. Initial shared-slot
queue waits executed no Rust checks. These are real socket contracts against
synthetic upstreams; they are not local-model coding scores.

The full unchanged per-change gate is **FAIL**: 2,993 Rust tests pass, 72 fail,
78 retain their existing ignored status, and zero are filtered. The exact 53
host and 19 Linux failure identities equal the retained T-301 baseline union;
there are no added or missing failures. The test command takes 288.82 seconds
including its reservation. These remain failures under deferred T-301.1, not an
allowlist or full-gate PASS. No trust check, test or threshold was relaxed.

## Fixed development measurement policy

The separate [`coding_proposal_measurement`](../../scripts/coding_proposal_measurement.py)
collector is frozen with this source batch before any requests:

- All 30 frozen development tasks, with their complete original editable base
  snapshots. Only statement, source path and editable source enter this API;
  references, hidden tests and Git history do not.
- 6,000 application message bytes, 2,048 output tokens, one attempt per task,
  no retries, test feedback or command execution. These budgets differ from the
  original 8K/4,096 chat-only baseline and are separate measurement variables.
- Existing local Muse and GPT-OSS weights, existing bounded model launcher,
  observed 8,192-token context, temperature zero, seed 42 and family-specific
  medium reasoning setting. No downloads or role/model-quality selection.
- At most five tasks per shared build/GPU reservation, releasing resources and
  recording progress between batches. Reserved GPU work minutes are launcher
  wall time, not active kernel minutes. Observed backend counters supplement
  unavailable API usage; unobserved token counts remain null.
- Every refusal retains an explicitly labelled base fallback in the denominator.
  Candidate snapshots are scored separately by the unchanged independent
  admission command and accepted complete controls, never by model prose or
  collection success. Hidden assertions, task statements, source pins and
  thresholds remain unchanged.

The collector intentionally has no held-out surface. This is a non-closing
T-303/T-306 implementation batch; the held-out set is reserved for the combined
row-closing scoreboard, never intermediate tuning. Development measurements
are pending at this checkpoint; no model request or generated score is claimed.

## Remaining scope

Byte-bounded retrieval is not exact rendered-codec token admission, complete
repository understanding or automatic compaction. Names and excerpts remain
untrusted data, and omissions stay visible. The new route has process-only
activity/stream retention, not a durable run record. The original transport and
its documented backlog remain in use. No production model, supported platform,
independent row closure, release or owner-test-ready state is promoted.

Supply-chain and historical acceptance evidence are not regenerated mid-batch.
The mission fast gate retains its existing scope and the long acceptance gate
remains T-310 work. Subsequent coding tools must preserve exact approval,
workspace confinement, source freshness and independent checking.
