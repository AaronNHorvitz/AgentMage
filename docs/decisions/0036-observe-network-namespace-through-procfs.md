# Decision 0036: Observe Network Namespaces Through Procfs

## Status

Accepted for Linux Docker topology evidence.

## Context

Live Fedora KVM execution proved that moving the collector thread into the
runner network namespace updates namespace-local `/proc/net/*` records, while
the existing `/sys/class/net` mount can continue to expose the host namespace's
interface set. Combining those two views produced an impossible observation:
the private namespace had only loopback routes and sockets but three active
interfaces. The Docker preflight correctly refused that mixed observation.

## Decision

The production observer derives the complete conservative interface inventory
and transmitted-byte counters from namespace-local `/proc/net/dev`. Every
listed interface is counted; the observer does not treat an uncertain interface
as inactive. The required loopback state is proven independently by the exact
IPv6 host-scope loopback address in `/proc/net/if_inet6`, which is necessary for
the pinned dual-stack wildcard listener and IPv4 loopback guard connection.

Malformed interface, IPv6, route, or listener records fail closed. An extra
interface, a missing IPv6 loopback record, any non-loopback route or listener,
or any outbound bytes still prevents Docker-mode admission.

## Consequences

- One observation no longer combines network-namespace procfs records with a
  potentially host-bound sysfs mount.
- A down or otherwise ambiguous extra interface is conservatively counted and
  refused rather than omitted.
- The runtime profile, guard profile, endpoint, image, model, and resource
  identities do not change.
- Fedora and Ubuntu live KVM acceptance must pass before this correction can be
  represented as interoperability or release evidence.
