# Decision 0077: Harmony JSON Format Delimiter

| Field | Value |
|---|---|
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-23 |
| Authority | Decision 0054 and the owner's real-model integration assignment |
| Scope | Exact optional-space JSON format delimiter in the existing GPT-OSS codec |
| Preserves | Closed native arguments, frozen tools, permissions, confinement, verifier, exact profiles and all budgets |

## Evidence

GPT-OSS campaign10 new-file1 at `ba1158e8` created the authorized missing file
before the requested failing validation, then exhausted its existing parser budget.
The first rejected response, SHA-256
`c17d3ae52c88d0a0d6597fb6df3af7a7e2a2b29252643ab3cfd2f361469cb42b`,
contains a single commentary channel, tool recipient and immediately adjacent
`<|constrain|>json` token. The decoder accepted that suffix only with a preceding
literal space. The exact retained payload reproduces `tool-channel-invalid` in
the before-fix codec regression. Its invalid native validation arguments are a
separate defect in this proposal and must not be repaired by the decoder.

The [official Harmony reference](https://developers.openai.com/cookbook/articles/openai-harmony#preambles),
fetched 2026-09-23, demonstrates the unspaced delimiter; its receiving-tool-calls
example demonstrates the spaced form. The format token, not whitespace, separates
metadata. The pinned upstream template's historical `commentary json` rendering
remains unchanged.

The second rejected reply uses duplicate channel markers and lacks a required
argument. It remains invalid. Both completed with EOS and substantial context
headroom, generating only 320 and 318 tokens under the unchanged 4096 reserve.
No context, generation or resource limit explains the first header rejection.

## Decision

Accept precisely the existing JSON format suffix with zero or one preceding
space. Do not trim arbitrary metadata, accept another content type, duplicate
format/channel markers, unknown recipients, extra frames or malformed JSON.
Retain both supported recipient positions and exact native catalog binding.

Preserve every original JSON member for the existing closed native validator.
The actual retained first body contains extra command/template/spec fields and
omits `template_sha256`; a host regression must still reject it before authority.
No schema, correction budget, grant, model sampler, context, generation or
resource policy changes. Recompute the exact codec-bound catalog normally;
do not weaken its source binding or replace the pinned model/template/runtime.

Retain the failed native attempt and the before-fix regression. A valid frame
does not make this attempt successful: it missed failed-test ordering and never
ran validation. Subsequent source-pinned campaigns are separate. Independent
review, coding-model admission, daily-use, platform and release gates remain open.
