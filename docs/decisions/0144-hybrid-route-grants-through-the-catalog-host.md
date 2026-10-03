# Decision 0144: Hybrid Route Grants Through the Catalog Host

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-10-02 |
| Date | 2026-10-02 |
| Authority | Decisions 0054, 0081, 0124, 0128, 0131, 0132, 0135 and 0143; current owner restart |
| Scope | AMR-05.9.7 split into AMR-05.9.7.1 and AMR-05.9.7.2 |

## Findings

AMR-05.9.7 asks the development host to offer hybrid routing only under a
host-owned grant owner. That owner must:

- keep each separately granted remote route's expiry and budget counters;
- source those counters into each routing request;
- let a person grant and revoke routes through the CLI;
- record each remote route, its provider and the data it receives in the
  run's route receipt and history;
- never substitute a remote route silently.

The tests must cover expiry, budget exhaustion, revocation, a refused fallback
and local-only refusal, through the host and the CLI.

Five facts shaped this decision:

- The kernel router (Decision 0124) already decides hybrid mode. A remote route
  is eligible only under its own grant: for its exact candidate, before the
  grant expires, for data the grant covers and within its counters. The router
  orders routes strict-local first and selects the first eligible one, so a
  remote route is chosen only when every local route is ineligible, or as an
  explicit fallback.
- The development host offers one route, the run's own profile (Decision 0128),
  and records it as eligible. It holds no grant and routes in local-only mode.
- No remote model provider is configured, and none may be acquired under the
  current authority: no accounts, paid services or downloads. So the
  development host has no remote route to offer and no adapter to send a
  request to one.
- The catalog host already keeps per-workspace owner states in the operational
  store for memory and extensions (Decisions 0131 and 0132). It runs as an
  actual process in this sandbox. Owner states take a new owner name without a
  migration.
- Any new catalog operation raises the IPC wire version. Under its change
  control, that raises the runtime producer contract's version (Decision 0143).

## Decision

### Split

AMR-05.9.7 is split. It closes when both new rows close.

- AMR-05.9.7.1, a component row. It covers:
  - the grant owner in the catalog host;
  - granting, listing and revoking through the CLI;
  - the development host sourcing live grants and their counters into each
    routing request;
  - the client's verification and display of local and remote selections.

  Remote routes appear only in host tests.
- AMR-05.9.7.2, which remains open. It covers offering an actual remote route
  of a configured, independently qualified provider:
  - its adapter;
  - consuming the grant's counters before each request is sent;
  - its proof through actual processes on a native host.

  It needs a provider the owner configures and AMR-05.10's qualification.

### Grant owner

The catalog host keeps one route grant catalog as the owner state
`route-grant-catalog`. A scope is a workspace identity, the same identity the
run request carries. A scope holds at most 64 grants, of which at most 16 are
live, the router's own bound. The catalog holds at most 64 scopes.

A grant is the kernel's hybrid route grant:

- its identity, the exact route and candidate, and the provider named to the
  person;
- the data classes the route may receive;
- the most requests and input tokens it admits;
- whether it may replace a failed route;
- its exclusive expiry and its digest.

For each grant the owner also keeps:

- when it was granted;
- the digest of the person's decision;
- the requests and input tokens counted against it;
- when it was revoked, if it was.

A grant is accepted only when all of the following hold:

- Its digest recomputes.
- Its identities are plain and bounded.
- It names at least one data class.
- Its budgets are at least one and at most 100,000 requests and 1,000,000,000
  input tokens.
- It expires after the host's clock and at most 30 days and one hour later.
- No grant of the scope has its identity.
- No live grant of the scope names its route.

Revoking is permanent. It refuses an unknown or already revoked grant. A
revoked or expired grant stays listed with its counters, so the history of a
scope's disclosures stays inspectable.

A grant is live when it is neither revoked nor expired at the host's clock. Its
listed state is one of four:

- live;
- exhausted, when either counter has reached its budget;
- expired;
- revoked.

The owner offers one more operation for a later remote adapter. It counts one
request and its input tokens against a live grant, under the revision the
owner read, and refuses one that the grant cannot admit. Nothing in the
development host calls it yet.

### Through the CLI

