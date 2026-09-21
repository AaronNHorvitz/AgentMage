# Decision 0062: Bounded Two-Model Coding Preparation

| Field | Value |
|---|---|
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-21 |
| Authority | Decision 0054 and the owner's explicit request to download gpt-oss-20b and prepare larger-context, lightweight Muse and GPT-OSS tests |
| Scope | Local synthetic model preparation for Tasks 48.2.4-48.2.6 and 50.2.4 |
| Preserves | Existing rejected profiles, product activation, confinement, independent review, resource caps and the operator stop marker |

## Decision

1. Prepare both Muse Glimmer 30B and OpenAI gpt-oss-20b for the coding workstream.
   The owner clarified that "light" means lightweight resource use, not low
   reasoning effort. Keep reasoning and resource settings separately identified.
2. Reuse the existing pinned first-party Muse Q4_K_M and packaged llama.cpp
   b10423 Vulkan runtime. Do not replace the document-QA demo's 8K profile or
   the user's separate Muse service. Target 32,768 context tokens in a new
   text-only development profile and measure actual served capacity and memory.
3. Download only the pinned primary MXFP4 GGUF from `ggml-org/gpt-oss-20b-GGUF`
   revision `ef9b12f2ff56c69cf32153a02784e7a3c88bf524`, plus small provenance
   records. Its published SHA-256 is
   `27cd6c432c7672cb812a92f611cf3ba7bbc35928262bb1e1253ff4ee6ae35901`
   and size is 12,109,566,624 bytes. `.src_sha` names OpenAI source revision
   `6cee5e81ee83917806bbde320786a8fb61efebee`. No speculative model is acquired.
4. This GGUF is a llama.cpp-maintainer conversion, not a first-party OpenAI
   artifact. The owner's acquisition request and standing technical delegation
   authorize this exact synthetic, offline development evaluation before the
   post-GA lab. This narrowly refines Decision 0027's evaluation sequencing;
   it does not waive production conversion provenance or model admission.
   A published hash verifies downloaded identity, not conversion equivalence.
5. Store the download under `~/.local/share/agentmage/coding-model-lab`, outside
   Git. Keep partial bytes quarantined, verify complete size and SHA-256 before
   promotion, and retain source metadata, terms, conversion log and their hashes.
   No login, paid service, account or credential acquisition is needed.
6. Use one model at a time, one inference slot, at most four CPU threads, a
   two-core CPU quota, low scheduling priority, small prompt batches and the
   existing 5G/6G/512M systemd memory/swap limits. Require at least 16 GiB
   available RAM at start. Reserve desktop VRAM headroom; a sampled GPU guard
   is not a hardware-enforced GPU-memory or utilization quota. Do not change
   GPU power limits, drivers, system services or other project processes.
7. Follow the existing Linux inference sandbox mount pattern: private network
   and process namespaces, an empty environment, read-only runtime/model and
   driver inputs, no home/workspace mount, and a private Unix socket with an
   ephemeral API key. Model proposals have no tool execution authority.
8. Record exact context, KV-cache format, reasoning controls, decoding, runtime,
   hardware and measured results. Start with 32K; a resource or compatibility
   failure stays visible. Any smaller fallback is a separate explicit profile,
   never a silently shortened context. Do not confuse hidden reasoning with
   disabled reasoning or arbitrarily suppress it to fit an output budget.
9. Preparation tests cover load, served context, synthetic replies, tool-call
   parsing and long-context retrieval. They do not complete coding integration,
   production admission, quality/repeatability, platform or independent review.
   Subsequent coding campaigns run both models separately through the actual
   AgentMage binaries, with model-specific codecs and all failures retained.
10. The later implementation scope is exactly Tasks 48.2.4, 48.2.5, 48.2.6 and
    50.2.4 plus their necessary prerequisites, not every intervening task number.
    Preparing models does not restart the stopped worker or authorize publication.

## Sources

- [OpenAI model source](https://huggingface.co/openai/gpt-oss-20b/tree/6cee5e81ee83917806bbde320786a8fb61efebee)
- [Pinned GGUF conversion](https://huggingface.co/ggml-org/gpt-oss-20b-GGUF/tree/ef9b12f2ff56c69cf32153a02784e7a3c88bf524)
- [Existing Muse preparation](../verification/demo-model-probe.md)
- [Standalone coding architecture](../architecture/standalone-coding-harness.md)
