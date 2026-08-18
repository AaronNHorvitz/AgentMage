# Packaging

Platform package definitions live here. Packaging consumes verified build output
and cannot change source, dependencies, policy, entitlements, or release identity.

Package builders emit explicit unsigned candidates by default. No candidate may
be relabeled as signed, release-verified, shipped, or supported.

Linux payloads include the authenticated host, isolated runtime adapters,
one-shot model installer, Docker guard and observer, stateless read-only
worker, Visual Studio Code extension, and license. The package manifest binds
each executable by path, mode, size, and SHA-256; including a worker does not
by itself activate or authorize it.
