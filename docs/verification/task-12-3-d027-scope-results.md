# Task 12.3 D027 Scope Results

## Result

Pass for `D027-S12-SCOPE`; accepted planning additions are recognized without
weakening preservation controls.

Decision 0027 remains an immutable **241 stable requirement / 30 normative
mapping / 17 epic / 169 sprint** baseline. Decision 0040 subsequently changed
only the current normative-map result to **31** through one accepted execution-
policy addition and one statement supersession; stable requirements, epics, and
sprints remain 241, 17, and 169.

## Cases

| Case | Boundary | Expected result | Result |
|---|---|---|---|
| `D027-SCOPE-01` | Historical counts | Recompute pre-0027 as 229/26 and post-0027 as 241/30 | Pass |
| `D027-SCOPE-02` | Current counts | Recompute post-0040 and canonical sources as 241/31/17/169 | Pass |
| `D027-SCOPE-03` | Additive identities | Preserve 229 identifiers and admit only the 12 Decision 0027 identifiers | Pass |
| `D027-SCOPE-04` | Requirement preservation | Reject mutation, deletion, duplication, reordering, and renumbering | Pass |
| `D027-SCOPE-05` | Unsupported addition | Reject an added identifier even when its reported count is edited | Pass |
| `D027-SCOPE-06` | Decision authority | Reject missing approval and corrupt snapshot linkage | Pass |
| `D027-SCOPE-07` | Normative reconciliation | Reject corrupt appended, superseded, or count records | Pass |
| `D027-SCOPE-08` | Current status | Reject status-model count disagreement | Pass |
| `D027-SCOPE-09` | Model direction | Preserve Decision 0001 history and require Decision 0027 supersession | Pass |
| `D027-SCOPE-10` | Generated linkage | Reject registry/traceability identity or count disagreement | Pass |
| `D027-SCOPE-11` | Source identity | Reject a changed canonical-source or decision-document hash | Pass |
| `D027-SCOPE-12` | Clean checkout | Run all declared planning and documentation checks from committed source | Pass |

## Negative Controls

The focused campaign executes and rejects all of the following:

1. `mutated-preserved-requirement`
2. `deleted-preserved-requirement`
3. `duplicated-preserved-requirement`
4. `reordered-preserved-requirement`
5. `renumbered-preserved-requirement`
6. `unapproved-added-requirement`
7. `missing-decision-approval`
8. `corrupt-appended-id-set`
9. `corrupt-status-count`
10. `corrupt-supersession-boundary`
11. `corrupt-generated-registry-linkage`

## Evidence

- The closed planning manifest retains the exact pre-0027, post-0027, and
  post-0040 requirement identifiers and normative mappings, their accepted Git
  revisions, canonical source hashes, decision hashes, and approval markers.
- The validator independently counts canonical registry records, normative
  mappings, epics, and sprints and reconciles the additions-only and generated
  traceability identities.
- The status validator requires both rejected Gemma records to remain disabled
  historical evidence but no longer rejects a future eligible candidate merely
  because it is not one of those two records.
- Markdown, Mermaid, documentation, registry, additions-only, normative
  coverage, schema, policy, traceability, and isolated clean-checkout checks all
  run without an ignored or waived result.

## Limits

- This evidence closes planning-integrity behavior; it does not enable a model,
  complete a product capability, or establish a release claim.
- Decision 0040's 31st current mapping is not retroactively attributed to
  Decision 0027.
- Manual fuzzing remains deferred to its separately approved final campaign.
