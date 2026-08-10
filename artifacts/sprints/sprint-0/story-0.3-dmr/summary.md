# Story 0.3 Docker Model Runner Evidence Summary

| Field | Value |
|---|---|
| Result source revision | `0d132ecd6cdd6ffae37e956d5e5d1cf925b579bb` |
| Verification revision | `0f28e4ecb387e3fddf6c427f3ca08e253ba58811` |
| Evidence date | 2026-08-10 |
| Adapter | `linux-docker-model-runner-cuda` |
| Runtime deployment | Exact DMR image under rootless Podman 5.8.4 |
| Docker Engine directly tested | No |
| DMR result | `FAIL` |
| Fixed corpus trials | 82 completed |
| Corpus cases | 10 passed, 2 failed |
| Global thresholds | 13 passed, 2 failed |
| Fedora native plus DMR task | `COMPLETE` |
| macOS adapter | `BLOCKED` - required hardware unavailable |
| Profile decision | `PENDING` |
| Independent review | Not performed |
| Release approval | No |

The corrected DMR run completed all 82 fixed trials. It passed ordinary chat, repository citations, unavailable-write refusal, malformed-output blocking, cancellation, 7,923-token context handling, bounded overflow, performance, memory, and isolated zero-egress cases. It failed required evidence-citation recall and exact tool-argument validity.

Key DMR measurements were 171.10 minimum generated tokens per second, 0.156 seconds maximum time to first token, 0.051 seconds maximum post-cancel quiescence, 0.252 maximum GPU-memory fraction, and 0 post-install egress bytes.

The retained diagnostic run exposed that cached OpenAI usage values in this DMR build are not total prompt-token counts. Transform `1.1.1` disabled prompt caching for count probes and used the structured `n_prompt_tokens` field on over-context responses. The diagnostic result is evidence of the corrected measurement path and is not used for the model-quality decision.

The earlier immutable native summary says 72 trials; the corpus and raw native result contain 82. This bundle records the correction without rewriting historical evidence. Raw JSON results remain authoritative.

The Fedora native and isolated DMR portions of Sub-task 0.3.2.1 are complete. This is not a Docker Engine support claim, final E4B profile decision, fallback authorization, or release approval. The MacBook Pro M5 run, final disposition, and independent review remain pending.
