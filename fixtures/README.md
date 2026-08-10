# Fixtures

This tree contains synthetic or explicitly admitted public test inputs and their
pinned expected results. It must not contain private user workspaces or operational
data.

The versioned corpus and its self-hashed provenance ledger live under
[`corpus/v1`](corpus/v1). The ledger closes over every root fixture contract and
records repository-relative, SHA-256-bound lineage only.
