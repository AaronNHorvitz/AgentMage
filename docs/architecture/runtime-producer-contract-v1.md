# AgentMage runtime producer contract — version 1

Revision: 2026-10-01, version 1. Registered under AMR-06.1 in [TASKS.md](../../TASKS.md);
design in [Decision 0135](../decisions/0135-review-fixes-exact-wire-decoding-and-the-runtime-producer-contract.md),
row split in [Decision 0134](../decisions/0134-amr-06-decomposition.md). Exact decoding of numbers
and repeated members, the omitted members and the content obligation were corrected under
[Decision 0136](../decisions/0136-review-fixes-for-exact-decoding-and-the-research-decomposition.md);
no record, encoding or version changed.

This is an unexecuted producer specification. It states which records the AgentMage runtime
produces for a consumer and how a consumer verifies them. It is not a consumer's contract, a
counterpart's approval, evidence of a running consumer or an integration proof. It names roles
only. A consumer's own contract is pinned separately, when one is offered, and checked against
the fixtures below. Independent acceptance remains open.

## Roles

| Role | Position |
| --- | --- |
| Runtime | This project. It frames runs, holds every grant and effect, and produces the records below. |
| Coordinator | A separate consumer that may start runs, follow their jobs and read what they declared. Publication, scheduling and team identity stay with it. |
| Memory | A separate consumer that may read ended runs' histories as derived, rebuildable input. |
| Host | A separate consumer that presents runs to a person. |

No consumer's contract is in this repository. Nothing here grants a consumer authority: the
runtime's own approvals and kernel grants remain the only source of authority for an effect.

## Versions

| Item | Version | Where it is defined |
| --- | --- | --- |
| This contract | 1 | `fixtures/runtime-producer/v1/manifest.json` |
| Transport wire | 15 | `shells/host/src/runtime_ipc.rs` |
| Run request | schema 2 | `kernel/contracts/src/runtime_run.rs` (`RuntimeRunRequest`) |
| Run recipe (sent by a client) | manifest schema 1 | `shells/host/src/coding_recipe.rs` (`RuntimeRecipeRequest`) |
| Run declarations | schema 4 | `shells/host/src/runtime_transport.rs` (`RuntimeRunDeclarations`) |
| Recoverability report | schema 1 | `shells/host/src/coding_recoverability.rs` |
| Context inspection | schema 1 | `kernel/engine/src/context_inspection.rs` |
| Action history entry | schema 1 | `kernel/engine/src/action_history.rs` |
| Route receipt | schema 2 | `shells/host/src/coding_route.rs` |
| Recipe plan | schema 1 | `kernel/engine/src/engineering_recipe.rs` |
| Job control request | schema 1 | `kernel/engine/src/job_control.rs` |
| Job status | schema 1 | `shells/host/src/runtime_transport.rs` (`RuntimeJobStatus`) |
| Ended-run histories | schema 1 | `shells/host/src/coding_action_history.rs` |

A consumer treats a record of any other version as unavailable. It never reads an unknown
version as an older one.

## Transport

- One authenticated local IPC channel per client process. The host authenticates the peer
  and derives the client's scope from it; a client never names its own scope.
- Each frame is one JSON envelope, `{"version": 15, "payload": {...}}`, of at most 4 MiB.
  Run declarations are at most 4 MiB less 64 KiB.
- Every frame is decoded exactly in both directions. The frame decodes into its types, and its
  re-encoding, read back as JSON, must equal the frame read as JSON. A member the types do not
  name is refused at any position, and so is a member named twice in any object, a map's
  included. The host answers such a request as a denied request
  (`host.runtime.request_denied`); a client refuses such an answer as untrusted evidence
  (`host.runtime.evidence_denied`).
- Numbers are written as the runtime writes them. The run request's decoding values
  `temperature`, `top_p` and `repeat_penalty` are 32-bit values written as their shortest
  decimal, for example `0.95`. A consumer reads them as 32-bit values and sends a request back
  unchanged; the same value written another way, such as `0.950000001`, or `1` for `1.0`, is not
  exact and is refused.
- An optional member is encoded as `null` when absent. Three are omitted instead: the prepare
  request's `recipe`, the recoverability report's `run_id` and a step's `suspended`. An explicit
  `null` for one of these is not exact and is refused. A record's members never change meaning
  within one schema version.

The operations that carry these records are `prepare` (answer `prepared`, the run request),
`run_declarations`, `job_status`, `control_job` (answer `job_control`) and
`ended_run_action_histories`. The coding host answers the first four; the store-only catalog
host answers the last.

## Records

Each record has one fixture in `fixtures/runtime-producer/v1/`.

### Run request

Fixture: `run-request.json`. The sealed request of one coding run, as `prepare` returns it and
as every later operation names it.

- A consumer may rely on `run_id`, `session_id`, `mode`, `task` (objective, acceptance criteria
  and constraints), `workspace_id`, `model_profile`, `limits` and `request_sha256`.
- `event_cursor` is `null` for a new run and names the checkpoint for a resumed one.
- Seal: `request_sha256` is the SHA-256 of the canonical request with this member set to 64
  zeroes. It covers every member. A run held to a recipe plan carries the constraint
  `Recipe plan digest: <64 lowercase hex>`, so the seal covers the plan.
- Verify with `verify_runtime_run_request` (`kernel/engine/src/runtime_coordinator.rs`).
- A run is identified by `run_id` and `request_sha256` together. A consumer never edits or
  re-seals a request; it sends it back unchanged.

### Run recipe

