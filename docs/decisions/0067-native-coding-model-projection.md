# Decision 0067: Native Coding Model Projection

| Field | Value |
|---|---|
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-23 |
| Authority | Decision 0054 and the owner's explicit real-model integration assignment |
| Scope | Family codecs and model-visible contracts for Tasks 48.2.4-48.2.6 and 50.2.4 |
| Preserves | Exact model admission, context/resource limits, native schema validation, grants, confinement, verifier ownership, canonical stores and independent review |

## Evidence

The retained Muse response selected an existing validation tool in an ATEM
recipient frame. Its final-only decoder refused it before any effect. The
GPT-OSS response reasoned about missing edit syntax and cryptographic envelope
construction before emitting malformed final output. Raw `/completion` uses the
family codec's rendered prompt, not llama.cpp's chat-route Jinja rendering.
The model-facing patch schema omitted the native edit variants, and observations
represented readable JSON as decimal byte arrays inside escaped strings.

These are integration defects, not sufficient evidence of model incapability or
an outside-owner blocker. Failed payloads remain evidence and regression inputs.

## Decision

1. Development codecs explicitly translate their native ATEM/Harmony tool frames
   to the existing proposal contract. Only frozen native tool identities are
   representable. Trusted code binds schema, version and envelope digests; the
   model supplies arguments, never grants or authority. Reasoning is not a tool
   call or completion candidate.
2. Native JSON may use ordinary whitespace and object-key order. A recursive
   duplicate-rejecting parser canonicalizes it before the unchanged closed
   argument validation and dispatch. Unknown fields are preserved for rejection;
   malformed syntax, extra frames and unknown recipients are not repaired.
   Muse also uses the pinned template's ATEM invoke/parameter delimiter grammar:
   raw strings keep their whitespace and compound values are strict JSON. This
   is not a general XML parser and performs no entity expansion. GPT-OSS keeps
   analysis, commentary recipients and JSON-format metadata separate from final.
3. The model sees complete edit-variant schemas, the supplied objective digest,
   and verified readable tool observations. Stored contracts remain byte-exact.
   Final JSON is bound to the verifier's completion-candidate schema, not treated
   as proof of success. The existing verifier owns all completion checks.
4. Explicit session-recording consent permits private rendered-prompt and raw
   model-output diagnostics through the existing Linux development-retention
   owner. Records are create-new, owner-only and hash-bound. Rejected output is
   retained separately by disposition and never promoted to trusted evidence.
5. Exact codec identities bind the family source plus its shared JSON parser.
   No output/context limit, sampler, production activation or application
   permission is changed by this decision. Actual binaries and real models must
   independently demonstrate the coding campaign before qualification.
6. Muse native-4 exercised an additional real read and hit the unchanged 32K
   preflight limit before finalization. The host contract repeated identical read
   and artifact schemas for each operation. Factor those complete schemas into
   one digest-keyed map while retaining every frozen tool definition and schema
   reference. Regression reconstructs every original definition/schema exactly.
   This is lossless serialization, not context reduction, evidence omission,
   weaker binding or a raised capacity/output limit. Exact token preflight remains
   mandatory and failures report the input, output reserve and profile capacity.
7. Muse native-5 produced only a reasoning-channel context echo; GPT native-6
   emitted duplicate channel and JSON fields. Preserve both as negative raw
   regressions. The native message bodies now contain their actual content,
   rather than an extra message-envelope JSON object. Muse renders its pinned
   template's named function/description/parameter-schema block; the duplicate
   digest-keyed map is removed from that rendered body only after exact equality
   with those schemas is checked. The original context packets, all source
   bindings and stored artifacts remain unchanged. Harmony historical calls
   include the template's `commentary json` header. None of these presentation
   corrections accepts the malformed replies or claims to prove model quality.

## Verification boundary

Codec regression and scripted process success are not real-model qualification.
Retain every attempted live run and its exact source/profile/runtime identity.
Independent review and all downstream milestone gates remain open until their
own evidence is present.
