# Gateway Codec, Routing, and Fallback

Each protocol codec declares an exact version, implementation digest, supported canonical message
parts, streaming, structured-output, tool-proposal, usage, cancellation, health, concurrency, and
typed-failure capabilities. Ordered canonical events must be contiguous, request-bound, and
terminal. An unknown external event or unsupported semantic produces an explicit non-success;
translation never drops or guesses a field.

Routing considers only separately activated exact candidate tuples. Every decision checks the user
profile, deterministic classification, disclosure ceiling and acceptance, role, capability digest,
health, resource fit, quota, cost, platform, context, concurrency, qualification, candidate, route
policy, and invariant-control identities. Candidates are audited in deterministic privacy-class and
route-identity order. The receipt retains every considered route and reason, exact selection,
disclosure class, route/fallback policy, and its own digest.

Fallback is absent by default. A failed prior route does not authorize another destination. A
fallback needs an explicit ordered policy beginning with the prior route, an independently named
destination authorization, accepted destination disclosure, and identical invariant controls. Any
missing, stale, reordered, unauthorized, more-disclosive, unhealthy, over-quota, or control-drifted
case blocks visibly without changing endpoint, disclosure, cost, or authority.

The gateway continues to return inert model proposals. A protocol terminal is not workflow
completion, and a route receipt is not a capability grant or credential.
