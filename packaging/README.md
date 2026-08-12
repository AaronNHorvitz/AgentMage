# Packaging

Platform package definitions live here. Packaging consumes verified build output
and cannot change source, dependencies, policy, entitlements, or release identity.

Package builders emit explicit unsigned candidates by default. No candidate may
be relabeled as signed, release-verified, shipped, or supported.
