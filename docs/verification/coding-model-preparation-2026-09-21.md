# Two-Model Preparation Results: 2026-09-21

## Disposition

**Both 32K development profiles passed direct inference preparation checks.**
Neither is an admitted coding model, and Tasks 48.2.4-48.2.6 and 50.2.4 remain
open. No actual AgentMage coding command, edit worker, test runner, installed
transport, persistent context or independent review was exercised by this batch.

Authority is Decision 0062 and the owner's explicit model-preparation request.
The owner clarified that "light" means lightweight resource use, not low
reasoning effort. The existing demo and separate Muse service were not changed.

## Artifacts and Runtime

- Muse reuses the existing 16,756,683,904-byte first-party Meta Q4_K_M file,
  SHA-256 `4cc57c0f51040a226e5a72cc47b7613f7772950e460a665f7083de89f183f60e`.
- GPT-OSS was downloaded without credentials from pinned ggml-org revision
  `ef9b12f2ff56c69cf32153a02784e7a3c88bf524`. All 12,109,566,624 bytes matched
  published SHA-256
  `27cd6c432c7672cb812a92f611cf3ba7bbc35928262bb1e1253ff4ee6ae35901`
  before promotion from quarantine. Its path is
  `~/.local/share/agentmage/coding-model-lab/models/gpt-oss-20b-MXFP4.gguf`.
- The GGUF is a llama.cpp-maintainer conversion, not an OpenAI-published file.
  The conversion's `.src_sha` identifies OpenAI revision
  `6cee5e81ee83917806bbde320786a8fb61efebee`. The conversion metadata/log and
  first-party license, usage-policy, model-card, configuration and template were
  retained locally. Conversion equivalence and production admission remain open.
- Both use the existing fully inventory/hash-verified llama.cpp b10423 Vulkan
  package, source `a94d563ed801d1da1b8c2432946de07d0231bb3d`. No new runtime,
  driver, Python dependency, Ollama service or speculative model was installed.
- Hardware: NVIDIA RTX 4090, approximately 24 GiB VRAM; approximately 62 GiB
  system RAM. Tests ran sequentially, with no other project process stopped.

## Exact Preparation Profile

Both profiles use 32,768 context tokens, 4,096 reserved output tokens, Q8_0
key/value caches, flash attention, one slot, four CPU threads, a two-core CPU
quota, niceness 10, batch 256, microbatch 128, no CPU prompt cache and no context
shifting. Text only; no perception encoder or draft model. Muse uses
`reasoning_strength=medium`; GPT-OSS uses `reasoning_effort=medium`.
Temperature zero and seed 42 identify diagnostic decoding, not portable
determinism or model-quality qualification.

The live cgroups reported `memory.high=5368709120`, `memory.max=6442450944`,
`memory.swap.max=536870912` and `cpu.max=200000 100000`. Cgroup peak memory
includes charged file cache and varies with cache ownership; it is not a
portable model-RAM estimate. Earlier preparation loads reached approximately
5 GiB inside the existing 6 GiB ceiling. GPU usage is sampled total device use,
including the desktop; it is not a hardware-enforced GPU quota.

Inference used the existing Linux sandbox mount pattern with isolated network
and process namespaces, a cleared environment, read-only runtime/model/driver
inputs, no repository/home mount, no executable tools, and a private Unix socket
with an ephemeral key. These observations do not replace security qualification.

## Final Smoke Runs

| Observation | Muse Glimmer | gpt-oss-20b |
|---|---|---|
| Served context, checked through `/props` | 32,768 | 32,768 |
| Basic reply | PASS | PASS |
| Native `read_file` tool proposal with exact synthetic arguments | PASS | PASS |
| Synthetic tool-result feedback and corrected return statement | PASS | PASS |
| Beginning/middle/end retrieval from long synthetic input | PASS | PASS |
| Preflight and actual long-prompt tokens | 28,145, matching | 28,146, matching |
| Long-context request elapsed time | 20.997 s | 10.051 s |
| Sampled peak total GPU memory | 16,937 MiB | 13,624 MiB |
| Resource guard triggered | No | No |

Each model also passed an earlier four-case preparation run. These few synthetic
checks are not a coding-quality benchmark or statistically sufficient reliability
campaign. The test supplied a synthetic failed-test observation; **no actual
test failure or file edit occurred**. No model-generated tool call was executed.
Every response ended normally; a length-terminated response would fail. The
long-context check required actual input-token usage to equal tokenizer preflight.

## Evidence Locations

The final local reports are under `~/.local/state/agentmage-model-lab`:

| Report | SHA-256 |
|---|---|
| `20260921T165607-muse/report.json` | `2c7e9cb796b59218c16e05a41ddad7aef1a27013b8dc68c73c0f5d1e96c345e1` |
| `20260921T165721-gpt-oss/report.json` | `aaf7ef55bdb150590e23c575d4f2fac587f2f091d97a1bc42089912f77492de4` |

Adjacent files preserve synthetic requests, raw responses, server properties and
logs. Earlier reports remain in their own directories. The provenance manifest
is at `~/.local/share/agentmage/coding-model-lab/provenance/20260921T165112/manifest.json`,
SHA-256 `b569477ed989b353772ff919b1045e98249c753a38c8c057570f87ce6c2a5746`.

Both final reports bind:

- Preparation script SHA-256:
  `45e030269e7403aae94fa4d72084b605fb0131e5e2880a2dc2d35f001ff9ec1f`.
- Development profile SHA-256:
  `0dd71a32c731907dbe196fe313fe27087b9c75f802855cde51f183d2374b08b5`.

After shutdown no `llama-server` or preparation process remained, the per-run
key/socket were removed, and total GPU memory returned to approximately 1.2 GiB.
There is no always-on model service from this preparation.

## Verification and Next Work

The 14 focused launcher tests and 42 existing planning/task-selection tests pass
(56 total). Markdown lint and `git diff --check` pass. The full historical
`docs:check` pipeline and Rust product test suite were not rerun for this separate
preparation batch. No Rust runtime source or product status record changed.

Follow [the preparation guide](../guides/coding-model-lab.md) and the existing
coding handoff. Native profile-driven context handling and an explicit GPT-OSS
Harmony codec still need implementation. Then run both exact models through
AgentMage's real CLI/host and preserve all admission and coding acceptance gates.

The AgentMage worker remains stopped with its marker present. Other agents were
not altered. This preparation batch remains local and uncommitted; no push or
agent restart was performed.
