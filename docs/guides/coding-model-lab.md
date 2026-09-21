# Two-Model Coding Preparation

This is an offline synthetic preparation environment under Decision 0062, not
AgentMage's coding runtime or a production model activation. Both models must
later run the same coding acceptance tasks through AgentMage's actual binaries.

## Prepared Profiles

The source configuration is
[`coding-model-lab.json`](../../model-profiles/development/coding-model-lab.json).
It reuses the fully hash-verified llama.cpp b10423 Vulkan package identified in
`demo/model.json`, not the unrelated global `llama-server` symlink.

| Setting | Muse Glimmer | gpt-oss-20b |
|---|---|---|
| Artifact | Existing first-party Meta Q4_K_M | Pinned ggml-org MXFP4 conversion of OpenAI weights |
| Configured context | 32,768 tokens | 32,768 tokens |
| Output reservation | 4,096 tokens, including reasoning | 4,096 tokens, including reasoning |
| KV cache | Q8_0 keys and values | Q8_0 keys and values |
| Reasoning template control | `reasoning_strength=medium` | `reasoning_effort=medium` |
| Modality | Text only | Text only |
| Slots | One | One |
| Coding qualification | Not qualified | Not qualified |

"Light" means resource-conscious configuration, not reasoning disabled. A larger
context consumes additional memory and computation; it is not free memory.
Diagnostic smoke decoding uses temperature zero and seed 42, not a claim of
portable determinism or a substitute for each model's quality profile.

## Resource and Access Limits

- One model runs at a time under a common local lock; the launcher refuses an
  occupied GPU rather than terminating another application's process.
- Four CPU threads, a two-core quota, niceness 10, batch 256 and microbatch 128.
- `MemoryHigh=5G`, `MemoryMax=6G`, `MemorySwapMax=512M`, and a 45-minute scope
  deadline. The launcher verifies the actual memory and CPU cgroup limits.
- At least 16 GiB available RAM and 21,000 MiB free VRAM are required at launch.
  A sampled guard stops this model above 22,528 MiB total GPU usage. This is
  **not** a hardware GPU quota: inference may briefly use high GPU utilization.
- Network/process namespaces, cleared environment, read-only model/runtime and
  required driver files, no home or repository mount, no vision/draft model,
  no web UI, no built-in tools and no MCP execution.
- A private Unix socket and a temporary mode-0600 API key. The key and socket
  are removed at exit. Never paste the key into a prompt or Git.

The existing 8K document-QA demo and separate `muse-glimmer.service` are unchanged.
Nothing starts at login, and preparation does not leave a model resident.

## Commands

Run from the AgentMage repository. Each command enters its own bounded systemd
scope; no additional Python package, Ollama service or cloud account is needed.

```bash
python3 -m scripts.coding_model_lab verify-download
python3 -m scripts.coding_model_lab probe muse
python3 -m scripts.coding_model_lab probe gpt-oss
```

Probes run sequentially and stop their own model afterward. Each checks the
actual served context, basic output, a synthetic tool proposal, tool-result
feedback and retrieval across more than 24,000 input tokens. Actual prompt usage
must match the tokenizer preflight; length-terminated output cannot pass. No
proposed tool is executed, no workspace is edited and no coding gate is closed.

For a bounded, manually stopped development inference session:

```bash
python3 -m scripts.coding_model_lab serve muse
# Or, after the first model has stopped:
python3 -m scripts.coding_model_lab serve gpt-oss
```

The command prints the private socket path. Ctrl-C stops only that session.
This endpoint is for synthetic development, not an alternate product transport.
The future native runtime must launch its own confined worker under its own
accepted activation contract; it must not bypass admission by attaching here.

## Acquisition and Evidence

The primary GPT-OSS file is 12,109,566,624 bytes. The complete digest, immutable
source revisions and artifact paths are in the profile. Partial data remains
under `~/.local/share/agentmage/coding-model-lab/quarantine` until verified.
The primary model is not downloaded again if the verified destination exists.

`python3 -m scripts.coding_model_lab provenance` records the pinned conversion
metadata/log, source revision, model card, license, usage policy, template and
configuration. This command accesses Hugging Face; inference commands do not.
These records establish acquisition identity, not conversion equivalence or
production qualification. Do not describe the ggml-org file as OpenAI-published.

Local logs, synthetic requests/results and reports are retained in unique
private directories under `~/.local/state/agentmage-model-lab`. Reports distinguish
preparation from coding acceptance and retain failed or interrupted runs.
The [measured preparation results](../verification/coding-model-preparation-2026-09-21.md)
remain separate from the production model registry.

## Required Coding Integration

Codex's implementation scope is **48.2.4, 48.2.5, 48.2.6 and 50.2.4**, plus
their necessary prerequisites. Do not interpret that as permission to execute
every intervening task or resume desktop work.

1. Replace the native driver's hard-coded 8,192-token prerequisite with validated
   exact-profile context handling and served-limit checks. Do not simply remove
   the guard or replace every constant with 32K.
2. Keep Muse ATEM and GPT-OSS Harmony message/tool/reasoning handling in explicit
   family codecs behind the common model boundary. Compatible HTTP endpoints
   alone do not establish compatible parsing or trustworthy tool proposals.
3. Bind the full model, quantization, tokenizer/template/codec, runtime, context,
   KV-cache, decoding and resource configuration. Neither old Muse rejection nor
   this preparation result determines the new coding profile's disposition.
4. Run both models sequentially through the same real CLI/host edit-test-repair
   campaign. Preserve failures and compare repeated success, invalid calls,
   interventions, latency, RAM/VRAM and cancellation behavior. Use synthetic
   disposable repositories before real user work.
5. Preserve all permission, confinement, cleanup, independent-review and release
   gates. A completed server smoke probe is not a completed coding harness.

The AgentMage worker remains stopped. Model preparation does not remove its
operator stop marker or authorize its restart.
