# Phase 12 Stabilization Audit

| Field | Value |
|---|---|
| Audit date | 2026-08-12 |
| Phase 11 source commit | `1f3da44a617361905856b97e96b979eef651f00c` |
| Phase 11 source tree | `4d7fa4c7bde621ce42a8ca3be2e44782c40b961b` |
| Audit scope | Architecture, authority, product, native Linux, package candidates, Windows boundary, current evidence, traceability, documentation, status, and overclaim review |
| Provenance | Implementation-session engineering audit plus deterministic self-checks; not independent human or external review |
| Current disposition | Accepted by the user for roadmap resumption under Decision 0021; independent provenance review remains open before signed release or connected-authority promotion |

## 1. Executive Result

The audited tree has no known unresolved P0 finding after the Phase 12 package
verification correction described below. The source, candidate package, current
evidence, status, and documentation gates pass. Candidate RPM, DEB, and VSIX
lifecycles pass without creating a signed release, supported platform, enabled
model, or integrated-product claim.

The same-session audit is complete but does not satisfy the repository's
independent-human or external-review provenance classes. Real fuzz execution is
still a separately approved manual P1 task. Release signing and trusted package
bootstrap, Windows-native implementation and evidence, and one privilege-bound
Linux mount test remain explicit P2 blockers. Decision 0021 records the user's
acceptance of the residual set and authorization to resume development without
misrepresenting the audit as independent review.

## 2. Audit Method

The audit used the following distinct passes against the committed Phase 11
source and the final Phase 12 correction:

1. Read the Phase 11 decision, remediation gate, package builder, package
   lifecycle runner, host verifier, Windows contract, platform status, CI
   contract, and current status model.
2. Re-ran the complete product build, formatting, warnings-denied lint, strict
   local, effect mediation, Rust tests, compile-fail tests, and VS Code tests.
3. Rebuilt RPM, DEB, and VSIX candidates twice and compared exact bytes.
4. Exercised package extraction, payload verification, mutation denial, real VS
   Code CLI lifecycle, and network-isolated Fedora and Ubuntu package-manager
   lifecycles.
5. Ran all ignored Linux tests serially where host facilities were available and
   separately ran both ignored host end-to-end workflows.
6. Re-ran current evidence, traceability, platform, model, schema, requirement,
   status, dependency, architecture, and documentation checks.
7. Inspected the exact diff for retained-artifact mutation, unsupported claims,
   new third-party dependencies, unsafe Git state, credential files, and package
   output tracking.

No real fuzz engine, sanitizer, exploit development, hostile external target,
or vulnerability probing was performed during this audit.

## 3. Corrected Finding

### P0-001: Signed-release mode lacked a cryptographic gate

**State:** Corrected in Phase 12; regression tests pass.

The Phase 11 host verifier selected either `unsigned-candidate` or
`signed-release` by a manifest status string. Although an unsigned candidate was
rejected by release mode, relabeling the manifest and recomputing no payload
hash could have reached release-mode success without an independently anchored
signature. That contradicted Decision 0020's fail-closed release boundary.

The correction makes release verification unconditionally return
`agentmage.package.release_verification_unavailable` until a real detached
signature and independently distributed trust anchor are implemented. Candidate
verification remains available only through its explicit command. The verifier
also now requires exactly the host, VSIX, and license records; walks every path
component descriptor-relative with `NOFOLLOW`; requires one-link regular files;
and revalidates descriptor identity, mode, size, and link count after reading.

Regression tests prove that a relabeled `signed-release` manifest remains
blocked, an incomplete manifest is denied, a symlinked payload parent is denied,
and exact candidate bytes continue to verify.

## 4. Residual Risks and Blockers

