# GitHub Mutation and Recovery

Hosted writes begin as local, content-free draft packages. Each request binds one canonical host,
repository, actor, credential reference, mutation class, object, payload, expected effect, preview,
single-use grant, expiry, and idempotency key. Hosted and local state, effective permissions,
protection, rulesets, required checks, and push rules are re-read before submission; any drift
invalidates the approval.

Commit, push, review, workflow, merge, and release are separate classes. A push is limited to one
full `refs/heads/agentmage/` task ref and binds its old and new objects. It additionally requires a
verified signature and distinct approvals for the exact local commit and the network push.

Implicit or configured destinations, multiple refspecs, protected and tag refs, deletion, push
options, upstream mutation, submodule recursion, scope-expanding flags, force variants, bypass,
automatic publication, administration, secrets, and ruleset changes are never admitted.

A terminal receipt names whether external state changed and whether exactly one expected effect was
observed. Verified non-effect may be reconsidered only from a fresh request. Partial or unknown
effects stop for current remote-state reconciliation and are never blindly retried. Recovery for
expired credentials, changed permission or protection, renamed repositories, deleted branches,
moved lines or bases, and partial publication creates a new preview and grant; it never repairs by
replaying the old request.

The current module is an inert boundary. It creates previews and validates observations but has no
Git, filesystem, process, credential, or network executor. Native GitHub mutation, signed-commit,
push, failure-injection, rollback, API audit, and independent-review evidence remains required.
