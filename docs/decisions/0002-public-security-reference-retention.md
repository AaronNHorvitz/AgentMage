# Decision 0002: Public Security Reference Retention

| Field | Value |
|---|---|
| Status | Accepted |
| Date | 2026-08-10 |
| Scope | Public product-security references used by the planning baseline |
| Supersedes | None |

## Context

AgentMage uses public security, privacy, accessibility, AI-risk, and software-supply-chain references as engineering inputs. A reviewer must be able to identify the exact public authority consulted and distinguish a citation from certification, endorsement, or incorporation of third-party material.

Some cited pages are living publications whose response bodies change without a versioned URL. Other references are subject to publisher access or reuse restrictions. Committing local copies by default would create stale evidence and unnecessary licensing risk.

## Decision

1. `requirements/security-references.json` is the canonical public-reference register.
2. Every citation in Section 5 of `SECURITY-REVIEW.md` has exactly one register record with publisher, title, version when available, publication and effective dates when declared, source URL, retrieval evidence, status, and snapshot decision.
3. The planning baseline retains metadata and a SHA-256 of the exact retrieved HTTP response body, not a local copy of third-party source content.
4. Living or dynamic sources are re-retrieved during the applicable review. The recorded hash proves what response was observed on the retrieval date; it does not claim that the page is immutable.
5. Material subject to publisher access or reuse constraints is not copied into the repository. An access-denied response may be recorded transparently, but it is not treated as a verified content snapshot.
6. A future local snapshot requires an appended decision, a permitted-use review, a repository-relative path, and a hash of the retained artifact. It must not replace the public source URL or provenance record.
7. Citation records and derived controls do not claim certification, conformance, approval, sponsorship, or endorsement by any publisher or standards body.
8. `requirements/security-reference-baseline.json` pins the accepted review state. Any source change produces a blocking impact review; routine validation never updates that baseline.

## Consequences

- Documentation checks can detect missing, duplicate, orphaned, or malformed public-reference records without network access.
- Routine builds remain deterministic and do not fail because a publisher changed a dynamic page.
- Reviewers can identify inaccessible or changing sources without mistaking a response hash for the underlying standard's artifact hash.
- Source-content retention remains fail-closed until its license and evidence purpose are explicitly approved.

## Verification

- `npm run references:check` reports an exact one-to-one mapping between Section 5 citations and register records.
- Tests reject malformed provenance, unapproved snapshots, inconsistent status, and certification or endorsement claims.
- Mutation tests require an impact review for removal, integrity change, supersession, redirect, or source substitution and prove the pinned baseline is unchanged.
- The documentation gate performs no network request and does not write or update source material.
