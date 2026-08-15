# Presentation Review Guide

## Inspection Review

1. Confirm the source path and SHA-256 identify the caller-authorized `.pptx` bytes.
2. Confirm `inspection_complete` is true and review every content-minimized finding.
3. Treat `safe_for_reuse: false` as a hard stop; do not open, render, or reuse quarantined content.
4. Compare slide order, notes, links, images, captions, alternative text, and layout hashes with the
   expected source provenance.
5. Remember that links were retained but never followed and embedded content was never executed.

## Generated Deck Review

1. Review the complete specification, proposed output path, and fixed metadata timestamp.
2. Verify every table, chart, and diagram data-source hash against the approved ordered input.
3. Review each structural slide preview in slide and object order.
4. Confirm the reopened inspection source hash equals the generated package hash and reports safe
   reuse.
5. Do not interpret structural preview success as visual, font, pagination, or accessibility proof.

## Edit Review

1. Confirm the request source hash equals the exact original generated package hash.
2. Review complete replacement slides rather than accepting partial XML patches.
3. Compare each before/after preview digest and confirm untouched slide digests remain identical.
4. Preserve the original package until an independently authorized persistence action succeeds.

## Native Verification

Before a presentation can be considered visually complete, render the exact package on each
required native platform and retain renderer identity, version, package hash, slide images, pixel
diffs, font substitutions, clipping/overflow results, and accessibility findings. Missing,
synthetic, stale, skipped, or unreviewed native evidence blocks completion.
