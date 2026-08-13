# Decision 0039: Verify Independent Docker Control Disablement

## Status

Accepted for Story 9.2 verification.

## Context

The production Docker preflight already rejects drift across seven ordered
control classes. Deterministic unit mutations prove the classification logic,
but Story 9.2 also requires Fedora and Ubuntu evidence that observable live
topology changes reach those terminal refusals without activating a weaker
adapter.

## Decision

Run one admitted positive control and seven independent negative cases in fresh
disposable KVM topology instances on each Linux target. Each negative case
changes exactly one declared control fact:

1. add the held runtime peer to the Docker socket group;
2. broaden the Docker socket mode;
3. introduce a management listener in the private network namespace;
4. broaden the guarded kernel socket mode;
5. substitute the declared model-manifest identity;
6. change the runner memory limit; or
7. introduce an ambient proxy setting.

The production collector and preflight must return the corresponding stable
content-free refusal. A case passes only when Docker is not admitted, no native
fallback is selected, no native listener appears, no inference occurs, and all
case-specific state is removed before the next case. The reusable guest starts
with restricted networking, uses only cached digest-pinned images and model
content, and is destroyed after the campaign.

## Consequences

- A passing baseline distinguishes refusal caused by each changed control from
  an unusable fixture.
- The test does not weaken production policy or add a degraded runtime mode.
- The seven cases prove live collector-to-preflight propagation within the
  declared KVM scope; they do not prove model quality, physical-host behavior,
  macOS parity, or release support.
- Independent review and product-security mapping remain separate gates.
