# Supply Chain

This directory contains deterministic dependency provenance, a CycloneDX 1.6
software bill of materials, and SHA-256 hashes for their generating manifests,
locks, and outputs.

Run `npm run supply-chain:build` after an intentional manifest or lock change.
Run `npm run supply-chain:check` in normal verification. The check fails for a
stale component closure, changed lock integrity, undeclared registry source,
missing license disposition, missing hash, or promoted macOS status.

Development-only packages remain in the SBOM with `excluded` scope so the build
toolchain is reviewable without representing those packages as shipped runtime
dependencies. `NOASSERTION` records absent upstream license metadata visibly; it
is not license approval. Later license and package-admission gates must disposition
every such record before release.
