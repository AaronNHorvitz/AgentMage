# Guided worktree coding

The terminal can connect the configured endpoint's strict proposals, reviewed
worktree edits and project tests in one guided command. Every operation still
uses the existing versioned local API and its original native owner. This is a
person-guided client workflow. The separate retained endpoint coordinator
remains read-only; this command does not add an autonomous coordinator or
recoverable run. T-303/T-306 remain open.

## Prepare

Create the isolated worktree through the running service as described in
[worktree ownership](coding-worktrees.md). Select its exact source files and
approve disclosure to the configured endpoint with `select-model` or
`map-select`. Read the complete selected paths; repository ranking is lexical.

Prepare a complete [project-test definition](coding-validation.md): its sealed
command, test kind, minimum count, optional built-in output format and
`approve_definition: true`. Initially omit `prior_receipt_sha256` or set it to
null. The command must fit the original offline sandbox. No model output may
choose or change the test command, parser or minimum count.

Use a worktree with no pending edit or validation and no previous validation
attempt. The guided command refuses an existing series rather than resetting
its budget or guessing how to recover interrupted work.

```sh
target/debug/agentmage-code <loopback-ip:port> <private-auth-directory> code-worktree <absolute-worktree> <model> 16384 2048 '<task>' '<complete-test-definition-json>'
```

The default guided budget is 3600 elapsed seconds. Choose a smaller budget with
the same arguments after an explicit number of seconds (1 through 3600):

```sh
target/debug/agentmage-code <loopback-ip:port> <private-auth-directory> code-worktree-bounded 300 <absolute-worktree> <model> 16384 2048 '<task>' '<complete-test-definition-json>'
```

The clock is monotonic running time and excludes system suspend. One clock
covers preparation, model requests, every approval wait, tests and
repairs; a new attempt never renews it. SIGINT, SIGTERM and SIGHUP latch a stop.
After expiry or interruption, the terminal stops sending further requests and
closes a pending API read. Prompt waits poll for a stop; socket reads retain
their short polling timeout. Connect/write system calls can take up to their
existing two-second bound to return. This is cooperative client stopping:
server work already sent may continue. Inspect service records and disk;
use a separate explicit control action if needed. The client sends no automatic
cancellation, denial, release or retry. A pending preview can remain.
Keep draining terminal output: presentation backpressure and filesystem stalls
are not bounded by an operating-system watchdog.

No model is bundled or downloaded. The ordinary configured endpoint transport
owns its key; the terminal never requests or stores that key.

## Approvals and stopping

The command displays its plan and first prepares the exact native test
definition. Native admission failure stops before requesting model output.
Preparation executes nothing. It then requests an ordinary strict proposal
under the existing selected-source disclosure consent.

Each attempt requires these separate controlling-terminal decisions:

1. `read <root-sha256>` permits reading the displayed selected worktree files
   to prepare the inert diff.
2. `edit <preview-sha256>` approves only the complete displayed worktree diff.
3. `test <preview-sha256>` approves one exact offline test execution.

After a retryable assertion or compilation failure, `share-failure
<feedback-sha256>` separately permits sending the displayed diagnostics to the
configured endpoint. Then another read, diff and test decision is required.
Declining any decision stops the command without sending that action. Consent
cannot be piped or supplied with an automatic-approval option.

There are at most three test executions and three proposal requests, including
the initial attempt. Each uses the existing source and output limits. The
original service separately enforces its test-attempt ceiling and one repair
request per failed receipt. Settings and source-selection identity stay pinned
between attempts. Each new preview must retain the complete test template,
worktree creation binding, expected attempt and exact previous receipt.
The terminal keeps the complete original edit-prepare reply, verifies its
worktree root and creation binding, and requires both subsequent pending-edit
inspections to equal that reply. Replacing an edit before either inspection
stops the attempt before edit consent, even when root and ownership are unchanged.
After the person approves, the service still requires the exact preview digest
and original strict preimages before it can write. No automatic retry follows
a replacement or lost reply.

Every proposal request carries the original workspace selection identity,
settings revision, exact endpoint and held worktree root digest. The original
API checks these under its operation lease before reading source or contacting
the model. Fresh helper reads cannot replace the workflow's selection or
settings. A concurrent replacement after the terminal's checks is refused at
admission. The root pin also refuses a source selection from another directory
before initial disclosure. These pins narrow existing source consent; they do
not authorize disclosure, edits or tests.

The original kernel command and validation verifiers check returned receipts
against the selected command and template. Only a full passing normalized
receipt ends successfully. The original rerun planner decides repair eligibility;
malformed, zero-test and flaky results cannot trigger another request. This
checks approved-command test observations, not an independent task grade,
source correctness or a coding-model score.

Every API request is sent once. A disconnect, timeout, refused request, uncertain
result or lost effect reply stops the workflow. Inspect the original API's
`edit`, `validation`, `command` and status records and the worktree before
continuing manually. The command neither cancels nor repeats an uncertain
effect. A declined prepared preview may remain for explicit inspection or denial.
Restart loses the guided client's state; it never reconstructs approvals,
ownership or retry capacity. Per-request socket deadlines retain the existing
180-second ceiling and are capped by the remaining guided budget. Human approval
waits share that same guided budget.

## Current evidence and limits

Actual API-service consumer tests cover both guided commands refusing a
directory without held worktree ownership, including after restarting the
service. They also cover repair refusal without an owned failed receipt, and
the individual `propose-worktree` command consuming complete or truncated
streams from a synthetic upstream through the original service. Complete
generation still cannot adopt that directory or prepare an edit. These are
refusal and protocol observations; a successful native edit/test/repair cycle
through the actual service remains unverified. See the
[service-consumer record](../evidence/t303-guided-service-2026-11-08.md).

Executable tests drive this terminal through a recording synthetic HTTP peer,
including failure, repair and success, all approvals, three-attempt exhaustion,
native preflight refusal and lost replies. Additional cases stop at all four
approval kinds and at stalled headers, bodies and proposal streams after model,
edit and test requests. They assert an exact request prefix and peer-observed
socket closure, without cancellation or retry. The peer supplies synthetic endpoint,
write and command observations. It does not execute a model or native worker or
change fixture source files.

Native executable admission remains required and currently prevents a positive
complete native workflow in the builder lane. Local-model coding measurements,
autonomous endpoint edit/test composition, durable execution recovery and
approved Git remain open. Original-checkout transfer stays a separate operation.
See the [builder record](../evidence/t303-guided-worktree-2026-11-08.md).
