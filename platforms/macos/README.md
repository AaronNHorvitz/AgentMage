# macOS Platform Adapter

This Swift module owns the retained Apple Silicon App Sandbox, XPC, Keychain,
security-scoped bookmark, code-identity, and native IPC boundaries.

The kernel-host source admission boundary now validates, in fixed order, arm64,
macOS 15 or newer, the running code signature, Hardened Runtime metadata, exact
bundle and Team identity, App Sandbox, one App Group, app-scoped bookmarks,
read-only user-selected files, one Keychain access group, and a closed entitlement
set. It returns only a content-free refusal or an opaque verified identity. The
contract entitlement file uses the repository's visibly synthetic `com.example`
and `AAAAAAAAAA` values; it is not a signing or release input.

Native compilation, signing, execution, and support remain `BLOCKED-MACOS` until
the manual Apple Silicon source check and the required physical-M5 evidence exist.
