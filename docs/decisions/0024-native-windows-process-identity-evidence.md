# Decision 0024: Native Windows Process Identity Evidence

| Field | Value |
|---|---|
| Status | Accepted implementation boundary |
| Date | 2026-08-12 |
| Scope | Windows process token, redacted user identity, session, elevation, executable identity, cross-target compilation, and genuine Windows-runner evidence |
| Advances | One bounded source portion of `AM-WIN-001`, Sprint 121, `SR-PLT-014`, and `RV-05` |
| Blocks | Named-pipe authentication, integrity level, package identity, standard-user lifecycle, NTFS, workers, DPAPI, model runtime, MSIX, removal, hostile matrix, support, and release |
| Preserves | Decisions 0001 through 0023, platform evidence non-substitution, redaction, and explicit incomplete enrollment |

## Context

Decision 0020 created a closed Windows control inventory and a Windows CI job,
but the crate contained no Windows API call and the job could prove only that a
portable scaffold compiled. The first native increment must produce evidence
that cannot execute on Linux while remaining too narrow to imply platform
enrollment.

Windows API bindings require a memory-safety boundary. Workspace crates
otherwise forbid unsafe code. Hiding the FFI behind an unaudited dependency or
weakening every crate would make the boundary harder to review.

## Decision

1. Only `agentmage-platform-windows` may opt out of the workspace-level unsafe
   prohibition. Every kernel, host, Linux, capability, and release crate retains
   `forbid(unsafe_code)`.
2. The Windows crate denies unsafe operations inside unsafe functions and keeps
   each unsafe call adjacent to a `SAFETY` statement covering pointer validity,
   buffer bounds, initialization, or handle ownership.
3. The first native observer opens the current process token with query-only
   access, reads `TokenUser` and `TokenElevation`, resolves the current Windows
   session, identifies the current image, and hashes the executable within a
   fixed byte limit.
4. Raw SID bytes and the executable path never leave the private module. Public
   evidence contains process and session numbers, elevation state, and only
   SHA-256 digests for user and executable identity. Debug output must contain
   neither SID syntax nor an executable path.
5. The Windows job executes the native test and a redacted evidence emitter on
   `windows-2022`, binds the record to `GITHUB_SHA` and runner image, and uploads
   it through a commit-pinned action. This is real Windows evidence for this
   identity observer only.
6. `enrollment_status` remains blocked. The source-control inventory records
   one partially implemented control, and all other controls and hostile tests
   remain open.
7. Linux compilation, Windows cross-compilation, or a Windows CI pass may not
   satisfy standard-user Windows 11, MSIX, IPC, path, sandbox, key, model,
   lifecycle, removal, or release evidence.

## Verification

- Linux runs the platform-independent control-closure and blocker tests.
- The `x86_64-pc-windows-msvc` target type-checks every native source and lint
  under Rust 1.95.0 before push.
- The genuine Windows runner executes repeated current-process observation,
  exact identity matching, nonzero digest checks, and debug redaction checks.
- The evidence command emits no SID, username, executable path, environment,
  token handle, or digest value. CI adds only the source revision and runner
  image before retaining the artifact.

## Consequences

- Windows now has one genuine native implementation and evidence lane rather
  than a contract-only compilation lane.
- The Windows platform remains blocked and unsupported. Eleven of the twelve
  control families have no implementation source, and the identity family
  itself still lacks named-pipe peer, integrity-level, package, hostile-client,
  and clean standard-user evidence.
- Every later Windows FFI addition must receive the same narrow review,
  cross-target lint, native test, redaction, and evidence treatment.
