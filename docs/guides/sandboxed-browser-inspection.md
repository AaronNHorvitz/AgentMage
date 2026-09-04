# Sandboxed Browser Inspection

Browser inspection is a visible, single-action contract for navigate, inspect, find, click,
screenshot, and download. Each action binds one session, a sorted domain allowlist, an exact
single-use grant digest, response and download ceilings, and a content-free preview. Downloads
must enter quarantine; inspection and screenshot output must be redacted.

Public ephemeral and authenticated brokered profiles are distinct. A public profile carries no
credential reference. An authenticated profile carries only a digest of its brokered identity;
cookies, tokens, and credential values never enter this contract, model context, or logs.

The contract opens no browser, performs no network action, and retains no result content. A native
sandbox adapter must enforce process containment, per-action grants, redaction, quarantine, owned
descendant termination, action trails, and cleanup before its result can verify. Live native and
privacy-review evidence remain required before support is claimed.
