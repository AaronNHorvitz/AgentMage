# Temporary Connected Profile and Cache

Sprint 70 defines an authority-free state machine for one visible, user-initiated connector read.
Strict-local remains the default. A temporary grant names actor, session, task, connector, exact
lowercase DNS host, GET/HEAD method, path, query hash, purpose, scope, expected data, byte limit,
issue/expiry, credential reference/scope hash, cache partition/retention, and cancellation boundary.
Preview and approval perform no network operation.

Activation requires the exact preview digest and unexpired grant. A separately authorized executor
may later supply one response observation; this module contains no DNS, socket, HTTP, proxy,
credential retrieval, or network-client API. Completed content can produce only an observed-encrypted,
sensitivity-labeled, freshness/expiry-bound cache record in the exact account/workspace partition.
Imported content carries no policy, grant, tool, instruction, or completion authority.

Cancellation, rate limiting, failure, and uncertain results never populate the cache. Every terminal
receipt requires a fresh grant for another request, requires derived-credential invalidation, and
restores the strict-local baseline. Startup polling, silent refresh, blind retry, source upload,
remote semantic indexing, and cloud-model fallback have no representable state.

Actual networking, secret-store derivation, encrypted cache persistence/deletion, packet capture,
native isolation, and a real connector account remain separate blocked campaigns.
