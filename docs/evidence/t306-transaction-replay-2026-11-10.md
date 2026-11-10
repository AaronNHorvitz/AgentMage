# T-303/T-306: bind local commits to durable transaction identity

Status: independent core review required. T-303/T-306 remain open without new
rows or sub-rows. The full affected gate remains FAIL.

The dormant local commit driver now requires the consumed authority transaction
identity to equal the identity in its approved plan. A reconstructed driver
cannot use a renamed transaction to bypass the canonical store's existing
duplicate-transaction refusal. Its first-entry latch, live clock/grant checks,
exact complete packet and request binding, cancellation and native boundaries
remain required. A changed plan identity needs a new exact approval.

Three new tests cover a fresh driver's exact transaction binding, encrypted
restart after success and encrypted restart at all six durable crash boundaries.
The latter two use actual encrypted-store checkpoints, recovery and the real
commit driver. The original issuer and coordinator are discarded before reopen.
Each case issues a new policy-valid grant in that same reopened store. Reusing
the approved transaction refuses before consumption; renaming it consumes the
new grant but fails at the driver with no executor launch. Another reopen
preserves the original success/failure/uncertainty receipt and the new refusal.
No terminal outcome is turned into a retry or a new success.

The older dispatch fixtures now select the plan's actual transaction identity
so clock, operation, scope and unwind tests continue to reach their intended
checks. The reentry test also repeats that exact identity under an independent
synthetic coordinator, keeping its latch assertion independent of the new guard.
The helper's test-only visibility permits reuse across the two test modules;
production constructors and authority types are unchanged.

This is component composition, not a resumed native coding workflow. Held
objects, manual approval, clocks and Git executor results are synthetic. It
authenticates neither consent nor checker/artifact provenance. Protection is
conditional on retaining the same canonical store; a fresh or replaced store
is not covered. Native admission, original-owner approval composition,
repository/signer recovery, model measurements and approved push remain open.

The parent single-use commit receives independent **Verdict: PASS**, with no
high or medium finding; its exact report is retained unchanged. Its low L-1
notes that an executor reconciliation error is currently reported as a failed
launch rather than an uncertain effect. That pre-existing dormant-driver issue
remains same-row backlog, alongside L-2 on initial-focused-run wording and the
earlier lows. This batch's durable uncertainty checks cover crash recovery,
not that post-executor error conversion. Prior isolated rollback and operation/
held-scope coverage lows are independently confirmed addressed. Neither row closes.

## Validation

The initial focused run passes all 21 local-commit methods, including all three
new tests: 13.25 seconds compilation and 2.11 seconds testing. A subsequent
kernel Clippy run with warnings denied passes in 21.97 seconds. Two mutations
are killed: removing transaction binding fails the isolated reconstruction
method and both durable methods; removing the latch fails behavioral reentry
under the unchanged approved transaction. Source bytes are restored after each
mutation. These checks establish test sensitivity, not acceptance or gate PASS.

The complete affected gate remains **FAIL**: **3,090 passed, 72 failed,
72 ignored**, zero measured or filtered, over **46 unfiltered targets** of the
kernel, host, Linux adapter and local API. Every per-target failure and ignored
identity matches the exact parent baseline. Three new kernel methods add three
passes; all 44 methods in the changed sources pass, including 41 existing
controls. Formatting and Clippy with warnings denied pass. All three source
hashes stay fixed throughout the run, with no later source edit.

The full run takes 399.67 seconds including shared reservations: formatting
2.41 seconds, Clippy 21.59 and Rust tests 374.21. No timeout occurs. Raw log
SHA-256:
`077a2133324e87a9f91c799d81b73465f65625979ed6f44caee3d15b42f3982a`.
The exact comparison exits zero; this is data comparison only, never gate PASS.
Private reproduction scripts, source freeze, baseline and raw logs are retained
in `/lane/state/tmp/t306-transaction`. The full runner reserves heavy commands
internally; never outer-wrap it. Only documentation and the exact retained
parent review follow source freeze.

No evaluator, hidden/held-out task, scorer, prompt, threshold, dependency, SBOM,
lifecycle, scoreboard or release change. No GPU, host key, download or push.
