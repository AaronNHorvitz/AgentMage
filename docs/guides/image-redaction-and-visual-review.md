# Image Redaction And Visual Review Guide

## Source Review

1. Match the caller-authorized source bytes to `provenance.source_sha256` and preserve any owning
   slide/object pair together.
2. Confirm the dimensions, image type, color space, byte size, ancillary-byte count, and unsupported
   feature list before accepting decoded pixels.
3. Treat metadata-only PNG inspection as non-viewable until a separately admitted decoder supplies
   exact pixels.
4. Never follow image links, launch embedded content, or infer a clean source from its appearance.

## Sensitive Pixels

1. Declare every sensitive rectangle before any model-context or export proposal.
2. Verify the source pixel digest and exact canonical region set in the receipt.
3. Reopen the regenerated BMP, scan the decoded rectangles, and confirm metadata removal.
4. Refuse the view/export when the current pixel digest or any rectangle differs from the receipt.
5. Retain a human visual review; opaque pixels can still cover the wrong region.
6. Before export, reopen the exact proposed BMP again and match its digest, decoded pixels,
   redaction identity, metadata-removal state, and workspace-relative destination.

For AgentMage-generated presentations, review the exact target hashes and all six layer checks:
specification, slide objects, speaker notes, internal relationships, absent thumbnails, and exported
package. Identity, path, or timestamp targets are unsupported and must fail instead of being renamed.

## Before/After Review

1. Bind both decoded RGBA digests and confirm identical dimensions.
2. Review changed-pixel count, parts-per-million ratio, maximum channel delta, and tight bounds.
3. For document and slide renders, independently review clipping, overlap, order, fonts, cropping,
   alternative text, and renderer identity.
4. For user interfaces, independently review focus order, contrast, scaling, and accessibility-tree
   output; a screenshot alone proves none of them.

## Optional Generation

1. Review the exact instruction hash, source-image hash, profile, route, and disclosures.
2. In strict-local mode, accept only the local route.
3. For a provider route, require explicit external-processing disclosure and byte-exact approval of
   the preview before any separate network boundary runs.
4. Bind the resulting image digest to the approved preview and scan its pixels and metadata before
   persistence or export.
