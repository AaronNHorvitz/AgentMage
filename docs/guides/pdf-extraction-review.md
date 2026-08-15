# PDF Extraction Review Guide

## Review Inputs

Use the exact committed source revision, `Cargo.lock`, the Cargo external-license catalog, the
generated software bill of materials, the Sprint 60 dependency manifest, and the 82-case review
corpus. Do not treat a locally installed PDF utility as admitted merely because it is available on
the host.

## Required Checks

1. Verify `lopdf` resolves to version `0.44.0`, the retained registry checksum, MIT license, and
   disabled default features.
2. Run the focused Rust extraction tests with `--locked` and confirm zero ignored focused tests.
3. Run the runtime-schema tests and mutate page, source, text, citation, OCR-admission, aggregate,
   completion, and effect fields to confirm rejection.
4. Validate the frozen review corpus and dependency manifest against their generator.
5. Recompute the retained evidence report from committed source and confirm every blocker remains
   present.

## Expected Local Behavior

A text PDF yields consecutive page identities and exact-page citations. Image-only pages remain scan
candidates with `ocr_required=true` and `ocr_performed=false`. Encrypted, malformed, truncated, and
over-limit sources fail closed or retain an explicit blocking limitation. Validation never opens a
path, accesses the network, writes an artifact, or launches a parser, renderer, or OCR child process.

## Prohibited Conclusions

A passing local extraction report does not establish PDF generation, redaction, native rendering,
visual fidelity, accessibility, native OCR, product integration, cross-platform support, release
approval, or completion of Sprint 60. Those conclusions require their separately named evidence.
