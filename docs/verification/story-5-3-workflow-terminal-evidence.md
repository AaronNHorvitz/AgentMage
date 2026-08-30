# Story 5.3 Workflow Terminal Outcome Evidence

This record covers repository-controlled implementation evidence for Sub-task 5.3.2.2. All data
is synthetic; no model, tool effect, network, native-platform acceptance, story completion, sprint
completion, or release completion is claimed.

## Closed terminal family

The canonical terminal result and durable workflow lifecycle now preserve nine distinct outcomes:
verified success, verified no-op, blocked, denied, failed, cancelled, timed out, resource exhausted,
and uncertain. `denied` is represented explicitly rather than being collapsed into blocked or
failed. Generated JSON Schema and Rust contracts use the same closed vocabulary.

## Completion authority

The terminal resolver exposes no success boolean or raw success constructor. It can construct
verified success or verified no-op only from the opaque proof emitted by the deterministic workflow
evaluator. Changed evidence maps to verified success; completely unchanged evidence maps to verified
no-op. The proof carries exact current state and verifier-result identities, and neither model prose
nor exit status can create it.

Every non-success requires one of seven closed runtime reason variants, a last-verified-state digest,
a stable diagnostic code, and a bounded safe next action. The generated canonical result is sealed
with `agentmage-runtime-verifier` ownership and an exact digest.

## Verification scope

Seven Rust integration tests include the two verified outcomes, all seven non-success outcomes,
malformed diagnostics, duplicated evidence identities, current-evidence binding, and false-success
inputs. The Engineering Runtime schema suite explicitly admits diagnostic `denied` and refuses an
undiagnosed denial. Strict Clippy and three evidence-integrity/overclaim tests pass.
