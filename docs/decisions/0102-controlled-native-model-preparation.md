# Decision 0102: Controlled Native Model Preparation

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-27 |
| Date | 2026-09-28 |
| Authority | Decisions 0054, 0061, 0081, 0088, 0095, 0099, 0100 and 0101; current owner restart |
| Scope | AMR-01/AMR-03 and the standalone coding runtime prerequisites |

## Findings

The coding factory loads a candidate before starting the existing live worker.
The worker's later exact token binding and dispatch preflight cannot pass run
control through the controller and native adapter. Native readiness also observes
a health reply before checking its startup deadline. These are source findings;
component fixtures do not qualify a model or native workflow.
An original-production CPU/socket regression observed readiness accepting a healthy
reply after its startup budget. The CPU child and held objects were cleaned up
before the assertion failed. Earlier wrong-package, fixture-identity and test-helper
compilation failures are retained separately; they did not demonstrate this defect.

## Decision

Construct the exact admitted candidate without loading it in factory composition.
Prepare it before token binding on the existing live worker. One attempted
controlled preparation consumes that attempt, including failure. No implicit
retry, fallback, alternate profile, second worker or second process owner is added.
Keep exact profile, manifest, resource, isolation, serving and context admission.

Borrow a narrow synchronous control interface from the coordinator's existing
clock and cancellation state. It exposes a positive remaining duration or a closed
stop observation. Preserve the original run start, cancellation identity and
clock-regression checks. The interface carries no workspace, grant or process
ownership. Extend the native contracts with explicitly controlled methods;
unsupported implementations refuse before calling legacy operations. Preserve
legacy entry points without claiming they acquired shared run control.

Carry control through manifest and artifact hashing, startup readiness, effective
serving checks, exact token binding, preparation and completion transport. Each
local startup or transport ceiling may shorten the original run budget; none may
renew it. Preserve every immutable request, preflight and published digest. Recheck
successful phase results before proceeding. Checks remain cooperative: they do not
preempt a blocked kernel call or promise a hard wall-clock ceiling.

Keep a port failure separate from a control stop. A native cancelled diagnostic
alone cannot acknowledge cancellation. Only the coordinator's retained exact signal
permits that transition. Preserve failed observations, foreign identity, real model
resource usage, stream results and retained artifacts before later advancement.

Preserve Decision 0095's cleanup proof and quarantine. If a startup failure triggers
cleanup, return cleanup uncertainty when cleanup cannot be proved; do not discard
it in favor of the original error or a latched cancellation. In particular, missing
runtime or namespace ownership remains uncertain. Never infer cleanup from a
closed socket, signalled launcher, elapsed time or absent numeric PID. Do not clear
or recover the lease, change owner capture rules, or call a run cancellation a
successful model unload. Cleanup retains its separate original deadline.
Validate a returned unload receipt's exact profile, adapter and empty state before
accepting failed-load cleanup. Call success alone is insufficient; an invalid
receipt remains uncertain. Keep legacy error categories at compatibility entry
points while controlled calls preserve the separate stop and cleanup categories.

## Verification boundary

Retain original-source regression attempts, including fixture and compilation
failures. Use CPU children, synthetic private socket peers and deterministic
controller/coordinator ports for component checks. Test deadline/cancellation
before manifest, load, readiness, token binding and preflight; no renewed budget,
no repeated failed preparation, no subsequent model/tool work, exact cancellation
identity, and cleanup uncertainty taking precedence. Preserve existing tests and
exact source-audit bindings. Complete the source batch before one applicable
SBOM/evidence pass.

Actual native CLI refusals, positive native workflows, real-model admission,
manual-user acceptance, independent review and release results remain separate.
No task or capability closes from this decision or from passing component checks.
