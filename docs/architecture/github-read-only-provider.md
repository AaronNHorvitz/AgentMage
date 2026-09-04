# GitHub Read-Only Provider Boundary

Sprint 71 adds a pure admission and observation boundary for GitHub.com and explicitly approved
Enterprise hosts. An authentication binding fixes canonical API and clone hosts, certificate or
SSH host-key digest, enterprise, organization, account, app installation, sorted repositories,
one read operation, minimum sorted permissions, SSO state, expiry, credential reference, and one
of command-line, REST, or GraphQL transport. GitHub App installation identity is preferred; the
other closed credential classes remain exact, secret-store-derived references.

The provider contains no HTTP, DNS, proxy, OAuth, device-flow, Git, credential-helper, or
secret-store executor. It admits a read only when the Sprint 70 active grant matches the same
host, actor, repository, operation, and credential reference. Redirects, upload hosts, aliases,
URL rewrites, cross-account repositories, write scopes, stale grants, and cancelled requests fail
closed. Only repository metadata, content, commit metadata, and reference metadata are
representable.

Caller observations normalize success, partial, paginated, empty, malformed, rate-limited,
expired, revoked, unauthorized, forbidden, unavailable, and cancelled states. Receipts bind host,
repository, actor, path, immutable object identity, request, result, freshness, pagination, cache
validator, rate state, and retry boundary; `external_state_changed` is always false. Actual
credential derivation and authenticated provider traffic remain separate external campaigns.
