# Decision 0133: Review Fixes for Batch 18, and Recipe Plans Through the Change Approval

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-10-01 |
| Date | 2026-10-01 |
| Authority | Decisions 0054, 0081, 0124, 0126, 0127, 0130 and 0132; current owner restart |
| Scope | Findings F1 and F2 and note N2 of the independent review of `e197fe95`; AMR-05.9.5.3 |

## Findings

An independent read-only review of `935cfdd8..e197fe95` passed with two low
findings and thirteen notes.

- F1: the CLI's rule that `--extension-workspace` goes only with an
  extension operation had no test that failed when the rule alone was
  broken. Every refused case in the parsing test was also refused by another
  rule.
- F2: a scope trusts an issuer key under an identity, but a revocation list
  was found by its key digest only. A list signed by the trusted key that
  named another issuer identity was accepted, and the accepted-list view
  showed only the key digest. A package's signer identity, by contrast, must
  match the trust statement.
- N2: the stored installation time accepted day 31 for every month.

The review's other notes need no change now. Note N5 is recorded under
Limits below.

AMR-05.9.5.3 needs a recipe plan to reach the ordinary change approval. The
recipe component of Decision 0126 admits a sealed manifest and instantiates
a plan against a validation template registry, and it can check a set of
changed paths against the plan. Nothing in the host or the CLI uses it, and
nothing ties a plan to the writes of a coding run.

## Decision

### Review fixes

F1. The parsing test gains a scope beside a memory list, a documentation
pack list, an ended-run read and a complete run. Each is refused only by the
rule under test.

F2. A revocation list must name, as its issuer identity, the identity under
which the scope trusts its issuer key, as a package must name its signer's.
A list that names another identity is refused as untrusted. A stored
accepted list that does not match its trust statement is refused as a store
integrity failure, so renaming a trusted issuer in a stored state is caught
too. The accepted-list view now shows the issuer identity beside its key
digest. This changes the extension answers, so the IPC wire version becomes
15.

N2. The installation time must name a day its month has, with leap years.

### Recipe plans

A recipe holds one coding run's file writes to a plan. It narrows what a
proposal may reach and grants nothing.

The person names one recipe manifest file and its parameter values with a
development run:
`--recipe ABSOLUTE_FILE [--recipe-param NAME=VALUE]...`. Each run of the
invocation, including follow-ups, is held to its own instance of the plan.

Before any host is launched, the CLI:

- reads the manifest only from an absolute path, without following a link
  at its last component, as a regular file of at most 64 KiB;
- parses and admits it with the component's closed parser;
- types each value by its declared parameter: a choice, an integer written
  exactly as it prints, a path or a version;
- refuses a name the manifest does not declare, a repeated name, a missing
  required value and a value out of its bounds, with the component's own
  check, which the engine now offers without a registry;
- refuses a recipe that needs a network grant. The development host holds no
  network grant.

A refusal is a closed `recipe.*` code on standard error, with exit 2, or
exit 3 for the network grant, and nothing is launched. Otherwise the CLI
shows the recipe on standard error: identity, version, kind, title, scope,
file bound, validation kinds, prerequisites, values and rollback, and that
each write inside the plan still needs approval.

A recipe never goes with `--resume`, with any session preauthorization
option or with `--approve-this-run`. Each write the plan admits is offered
to the person individually.

Transport. The prepare request carries the manifest and its typed values.
The field is absent for a run without a recipe and is then not encoded. The
host refuses a recipe beside a preauthorization, on a resumed host or in a
session that already holds a preauthorization.

Host. When the development host prepares the run, it instantiates the plan
against the workspace's own validation template registry. Every declared
validation kind must have a registered template, and the plan binds every
such template by digest. A refusal ends the preparation with
`host.runtime.recipe_denied`, and the host names the recipe code on its
diagnostic stream. The host adds two constraints to the run request. One
binds the plan's digest, so the request digest covers the plan. The other
tells the model which paths it may change and how many files. Both host
services refuse a prepared request that binds a plan exactly when no recipe
was sent, or binds none when one was.

Tool boundary. Before any grant is issued, any session preauthorization is
consulted or any approval is asked, the boundary checks each proposed write
against the run's plan: a structured patch, a file creation, a hunk
selection or a rollback. The path must lie inside the plan's scope, and the
distinct paths the plan admitted, this one included, must stay within its
file bound. Each distinct path counts once, whether or not the person later
approves its write. A path the plan refuses is not counted. A refused write
is a policy denial: the run ends declined with `recipe.out-of-scope` or
`recipe.too-many-changes`. The run's action history keeps it as an
unauthorized, denied entry whose evidence is the preview and the policy's
decision digest. Reads, Git inspection and registered commands and
validations are not file writes of the plan. Their own templates and
grants still bound them.

A write inside the plan goes on to the ordinary change approval exactly as
without a recipe.

Continuation. A run suspended and continued in the same host keeps its plan
and the paths it admitted. A restarted host has no plan scope. It refuses
to resume a run whose request binds a plan, with
`host.runtime.recipe_denied`.

Declarations. Run declarations schema 4 adds the plan the run was held to.
The CLI keeps a declared plan only when it is the plan of the recipe it
sent. The plan must verify and name the sent manifest, values, scope, file
bound, validation kinds, prerequisites and rollback, and the run's
workspace, and the run's constraints must bind its digest. Otherwise the
plan is shown as unavailable. The CLI then shows the plan's digest, scope,
bound, each bound validation template and rollback, as text or a
`recipe_plan` JSON row.

Harness. `recipe-sample --directory DIR` copies four committed synthetic
manifests into a new private directory. The four are a repair held to one
file under `src`, a plan scoped to `tests` only, a recipe that needs a
network grant, and a recipe verified by a build validation the development
profile does not register. `start` passes `--recipe` and `--recipe-param`. A
unit test reproduces every sample byte for byte. A recipe's kind names what
verifies it. The repair sample is of kind `tests` because a unit validation
verifies it.

## Limits

- The plan narrows proposals; the security boundary stays the person's
  approval of each write, which the plan never replaces.
- Prerequisites are shown, not checked by the plan. The development host
  already refuses to start in a worktree with uncommitted changes.
- Commands and validations a run executes are not held to the plan's scope;
  their templates bound them.
- The boundary glue runs only in native composition, which this sandbox
  refuses. The pure plan scope, the CLI and wire are tested here; a run
  refused by its plan has not been observed through actual processes
  (AMR-05.10).
- A run continued after a host restart cannot keep its plan and is refused.
- Extensions (review N5 of `e197fe95`): a scope's accepted list is the
  current revocation set, not a cumulative one. A later list with fewer
  entries reactivates what an earlier list revoked. The issuer of an accepted
  list cannot be distrusted while the list is kept. Nothing an extension
  declares is run today; this must be revisited before extensions gain
  effects.

## Consequences

- `kernel/engine`: `check_recipe_parameters`; recipe plans and values decode
  closed.
- `shells/host`: the issuer identity rule, view field and day rule of the
  extension owner and tests; `coding_recipe.rs` with its tests and samples;
  the prepare field, declarations schema 4, `RecipeDenied` and wire 15; the
  development factory, both host services, the tool boundary, the action
  history's policy refusal, the CLI options and rendering.
- `scripts/coding_harness.py`: `recipe-sample` and `start --recipe`.
- TASKS.md: AMR-05.9.5.3 is recorded with its evidence when the batch is
  verified.
