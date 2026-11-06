# Decision 0155: Project test output and producer fixtures

| Field | Value |
| --- | --- |
| Status | Accepted implementation detail under the owner mission |
| Date | 2026-11-06 |
| Authority | Owner coding priority; Decisions 0153 and 0154 |
| Scope | T-303/T-306 project test protocols and their exact producer fixtures |

The coding owner admits explicitly selected Rust libtest and verbose Python
unittest output through the existing kernel normalizer and exact command
approval/rerun owner. Parser selection precedes execution and is bound into the
immutable template. Counts, names, summaries and process exits must agree; the
approved project runner remains trusted to describe its tests. This is not
independent grading. Execution, minimum counts and the retry ceiling are unchanged.

Adding the two parser identities to the validation receipt schema changes the
model-visible tool-catalog digest. Synthetic producer requests and their dependent
records therefore have new input identities, even though the IPC wire and record
schemas are unchanged. Publish the current synthetic producer collection as
version 5, built from current production types and functions. The wire stays 18.
No record version is reinterpreted and no older fixture is regenerated.

Keep all version 1-4 fixture bytes and documents. Pin each complete original
manifest, verify every file against it and check the complete file set. Versions
2-4 must additionally decode exactly and pass the existing client verification
against their own frozen requests and recipes. Their original input catalog is
not the new catalog; comparing their input digests with newly generated inputs
would conflate distinct snapshots. Version 1 retains its unknown-declaration
schema refusals. The current version still requires exact reproduction and all
existing mutation/unknown-field/client checks. The Python checks retain equality
of the version 2-4 record files and add complete manifest pins.

This changes no evidence-artifact input set, dependency hashing, hidden evaluation
input, threshold, permission or release gate. Historical compatibility is not
current runtime acceptance. Both parent rows remain open pending their complete
workflow, measurements and independent review.
