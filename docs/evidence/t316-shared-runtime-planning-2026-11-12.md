# Shared-runtime direction: planning record — 2026-11-12

This records the owner's November 11 shared-runtime instruction after completion
of the six-model coding measurement. It implements no runtime capability and
claims no new model result. [Decision 0205](../decisions/0205-one-runtime-for-model-driven-work.md)
records one engine for model-driven work, typed flows and tools in both MCP
directions. [The roadmap](../../CAPABILITY-ROADMAP.md) adds CAP-49 and reconciles
CAP-03, CAP-29, CAP-37, CAP-40, CAP-44 and the runtime/memory ownership rules.
The [PRD](../../PRD.md#current-delivery-amendment-2026-09-24) records the direction;
[T-316](../../TASKS.md) is one later, unchecked everyday-tool qualification row.

The plan covers these boundaries:

- Tools have separate off / ask each time / allow grants; state-changing calls
  ask by default and show the exact call. MCP exposure retains engine authority.
- Model, tool, memory and approval steps form a typed graph on the same runtime,
  including later several-agent work. Step inputs, outputs, order, retries and
  limits are recorded. Replay uses recorded results without repeating effects;
  a re-run from a step requires current authority and may produce different
  model output. Protected records retain their existing privacy rules.
- API, approval and run records support folder-free tasks. CLI and conversation
  hosts work independently; the engine has no desktop interface or authoritative
  long-term memory. A versioned memory service and rebuildable runtime indexes
  retain the existing source authority and open memory-linking study.
- Tool, file, page and memory text cannot become instructions or approval.
  T-316 later freezes synthetic tasks and qualification scores before running,
  measuring tool/argument correctness, truthful results, asking first and
  injection refusal per profile and after relevant changes. Existing held-out
  and threshold rules continue; current coding priorities do not change.

Validation is limited to documentation: the fast gate, direct Markdown lint on
all five changed documents, whitespace and ledger-preservation checks. All pass;
the direct lint result lists all five documents with zero issues. These checks
establish documentation consistency, not implemented behavior. The
ledger check verifies exactly one new unchecked row, with all existing table
rows and the old plan below the active queue byte-identical. The status model,
scoreboard, runtime source, prompts, evaluation tasks and hidden tests do not
change. No supply-chain or acceptance evidence is regenerated. No GPU job is
started and no measurement run is interrupted. Independent review remains
required; T-303, T-306, T-315 and T-316 remain open.
