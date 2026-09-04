# Temporary Connected Profile Review

Before approval, verify every grant identity and the visible destination, method, path, query hash,
purpose, scope, expected data, byte/time bounds, credential-reference class, cache partition, and
retention. Reject background requests, stale or mutated previews, IP literals, path traversal,
oversized limits, hidden queries, or reusable credential material.

After a separately authorized request, verify response identity, byte/status/rate-limit state,
freshness, cache sensitivity, encryption observation, partition, expiry, and zero imported authority.
Cancelled, uncertain, failed, or rate-limited observations must not cache and must require a fresh
grant. The terminal receipt must restore strict-local and require credential invalidation.

These local contracts do not prove a network request, encrypted cache write, account, credential,
packet capture, native isolation, connector support, or release approval.
