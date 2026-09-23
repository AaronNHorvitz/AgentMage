# Decision 0071: Native Channel Token Preservation

| Field | Value |
|---|---|
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-23 |
| Authority | Decision 0054 and the owner's real-model integration assignment |
| Scope | Native reasoning-codec output transport and operation-specific tool guidance |
| Preserves | Pinned runtime/model, 32K/4096 profile, strict schemas, exact grants, confinement, verifier and existing stores |

## Evidence

Muse's first repair completed with verifier success. The subsequent new-file
attempt emitted multiple reasoning frames followed by a directory proposal;
the decoder rejected the number of frames before native argument validation.
The retained response also lacks intermediate end-of-message delimiters.

The pinned llama.cpp source determines rendered special-token bytes from its
`special` launch option or chat-route preserved tokens. Raw `/completion` does
not use the chat-route template. AgentMage enabled neither special-token output
nor chat-route preservation on this path, although its family codecs consume
those exact delimiters. See the pinned
[server rendering implementation](https://github.com/ggml-org/llama.cpp/blob/a94d563ed801d1da1b8c2432946de07d0231bb3d/tools/server/server-context.cpp#L3330)
and [launch option](https://github.com/ggml-org/llama.cpp/blob/a94d563ed801d1da1b8c2432946de07d0231bb3d/common/arg.cpp#L1780).

## Decision

Enable `--special` only for the existing explicit reasoning profiles. Preserve
the legacy non-reasoning 8K launch. The exact argv/launch configuration remains
hash-bound and checked against the actual process. No sampling, output/context,
concurrency or resource limit changes.

Muse accepts a bounded sequence of inert self-reasoning frames followed by
exactly one native action, including the server's preserved delimiters. Multiple
actions remain invalid. Recognize its pinned end-of-text terminal spelling as
well as end-of-turn; neither EOS nor reasoning alone creates a proposal.
Historical separator-stripped records remain readable, not rewritten.

The new-file response's `encoding=utf8` directory arguments remain invalid.
Expose the existing operation-specific encoding/query/range rules directly in
each registered read-tool description. Do not repair arguments or relax the
native validator. The actual retained multi-part response and invalid directory
encoding are regressions. Retain the unsuccessful campaign and start a new exact
tuple campaign after verification; the first successful diagnostic is not enough.
