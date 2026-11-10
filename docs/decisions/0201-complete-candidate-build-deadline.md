# Decision 0201: Complete candidate-build deadline

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-10 |
| Authority | Local agent mission; T-303/T-306; Decision 0190 |
| Scope | Existing dormant candidate-tree builder |

Start one monotonic thirty-second budget at entry to candidate construction.
Carry its absolute deadline through the initial complete preservation collection,
postimage verification, private scratch setup, every offline Git worker and the
final complete preservation collection. Existing collection and worker limits
may shorten that deadline; no phase can renew it. Check cancellation and expiry
before and after phases and between sixty-four-KiB postimage hashing chunks.

Scratch setup refuses stopped entry before creating a directory. Recheck after
capture while its original empty-directory rollback guard remains armed. If
cancellation or expiry becomes observable after setup, perform the existing
identity-bound cleanup even though no further worker or collection may start.
Unknown entries and displaced scratch remain retained for explicit recovery.
Cleanup and individual blocking filesystem operations are not forcibly preempted.

A late successful worker or final collection cannot produce a successful
candidate result. A refusal after dispatch cannot prove that no objects were
written; it supplies neither rollback nor retry authority. Preserve complete
manifest bindings, exact postimages, object-format checks, unchanged user index,
offline confinement and native executable admission.

The native builder remains dormant. Its public entry always selects the concrete
native collector and worker; synthetic sequencing and controlled clocks exist
only behind private test seams. This adds no API, grant, independent checker,
candidate activation, commit or push route. Original reviewer/artifact provenance,
approval composition, native execution, durable recovery and model measurements
remain open. T-303/T-306 remain open with no new rows or sub-rows.
