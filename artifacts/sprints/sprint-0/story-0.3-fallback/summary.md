# Story 0.3 Gemma 4 12B Fallback Evidence Summary

| Field | Native Linux | Docker Model Runner compatibility |
|---|---:|---:|
| Result | `FAIL` | `FAIL` |
| Fixed corpus trials | 82 | 82 |
| Minimum generation rate | 58.99 tok/s | 91.30 tok/s |
| Maximum time to first token | 4.454 s | 0.177 s |
| Citation recall | 0.500 | 1.000 |
| Repository fact accuracy | 1.000 | 0.000 |
| Tool-call validity | 0.000 | 0.000 |
| Unsupported action rate | 0.000 | 0.667 |
| Post-install egress | 0 bytes | 0 bytes |

Both available Linux adapters completed all 82 fixed public-synthetic trials with unchanged corpus-v1 thresholds. Native Linux passed the resource, isolation, schema, repository-fact, and unsupported-action thresholds but failed citation recall and exact tool-call validity. The DMR compatibility path passed resource, cancellation, malformed-output, zero-egress, and citation-recall thresholds but failed repository accuracy, citation precision, schema validity, exact tool-call validity, and unsupported-action thresholds.

The selected GGUF and projector were hash-verified before native inference. The exact OCI model manifest, config, model layer, projector layer, and DMR runtime image were hash-verified before isolated DMR inference. This resolves evaluation staging only; source-lineage and reproducible-conversion admission blockers remain.

The exact DMR image ran under rootless Podman 5.8.4, not Docker Engine. GPU access required manual NVIDIA device exposure, an explicit bundled CUDA compatibility-library path, and disabled SELinux label separation for that container. These compatibility findings do not establish Docker Engine support or an approved production topology.

The MacBook Pro M5 run remains unavailable and Linux evidence was not substituted. The Linux quality failures are independently dispositive: Gemma 4 12B Unified remains disabled and is rejected for this fixed v0.1 profile. No automatic fallback, profile activation, support claim, independent review, or release approval follows from this evidence. Raw JSON results are authoritative over this summary.
