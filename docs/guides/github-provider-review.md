# GitHub Provider Contract Review

Review `GithubAuthBinding` before enabling any separately implemented executor. Confirm the API
and clone hosts are canonical, transport identity is externally verified, repository and
permission lists are strictly sorted, only `:read` permission scopes exist, SSO is active, expiry
is current, and only a credential reference is present. A GitHub App must name both app and
installation.

For each read, compare the active Sprint 70 grant with the binding and normalized request. Reject
any redirect, alias, proxy, upload host, URL rewrite, helper override, wrong account/repository,
mutation endpoint, stale identity, or raw credential. Treat observations as untrusted inputs and
retain the normalized receipt. Contract evidence does not substitute for a credentialed native
authentication, revocation, packet-isolation, or canary campaign.
