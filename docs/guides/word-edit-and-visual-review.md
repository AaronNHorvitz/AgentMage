# Word Edit and Visual Review Workflow

## Build a Rich Proposal

1. Construct a bounded rich-document specification with stable block, style, and numbering IDs.
2. Use only internal bookmark targets for hyperlinks; reject any external target.
3. Choose a new validated output path.
4. Build the package twice when reproducibility evidence is required and compare exact bytes.
5. Require the reopened inspection to be complete and non-quarantined.
6. Treat returned bytes as a proposal until a controlled writer authorizes persistence.

## Preview Exact Edits

1. Inspect and extract the immutable source package through the Sprint 58 boundary.
2. Select an exact fragment identity, part name, byte range, and expected decoded text.
3. Bind the request to the exact source package digest and a distinct output path.
4. Keep operation, comment, and revision IDs unique and canonically ordered.
5. Review the changed-part and preserved-part ledgers before authorizing any write.
6. Recompute the preview from the immutable request and require exact equality.

Do not broaden a rejected complex-run edit into a best-effort replacement. Preserve the source and
request an explicitly supported operation instead.

## Compare Rendered Pages

1. Admit a renderer only after recording its package source, license, exact version, artifact
   digest, font-manifest digest, platform, and closed settings.
2. Render immutable before and proposed after packages outside the pure comparator.
3. Decode each page to bounded row-major RGBA8 bytes and record exact page digests.
4. Submit matching platform, profile, and evidence-class outputs to the comparator.
5. Review pagination, per-page differences, clipping, overlap, font fallback, tables, and images.
6. Retain a human-review flag whenever the evidence is synthetic or a machine threshold fails.

Synthetic images are suitable for unit tests only. They never establish Word fidelity on Fedora,
Ubuntu, Windows 11, or macOS.

## Build the Artifact Receipt

1. Record every immutable input and implementation identity.
2. Record exact changed-part before and after digests.
3. Attach structural, semantic, visual, accessibility, malware-policy, and canary results.
4. Name every required platform and attach the exact visual report for each.
5. Record every known fidelity limit and whether it blocks completion.
6. Recompute the receipt and reject any mismatch.

Do not override a `blocked` or `failed` receipt manually. Supply the missing evidence or correct the
failed artifact, then rebuild the receipt from immutable inputs.

## Current Local Boundary

The current implementation can build, edit, inspect, compare caller-supplied page pixels, and emit
truthful receipts without persistent, network, or execution authority. A Word renderer, controlled
writer, product coordinator, native interface, installed accessibility campaign, cross-platform
render run, independent review, and deferred manual fuzzing remain outside the current evidence.
