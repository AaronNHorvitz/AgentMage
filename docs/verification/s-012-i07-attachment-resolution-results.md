# S-012-I07 Attachment Resolution Results

**Status:** Pass for bounded attachment metadata and path resolution

**Task:** `12.1.1.7` / legacy `S-012-I07`

**Scope:** Captured provenance, canonical workspace paths, independent preimage
verification, and parser deferral

## Result

The kernel now resolves one metadata-only attachment reference through the
existing platform path-adapter contract. Resolution requires one captured
attachment identity, exact size and content digest, the active workspace,
matching authorized handle and adapter, `ContentHash` intent, a regular file,
and an independently returned preimage that still matches the capture. The
held object is dropped and only content-free identities, size, digest, format,
and parser disposition remain.

Focused cases closed: **6 of 6**.

| Case | Boundary | Verified result |
|---|---|---|
| `ATT-01` | Exact resolution | One captured canonical path independently resolves and revalidates its complete preimage. |
| `ATT-02` | Format closure | All 12 formats receive exactly one raw-text, deferred-release-pack, or unsupported disposition without parser execution. |
| `ATT-03` | Provenance | Malformed, absent, changed-size, or changed-digest metadata fails before adapter contact. |
| `ATT-04` | Affinity | Foreign workspace handles and adapter identities fail before resolution. |
| `ATT-05` | Revalidation | Adapter refusal, missing preimage, changed size, changed digest, or returned-object drift fails closed. |
| `ATT-06` | Authority | Attachment metadata and resolved records remain descriptive and are always denied as authority. |

## Parser Boundary

- Plain text, Markdown, JSON, and logs are only marked eligible for a future
  bounded raw UTF-8 read. No UTF-8 decoder or structural parser runs here.
- PDF, Word-processing, spreadsheet, presentation, image, archive, and
  notebook parsing is explicitly deferred to later release packs.
- Unknown formats are explicitly unsupported.
- Format classification never changes workspace, path, grant, or operation
  authority.

## Limits

- The focused suite uses a deterministic fake adapter and synthetic preimages;
  it does not claim live file, Linux, Windows, or macOS execution.
- The boundary resolves and hashes metadata only. It does not retain file
  bytes, extract text, parse structure, scan malware, or render content.
- Shell-side attachment discovery, format detection, content reading, parsing,
  persistence, checkpoint integration, and UI rendering remain later work.
- No private user data, model, external network, publication, or release path
  is used.
- Cross-platform execution, packaging, release acceptance, and manual fuzzing
  remain later gates.