Fixture: `run-recipe.json`. Not produced by the runtime: it is what a client sends with a run
that is held to a recipe, and what the client needs to check the declared plan. The manifest is
sealed by `manifest_sha256`; parameter values are typed by the manifest's declarations.

### Run declarations

Fixtures: `run-declarations.json`, every part present, and `run-declarations-absent.json`,
every part absent. Read after a run ends and before it is released. They describe the run and
grant nothing.

The whole record is dropped when `schema_version`, `run_id` or `request_sha256` differs from the
run. Each part is then verified alone, and a part that fails is dropped alone:

| Part | Verification | Absent means |
| --- | --- | --- |
| `recoverability` | `verify_run_recoverability(report, session, task, run)` | the host cannot declare every effect's recoverability, for example after a resume |
| `context_inspections` | content-free views; shown, not recomputed | the host cannot show every composed context |
| `effect_history` | `verify_run_action_history(history, Effects)` | the tool boundary could not keep every entry |
| `job_control_history` | `verify_run_action_history(history, JobControl)` | the job service cannot declare every decided request |
| `route_receipt` | `verify_run_route_receipt(receipt, request)` | the run's model requests were not routed by this host |
| `route_history` | `verify_run_route_history(history, request, receipt)` | absent with the receipt, or its entry could not be kept |
| `recipe_plan` | `verify_declared_recipe_plan(plan, request, sent)` | the run had no recipe, or the plan was not declared |

A present part is complete for its owner; a part is never partial. Absent means unavailable,
never that nothing happened. `verified_run_declarations` (`shells/host/src/cli_runtime.rs`)
applies every rule above, as the development CLI does.

An action history is a hash chain. Each position is a kept entry or, after its 30-day retention
deadline, an expired position that keeps only its sequence and digests. Entries hold
identities, digests and closed reason codes, never content.

### Job control request and answer

Fixtures: `job-control-request.json` and `job-control.json`. A client asks to suspend, resume or
cancel a run's job, naming the job revision it observed. The job identity is the run identity.
A retry with the same request identity and content from the same client returns the original
decision. The answer is the decision the host's durable job ledger recorded, `applied` or
`refused` with a closed reason, and the job state after it. Verify with
`RuntimeJobControl::answers(request, control_request)`.

### Job status

Fixture: `job-status.json`. The job state replayed from the host's durable job ledger: phase,
revision, whether cancellation was requested and the ledger's head digest. Verify with
`RuntimeJobStatus::describes(request)`.

### Ended-run histories

Fixture: `ended-run-histories.json`. The stored action history chains of one ended run, read
from the operational store by the catalog host. Each chain has its positions, its stored head,
whether it is complete and whether it was closed. A chain is absent when the store holds none;
it is incomplete when its owner knew it missed an entry or the run was resumed after the host
that kept it ended. Retention is applied to each closed chain before it is read. Verify with
`verified_ended_run_histories(answer, run_id)`, which drops alone a chain that does not replay
to its head or holds another owner's kinds.

## Consumer obligations

1. Decode every frame and record exactly, and treat an unknown version as unavailable.
2. Verify each record with the function named above before using it, and drop what fails.
3. Never read a record as a grant. Declarations, histories, plans and statuses describe; an
   effect needs the runtime's own approval and grant.
4. Never read absence as success or as nothing having happened.
5. Identify a run by its run identity and request digest together.
6. Treat two parts as the person's content: the run request's `task` (its objective, acceptance
   criteria and constraints) and the run recipe's parameter values, which may name workspace
   paths. Apart from those, records carry identities, digests and closed codes only.

## Fixtures

| File | Record | Shows |
| --- | --- | --- |
| `run-request.json` | run request | a controlled-write coding run held to a recipe plan |
| `run-recipe.json` | run recipe | the committed repair recipe sample with two typed values |
| `run-declarations.json` | run declarations | every part present and verifying |
| `run-declarations-absent.json` | run declarations | every part absent |
| `job-control-request.json` | job control request | a cancellation request |
| `job-control.json` | job control | the request applied |
| `job-status.json` | job status | the job cancelled after its owner stopped |
| `ended-run-histories.json` | ended-run histories | three closed, complete chains |
| `manifest.json` | — | contract version, wire version and each file's schema and SHA-256 |

The fixtures are synthetic. The request is the coding-run test fixture with the recipe's
constraints added and re-sealed; the histories are kept by the runtime's own recorders over
synthetic digests; the job was cancelled by one client request. They show each record's shape
and verification, not a coding run or a model.

The host unit test module `runtime_producer_contract_tests` builds every fixture from the
runtime's types and functions and compares it byte for byte, decodes each exactly, runs each
verification, refuses an added member at every object position and shows that a changed record
is refused or dropped. `tests/test_runtime_producer_contract.py` checks the manifest's digests
and that this document names every fixture. After a deliberate change to a record,
`python3 scripts/runtime_producer_fixtures.py --write` regenerates the files from the runtime's
types; without `--write` it only compares.

## Change control

A change to the encoding or meaning of any record raises that record's schema version and this
contract's version, and adds a new fixture directory beside `v1`. Version 1 stays as it is.

## Limits

- Unexecuted: no consumer has read these records through the transport.
- Local only: the transport is an authenticated local IPC channel; no network transport is
  part of this contract.
- The coding host's actual runs, and so actual records from a coding run, still need a native
  Linux host (AMR-05.10). Rows AMR-06.2 to AMR-06.10 stay open until their consumer contracts,
  grants, accounts or native hosts exist.
