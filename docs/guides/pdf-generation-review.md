# PDF Generation and Redaction Review Guide

## Review Inputs

Use the exact committed revision, `Cargo.lock`, dependency catalogs, software bill of materials,
Sprint 61 dependency manifest, 101-case corpus, runtime schemas, and retained evidence report. Do
not infer product renderer admission from a development-only Mermaid installation or from
synthetic RGBA fixtures.

## Required Checks

1. Run all focused PDF Rust tests with `--locked` and confirm zero ignored focused tests.
2. Reproduce an exact generated report twice and confirm identical HTML and PDF hashes.
3. Mutate output paths, content, internal page links, form names, Unicode text, metadata, and active
   structures; confirm fail-closed behavior.
4. Regenerate a report with exact redaction targets and independently scan HTML, PDF bytes, parsed
   objects, metadata, form content, extracted text, and incremental-update markers.
5. Mutate every runtime evidence record and confirm byte, path, identity, residue, completion,
   synthetic-evidence, and effect drift rejects.
6. Validate passive and hostile SVG fixtures without launching a renderer from the core library.
7. Recompute the 101-case corpus, dependency manifest, supply-chain records, documentation gate,
   and retained evidence report.

## Expected Local Behavior

Generated artifacts are proposals returned in memory. They use no remote asset, current clock,
ambient filesystem, network, browser, or child process. Redaction is accepted only for an exact
AgentMage-generated source specification and creates a full new artifact. Synthetic visual inputs
may pass machine thresholds while remaining explicitly non-release evidence.

## Prohibited Conclusions

A passing local report does not establish a native renderer, product-integrated Mermaid execution,
arbitrary-PDF redaction, merge/split support, OCR execution, accessibility conformance,
cross-platform acceptance, independent review, manual fuzzing, release approval, or Sprint 61
completion.
