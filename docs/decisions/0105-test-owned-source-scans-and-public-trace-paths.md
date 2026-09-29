# Decision 0105: Test-Owned Source Scans and Public Trace Paths

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-29 |
| Date | 2026-09-29 |
| Authority | Decisions 0054, 0061, 0081 and 0104; current owner restart |
| Scope | Sprint 13 family-neutrality evidence and the Story 13.4 dispatch-preflight producer |

## Findings

Two model-family neutrality scanners kept only the text before the first
`#[cfg(test)]` in each kernel source file. That rule had three defects:

- It reported a comment in `kernel/engine/src/runtime_loop_tests.rs` as production code.
  That file is compiled only through `#[cfg(test)] #[path = "runtime_loop_tests.rs"] mod tests;`.
  This failed the Muse codec evidence test and the Sprint 13 aggregate refresh.
- It would drop any production item after an inline test module.
- Its non-recursive glob never read `kernel/engine/src/runtime_loop/artifact_preparation.rs`,
  a production module. The Sprint 13 report also bound none of the scanned files.

The Story 13.4 dispatch-preflight producer wrote raw Cargo output to its public log.
When compilation ran, that output included absolute crate paths under the private
checkout. The committed log already contains such lines. Earlier committed logs from
other producers contain the same kind of path. They are hash-bound historical records
and stay unchanged here, pending an owner decision.

## Decision

Add `scripts/rust_test_ownership.py`. It marks a whole file test-only only when the
file is referenced and every recognized reference comes from test-only code. Test-only
code is an inline `cfg(test)` module, an exact top-level `#[cfg(test)]` module declaration
with a sibling `#[path]` or default file, an `include!` inside test code, or another
test-only file. Any production reference keeps a file in production. So does an
unreferenced file or an unrecognized `cfg`. A non-sibling path, a non-literal
`include!`, a nested production module declaration, a stray `#[path]`, or malformed
lexical input makes ownership ambiguous. Ambiguity disables every whole-file exemption.
Inline test modules are blanked with line numbers preserved and trailing production
kept. This helper also accepts a visibility qualifier on a `cfg(test)` inline module.
The shared `rust_source_audit.py` helper, and so the strict-local and effect-boundary
scans, are unchanged.

Both scanners read every `.rs` file under their crate `src` trees recursively. They
bind each scanned file as a whole-file source input. The Sprint 13 report hashes files
at the recorded revision and refuses a scan whose working bytes differ. Its schema
becomes version 2, and the codec report becomes version 3.

The dispatch-preflight producer replaces the checkout root with `<repository-root>`.
Its validator refuses any remaining checkout or home-directory path. It does not guess
other redactions.

## Verification boundary

Unit tests cover ownership acceptance, production precedence, default module paths,
comments, strings, other `cfg` forms, every ambiguity class, trailing production,
path redaction and refusal. The corrected scans report no production family reference
in 253 kernel files. The committed Sprint 13, codec and dispatch-preflight artifacts
remain stale until the next evidence pass regenerates them. The current dispatch log
now fails its checker for the private path it contains. No task, model or release gate
closes here.