| ID | Priority | State | Owner | Exit condition |
|---|---|---|---|---|
| `P1-R01` | P1 | Open manual security task | Project maintainer | Separately approved real product-boundary fuzz run with exact toolchain, harness, corpus, sanitizer, resource, and result identities. |
| `P1-R02` | P1 | Open release and connected-authority provenance gate | Project maintainer | Review of the exact final tree by a provenance-valid independent human, external reviewer, or separately accepted independent process before signed release or connected-authority promotion. |
| `P2-R01` | P2 | Release blocked; detached verifier implemented after audit | Release owner | Approve an external signing identity and ceremony, distribute the trust anchor independently, sign RPM/DEB delivery metadata, and pass trusted packaged VS Code-to-host bootstrap and clean-install tests. Decision 0022 supplies bounded manifest signing and verification mechanics but does not satisfy these external gates. |
| `P2-R02` | P2 | Windows blocked | Windows platform owner | Native Windows adapter, hostile native matrix, standard-user MSIX lifecycle, and real Windows 11 x64 evidence pass without Linux substitution. |
| `P2-R03` | P2 | One native test unavailable | Linux platform owner | Run the isolated bind-mount swap test in a permitted user/mount namespace and retain exact native evidence. |
| `P2-R04` | P2 | Historical reports stale | Evidence owner | Migrate or supersede legacy supply-chain, locked-resolution, artifact-scan, and policy reports under the accepted historical/current evidence model. |
| `P2-R05` | P2 | Closed by Decision 0021 | Product owner | User accepted the residual risks and authorized resuming the original numbered roadmap on 2026-08-12. |

The stale historical reports are not treated as current passes. Their failure is
preserved by the historical replay while the current Phase 10 evidence gate
continues to report applicability separately.

## 5. Verification Results

### Passing

- Complete product formatting, warnings-denied lint, builds, unit and contract
  tests, strict-local scan, structural effect mediation, Rust documentation
  compile-fail tests, and 12 VS Code tests.
- Candidate package unit tests, exact repeated-build comparison, extracted RPM
  and DEB payload equality, candidate/release separation, mutation refusal, and
  real VSIX install, upgrade, failed-upgrade, rollback, and uninstall.
- Network-isolated Fedora and Ubuntu install, upgrade, corrupted-upgrade refusal,
  prior-version preservation, rollback, verification, uninstall, and residue
  checks.
- Fifteen environment-dependent Linux tests, including Bubblewrap, systemd user
  limits, Secret Service provisioning and cleanup, exact path projection,
  network denial, write denial, stale-object denial, and live inventory.
- Both ignored host workflows covering approved read, durable receipt, replay,
  restart, stale preview, cancellation, and no unauthorized worker or receipt.
- Current requirement, architecture, dependency, CI, status, evidence,
  traceability, platform, zero-model activation, schema, Markdown, Mermaid, link,
  and overclaim gates.
- No changed retained artifact under `artifacts/`, `fixtures/`, or `references/`;
  no newly tracked RPM, DEB, VSIX, secret key, model, database, log, or private
  review output. Existing historical synthetic logs and one public verification
  key remain unchanged.

### Blocked or Unavailable

- The isolated bind-mount swap test could not create a mount in this session and
  failed with the expected host privilege denial. No pass is claimed.
- The declared `windows-2022` CI job has not run against the local, unpushed
  commits. Local Windows contract tests pass on Linux, but this is not Windows
  native evidence.
- Signed-release verification intentionally returns unavailable. Candidate
  package success cannot change that state.
- Cargo vulnerability-database analysis and real fuzz execution were not run.
  No absence-of-vulnerability claim is made.

## 6. Truth and Resumption Decision

The repository continues to state:

- product lifecycle `scaffolded`;
- no integrated end-user workflow;
- no enabled model;
- no supported platform;
- no released package; and
- stabilization scope freeze inactive under Decision 0021.

The first five statements match the audited tree. The final statement records a
planning-state transition after the audit and does not promote product status.
The Phase 11 package candidates and Windows contract remain implementation
increments rather than release claims.

**Post-audit disposition:** the user accepted the recorded residual risks and
authorized roadmap resumption on 2026-08-12. Decision 0021 therefore closes the
development resumption gate. The provenance-valid independent review remains
open and must pass before signed release or connected-authority promotion; this
record does not attribute that review to the user.
