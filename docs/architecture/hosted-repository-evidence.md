# Hosted Repository Evidence

Sprint 72 projects separately observed GitHub data into immutable, coverage-aware evidence. Each
fact binds approved host, repository, account, commit and object digests, optional relative path and
range, retrieval time, canonical-link digest, classification, permissions, limitations, and one of
complete, partial, unknown, blocked, inaccessible, or stale. The closed family covers discovery,
search, source objects, revisions, releases, rules, workflows, and authorized security metadata.

Pagination, missing permissions, unavailable APIs, or incomplete facts make the whole view
non-complete. Optional local comparison records an exact revision match or mismatch. Secret names
may be counted, but secret values are rejected. Repository content is untrusted data with no
instruction, execution, or policy authority; hosted and local state-change fields are always false.
The module has no network or Git client and cannot resolve links or modify either repository.
