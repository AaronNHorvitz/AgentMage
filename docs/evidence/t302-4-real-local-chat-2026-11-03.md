# T-302.4: Real local streaming conversation, 2026-11-03

Builder evidence based on `dc27bad3b019a89fedb82b413411b392ddf87fcb`.
T-302.4 and parent T-302 remain **open** pending independent exact-commit core
review. The base commit retained the unchanged
[T-302.3 PASS](../reviews/1e73f76d0896-local-agent-t302-3-api.txt), closing only
that API component. Its four low findings remain T-309 backlog.

## Executed path

An actual `agentmage-local-api` process served two conversation turns through
the existing approved local transport to the existing Muse Glimmer 30B Q4_K_M
model. This was real inference, not a fixture or a scripted model response.
`scripts/local_chat_smoke.py` is a new repeatable recording client; its
[guide](../guides/local-chat-measurement.md) explains authority, commands,
records and limitations. It is not the later task CLI or coding harness.

The private launcher acquired the normal shared build reservation and an
exclusive `agentmage` GPU reservation with a 600-second ceiling. It reused the
fully hash/inventory-verified llama.cpp b10423 Vulkan package and hash-verified
existing model. No model, runtime or dependency was downloaded. Four-CPU
affinity applied to the launcher and its children, with four inference threads,
two HTTP threads, one model slot, batch 256 and microbatch 128.

The model ran in the existing bubblewrap mount pattern: private network/process
namespaces, cleared environment, dropped capabilities, read-only model/runtime
and driver inputs, no repository/home mount, and an ephemeral keyed Unix socket.
A launcher-owned loopback TCP bridge forwarded bytes only to that fixed private
socket, because the current API transport accepts numeric loopback TCP. This
bridge is measurement infrastructure, **not** a new product adapter or native
confinement qualification. No other worker or service was stopped or altered.

The dedicated API had separate random control/client tokens. The recording
client explicitly chose the endpoint with the control token and a model key
file; ordinary listing/chat used the client token. The model key and API tokens
were distinct, never command-line values, never model input and never recorded.
The two fixed synthetic prompts contained no repository or personal data.
The second turn included the first actual answer in its request history.

## Observed conversation

1. User: `Name a color in one word.` Answer: `Azure`.
2. User: `What color did you just name? Reply with only the same word.` Answer:
   `Azure`.

The raw client events were:

```jsonl
{"turn": 1, "seconds": 5.9018102720147, "event": {"text": "Azure", "type": "text"}}
{"turn": 1, "seconds": 5.926128766004695, "event": {"finish": "stop", "type": "complete"}}
{"turn": 2, "seconds": 0.28593411200563423, "event": {"text": "Azure", "type": "text"}}
{"turn": 2, "seconds": 0.31015633698552847, "event": {"finish": "stop", "type": "complete"}}
```

Each turn delivered one actual text fragment before its final completion event;
the client then observed EOF. Both finished with `stop`, not `length`. These very
short answers do not demonstrate sustained token-by-token throughput. The
separate synthetic socket contracts establish incremental delivery while an
upstream is still producing text. This transcript is a connection observation,
not an answer-quality benchmark or coding admission.

| Measurement | Observed |
| --- | --- |
| Served model ID | `muse` |
| Served context, from model properties | 8,192 tokens |
| Requested output ceiling, each turn | 1,024 tokens |
| Server diagnostic decoding | Temperature 0, seed 42; medium reasoning template setting |
| KV caches | Q8_0 keys/values, flash attention on |
| First text / total wall time, turn 1 | 5.902 / 5.926 seconds |
| First text / total wall time, turn 2 | 0.286 / 0.310 seconds |
| Text bytes / events, each turn | 5 / 2 |
| Model ready, including identity verification | 30.014 seconds after reservation work began |
| Successful reservation work, including cleanup | 36.864 seconds, approximately 0.614 reserved GPU minutes |
| Sampled total device memory at readiness | 17,416 MiB; includes baseline use, not peak or per-process memory |
| Upstream token usage | Unavailable through this API; recorded as null |
| Recording client exit | 0, connection PASS |
| Settings revoked / owned child processes reaped | True / true |

This is a separate **8K chat-only** profile, not a renewal of the earlier 32K
coding-preparation claims. The current gateway supplies messages/model/output
ceiling; sampling defaults belong to the configured server. Temperature zero
and a seed do not establish portable determinism. GPU utilization and active
kernel minutes were not measured; reservation duration is not active GPU time.

