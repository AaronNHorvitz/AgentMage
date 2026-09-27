# Amendment work selection, 2026-09-27

[Decision 0088](../decisions/0088-amendment-coverage-in-work-selection.md)
corrects the existing blocker register's omission of all AMR table rows. Source
base: `50776dd127c9ee65b5f3fc8aaf07f5d01793ccc4`. This is a planning-tool
correction; runtime sources, task checkboxes and product status are unchanged.

The register now includes all twenty accepted package/component rows, preserves
exact row bytes and dependency descriptions, and reports their denominator
separately from older checkboxes. Duplicate or malformed rows, missing accepted
identities and unregistered additions refuse the production build. Only the
dependency column supplies referenced AMR identities; deliverable examples do not
appoint an owner, grant execution or declare an external blocker.

All twenty currently open AMR rows require assessment. A source contract, native
boundary or exact model profile is not interchangeable with completion of its
parent package. The register retains those descriptions and references without
inventing resolved completion-gate edges. Checked TASKS rows are not reopened or
kept as open work in a second ledger. Coding priorities remain first; subsequent
AMR assessment follows package/component order before unrelated older work.

## Executed checks

The first 27-test focused run exposed two failures. An extra-column fixture had
accidentally appended description text; it was corrected to add an actual column.
An interleaved AMR description could also change the preceding legacy paragraph's
classification; legacy source spans now stop at AMR row boundaries. All 27 tests
then passed.

The broader 51-test run found existing Decision 0081/0083 title/filename mismatches.
Their first lines now use the established decision-title form; their bodies,
acceptance, dates and authority are unchanged. The original validator remains in
force. A later test refinement verifies that completion stays solely in TASKS,
without freezing today's open-row count in a test.

The final 52 tests passed, including all eighteen existing selector tests and ten
new amendment tests. Task-graph, context-safety registration, product-status and
supply-chain validators passed, as did Markdown lint across 552 files and the
frozen-source/whitespace checks. Heavy checks used the shared build reservation.
No workspace member or dependency changed, so the existing SBOM was checked and
remains unchanged; it was not regenerated.

The derived register was generated once after source verification and then checked.
Its current result covers 1,622 open rows: 66 local, 1,065 dependency, 163 external
and 328 unknown. Against a retained run of the previous parser over the identical
TASKS bytes, all 1,602 legacy row objects match exactly, including their source
spans, hashes, classifications, dependencies and blockers. The twenty additional
unknowns are the AMR rows. The next selected row remains `48.2.4.1`.

The [manifest](amendment-work-selection-2026-09-27.json) binds changed source,
authorities, the derived register and actual success/failure logs. Older
verification observations retain their original pins. This bounded correction
does not establish global evidence freshness or resolve the 328 assessments.

## Acceptance limits

Selector readiness describes repository work selection; it grants no runtime or
publication authority. Unknown work cannot justify an external-only declaration.
No native CLI, provider or model campaign ran in this increment, and no independent,
human-only or release gate closed. The sandbox's native executable ownership and
user-systemd prerequisites remain unavailable; the actual Linux demo milestone is
still open as recorded in [local testing](../LOCAL-TESTING.md).
