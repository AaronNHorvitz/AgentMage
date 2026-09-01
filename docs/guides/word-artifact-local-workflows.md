# Word Artifact Local Workflows

## Inspect And Extract

1. Read the source through an authorized workspace reader and retain its validated relative path.
2. Pass immutable source bytes and the exact conversion profile to the OOXML inspector.
3. Stop on quarantine or incomplete part hashing; do not attempt a best-effort extraction.
4. Review the canonical part inventory, feature counts, and every fidelity warning.
5. Treat the original DOCX as authoritative and the generated text sidecar as a search aid.
6. Persist a sidecar only through a separate controlled-write preview and approval.

The product coordinator performs inspection, sidecar extraction/cache reuse, and canonical source
preparation over the same exact request. A successful coordinator outcome still grants no write or
renderer authority.

## Generate From Markdown

1. Parse the approved Markdown bytes through the byte-preserving Markdown boundary.
2. Choose a new validated output identity; never reuse the source identity.
3. Review generation warnings for frontmatter, links, raw HTML, reference links, setext headings,
   and duplicate headings.
4. Require the reopened inspection to be complete and non-quarantined.
5. Treat returned DOCX bytes as a proposal until a controlled writer authorizes persistence.
6. For persistence, bind the proposal to one held absent destination and review its exact digest,
   mode, generated classification, verification plan, and rollback narrative before issuing any
   single-use write grant.

## Refusal Expectations

Reject or quarantine encrypted packages, duplicate or unsafe paths, unsupported compression,
macros and active content, external relationships, malformed relationship XML, resource-limit
violations, source overwrite requests, and any request to execute or resolve document content.

Do not claim visual fidelity, accessibility acceptance, an approved or executed persistence effect,
native-interface integration, installed product acceptance, or supported-platform execution from
the local Sprint 58 evidence. Those remain later verification work.
