# Decision 0195: Dormant RSA commit signature assembly

| Field | Value |
| --- | --- |
| Status | Dormant implementation detail under the accepted owner mission |
| Date | 2026-11-10 |
| Authority | Local agent mission; T-303/T-306; Decisions 0192 and 0194 |
| Scope | Exact content, RSA/v4 packet assembly and mandatory verification |

Prepare the exact unsigned commit bytes from a kernel-verified plan. Preserve
every approved message byte, including missing final newline and carriage
returns. Refuse mixed Git object formats. Use the selected format's signature
header when inserting armor; no Git object framing is signed. Bind signature
metadata to the approved commit timestamp, refusing values outside the unsigned
32-bit signature-time field rather than truncating them.

The internal assembler supports only v4 RSA signatures with SHA-256. Form the
binary-document signature digest over the content, fixed signature metadata and
v4 trailer. Convert exactly one bounded canonical RSA signature expression from
the foreground agent to a signature packet and armor. Refuse unknown algorithms,
extra fields, ambiguous lengths, malformed integers and trailing bytes. Other
algorithms remain unsupported by this assembler; there is no direct fallback.
This is original wire-format code written from the public protocol specification,
not reference implementation code. The specification source is
[RFC 4880](https://www.rfc-editor.org/rfc/rfc4880.html), sections 3.2,
4.2.2.3, 5.2.3, 5.2.4 and 6.2-6.3, published by the IETF Trust in 2007.
The specification publishes its copyright terms under BCP 78. No specification
prose or reference code is incorporated, and no dependency is added.

The dormant composition checks the exact pinned signer/public export, supplies
the complete digest to the socket-free primitive, reconstructs and compares the
signed content, and requires the existing public-only offline verifier before
returning raw bytes. The issuer key ID is advisory only: full-fingerprint
verification determines identity. Cancellation, deadline, malformed output or
verification failure returns no commit. Cleanup uncertainty propagates intact.
One bounded deadline covers signing and verification. No object is written.

The existing direct signed-creation adapter stays dormant and unchanged. This
composition has no product caller, route, approval, commit or push authority.
Disposable primary/subkey cryptography is component evidence only. Native
sandbox/artifact admission, confined object write, complete approval and
preservation composition, durable recovery and model measurements remain open.
Neither T-303 nor T-306 closes, and no row or sub-row is added.
