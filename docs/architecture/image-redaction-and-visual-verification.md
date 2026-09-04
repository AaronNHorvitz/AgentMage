# Image Redaction And Visual Verification

## Boundary

Sprint 65 admits caller-supplied image bytes and decoded RGBA pixels only. The capability performs
bounded metadata inspection, deterministic 32-bit BMP decoding and regeneration, exact decoded-pixel
redaction, pure before/after comparison, and disclosure-bound generation routing. It never discovers
or writes a file, captures a screen, launches a viewer or renderer, executes embedded content,
resolves a link, or contacts an image provider.

PNG input receives bounded header metadata inspection only. PNG pixels, JPEG, animation, vector
images, color-profile conversion, steganography detection, optical character recognition, native
screenshot capture, and native rendering remain outside the local decoder boundary.

## Metadata And Provenance

`inspect_image` binds the exact source-byte digest to a stable source identity and optional owning
presentation slide/object pair. It reports dimensions, admitted format, declared color space, byte
size, ancillary-byte count, decoded-pixel availability, unsupported features, and all three denied
effect classes. A presentation owner is never accepted partially.

Only uncompressed 32-bit BMP is decoded locally. The decoder checks dimensions, planes, bit depth,
compression, offsets, byte ceilings, row order, and exact RGBA length. Deterministic regeneration
uses a fixed 54-byte header and reopens to the same top-down RGBA digest.

## View And Redaction

`prepare_image_view` returns an in-memory proposal for an explicitly vision-capable profile. It
does not capture or display a screenshot. Declared sensitive regions block the proposal unless the
exact proposed pixel digest and canonically ordered region set match a successful redaction receipt.

`redact_image` replaces each half-open rectangle with opaque black pixels, regenerates the complete
BMP, reopens it, scans the decoded target pixels, and proves that no ancillary source bytes were
copied. The receipt contains only source/output hashes, region coordinates, layer outcomes, and
denied effects. Native visual inspection is still required because a deterministic pixel scan does
not establish human-perceived correctness.

## Visual Comparison

`compare_images` accepts exact RGBA before/after values for document pages, presentation slides,
standalone images, or user-interface screenshots. It records changed-pixel count, integer
parts-per-million ratio, maximum channel delta, and the tight changed rectangle. Thresholds are
integer-only and dimensions must match. Synthetic/local comparison always retains
`human_visual_review_required: true`; it cannot substitute for native renderer, font,
accessibility, or cross-platform evidence.

## Optional Generation And Editing

The generation boundary is an admission and receipt contract, not an image model. A preview binds
the exact operation, approved profile, local/provider route, strict-local state, instruction hash,
optional source-image hash, and canonical disclosure codes. Provider routes are rejected in
strict-local mode and without `provider.external-processing`. A completion receipt requires the
byte-exact approved preview digest and exact output digest. No provider call or persistence occurs
inside this capability.

## Runtime Records

The closed runtime schemas are `image-inspection`, `image-redaction-receipt`, and
`image-visual-comparison`. Their semantic validator preserves complete presentation ownership,
decoded safe-context state, canonical regions, difference aggregates, and zero hidden effects.

## Current Limits

Native screenshot capture/viewing, decoded PNG/JPEG coverage, steganographic analysis, native
document/presentation/UI rendering, installed-font comparison, accessibility review, Fedora,
Ubuntu, Windows 11, and macOS native campaigns, provider execution, independent boundary review,
and manual fuzzing are not admitted by local evidence. Their absence blocks the complete Sprint 65
gate and is never replaced by synthetic pixels.
