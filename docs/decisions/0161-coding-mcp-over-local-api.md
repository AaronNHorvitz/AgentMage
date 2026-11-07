# Decision 0161: Coding MCP over the local API

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-07 |
| Authority | Decisions 0153 and 0154; T-303/T-306 |
| Scope | Client-credential-only coding tools over standard input and output |

Expose the existing workspace read, literal search, strict model proposal and
pending diff routes through an executable MCP bridge. Use the ordinary API
credential exclusively. A delegated private directory can contain only
`client.token`; never open `control.token` for a client operation. Require an
explicit launch flag consenting to share the already selected workspace with
the calling program, and pin its revision and fresh random selection identity for the entire bridge
session. Check the identity again under the API source owner; numeric revisions
alone can repeat after a service restart. This adds no authority and leaves
existing revision-only clients compatible.

The person selects source paths and approves any model endpoint disclosure
through existing controls first. MCP arguments cannot change the selection,
revision, endpoint, authentication, authority store or HTTP destination. Recheck
the revision before every tool call and retain API scope and drift checks.
Share only the current pending diff, excluding historical write records from
earlier scopes. Treat all returned source, model text and diffs as untrusted.

Stage strict diffs only as proposals for existing person-facing controls. Do not
offer an approval tool, invoke a command, create or adopt a worktree, apply a
write, commit or push. Staging through the workspace route does not establish
the separate repository owner's transferable edit lineage. No additional agent
loop or model-admission claim is introduced.

Implement the fixed MCP 2025-06-18 stdio profile with bounded exact JSON frames,
initialization, ping, static tools and tool calls. Advertise only implemented
capabilities. Process one request at a time; offer no sampling, elicitation,
resource subscriptions or acknowledged in-flight cancellation. Do not retry
HTTP operations on refusal, timeout or disconnect. The API remains the single
owner of permissions, effects and operation records.

This is a non-closing interface increment. Native configured-endpoint coordinator
composition, owned edit/test/retry scoring, durable ownership, approved Git
publication and broader MCP capabilities remain open. Independent core review
is required; no mission parent, score or release status changes here.
