# Decision 0152: Local Chat Protocol and Media Queue

| Field | Value |
| --- | --- |
| Status | Accepted under the owner mission and bounded implementation choices |
| Date | 2026-10-25 |
| Authority | Decision 0151; owner vision and speech request, 2026-10-25 |
| Scope | T-301 disposition, T-302 decomposition and T-305a |

## Decision

1. Retain the third independent T-301 review unchanged. Its verdict is
   INCONCLUSIVE, with no open high or medium finding. The native Rust verification
   blocker already has sub-row T-301.1. Under Decision 0151's three-strike rule,
   leave both rows open, carry low findings to T-309 and proceed with independently
   useful T-302 protocol work. No test failure becomes an exclusion or a pass.
2. Split T-302 into protocol components (T-302.1), credential-owning local transport
   and first-run connection testing (T-302.2), versioned local HTTP API and UI
   (T-302.3), and a reserved real-model observation (T-302.4). Each requires review;
   the parent requires all four. The protocol component has no user interface or
   effects of its own. Existing qualification-bound gateway routes remain intact;
   this component does not activate them or qualify a coding model.
3. Start endpoint configuration with HTTP to explicit loopback addresses and a
   configurable API base path. Canonicalize `localhost` to IPv4 loopback without
   DNS. Refuse credentials in URLs, remote destinations, ambiguous addresses and
   path traversal. A configured destination is not a permission grant. Remote
   model selection, TLS and its disclosure authorization require separate work
   before that destination can receive data. Do not silently route to a cloud.
4. Text chat encodes explicit conversation messages without hidden instructions,
   tools or fallback. Incrementally decode bounded event-stream bodies, preserving
   Unicode and distinguishing normal completion from the token ceiling. Require
   both a server finish reason and final completion marker. A truncated stream
   remains partial. Reject tool/function proposals on this chat-only path.
   Model text remains untrusted plain text. These are protocol guarantees, not
   transport, sandbox, agent-loop or task-completion guarantees.
5. Add T-305a after research and before coding help. It covers vision inputs,
   scanned document page/region citations, research image attachments, voice input,
   read-aloud and hands-free controls through the local API and UI. Size and time
   bounds, explicit model destination, transient media by default and confined
   speech children are required. The owner's binding no-bundle/no-download rule
   still governs models: prepare licensed local fallback support for models the
   owner supplies; agents acquire or distribute no weights. Handling is core-tier;
   controls alone are plugin-tier. Split implementation into reviewed sub-rows.

## Limits

This decision closes no row. T-302.1 supplies effect-free configuration and codecs
only. No endpoint request, credential storage, authenticated local API, UI, real
model inference, image/audio handling or owner installation is demonstrated by it.
T-301.1 must be revisited by T-309. Historical evidence keeps its original binding;
source changes in this T-302 batch do not trigger per-edit supply-chain renewal.