## Failure retained

The first reservation attempt failed: it treated socket creation as readiness
while the model was still loading. The API did not complete a conversation.
The recording client exited 1 with zero completed turns, revoked settings and
recorded FAIL; its launcher reaped all owned processes and removed keys/socket.
That attempt took 13.407 seconds of reserved work. Its private logs remain.

The launcher was corrected to wait for the owned model's `/health` success,
then inspect its properties before invoking the unchanged recording client.
The second reservation produced the conversation above. No product parser,
permission, timeout, threshold, prompt or test was relaxed to obtain success.
The combined observed reservation work was approximately 0.838 GPU minutes.

## Checks and artifact identity

The recording client has 13 passing standard-library tests, including a
synthetic API on actual loopback HTTP sockets. They check full second-turn
history, explicit framing, separate tokens, incremental recording, private
output, credential-file refusals, preconfigured-instance refusal, incomplete
streams, error/length/trailing/oversize events, deadlines and failed cleanup.
These tests are fixtures, separate from the real model observation above.

The unchanged Rust API contract target was rerun through the build slot:
**15 passed**, zero failed/ignored/filtered, in 4.21 seconds, including its
actual executable and synthetic upstream socket tests. No Rust code, manifest,
lockfile, dependency or feature changed in this batch.

| Retained artifact | SHA-256 |
| --- | --- |
| Recording client used for both attempts | `da8d84a1b25831ac7407451c4f8d0109843d229bf0f60fd750af8bdb36d4a458` |
| Model file, 16,756,683,904 bytes | `4cc57c0f51040a226e5a72cc47b7613f7772950e460a665f7083de89f183f60e` |
| Inference server executable | `f7c93f0de9fed7b596e68557bd4d42a084204027f87d61c893777908e4c62bfe` |
| API executable | `dcf66344bc1733a293ce278dd4c7d04d60903261c5c2b0aa3cbac2b37593ad1b` |
| Successful raw events, exactly as above | `fb88eb374951b63229edc44a95546659c7005ed374f02728303c611728b31a19` |
| Successful client report | `2d3635c8c4a128a6a1ce7a70cad88c571fdf808e80a199ea34b296089367fa16` |
| Successful reservation summary | `10ec9087135e66873bcd6e206ad0bbcbe27a4605f0115c46f37b95032efe6104` |
| Successful model log | `c4465fc433af5dab6ca609d2018b144f9e4236e96543355730d3ea1a2e64c4ab` |
| Served properties | `458433b8b436bc1b23ad79a0a69884d1a73b77b730fb50df753bf3a4452247e1` |
| Corrected private reservation launcher | `e292dac156a25e1721439fc06a05cb37b3e737daf63d51e1dcf3dd01d5d2a684` |

Raw reports, properties, model/API logs, exact launch commands and the initial
failed attempt remain in private lane storage for the independent reviewer.
Machine-local paths are excluded from committed evidence. Ephemeral bearer
files and model sockets were removed; actual response text is retained.

The per-change fast gate passed after the final source changes: whitespace,
Markdown lint on three files, Python syntax/indentation/whitespace checks and
all 13 recording-client tests (0.517 seconds). No Rust target is automatically
selected for this Python/documentation-only change; the 15 API contracts above
were an additional explicit check. Scoped links, sensitive-text, claims,
identifier and selected host-identity checks passed, as did measured-client
hash and historical-ledger preservation checks. The reserved ordinary
documentation validator passed 691 Markdown files and all policy invariants.
The final result prose then received the scoped checks and fast gate again;
measured Python and Rust source bytes were unchanged. The preceding full Rust
gate remains FAIL on
the documented 53 host-environment failures under deferred T-301.1. This batch
does not reinterpret that failure as native acceptance. The long documentation
gate still runs once at T-310. No historical ledger, status model, supply-chain
output, permission rule or evidence binding was changed or regenerated.

## Remaining scope

The minimum T-302 components now have running implementation and measurements;
T-302.4 and parent T-302 await independent disposition. Settings remain memory
only; same-account clients are trusted, model-port trust is disclosed, and API
activity is process-only metadata. Durable recording/resume/replay of agent
runs, confined tools, API-backed task CLI/MCP and coding quality remain open.
This connection transcript does not substitute for those capabilities.

The next coding work is the fixed T-311 development/held-out suite and baseline,
**before** any harness or prompt tuning. No evaluation task or hidden test was
read or changed in this connection batch. There is no coding-score, owner-test,
install-kit or public-release claim.
