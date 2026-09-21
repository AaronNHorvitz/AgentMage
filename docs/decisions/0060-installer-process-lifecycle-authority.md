# Decision 0060: Sub-task 14.1.3.3 Requires Expanding the Installer Process Authority

| Field | Value |
|---|---|
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-20 |
| Scope | The remaining evidence named by Sub-task 14.1.3.3, and what producing it would require |
| Authority | Decision 0054 standing owner delegation |
| Preserves | The installer's declared inactive process boundary, Decision 0052's withheld acquisition and network authorities, and every existing in-process interruption test |
| Expands no authority | Correct. This record identifies what the row would require; it grants nothing |

## Finding

The row-level register selected Sub-task 14.1.3.3 as the next dependency-ready local row. Its
stated remaining work is "actual process termination at every download/import boundary and
transport socket-closure evidence". Reading the implementation shows that evidence cannot be
produced today, for a reason the row's own text does not state.

There is **no process that performs a download or import**:

- `execute_model_installer` admits exactly two operations, `self-check` and `preflight-stdin`, and
  returns `ModelInstallerProcessError::OperationUnavailable` for anything else. Its own self-check
  descriptor declares `"normal_operation": false`, `"network_authority": false`,
  `"activation_authority": false` and `"one_shot": true`.
- `download_model_artifact` is called only from unit tests inside `model_download.rs`. No shipped
  binary drives it.

The existing interruption coverage — `interruption_at_each_stage_recovers_to_prior_or_fully_committed_state`
and `rollback_interruption_recovers_current_or_verified_predecessor` — is in-process and simulated,
which is exactly why the row says actual process termination remains open.

## Decision

1. Producing the named evidence requires first **admitting download and import lifecycle
   operations through the installer executable's protocol**, so that a real process exists to
   terminate at real boundaries and hold real transport sockets.
2. That admission expands the installer's declared authority: `normal_operation` and, for the
   download path, `network_authority` would both have to become true for the operations concerned.
   Decision 0052 withholds acquisition and network authorities and requires model acquisition to
   be a separately consented, fail-closed phase. Expanding a deliberately inactive security
   boundary is a product-security change, not routine verification work.
3. Sub-task 14.1.3.3 is therefore recorded as **dependent on that admission**, not as a
   ready-to-execute local leaf. The register should not offer it as safe unattended work while the
   only way to satisfy it is to widen an authority the product deliberately withholds.
4. Nothing is granted, stubbed, or pre-implemented here. No operation was admitted, no authority
   flag was changed, and no test was weakened or skipped.

## What this does not say

This is not a claim that the installer boundary is wrong. It is deliberately inactive and its
self-check descriptor says so explicitly. The finding is narrower: a verification row was written
as though the boundary already admitted these operations, and it does not.
