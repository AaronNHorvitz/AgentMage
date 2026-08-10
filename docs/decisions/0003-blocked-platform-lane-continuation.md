# Decision 0003: Blocked Platform Lane Continuation

| Field | Value |
|---|---|
| Status | Accepted |
| Date | 2026-08-10 |
| Scope | Development sequencing while required MacBook Pro M5 hardware is unavailable |
| Supersedes | The rule that every blocked platform check prevents all independent downstream development |

## Context

Story 0.3 requires a physical MacBook Pro M5 run. The exact Apple Silicon runtime admission, native Metal corpus runner, verification procedure, and immutable evidence importer are implemented, but the required personally controlled hardware and its raw result are unavailable. Linux evidence cannot satisfy that requirement.

The original execution rule stopped all later development whenever any dependency sprint was not fully `PASS`. Applied literally, one unavailable platform blocks shared kernel, platform-neutral contract, Linux adapter, fixture, deterministic-tool, and other independently verifiable work that neither consumes nor assumes a Mac result. That sequencing restriction is no longer useful, but the Mac requirement, release scope, and gate must not be weakened or represented as complete.

## Decision

1. Work continues in numeric `TASKS.md` order through a **blocked platform lane**.
2. A Mac-specific implementation, execution, acceptance criterion, or evidence item remains unchecked and is recorded as `BLOCKED-MACOS` until genuine Mac evidence satisfies it.
3. Linux evidence, mocks, cross-compilation, static analysis, declared compatibility, and shared-contract tests never substitute for a required Mac execution, signing, notarization, sandbox, Keychain, XPC, Metal, packaging, installation, accessibility, or release result.
4. A later platform-neutral or Linux item may begin when all of its non-Mac inputs are complete and it neither consumes a missing Mac artifact nor assumes a Mac result. The nearest blocked Mac dependency remains visible in every affected status and evidence summary.
5. Shared contracts continue to include the declared macOS boundary. Interface design, schemas, deterministic fixtures, and compile-time portability work may proceed, but a Mac implementation or support claim is not complete until tested on the required hardware.
6. A story, sprint, epic, product increment, or release whose acceptance criteria include Mac work remains `BLOCKED-MACOS`; it cannot be marked `PASS`, released, or used to claim supported macOS behavior.
7. No Mac requirement, task, acceptance criterion, platform statement, or release obligation is deleted, renumbered, weakened, waived, or silently reclassified.
8. When the hardware or untouched raw result becomes available, execution returns to the earliest blocked Mac identifier before any affected gate can close.
9. Work that is technically inseparable from an unavailable Mac dependency is skipped without implementation and remains unchecked. The next independent numbered item becomes the active development item.
10. Final completion still requires every retained Mac item and every affected cross-platform or release gate to pass with current evidence.

## Consequences

- Sprint 0 remains `BLOCKED-MACOS`, but independent Sprint 1 work may begin.
- Subsequent shared and Linux development can advance without producing a false Sprint 0 pass.
- Progress percentages distinguish implemented items from closed stories, sprints, and releases.
- A Linux-only development build may become highly complete while v0.1 and final product release remain blocked.
- The blocked Mac lane may accumulate multiple identifiers; each must be revisited individually rather than cleared by one broad assertion.

## Verification

- Documentation validation confirms that macOS remains a required v0.1 platform and Linux evidence cannot satisfy a Mac gate.
- `TASKS.md` keeps every blocked Mac checkbox unchecked and identifies downstream continuation as development-only.
- Evidence and release manifests reject `PASS`, supported-platform, or release claims while any applicable Mac item is unavailable.
- The first independent item after the current blocker remains the next numeric non-Mac implementation item.
