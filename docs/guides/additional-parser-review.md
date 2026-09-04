# Additional Parser Review Guide

1. Match the caller-supplied byte digest and parser profile before reviewing any extracted record.
2. Confirm every range is half-open, ordered, inside the source, and bound to the exact record or
   containing cell span.
3. Treat HTML scripts, frames, JavaScript URLs, and external references as inert unsupported
   findings; never preview them in an active browser.
4. Reject XML document types/entities and YAML tags, constructors, anchors, aliases, or merge keys.
5. Confirm YAML secret values are absent while their hashes and redaction counts remain reviewable.
6. Review notebook code and outputs as inert source and content hashes; execution counts are metadata,
   not proof that AgentMage ran a cell.
7. Reconcile log sequence, ranges, timestamp observations, JSON objects, and stack continuations
   without evaluating content.
8. For archives, review names, compressed/uncompressed sizes, compression ratios, duplicate/path
   checks, nested quarantine, and total expansion. Do not open members to make the inventory pass.
9. Keep every deferred or proprietary format disabled until its own accepted promotion and evidence
   exist.
