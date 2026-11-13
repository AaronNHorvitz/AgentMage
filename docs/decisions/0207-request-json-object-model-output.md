# Decision 0207: Request JSON-object model output

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-13 |
| Authority | Measure-first directive; T-303/T-306 |
| Scope | Local model transport and trusted API settings |

The six complete-response development repeats at source
`974d2bb55afa6be47bd58ffcf1dd30a6450f70cd` recorded 51 strict format refusals
in 180 attempts, the largest aggregate refusal cause. Inspection of those
development answers finds 27 starting with Markdown fences, 21 other malformed
JSON answers and three JSON answers with duplicate keys or missing fields.
These observations select a narrow generation change, without task grades or
held-out feedback.

Trusted endpoint configuration may request `response_format: json_object`.
The transport encodes `response_format: {"type":"json_object"}` in the exact
bound upstream request. Text remains the default and omits that field. This
choice is independent of streaming versus complete response framing and works
without a workspace or repository. It adds no prompt, provider selection,
filesystem access, network destination or tool authority. An unsupported
endpoint can reject the request; no fallback or retry is introduced.

The setting requests a syntactic constraint from a supporting endpoint. It
does not prove that the endpoint honored it. Chat responses remain untrusted
text. Coding proposals must still pass the original strict checker, including
object shape, unique keys, exact visible preimages, paths, overlap and size
bounds. No fence stripping, JSON repair, permissive parser, threshold change
or execution bypass is permitted. All existing request/response byte ceilings,
deadlines, approvals, completion checks and settings-revision rules remain.

Repeat the development set through the available coding path before further
tuning, publishing changed variables and outcomes separately from task grades.
Native artifact admission and the missing original grading controls still
block execution, tests and retries. No model role, qualification or row closure
follows from this change. Independent core review remains required.
