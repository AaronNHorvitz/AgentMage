# AgentMage v0.1 Release Documentation

## Status

No supported AgentMage v0.1 release exists. The files in this directory are versioned pre-release
procedures and disclosure material for Sprint 25. They do not convert package candidates, synthetic
signatures, local tests, or historical platform evidence into a production release.

Decision 0008 defines v1.0 GA as the first supported release. Sprint 25's v0.1 release language is a
pre-release validation milestone and cannot be used to imply supported GA status.

## Documents

- [`operator-guide.md`](operator-guide.md) covers installation status, first run, model management,
  diagnostics, permissions, evidence, repository maps, privacy, offline proof, recovery, and
  troubleshooting.
- [`maintainer-guide.md`](maintainer-guide.md) defines the clean-source, build, signing, platform,
  evidence, support, and publication sequence.
- [`capability-matrix.md`](capability-matrix.md) separates implemented local contracts from usable
  production paths, unverified behavior, and exclusions.
- [`release-notes-v0.1.0-draft.md`](release-notes-v0.1.0-draft.md) is a non-published release-note
  template with required limitations and evidence fields.

## Truth Rule

A command passing on one developer machine proves only the named command on its recorded revision
and environment. A release claim additionally requires signed artifacts, clean standard-user
installation, the complete supported workflow, offline evidence, accessibility evidence, recovery,
uninstall, and independent review on every declared release platform. Missing, skipped, ignored,
stale, unavailable, flaky, quarantined, suppressed, or unreviewed evidence blocks the claim.
