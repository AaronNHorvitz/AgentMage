# Canonical Path Corpus

`v1/corpus.json` is a deterministic synthetic corpus for the shared
`WorkspacePath` constructor and deserializer. It contains no real usernames,
workspace content, or ambient paths. `v1/manifest.json` binds the corpus digest,
class distribution, and expected results.

Case-collision candidates are accepted as distinct components by the shared
contract and case-sensitive Fedora filesystem. They exist to prove that the
shared layer does not silently fold their spelling. They are not evidence for
macOS case-collision handling; that platform work remains blocked.