The CLI gains three catalog operations. A workspace is required for a grant or
a revocation, and optional for a list.

- `--route-grant FILE --route-grant-workspace ID` reads a closed grant request
  of at most 4 KiB. The request names the grant identity, the route and
  candidate, the provider, the data classes, both budgets, whether a fallback
  is allowed, and a validity of 1 to 720 hours. The CLI computes the expiry
  from its clock and the grant's digest. It shows the grant, as text or as one
  JSON row: what the route may receive, from which workspace's runs, for how
  much, until when, and that this development host offers no remote route yet. It sends the grant only after
  the person types `yes`. Every other answer, end of input and cancellation
  decline, and nothing is sent. The decision digest is the digest of the
  exact request, which names the scope and the sealed grant.
- `--route-grant-list` lists every grant with its state and counters.
- `--route-grant-revoke GRANT_ID` revokes one grant.

Both output formats are supported. A catalog answer is shown only when it
acknowledges the request that was sent.

### Routing

The development factory reads the run's scope from the store it already opens
for the composition. It routes in hybrid mode exactly when the scope holds a
live grant, and in local-only mode otherwise. In hybrid mode the routing
request has these properties:

- It carries every live grant with the owner's counters.
- It admits disclosure up to a remote managed route, which the grants accept.
- It names its own fixed hybrid policy digest.

The development host still offers only its profile route, so every run
selects its local route. The receipt says the run was decided in hybrid mode,
names no grant and no provider, and its route history entry records the mode.

If a remote route were ever selected, the factory would refuse the composition
before building any model. No remote adapter exists, and counting a request
that is never sent would be false.

Host tests drive the same routing function with fixture remote routes. They
make the local route ineligible, or name it as a failed prior route, and check
that:

- a granted remote route is selected and its receipt names the grant, the
  provider and the data;
- expiry, exhausted counters and revocation make it ineligible;
- a fallback is refused unless its grant allows one;
- local-only mode refuses the remote route.

### Client verification and display

The client keeps a route receipt in three cases:

- a local-only receipt, as before;
- a hybrid receipt that selected the run's profile route with no grant;
- a hybrid receipt that selected a remote route, when all of these hold:
  - the selected route is eligible and of a remote class;
  - the receipt names a grant digest and a provider;
  - the data it transmits is the run's declared data;
  - it is not a fallback, because the development host never falls back.

Every other receipt is dropped as unavailable. A remote selection is shown on a
line of its own: the provider, the route, the grant and every data class sent.
So a remote route is never used silently. The route history entry of a remote
selection records the grant digest as evidence.

### Wire and contract

A catalog request and answer `route_grant` cross the authenticated IPC
channel. The Linux IPC wire version becomes 17. The development host refuses
route grant operations. Only the catalog host answers them.

The runtime producer contract moves to version 3:

- `fixtures/runtime-producer/v3` holds the same records with wire 17;
- its document states the hybrid receipt rules;
- version 2 keeps every byte its manifest names, and a test shows that it
  reads as an older wire.

## Limits

- The development host offers no remote route. Every actual run selects its
  local route, and no request leaves the machine.
- Remote selection is exercised only by host tests with fixture routes.
- No counter is ever consumed outside tests, because no remote adapter
  exists. Consumption, an actual remote route and their native proof are
  AMR-05.9.7.2.
- The factory's grant lookup runs only in native composition, which this
  sandbox refuses. The grant owner, the CLI and the catalog host run as
  actual processes here.
- Independent review remains open.

## Consequences

- `kernel/engine`: the owner state name `route-grant-catalog`.
- `shells/host`:
  - the `coding_route_grants` module with its tests;
  - hybrid routing and the client's receipt rules in `coding_route`;
  - the factory's grant lookup;
  - the catalog host's operation, with the transport and IPC (wire 17);
  - the CLI's route grant operations;
  - the producer contract tests for version 3.
- `fixtures/runtime-producer/v3`, the version 3 contract document, its script
  and test.
- TASKS.md: AMR-05.9.7.1 and AMR-05.9.7.2 are added, and AMR-05.9.7 names them.
  AMR-05.9.7.1 is recorded with its evidence when the batch is verified.
