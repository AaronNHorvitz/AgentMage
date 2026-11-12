# Decision 0206: Bounded complete model responses

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-12 |
| Authority | Local agent mission; measure-first directive; T-303/T-306 |
| Scope | Credential-owning local model transport and trusted API settings |

The six-model development repeat recorded 48 stream-size refusals, the largest
aggregate cause. In the A4B repeat all 29 refusals emitted no visible answer
before the 4 MiB stream ceiling. These observations do not separate event framing
from non-answer output, or establish that a larger token allowance would finish.

Allow trusted endpoint settings to select `response_mode: complete`. The default
remains `stream`. Complete mode sends `stream: false` to the same approved chat
endpoint and validates one JSON completion after HTTP EOF. This avoids repeated
event envelopes without increasing the existing 4 MiB body, 5 MiB wire or 1 MiB
decoded-text ceilings. A prepared request binds its actual body and immutable
transport. No server error, tool call, wrong model, malformed completion or
incomplete HTTP response produces completed text or usage. The strict edit
checker and its separate smaller answer ceiling remain unchanged.

The independent review found that the initial complete decoder accepted empty
answer text, unlike streaming. Empty content must return the same incomplete
response error in either mode, for both `stop` and `length`, without text or a
completed exchange. Non-answer metadata does not count as answer text. A
nonempty answer cut at its token limit retains its visible finish reason.

The versioned local API keeps its existing event contract. Complete mode emits
at most one text event after upstream validation; it provides no intermediate
progress. Existing idle and total deadlines still apply while waiting. Trusted
controls must choose suitable existing timeout settings explicitly; selection
does not adjust them, retry or fall back. Model-facing chat and proposal
arguments cannot select a response mode. Ordinary conversations retain their
streaming default. The option also supports conversations without any folder.

Measure the development set in resumable GPU jobs. Report response framing and
any changed idle budget as changed variables. Do not infer task passes from
strict ungraded proposals, restore missing grading controls, bypass executable
trust, change hidden tests or use held-out tasks. This adds no row, model role,
native admission, permission, closure or release claim.
