# AgentMage

this is yet another harness for the world to use and improve upon

AgentMage is an open source agent runtime in development. Its interface is one
versioned local HTTP API, an MCP server and a command line over that API, with
a thin editor client of the API planned under T-310a. It has no desktop screens
of its own. It can be used by a desktop host application through the local API.

The runtime owns the agent loop, tools with permissions and approvals, budgets
and recorded execution. Running flows and directing several agents also belong
here, as later work on this same runtime; they are not completed capabilities.

AgentMage is an independent, privately developed product created by Aaron N. Horvitz on personal time, on personally controlled hardware, with independently obtained tools and services. It is not sponsored, commissioned, or developed on behalf of an employer. It is intended for public distribution. Any future installation on a managed device is a separate decision by that device's owner or operator and does not change project ownership.

## Where it stands

Current product lifecycle: `scaffolded`.
Current enabled models: none.
Current supported platforms: none.

This is pre-alpha development, with no supported release, platform or admitted
production model. The [engine inventory](docs/evidence/t301-2-engine-inventory-2026-11-02.md)
classifies every crate and executable as works, partial or scaffold, with its
bounded evidence. That inventory is dated; the newer records linked below show
subsequent work. The [status model](architecture/status-model.json) remains the
authority for lifecycle and verification status; component tests do not establish
an integrated product.

| Area | Recorded behavior and limits |
| --- | --- |
| Model connection | Configurable OpenAI-style transport, streaming and model listing have [contract tests](docs/evidence/t302-2-local-chat-transport-2026-11-03.md), a [local chat API](docs/evidence/t302-3-local-chat-api-2026-11-03.md), and [one real local chat](docs/evidence/t302-4-real-local-chat-2026-11-03.md). This does not qualify a coding model. |
| Runtime and coding | Typed permissions, approvals, run records, repository context, strict edits and isolated-worktree components exist. T-303 and T-306 remain open. Current measurements cannot execute native effects, project tests, feedback or retries because native executable admission and original grading controls are unavailable. |
| Measured coding | The latest complete frozen repeat records 180 development attempts across six profiles: A4B, E4B, 31B, 27B, gpt-oss and Muse, yielding 17/30, 11/29, 22/30, 27/30, 8/30 and 22/30 strict **ungraded proposals** per completed attempt, respectively, plus one preserved E4B resource interruption. First-attempt and within-budget task passes remain unavailable. No candidate, hidden or held-out test ran in this repeat; these numbers are not solved tasks. A raw-source presentation repeat has reached 48/180 attempted: gemma4-31b 6/8; qwen35-27b 8/8; gpt-oss 1/8; muse 4/8; gemma4-e4b 0/8; gemma4-26b-a4b 4/8 strict ungraded proposals. See the [scoreboard](docs/evidence/coding-scoreboard.md) for commits, budgets, timing and failure causes. |
| Documents and research | Parsing, indexes, citations and bounded-worker components have tests. The mission's complete document and research workflows (T-304/T-305) remain unimplemented as integrated API capabilities. |
| Interfaces | The local chat API and coding API/MCP/CLI increments have bounded evidence. The complete interface coverage in T-307 and the thin editor client in T-310a remain open. Existing editor code does not establish that planned integration. |
| Scaffolds | The inventory identifies diagnostic-only entry points and placeholder executables separately from reusable libraries. They are not running agent workflows. |
| Flows and several agents | Planned under CAP-49 and later rows T-317/T-318; no complete typed-flow or several-agent workflow is claimed. |

Current work follows **MEASURE FIRST**: finish the development measurements,
then use their largest failure cause to choose improvements. Held-out tasks are
never used for tuning. [TASKS.md](TASKS.md) is the only completion ledger;
only independent exact-commit review closes a row. Local owner testing precedes
any public release.

## Change of focus

From August 2026 the project was planned as a standalone assistant. On
**2026-10-25**, the owner decided that this repository would become self-contained
engine crates, usable through the local API, MCP and CLI, with no desktop UI of
its own. That decision was recorded here on **2026-11-02** as
[Decision 0153](docs/decisions/0153-engine-and-local-coding-priority.md), alongside
the measured local-coding priority. The editor client stays in the plan.

The [historical planning list](Agent-Scaffolding-Inventory.md), August to
October 2026, is preserved for history and no longer defines this repository's
scope. [Decision 0210](docs/decisions/0210-historical-inventory-and-public-readme.md)
records that supersession without deleting its original text.
[Decision 0205](docs/decisions/0205-one-runtime-for-model-driven-work.md) records
the later flow and several-agent direction. No second runtime is planned.

## Development and safety

Use the [development guide](docs/guides/local-agent-development.md) for the
minutes-scale gate and its limits:

```bash
python3 scripts/local_agent_fast_gate.py --base HEAD
```

Choose a base before the changes to check an already committed increment.
The gate includes a private-home-path and local-username check. The 49 explicitly
listed historical files remain byte-identical; that exception cannot be used
for new files or modifications. The long acceptance check remains:

```bash
npm run docs:clean-check
```

Neither command is a statement that today's full tree passes. The scoreboard
and evidence disclose inherited source-gate failures. The long gate runs at
acceptance, rather than after each source edit. Follow [IP-POLICY.md](IP-POLICY.md),
[AGENTS.md](AGENTS.md) and the [main-first workflow](DEVELOPMENT-WORKFLOW.md).

Consequential actions stay subject to visible permissions and approvals.
Untrusted files, tool results and memories cannot supply authority. See
[SECURITY.md](SECURITY.md), [SECURITY-REVIEW.md](SECURITY-REVIEW.md),
[RUNTIME-BOUNDARIES.md](RUNTIME-BOUNDARIES.md) and
[MODEL-PROVENANCE-POLICY.md](MODEL-PROVENANCE-POLICY.md) for the safety and release
gate boundaries. No model is bundled. Fedora Kinoite is the local development
and owner-test target, not a supported-platform claim.

## Licence and contributions

From the relicensing commit implementing
[Decision 0211](docs/decisions/0211-mit-or-apache-2-licensing.md), AgentMage's own
work is licensed **MIT OR Apache-2.0**, at your choice. See [LICENSE](LICENSE),
[LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE) and [NOTICE](NOTICE).
The Business Source License no longer applies to new versions.

Versions published before **2026-09-07** were under the Apache License 2.0.
Versions published from **2026-09-07 until the relicensing commit** were under
the Business Source License 1.1. Those published versions keep the licence they
were published with; this change does not rewrite their history.

Dependencies retain their own terms and notices. The
[third-party notices and findings](THIRD-PARTY-NOTICES.md) record the existing
dependency review, including unresolved distribution findings. Contributions
are accepted under both licences, with a Signed-off-by line on every commit;
see [CONTRIBUTING.md](CONTRIBUTING.md). The licences grant no right to the
project's name.
