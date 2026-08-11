# Adversarial Grant Corpus

The version 1 corpus is generated and executed by
`kernel/engine/tests/adversarial_grant_corpus.rs`. It contains 560 deterministic seeded cases:
40 independent mutations for each of actor, session, task, action, tool, path, argument,
preimage, side effect, expiry, nonce, use count, parent, and preview digest.

Context and clock mutations run through final atomic consumption. Every case must return its exact
first policy-denial scope, retain zero admitted attempts, leave use count at zero, and advance the
otherwise-current operation to `invalidated` or `expired`. Nonce, use-count, and parent mutations
are forged public `CapabilityGrant` candidates. They run through direct policy evaluation because
the consumption API accepts only an issuer-retained grant identity; they must fail at the grant
boundary and leave the real retained operation unchanged.

The corpus uses synthetic identities, hashes, and paths only. It exercises the in-memory shared
kernel and Linux reference build; no production worker, filesystem access, durable transaction,
or macOS implementation is represented.
