# Decision 0106: Search Disclosure Bound to Requests

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-29 |
| Date | 2026-09-29 |
| Authority | Decisions 0054, 0081, 0082, 0084 and 0087; current owner restart |
| Scope | AMR-02.2 research budget accounting and CAP-23 search disclosure |

## Findings

The canonical research owner reserved a request under a caller-supplied label:
a search query or a page visit. A query label only proved that the query's digest
had been disclosed. Nothing tied that query to the prepared request's actual target.
A request labelled as a visit skipped query accounting entirely, even when it was a
provider search. The owner's comment said trusted tool composition would choose the
label, but no such composition exists. Only test fixtures reserve research requests
today, so this is a contract gap, not an observed disclosure.

## Decision

A research plan discloses its search endpoint exactly: provider domain, unencoded
path, the one query field that carries a disclosed query, and any fixed query fields
sent with every search. Plan schema 2 requires the endpoint. Its domain must be one
of the plan's destination domains. The endpoint passes the same public-target syntax
and secret checks as a request. The research scope snapshot records it as schema 2,
and its policy digest covers it.

The owner derives the accounting class from the prepared request, never from a
caller label. A request to the provider domain must use the exact endpoint path and
exactly the fixed fields plus the query field. That query must be a disclosed query,
and it is reserved as a query. Any other request to the provider domain is refused
as a destination error. Requests to other allowed domains are reserved as visits.
The public reservation entry point no longer accepts a label.

Schema 1 plans and scopes stay decodable and verifiable, and their bytes and policy
digests are unchanged. They record no endpoint, so the owner cannot bind their
queries to requests. It refuses new reservations under them; a new schema 2 plan is
required. No production composition has used schema 1 reservations.

Residual, recorded under [Decision 0109](0109-review-fixes-for-component-batches.md):
a request to another allowed domain is a visit whatever its query fields hold, so a
site search listed as a destination could receive text that was never disclosed as a
query. The secret detector, the domain list and per-packet approval in ask mode still
apply. Before task-authorized issuance, the owner must refuse non-empty query fields
on visits or require the plan to disclose them.

This decision does not change the budget ceilings, the destination and DNS checks,
the ask or task-authorized approval semantics, or any grant. It adds no provider,
credential or network path.

## Verification boundary

Deterministic tests cover exact query reservation, undisclosed or altered queries,
missing, extra and changed fields, other provider paths, visits elsewhere, schema 1
refusal, plan and snapshot validation, reopen, and schema 1 byte compatibility.
They use no network, provider or model. Native research, provider and real-model
acceptance remain open, as do the distinct ask and task-authorized requirements.
