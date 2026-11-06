# AgentMage runtime producer contract — version 5

Revision: 2026-11-06. [Decision 0155](../decisions/0155-project-test-output-and-producer-fixtures.md)
updates the synthetic input catalog after the validation receipt adds project-test
parser identities. This is an unexecuted producer specification, not a consumer
integration or native coding-workflow proof. The record shapes and verification
obligations remain those in [version 4](runtime-producer-contract-v4.md), including
exact decoding, separate runtime authority, complete `session_recoverability`,
request/digest binding and unavailable rather than successful absent evidence.

| Item | Version | Location |
| --- | --- | --- |
| This contract | 5 | `fixtures/runtime-producer/v5/manifest.json` |
| Transport wire | 18 | `shells/host/src/runtime_ipc.rs` |
| Run request | schema 2 | `RuntimeRunRequest` |
| Run declarations | schema 5 | `RuntimeRunDeclarations` |
| Run recipe and job records | schema 1 | Unchanged production types |

Version 5 changes the catalog digest in the request and dependent record digests.
It does not change the transport wire, run-record schema, approval rules or any
consumer authority. A consumer verifies the exact request it received; a historic
catalog digest cannot establish admission against the current catalog. Unknown
record versions remain unavailable. New test receipts use the closed parser set
in `schemas/runtime/validation-receipt.schema.json` and its actual source binding.

| Fixture | Meaning |
| --- | --- |
| `run-request.json` | Current synthetic catalog-bound sealed request |
| `run-recipe.json` | Original repair recipe and typed values |
| `run-declarations.json` | Every part present and independently client-verifiable |
| `run-declarations-absent.json` | Every part absent |
| `job-control-request.json` | Exact cancellation request |
| `job-control.json` | Applied cancellation answer |
| `job-status.json` | Cancelled job state |
| `ended-run-histories.json` | Three closed, complete chains |
| `manifest.json` | Exact record closure, versions and all file digests |

The fixture producer still builds all current records from runtime types and
functions and compares every byte. Current exact decoding, mutation and refusal
checks remain. Versions 1-4 stay byte-for-byte unchanged: complete manifest pins
protect every original input and record; versions 2-4 also pass exact decoding
and client verification using their own frozen request/recipe. Version 1 retains
its unavailable old declaration checks. These are synthetic records, never model
scores, execution grants or proof that a coding task ran.

Run `python3 scripts/runtime_producer_fixtures.py` under the build reservation to
check current reproduction. `--write` regenerates only version 5 after a deliberate
input change; earlier directories are never written. Schema or meaning changes
still require their own record-version decisions. No consumer acceptance closes.
