# Decision 0162: Explicit endpoint observation delegation

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-08 |
| Authority | Decision 0161; T-303/T-306 |
| Scope | Read-only MCP observation of one retained configured-endpoint run |

Keep the existing endpoint HTTP entry point control-only. Add an explicit
trusted startup policy for delegating the exact request and canonical event
pages to the ordinary client credential. A host must deliberately choose
`DelegateObservations` through
`serve_coding_endpoint_with_observation_access`. HTTP input cannot change this
policy. Delegation shares potentially sensitive task text, catalog metadata and
tool observations. It permits only `GET /v1/code/endpoint/request` and
`POST /v1/code/endpoint/events`; the latter reads a bound event page and changes
no run state. Model disclosure and advancement, tool decisions, cancellation
and release remain exclusive to trusted controls. Other routes remain refused.

Extend the existing MCP executable with a separate launch selection:
`--share-selected-endpoint`. Pin the original run identity and sealed request
digest at launch, rather than a workspace revision. Reuse the same bounded
stdio lifecycle, ordinary client credential, fixed loopback HTTP client and
request/event validators as the existing terminal client. Advertise only two
read-only tools in this selection: `endpoint_request` and `endpoint_events`.
Recheck the original sealed selection before every call. Tool arguments cannot
select another run, grant, credential, route, service policy or model.

Launch consent shares that run's observations with the calling program; it is
not consent for a model call. The bridge cannot control the calling program's
subsequent handling of received data. Canonical event validation checks each
individual event's hash, identity and page bounds. It cannot establish complete
stream verification, accepted completion proof or a durable recovery cursor.
Never retry an uncertain HTTP operation or silently follow a new service run.

This is an interface increment under the existing open parents, with no new
row or sub-row. Actual native filesystem/grant composition, executable service
activation, edit/test/retry, durable recovery, approved Git publication and
parent measurements remain open. Independent core review is required; no
model qualification, score or release claim follows.
