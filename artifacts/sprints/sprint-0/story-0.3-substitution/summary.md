# Story 0.3 Model Substitution Evidence Summary

| Field | Value |
|---|---:|
| Profiles | 2 |
| One-field substitution scenarios | 16 |
| Passed scenarios | 16 |
| Inference invocations | 0 |
| Result | `PASS` |

Each E4B and Gemma 4 12B Unified admission record was changed in exactly one field for license, lineage, tokenizer, context, GGUF, model OCI digest, runtime OCI digest, and native runtime build. Every altered record produced a visible `MODEL-IDENTITY-SUBSTITUTION` quarantine receipt during identity validation, before its admission decision was evaluated and before any inference process could start.

The raw JSON report and source hashes are authoritative. These negative tests prove substitution refusal for the listed static admission inputs; they do not approve either rejected model, substitute for the unavailable MacBook Pro M5 corpus run, or establish runtime reachability isolation.
