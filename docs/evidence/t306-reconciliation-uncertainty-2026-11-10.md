# T-303/T-306: retain uncertainty after local commit reconciliation failure

Status: independent core review required. T-303/T-306 remain open without new
rows or sub-rows. The complete affected gate remains FAIL.

The dormant commit driver now distinguishes refusal before executor access from
failure to reconcile an executor result. After executor invocation, every
reconciliation error returns an uncertain outcome and state change instead of
claiming that the effect boundary was not crossed. It hashes only the stable,
content-free error code, retains the original boundary error and emits no local
success receipt. Neither a platform failure label nor missing evidence proves
rollback. Existing first-entry, clock, grant, exact transaction, packet and
request checks remain required.

One new test composes the actual driver, policy, grant consumption, coordinator
and encrypted store. Seven synthetic result cases cover incomplete cleanup,
unverified signature, resealed preservation drift, missing commit object and
failed, cancelled or uncertain platform outcomes. Each starts from a synthetic
observed ref update. All seven run through normal completion and crashes after
worker return and result reconciliation. Original issuer and coordinator objects
are discarded before recovery. The test asserts an uncertain terminal receipt,
uncertain transaction and grant, exact receipt preservation on a second reopen,
and transaction replay refusal with no second executor launch.

The existing single-use test now requires exact outcomes: uncertainty after
unproved results, failure before executor access and success only with a proved
result. Held objects, approval, clocks, ref updates and Git results remain
synthetic. These are component tests; native Git, signer recovery, authenticated
consent and an executable product workflow are not demonstrated.

This addresses the retained single-use review's low L-1 within the same rows,
subject to independent review. Its low L-2 wording is clarified in the original
evidence: the fourteen-method run preceded the final internal-assertion removal;
the final full gate validated the final test bytes. Other same-row lows remain
backlog. No row closes and no new row is added.

The parent transaction-binding commit receives independent **Verdict: PASS**
for exact commit `d46e86f0993e9e1f7b7a35d3bf6f7a83a0433dbb`, with no high
or medium finding. Its unchanged report is retained. It carries the same low
post-executor uncertainty issue; this batch's fix still requires its own review.

## Validation

The initial focused run passes all 22 commit methods: 14.35 seconds compilation
and 6.66 seconds testing. A mutation restoring the old post-executor
`EffectLaunch::failed()` conversion fails both the new durable method and the
strengthened single-use method. The production source is restored exactly.
This measures test sensitivity, not acceptance.

The complete affected gate remains **FAIL**: **3,091 passed, 72 failed,
72 ignored**, zero measured or filtered, over **46 unfiltered targets** of the
kernel, host, Linux adapter and local API. Every failed and ignored identity
matches the exact parent baseline. The new kernel method adds one pass; all
45 methods in the changed sources pass, including 44 existing controls.
Formatting and Clippy with warnings denied pass. All three source hashes stay
fixed throughout the run, with no later source edit.

The complete run takes 465.71 seconds including shared reservations:
formatting 88.69 seconds, Clippy 21.50 and Rust tests
353.94 seconds. No timeout occurs. Raw log SHA-256:
`02a4c39ad7e7db9454d2cb27035a20630240bdd6648137ab23fb7621d508caac`.
The exact comparison exits zero; this is data comparison only, never gate PASS.
Private reproduction scripts, baseline, source freeze and raw logs are retained
in `/lane/state/tmp/t306-reconciliation-uncertainty`. The full runner reserves
heavy commands internally; never outer-wrap it. Only documentation and any exact
retained parent review may follow source freeze.

Trusted checker/artifact integration, full original-owner approval composition,
native admission, repository/signer recovery, model measurements and approved
push remain open. No product commit route is enabled. No evaluator, hidden or
held-out task, scorer, prompt, threshold, dependency, SBOM, lifecycle, scoreboard,
release or readiness change. No GPU, host key, model download or push.
