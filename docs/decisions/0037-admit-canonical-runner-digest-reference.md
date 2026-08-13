# Decision 0037: Admit the Canonical Runner Digest Reference

## Status

Accepted for Linux Docker topology evidence.

## Context

Live Fedora KVM execution proved that Docker retains the exact immutable launch
reference in container configuration as
`docker.io/docker/model-runner@sha256:<digest>`. The collector previously
compared that value only with the bare `sha256:<digest>` and therefore
misclassified the canonical repository-at-digest form as a mutable tag. The
Docker preflight correctly refused the resulting image-identity observation.

## Decision

The production collector admits exactly two immutable configuration forms: the
bare pinned digest and the pinned digest prefixed by the one declared canonical
repository, `docker.io/docker/model-runner@`. A tag, alternate repository,
partial digest, case change, or any other spelling remains mutable or unknown
and is refused.

The independent image inventory check is tightened at the same time: the image
must expose the canonical repository-at-digest value, not merely any repository
whose final digest text happens to match.

## Consequences

- The exact Docker launch command used by the compatibility profile is
  representable without weakening immutable image admission.
- Repository substitution is refused even when the digest suffix is copied.
- The pinned image digest, runtime profile, model identity, and release status
  do not change.
- Fedora and Ubuntu live KVM acceptance remains required before interoperability
  or release claims can be made.
