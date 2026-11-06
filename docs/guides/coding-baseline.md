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
  candidate weights, one at a time, with 8K context (the product chat value)
  and the lab's 4,096-token output reservation. The product chat profile itself
  uses 2,048 output tokens; this baseline is a mixed configuration. This does
  not claim the separate 32K coding-preparation scope.
- Keep the API's existing 120-second operation deadline, 30-second idle budget,
  byte limits, authentication and explicit endpoint choice.
- At most five tasks per GPU reservation; stop/reap owned servers and release
  the slot between batches. No download or other worker termination.
- Keep every task, refusal and failure in the denominator. Refused generations
  retain an explicitly labelled base snapshot, never a fabricated model answer.
- Admission still requires current full-suite controls and verified lane quotas.
  Its finite-observation, self-modification, temporary-fixture and identity/state
  limits accompany all scores. Known dev-003 process-observation and dev-004
  SIGINT timing races remain failures; no hidden test or threshold changes.
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

## Retained-log termination report

The baseline's backend failure causes can be regenerated without a model,
GPU, hidden tests or another held-out attempt:

```sh
python3 -B scripts/coding_baseline_termination.py \
  --root <private-retained-baseline-root> \
  --manifest-sha256 932463662e05b984a28fd3b329c24df898839d10a749e08b28fc8574725ba9f1
```

The command verifies the original manifest's digest and every consumed file's
binding, then verifies the individual log slices against their timing records.
It checks observed 8,192 context settings and bound requests for 4,096 output
rather than deriving a cause from an API error. It refuses missing, duplicated,
changed, ambiguous or contradictory records. Output contains per-development-task
classes and held-out aggregate counts; no held-out identifiers or raw text.
This supplemental report changes no original aggregate, collector or score.
Its four classes are `prompt-exceeds-context`, `context-exhausted`, `output-cap`
and `stopped`, as defined in the [scoreboard](../evidence/coding-scoreboard.md).

The observed 8K/4K budget blocked 36 of 40 Muse and 38 of 40 GPT-OSS attempts,
including every held-out attempt. These scores do not establish relative model
quality. Publish changed context and output budgets as separate variables in
later scoreboard entries; do not attribute their effects solely to the engine.

Review backlog: deeply nested JSON can raise `RecursionError` in the frozen
collector and abort collection before its batch record. No scored attempt did
so. Preserve this collector for baseline replay; handle this as `edit-refused`
and add a synthetic test before reusing its parser in the T-303/T-306 loop.
The API's context-overflow and length-stop diagnostics also need clearer codes
at hardening. Neither low-priority repair authorizes repeating this baseline or
using held-out responses to tune a prompt or the coding engine.
