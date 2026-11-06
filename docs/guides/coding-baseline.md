# Chat-only coding baseline

T-311.18 measures the running version 1 local chat API before implementing the
new coding engine. This text-proposal measurement is separate from the older
native coding CLI, whose launch remains subject to the deferred T-301.1 host
executable-trust blocker. It is not evidence that the native agent loop runs.
The current API has no repository tools, test execution or retry loop.

The fixed collector is `scripts/coding_baseline.py`. Each task receives exactly
one fresh conversation: the fixed instruction, complete frozen editable source,
its repository-relative path and the frozen statement. It receives no helpers,
tests, Git history, reference source, feedback or prior task conversation. Source
is encoded as JSON data. Full-file context deliberately has no retrieval or
truncation policy. Request/context refusals remain unsuccessful attempts.

The instruction asks for JSON exact-substring replacements. The collector only
applies bounded, nonempty, unique matches in order, to the one allowed file.
Malformed, ambiguous, unchanged or partial answers refuse; no automatic repair,
fence removal or second prompt exists. This is measurement plumbing, not a tool
or permission granted to the model. It executes no model text. Its snapshots
must subsequently pass the existing independent admission boundary.

## Fixed measurement policy

- One attempt per task, no retries or hidden-test feedback. First-attempt and
  within-budget rates are identical because the current chat path has no retry
  capability. Later engine rows must publish their changed budgets separately.
- Request 4,096 output tokens, including reasoning. Use the existing local
  candidate weights, one at a time, with the current 8K chat profile and recorded
  server defaults. This does not claim the separate 32K coding-preparation scope.
- Keep the API's existing 120-second operation deadline, 30-second idle budget,
  byte limits, authentication and explicit endpoint choice.
- At most five tasks per GPU reservation; stop/reap owned servers and release
  the slot between batches. No download or other worker termination.
- Keep every task, refusal and failure in the denominator. Refused generations
  retain an explicitly labelled base snapshot, never a fabricated model answer.
- Admission still requires current full-suite controls and verified lane quotas.
  Its finite-observation, self-modification, temporary-fixture and identity/state
  limits accompany all scores. Known dev-003 process races remain failures.
- The API does not expose upstream token usage: token fields are null, never an
  estimate or zero. Separately retained backend timing counters may supplement
  the record with an explicitly stated scope. Reserved GPU minutes include
  model loading and cleanup; they are not active GPU kernel minutes.

Freeze the collector/policy before the first model request. Do not revise it
from held-out feedback. All model responses, requests and raw grading logs stay
in private operator storage; publish only checked aggregates and content hashes.

## Collection and grading

Start a dedicated authenticated API and explicitly configure the owned local
endpoint following the [API guide](local-chat-api.md). No model is started by
this command. Run collection while the operator's normal GPU reservation is
held. The first index is zero-based and count is at most five.

```sh
python3 -B -m scripts.coding_baseline \
  --api-address 127.0.0.1:43127 --auth-directory <private-api-auth> \
  --model <listed-model-id> --first 0 --count 5 \
  --output <new-private-batch> --candidates <private-snapshots> \
  --approve-task-context
```

Use six successive batches for the complete 30-task development set, with
fresh output directories and the same snapshot root. A task directory cannot
be overwritten. Then run the [independent admission command](coding-evaluation-worker-oracle.md#generated-candidate-entry)
on the complete snapshot root. The collector's `scoreboard` function verifies
complete nonduplicate task coverage, frozen policy/source identity, raw-log and
record bindings, candidate identity and matching independent grading before
publishing a private aggregate. Only generated, independently passing proposals
earn a pass; successful collection alone earns none.

Held-out collection and aggregation require the explicit `held-out` split,
private frozen descriptor and `row-close-scoreboard` purpose. Run those only at
a row close. Do not forward grading feedback to a model, inspect it for tuning,
change hidden tests, or replace any failed scored attempt with a diagnostic run.
The narrow held-out protocol domain is described in the [suite guide](coding-suite.md).

Synthetic collector tests and synthetic held-out entry tests do not import or
run the actual held-out modules. They do not count as model or task scores.
