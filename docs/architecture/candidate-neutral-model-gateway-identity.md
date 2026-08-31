# Candidate-Neutral Model Gateway Identity

Gateway admission treats a familiar model name and an API compatibility label as insufficient.
One disabled candidate independently binds the exact model profile, manifest and artifact revision,
runtime adapter contract and build, protocol codec version and implementation, canonical endpoint
record, candidate route policy, reviewed operator registry entry, optional brokered credential
reference, and exact-tuple qualification evidence. Every identity has its own version and/or digest;
the whole record has a separate digest.

All four deployment classes—strict local, private local network, private remote, and managed
remote—use the same closed candidate record. Local classes reject credential references. Remote
classes require an exact broker reference matching the endpoint record; no secret bytes are
accepted. Unknown operators, cross-class credentials, missing remote fields, nested substitutions,
activation, and automatic fallback fail before a route exists.

Successful identity admission is still blocked: it selects no route, enables no fallback, and
carries no inference, credential, tool, grant, policy, disclosure, or completion authority. Model
proposals are sealed into the kernel's descriptive-artifact family, whose only authority result is
denial. Route activation and fallback remain separate Story 13.6 decisions.

The closed wire/evidence shape is
[`disabled-gateway-candidate.schema.json`](../../schemas/model/disabled-gateway-candidate.schema.json).
Any field or implementation change creates a different tuple and requires new qualification; no
family-wide or compatibility-label evidence is borrowed.
