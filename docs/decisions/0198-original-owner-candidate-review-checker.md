# Decision 0198: Original-owner candidate review checker

| Field | Value |
| --- | --- |
| Status | Library composition under the accepted owner mission |
| Date | 2026-11-10 |
| Authority | Local agent mission; T-303/T-306; Decisions 0183, 0187 and 0197 |
| Scope | Explicit trusted checker over original candidate-review evidence |

Add an explicit trusted review port to the existing borrowed two-root review
view. The host selects its checker outside model and tool input. Provide no
default accepting checker and no dynamic reviewer selection. The implementation
must independently check acceptance criteria, the complete packet and findings,
original validation history and trusted capture/scanner provenance for every
artifact claim. Missing evidence refuses. This interface cannot authenticate
its installed implementation or establish that it performed those duties.

Before calling the checker, perform the complete original-owner candidate plan
checks. Deliver borrowed references to the exact packet, checked inert plan,
original native creation identity and complete original validation records.
Do not substitute serialized records or allow public observation injection.
A checker rejection or unavailable result stops without post-check reads.

After checker success, repeat the complete candidate plan checks. Require the
same full private Git observation across all pre/post samples, including linked
metadata and task inventory not represented in the candidate plan. Require
identical complete candidate plans. Preserve existing source, mode, immutable
base, write-history, validation and held-root checks. Check cancellation before
and after each Git observation and at checker boundaries. Individual filesystem
operations and an installed blocking checker are not made interruptible; the
host must bound its checker work. Samples are not an atomic state lease.

Return a process-only read-only result borrowing the original owner and packet.
It has no public constructor, cloning, deserialization or restoration from
receipts. It proves only that the installed checker returned success and these
observations matched. No executor consumes it; it grants no candidate build,
commit approval, push or publication. The preexisting inert planning method
remains explanatory and cannot substitute for this checker. No HTTP, CLI or MCP
route is added. Native admission, actual independent checker/artifact integration,
complete exact product approval, durable recovery and model measurements remain
open. T-303/T-306 stay open without new rows or sub-rows.
