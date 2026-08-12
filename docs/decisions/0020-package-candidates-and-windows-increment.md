# Decision 0020: Package Candidates and Windows Increment

| Field | Value |
|---|---|
| Status | Accepted |
| Date | 2026-08-12 |
| Scope | Linux RPM and DEB candidates, Visual Studio Code VSIX candidates, installed-payload verification, package lifecycle gates, and the first versioned Windows boundary |
| Resolves | Candidate implementation portions of `RM-026` and `RM-027` |
| Blocks | Signed release activation, trusted packaged host bootstrap, Windows-native enforcement, Windows MSIX, and Windows-native evidence |
| Preserves | Decisions 0001 through 0019, strict-local authority, platform evidence non-substitution, and no pre-release support claim |

## Context

Phase 9 implemented and tested a source-composed Fedora read-and-receipt path,
but ordinary Visual Studio Code activation still uses an unavailable bridge. The
release orchestrator was empty, package directories contained prose and
historical contract fixtures, and no actual RPM, DEB, or VSIX lifecycle could be
tested. Windows was represented in product intent and the build matrix but had
no source module or independently executable lane.

A package candidate is useful for validating deterministic contents and package
manager behavior, but it cannot stand in for a signed release. Committing a test
private key, silently signing with an ephemeral identity, or treating Linux
execution as Windows evidence would weaken the established trust boundary.

## Decision

### Candidate Packages

1. The bounded release `xtask` invokes one standard-library package builder and
   accepts only an explicit version and output directory.
2. The builder emits a Fedora RPM, Ubuntu DEB, and VSIX from the exact release
   host binary, compiled extension runtime, and repository license.
3. Generated packages are ignored build output. They are never committed as
   source or treated as release evidence merely because construction succeeds.
4. Package versions use exactly three unsigned decimal components. The version
   is bound into OS package metadata, the VSIX manifest and packaged extension,
   and the installed payload identity.
5. RPM post-build stripping is disabled because it would mutate the already
   hashed release host. The VSIX includes runtime JavaScript, its manifest, and
   its license, but excludes tests, declarations, dependencies, and source.

### Installed-Payload Verification

1. Every Linux candidate installs the host, VSIX, license, and a canonical
   manifest containing the exact path, byte count, mode, and SHA-256 of each
   payload file.
2. The host accepts a package root only when the root, manifest, and every named
   payload are regular, bounded, normalized, complete, correctly ordered, and
   byte-identical to the manifest.
3. Candidate verification and signed-release verification are different command
   modes. Signed-release mode returns unavailable until an independently
   anchored detached-signature boundary exists; changing a manifest status can
   never enable it.
4. Manifest mutation, file mutation, mode mutation, missing files, extra or
   duplicate manifest paths, path aliases, and symlinks fail closed with
   content-free codes.
5. A future enabled signed-release mode must verify an independently
   anchored release manifest and detached signature before activation. Phase 11
   does not invent or retain signing credentials.

### Lifecycle Verification

1. RPM and DEB candidates are installed, upgraded, subjected to a corrupt
   upgrade, rolled back, verified, and uninstalled in network-isolated disposable
   Fedora and Ubuntu containers.
2. VSIX candidates are installed, upgraded, subjected to a corrupt upgrade,
   rolled back, and uninstalled through the real Visual Studio Code CLI in a
   disposable user-data and extension directory.
3. Failed upgrades must preserve the last valid installed version. Uninstall
   must remove the host, extension package, and package manifest without writing
   to AgentMage operational state.
4. These tests establish candidate package mechanics only. They do not establish
   signed release identity, supported installation, or the ordinary VS Code to
   host workflow.

### Windows Increment

1. `platforms/windows` freezes Windows platform contract version 1 and binds it
   to the unchanged shared platform-adapter API.
2. The contract names the closed native-control inventory: package and standard
   user identity, authenticated named pipes, handle-relative NTFS, reparse and
   hard-link denial, restricted tokens, Job Objects, DPAPI, durable state,
   native model runtime, package lifecycle, and removal reconciliation.
3. The crate exposes no native effect implementation and always returns a stable
   native-implementation blocker. It cannot activate, import Linux enforcement,
   or satisfy a Windows release gate.
4. A separate Windows CI job compiles and tests this boundary on a Windows
   runner. Passing that job is contract evidence, not Windows-native security or
   release evidence.
5. Windows remains blocked until all enforcement, hostile native tests, MSIX
   lifecycle, and standard-user evidence required by `WINDOWS-BOUNDARIES.md`
   pass on a real Windows 11 x64 environment.

## Verification

Phase 11 verification includes:

- deterministic VSIX and DEB reconstruction;
- RPM, DEB, and VSIX content inspection;
- candidate/signed mode separation and payload mutation refusal;
- native container install, upgrade, corrupt-upgrade refusal, rollback,
  uninstall, and residue checks for Fedora and Ubuntu;
- real Visual Studio Code CLI lifecycle checks in an isolated profile;
- Windows contract version, control-closure, and enrollment-blocker tests;
- a separately declared Windows workflow with no Linux evidence substitution;
  and
- product, architecture, dependency, platform, documentation, and overclaim
  gates against the complete candidate tree.

## Consequences

- AgentMage now has reproducible real package candidates rather than package
  prose or synthetic package identities.
- Package candidates are useful for integration work while remaining visibly
  incapable of satisfying a signed-release gate.
- Fedora and Ubuntu package mechanics can progress independently of Windows.
- Windows has an exact source and CI boundary, but no claim of native
  implementation, native verification, packaging, support, or release.
- Decision 0024 later supersedes only that native-implementation statement for
  one current-process identity observer. Windows enrollment, packaging,
  support, and release remain blocked.
- The trusted packaged host bootstrap and external signing ceremony remain
  release blockers owned by the original roadmap after stabilization resumes.

## Approval Record

The user authorized Phase 11 implementation and its local commit on 2026-08-12,
then separately authorized Phase 12 implementation and its local commit. That
authorization does not waive any blocker, authorize a push, or resume the
original numbered roadmap.
