# T-303/T-306: original terminal tool ownership through the HTTP service

The trusted host can select `serve_admitted_coding_endpoint_returning_tool`.
The existing protected release route retires only a known, fully published,
non-uncertain terminal through the original host registry checks. It retains
the original boundary until shutdown joins all HTTP workers, then returns it
to the host. Observation delegation supplies no release authority. No tool object
is serialized over HTTP and no new route is introduced. Ordinary service entry
points continue to discard on release.

An unreleased run returns no tool on shutdown, including after cancellation.
Live approvals, faults and uncertainty refuse release. Uncertain refusal keeps
the original coordinator inspectable. A later run needs fresh service identities,
admission, model-disclosure consent and exact approvals; previous responses and
bindings refuse. Prepared native boundaries must abandon their old previews
through `into_repository` before preparing subsequent work. This is process-local
continuation, not durable recovery. Service errors may drop retained state.

## Tests and limitations

Three added integration methods exercise the real HTTP service, real upstream
sockets, original coordinator and original host release checks with synthetic
tools, authority observations, endpoint replies and a rejecting verifier:

- Two services return the same heap allocation and same execution counter; each
  requires fresh approval, refuses old requests and approval responses, refuses
  observation-only or substituted release and permits only one successful release.
  Each verifier reports failure; tool execution is never presented as task success.
- Uncertain and faulted runs refuse repeated releases, retain their event page
  and terminal/fault state, never redispatch, and return no boundary on shutdown.
  This exercises the host refusal-restore path identified by the predecessor's
  low L1 coverage note.
- An unstarted run and an unreleased cancelled run return no tool. Explicit
  release after pre-dispatch cancellation returns its original owner without
  any model or tool dispatch.

The [predecessor independent review](../reviews/b62f35cfbaef-local-coding-terminal-owner-return.txt)
has **Verdict: PASS** for exact commit
`b62f35cfbaef6a786cc64e172c7729b9cc889e18`, with no high or medium finding.
Its low L2 (prepared-boundary positive admission/return coverage) stays open.
The [command-output independent review](../reviews/c7a63ebefe55-local-coding-command-output.txt)
also arrived with **Verdict: PASS** for exact commit
`c7a63ebefe55544444c3d7fc398ebba2ec7e082f`, with no high/medium finding.
Its low validation-metadata allowance and remaining aggregate-output preflight
findings remain same-row backlog; the overflow-helper note requires no change.
That report arrived after the final core gate; its byte-identical retention and
updated prose passed the final light checks. The Rust source freeze is unchanged.

The positive native factory path could not be exercised here: native artifact
admission rejects the sandbox's mapped ownership of system executables. No trust
requirement or executor is changed to bypass that refusal.

The first development invocation compiled but failed: malformed fixture approval
casing produced HTTP 400 in two tests, followed by a receiver-drop cleanup
panic. The fixture now serializes the contract enum directly and tolerates a
receiver dropped during assertion cleanup. The focused retry passed all three
methods (12 filtered, 0 ignored, 0 measured; 0.16 seconds). No production parser,
validator or threshold changed. Both raw logs are retained privately.

The first complete scoped gate passed with its source freeze unchanged in
52.02 seconds including shared-slot reservations. A final documentation precision
clarifies that the returned boundary retains its existing native authority; no
grant is renewed. The final complete scoped fast gate passed (exit 0): formatting,
Clippy with warnings denied, and all targets of `agentmage-local-api` passed.
Ten targets report 202 passed, 0 failed, 0 ignored, 0 measured and 0 filtered;
all three added methods pass. Formatting took 82.50 seconds including its shared
reservation, Clippy 1.50 seconds and tests 40.22 seconds. The complete gate took
126.05 seconds including reservations, with all four Rust hashes unchanged.

The existing fast-gate selector selects only `agentmage-local-api` for these
changes. This is not a workspace-wide or long acceptance run; the unchanged
host/Linux test packages with the inherited T-301 failures are outside this
selected scope. There are no test filters, failure allowlists or new ignores.
Final whitespace, retained-review formatting, Markdown and privacy/ledger audits
pass. The retained predecessor review is byte-identical to its independent file.
Owned runtime temporary namespaces were empty and removed by the runner. No
model, GPU, download, public release or owner-ready action occurred.

No native Git, confined command, local-model workflow or coding score is
established by these synthetic integration tests. Executable controlled startup,
general model-driven edit/test/retry, durable recovery, approved Git and parent
measurements remain open. No evaluation task, hidden test, held-out set, scorer,
threshold, dependency, SBOM or status authority is changed. T-303/T-306 remain
open; no row or sub-row is added or closed. Independent core review is required.
