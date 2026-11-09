# Decision 0180: Guided terminal continuation of the retained endpoint run

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-09 |
| Authority | Local agent mission; T-303/T-306; Decisions 0177 and 0178 |
| Scope | Explicit terminal scheduling over the existing authenticated API |

Add `endpoint-run <seconds> <run-id> <request-digest>` to the existing terminal
client. Pin the original sealed request, then show each service step. Ask for
fresh model disclosure before every advance and separate exact operation consent
for every tool challenge. Approval is held only in the client until the next
disclosure confirmation succeeds. A declined confirmation stops the client;
it does not send a denial or approval to the coordinator.

Use the existing controlling-terminal, signal and elapsed-time guard. Refuse
nonterminal input, stale selection, invalid challenge scope, repeated approval
identities and more challenges than the original tool budget. Bound advance
requests to that budget plus one terminal observation. The original service
owns counters, deadlines, grants, tools and completion verification throughout.
No HTTP operation is automatically resent, and stopping neither cancels nor
releases the run. Preserve the existing individual commands.

After inspection, a fresh client may continue the same still-held service run
with new confirmations. It does not recreate a service, model port, worktree,
event cursor, approval or budget from a file. Terminal envelopes remain service
observations, including failures; a zero client exit means the terminal response
was observed, not that the task passed. No signed commit, push, durable execution
recovery, native trust exception, model admission or parent closure is added.
