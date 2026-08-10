# Story 0.3 Partial Evidence Summary

| Field | Value |
|---|---|
| Result source revision | `a89cc70e18043e88f1eb8fcc978d87cf70cd0093` |
| Verification revision | `d5f2f08b5cd9a591f3d0be5d2e9fce0ff5619a48` |
| Evidence date | 2026-08-10 |
| Native adapter | `linux-native-vulkan` |
| Native result | `FAIL` |
| Corpus cases | 9 passed, 3 failed |
| Global thresholds | 12 passed, 3 failed |
| Docker adapter | `BLOCKED` - required runtime unavailable |
| macOS adapter | `BLOCKED` - required hardware unavailable |
| Profile decision | `PENDING` |
| Independent review | Not performed |
| Release approval | No |

The native run completed all 72 fixed trials. It failed repository citation precision, evidence citation recall, and exact tool-argument validity. Structured chat, unavailable-write refusal, malformed-output blocking, cancellation, context handling, performance, memory, and isolated zero-egress cases passed.

Key native measurements were 163.12 minimum generated tokens per second, 0.689 seconds maximum time to first token, 0.057 seconds maximum post-cancel quiescence, 0.247 maximum GPU-memory fraction, and 0 post-install egress bytes.

The raw JSON result is authoritative over this summary. The native failure is not a final E4B profile decision because the required Docker and MacBook Pro M5 adapter runs remain unavailable. No fallback, profile activation, support claim, or release approval is authorized by this bundle.
