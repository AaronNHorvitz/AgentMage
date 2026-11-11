# Decision 0202: Trusted local model timeout selection

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-11 |
| Authority | Local agent mission; measure-first directive; T-303/T-306 |
| Scope | Credential-owning local HTTP API settings |

The latest development measurement recorded 104 transport timeouts in 180
attempts. HTTP settings always selected the transport's 120-second total and
30-second idle defaults, although the existing transport already supports up to
300 seconds. Make that existing choice available through trusted settings.

Accept optional integer `total_timeout_ms` and `idle_timeout_ms` only with the
control-authenticated explicit endpoint choice. Omission retains the defaults.
The native transport constructor remains the validator: total is positive and
at most 300000 milliseconds; idle is positive and at most total. Invalid settings
return 400 before publication or network I/O and preserve the previous revision,
endpoint and grant. GET settings reports effective values, without credentials.

Retain immutable transport instances and the existing operation lease. A request
cannot change its deadline after preparation, progress cannot renew total time,
and timeout cannot produce completed text. Coding exchange bindings record the
selected durations. Model-facing chat and proposal bodies cannot set timeouts.
Neither authentication, cancellation, native admission, output/byte ceilings nor
the transport's five-minute maximum changes. Separate native development launch
configuration keeps its existing defaults.

Remeasure the frozen development set through resumable GPU jobs. Preserve the
native executable-trust and missing original-controls blockers: a completed
strict proposal remains ungraded, never a task pass. Do not load held-out tasks,
reconstruct acceptance receipts or weaken admission. T-303/T-306 remain open;
this decision adds no rows, grants, native tool activation or release claim.
