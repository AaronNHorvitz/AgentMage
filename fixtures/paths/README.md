# Canonical Path Corpus

`v1/corpus.json` is a deterministic synthetic corpus for the shared
`WorkspacePath` constructor and deserializer. It contains no real usernames,
workspace content, or ambient paths. `v1/manifest.json` binds the corpus digest,
class distribution, and expected results.

`v1/display-link-corpus.json` contains synthetic `file:///` links and rendered
line targets. The executable matrix feeds both forms to every current path
authority boundary and requires rejection before grant issuance or filesystem
observation.

`v1/logical-input.txt` is the exact content fixture used by the deterministic
fake, Fedora, and Ubuntu adapter-conformance tests. It contains no private data.

Case-collision candidates are accepted as distinct components by the shared
contract and case-sensitive Fedora filesystem. They exist to prove that the
shared layer does not silently fold their spelling. They are not evidence for
macOS case-collision handling; that platform work remains blocked.
