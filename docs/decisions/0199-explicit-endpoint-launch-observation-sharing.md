# Decision 0199: Explicit endpoint launch observation sharing

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-10 |
| Authority | Local agent mission; T-303/T-306; Decision 0162 |
| Scope | Private development launch choice for existing MCP observations |

Allow `share_run_observations: true` in the existing private endpoint launch
configuration. This selects the existing `DelegateObservations` service policy
for the read-only, strict-edit and combined edit/test development launchers,
including their original-diff review variants. Omission or false preserves
`ControlOnly`. Reject non-boolean and duplicate fields through the existing
closed configuration decoder. No HTTP or model input selects this policy.

Check the listener and stop latch before opening configuration, then load the
private document once. Admission consumes that same configuration; replacing
the file cannot change its observation choice during startup. The original
service repeats its listener/stop check immediately before admission. File,
credential, native trust, permission and confinement checks remain mandatory.

Selecting sharing displays that all ordinary client-credential holders may read
the complete sealed request and canonical events, which can contain task and
tool data. The observer must also explicitly launch the existing MCP bridge
with the exact run/request pin and its sharing flag. The bridge's recipient and
any subsequent destination remain the person's choice. Delegation cannot
advance, approve, cancel or release a run, change configuration, read a fresh
workspace or access original-diff controls. The bridge opens no control credential.

This connects the existing observation adapter to executable startup; it adds
no agent loop, new tool, disclosure grant, restored ownership or independent
reviewer. A terminal record is still only an observation; it cannot establish
task success. Released runs remain unavailable. Native admission in the lane,
measured complete coding, commit/push and durable recovery remain open.
T-303/T-306 stay open, with no new row or sub-row.
