# Desktop Status, Recovery, and Packaging Guide

Sprint 77 defines source-level desktop lifecycle behavior. It does not provide an installable,
signed, accessible, or supported desktop package.

The status projection displays exact digests for model, runtime, context, memory, plan, tools,
resources, and audit state together with the offline flag. It is display-only and cannot change
the underlying state.

One active writer lease is admitted for a conversation. A conflicting lease fails closed. After
forced termination, only a clean complete checkpoint with a reacquired writer lock may resume the
ordinary projection. An uncertain checkpoint, incomplete receipt relationship, unclean shutdown,
or unavailable lock enters read-only safe mode. Recovery never claims that canonical state changed
and never retries an effect.

The local package contract requires fonts, icons, themes, help, and update metadata under relative
local paths. Telemetry, advertisements, remote assets, automatic cloud checks, and startup-network
requirements are prohibited. The contract deliberately refuses to claim that a package is signed
or installation-tested.

Desktop message, event-cursor, cancellation, approval-response, deep-link, and file-picker
projections must forward the bound request through the shared kernel protocol. Direct file, model,
key, or tool access, client-granted authority, and client-applied effects fail closed.

Native package installation, protocol parity, crash injection, visual and accessibility testing,
offline operation, signing, and independent review remain blocked until untouched platform
artifacts are available.
