# Common Artifact Receipts and Audio Observations

Sprint 67 adds one authority-free receipt builder for converter, parser, generator, renderer,
redactor, verifier, and transcriber observations. The builder consumes exact caller-supplied
identities and observations, validates canonical ordering and resource ceilings, and returns a
self-digested record. It never discovers, opens, writes, renders, executes, or transmits an artifact.

Every receipt binds the operation and implementation identities, exact implementation and profile
hashes, ordered input/output references, fidelity disposition, limitations, unsupported features,
source invariance, cleanup, and observed file/network/content-execution effects. Exact fidelity
cannot carry limitations; limited and blocked dispositions require at least one limitation. The
closed runtime schema uses the same seven operation classes and denies all three effect flags in the
locally admitted record.

The audio surface validates an observation supplied by a separately admitted local engine adapter.
Segments retain exact half-open millisecond ranges, ordered indexes, text confidence, unclear-language
state, and optional speaker labels. An inferred speaker requires a confidence and is never represented
as observed fact. The observation also binds source audio, codec, duration, engine artifact, profile,
and original-audio retention disposition. Validation performs no transcription, file effect, or
network access, and its receipt remains `limited` until a labeled accuracy campaign exists.

No transcription engine, model, codec package, audio fixture, platform, or format is admitted by this
contract. Connecting one requires an exact separately approved local runtime and retained normal,
empty, boundary, malformed, oversized, unsupported-codec, mixed-encoding, attack, accuracy,
segmentation, timestamp, speaker, uncertainty, and retention results. Until then transcription and
the every-type integration rows remain blocked.
