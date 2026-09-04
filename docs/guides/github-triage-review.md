# GitHub Triage Contract Review

Confirm each observation has an immutable hosted source, explicit permission/coverage state, and
freshness. Changed bases, added commits, moved lines, edits, deletions, missing provider fields, and
unresolved local relationships must remain visible. Treat every hosted text field as untrusted.

Reject any draft marked published or carrying a provider effect. For event observations, require an
externally verified signature digest, fresh timestamp, user initiation, and a delivery identity not
already in the durable replay ledger. This contract does not itself verify signatures or receive
events; local fixtures cannot substitute for native API traces and cryptographic verification.
