# AMR package assessment — 2026-09-29

Scope: an assessment of AMR-01 through AMR-03 and their components in amendment order,
under [Decision 0088](../decisions/0088-amendment-coverage-in-work-selection.md). It
was made at `b09796d5d33ba3a5a959409fb1dfde804a335bff` by source and record inspection.
This is not a completion ledger. No checkbox, status-model entry or gate changes here.

## Lane constraints

This implementation lane has no network access, and GPU inference cannot launch. Real
root is unmapped, so `/usr/bin/git`, `bwrap`, `systemctl` and `systemd-run` fail the
root-owned executable check. There is no user systemd manager. No macOS, Windows,
independent reviewer or release authority is available. Rust tests with fakes, evidence
scripts and documentation can run.

## Dispositions

| Row | Source state | Remaining to close | Dependency-ready here |
| --- | --- | --- | --- |
| AMR-01.1 | Current source, binaries and doctor reconciled in each verification record. | A positive native launch at the current pin. | No: native. |
| AMR-01.2 | Command exits, repair, recovery and drift are implemented. | A current native fail/repair, denial and cancellation run, then model admission. | No: native and model. |
| AMR-02.1 | The descriptor-held inference lease has cross-process tests. | Native full-lifecycle qualification. | No: native. |
| AMR-02.2 | Policy, reservations and persistence exist. Query accounting is bound to the request under Decision 0106. | Task-authorized issuance, which belongs to the research coordinator mode in AMR-03.2. | No: gated by Decision 0097. |
| AMR-02.3, 02.3.1, 02.3.2 | The closed worker contract, the confined worker and refusal fixtures exist. | The native DNS, TLS, redirect and rebinding adversarial matrix. | No: native. |
| AMR-02.4 | The 32K preflight runs inside the lease before launch. | A driver-level refusal proof needs a test seam past the native sandbox check. | Test-only; low value. |
| AMR-03.1, 03.1.1 | Plans, durable reservations and reports exist with component records. | The connected native research composition. | No: native. |
| AMR-03.1.2 | Canonical retrieval binding exists. | Connected publication needs the qualified native worker. | No: native. |
| AMR-03.1.3 | Consumer requirements and scenarios exist. | An offered producer artifact and independent pinning. | No: external counterpart. |
| AMR-03.2 | Not started; every runtime session mode denies network access. | A provider, a model, native qualification and review. | No: external and native. |

## Next package

The next package in amendment order is AMR-04. Its capabilities (CAP-08, 12, 13, 16,
28, 35 and 39) build on existing contracts, so a component increment can proceed
without whole-package completion of AMR-01. [Decision 0107](../decisions/0107-selective-hunk-changes.md)
splits AMR-04 and starts with the CAP-12 kernel selective-change contract (AMR-04.1).
AMR-05 through AMR-07 are assessed after the AMR-04 components.

The remaining coding rows 48.2.4 through 48.2.6 and 50.2.4 have their source in place.
Their closing evidence needs a positive native run on a normal Linux host session;
[local testing](../LOCAL-TESTING.md) gives the steps.
