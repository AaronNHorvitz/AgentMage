# Decision 0073: Exact Coding Context Reflow

| Field | Value |
|---|---|
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-23 |
| Authority | Decision 0054 and the owner's real-model integration assignment |
| Scope | Exact measurement and bounded source selection in the existing coding context owner |
| Preserves | Served context, output reserve, model admission, schemas, grants, canonical originals and verifier ownership |

## Evidence

At source `2b54674a`, Muse repair repetition 1 completed seven native operations:
failing validation, hash, read, patch, passing validation, diff and status.
The next prompt failed exact preflight: 28792 input tokens plus the unchanged
4096 output reserve exceeds the admitted 32768 capacity. The run remains failed.
Its bytes/4 planning counter was not conservative for hash-rich native feedback;
selection also used total capacity without subtracting the output reserve.
Exact counting happened too late to inform the existing source selector.

Raw records remain in `2026-09-23-campaign3-muse-repair-1` under the private
coding-state run directory. The retained preceding prompt SHA-256 is
`d7f9920b79e48325a8712faff373186f413b31e06e4d3285d361d36749535588`.
An extracted, unchanged read observation and observed failure counts are pinned
in `shells/host/fixtures/muse-context-overflow-20260923.json` (SHA-256
`225327b39b4f03e88052e2c0429b708bf243f751fb25865637616725ff50c454`).
Unit measurements are explicit fixtures, not native qualification.

## Decision

The coordinator supplies only an inert token-binding callback from its existing
model port to its existing context owner. It grants no inference, tool or store
authority. Native ports use their already-loaded, exact family codec/tokenizer.
The coding context owner reserves the full admitted output budget, composes with
the existing canonical context manager, measures the complete rendered packet,
and on capacity pressure reduces the planning budget to remove at least one
whole nonessential source. At most candidate-count plus one measurements occur.
Other measurement failures remain terminal; essential overflow remains refused.

System/request/continuity contracts and the newest tool observation stay
essential. Recent complete tool/call pairs take priority over older pairs;
selected pairs render in original execution order. No partial tool result,
invented summary or semantic reinterpretation substitutes for an omitted source.
Existing composition accounting supplies visible omitted source identities,
revisions, digests and reasons. Canonical originals, receipts, evidence and
continuations remain unchanged. Omission notices are not completion evidence.

The exact selected packet is retained through the existing artifact owner before
generation. Dispatch independently recounts and enforces the original capacity
and served-profile guards. No new execution loop, store, smaller model, reduced
context, higher generation budget or evidence-binding change is introduced.
The failed tuple cannot qualify; a changed tuple requires a fresh full campaign.

## Follow-up correction: preserve the controller's safety margin

At `8d625866`, fresh repair repetition 1 passed, but repetition 2 reached all
seven native effects and failed final preflight with 28645 input tokens.
The first reflow implementation reserved output but missed the controller's
existing non-spendable 256-token margin: usable input is 28416, not 28672.
The unchanged dispatch guard correctly refused it; both attempts remain retained.

The controller now owns one shared usable-input calculation for source-selection
binding and final dispatch: minimum of approved and actually served capacity,
minus full output reserve and existing safety margin. The host does not copy a
256 constant or guess headroom. Exact binding revalidates serving identity and
returns capacity pressure to the same bounded source selector. Regression pins
the actual failed measurement, exact fit and one token over, with zero generation.
Fixture `kernel/engine/fixtures/coding-context-safety-margin-20260923.json` SHA-256:
`8eb00a9b68f17a47a38653d85d11118e1f3432dfd5c02897d24c64e7318d0245`.
