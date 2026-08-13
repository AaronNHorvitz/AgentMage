# Decision 0038: Bind the Cross-UID Guard Peer Through the Challenge

## Status

Accepted for the Linux Docker guard.

## Context

Live Fedora reachability evidence proved that the dedicated non-root guard
cannot open `/proc/<runtime-pid>/exe` for a process owned by the distinct
ordinary runtime user under the platform's process-access policy. Granting the
guard tracing capability or collapsing both processes onto one UID would weaken
the declared isolation boundary.

The guard can still obtain kernel-reported Unix peer credentials and can read
the peer's process start time and cgroup record. The separately authorized
launcher already supplies the exact executable digest and a fresh one-session
secret to the held runtime peer and guard through separate inherited channels.

## Decision

Before issuing a challenge, the guard verifies the kernel-reported peer UID and
PID plus the held process start time and cgroup digest against the launcher
bootstrap. The guard then uses the launcher-supplied executable digest in the
challenge transcript. The peer's response remains a SHA-256 transcript over the
fresh challenge, one-session secret, UID, PID, start time, executable digest,
and cgroup digest.

The executable identity is therefore challenge-bound rather than re-opened by
the different-UID guard. A caller with the wrong UID, PID, start time, or cgroup
is refused before receiving a challenge. A caller without the separately
delivered one-session secret and exact transcript cannot authenticate. The
secret remains one use and is not accepted from a path, environment variable,
argument, or serialized permit in the product contract.

## Consequences

- The guard retains zero effective capabilities and a distinct UID.
- Fedora's process-access protections remain enabled rather than bypassed.
- Executable substitution changes the challenge response and is refused; the
  root-authorized launcher and live topology collector still independently bind
  the held executable bytes.
- Reachability evidence may use a private evidence-only secret file inside a
  disposable guest, but must remove it and must not represent that delivery
  mechanism as the product launcher contract.
