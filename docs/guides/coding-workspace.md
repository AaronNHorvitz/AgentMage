# Read-only coding workspace tools

The [local API](local-chat-api.md) can read and search explicitly selected source
files, then feed those files to the existing [proposal API](coding-proposals.md).
It uses the native Linux path adapter. Source consent itself authorizes reads
only. [Reviewed workspace edits](coding-writes.md) provide a separate exact
preview/approval path for writes. Commands, worktree creation, test execution,
retries, commit and push remain open, along with parent T-303 and T-306.

## Selection and consent

The [terminal client](coding-terminal.md) provides selection, read/search,
streaming proposal and exact typed write approval commands.

Only a trusted person-facing client with the **control** credential can select
or revoke the scope. Model output and ordinary API clients cannot select a root,
add a file or authorize disclosure. After displaying the root, exact paths and
whether source may be sent to the configured endpoint, send:

```text
PUT /v1/code/workspace
```

```json
{
  "root": "<absolute-selected-directory>",
  "paths": ["src/calc.py"],
  "approve_read": true,
  "approve_model_context": false
}
```

All fields are required. False read consent refuses selection. Model-disclosure
consent requires an already chosen endpoint and binds to its settings revision.
Optional `settings_revision` and `settings_endpoint` fields bind selection to
the settings displayed by a trusted control. A mismatch returns `409 code.workspace.settings-stale` and
preserves the existing selection and pending preview. The terminal client uses
both fields for model-disclosure consent, including the exact endpoint so that
a service restart cannot reuse a revision for a different destination. Omission preserves the existing API
contract; other controls remain responsible for displaying their chosen endpoint.

Optional `expected_root_sha256` binds a selection to a preceding file map's
canonical root path/device/inode digest. The API verifies it before source
selection and against the selected native scope. A mismatch returns
`409 code.workspace.root-stale`, preserving the old grant and preview.
This field narrows selection; it confers no read, disclosure or write authority.
Omitting it preserves the original explicit-selection contract.

Every endpoint setting change, including choosing the same address again or
revoking settings, invalidates that disclosure consent. Reads remain authorized;
select again with explicit model consent to send source after a settings change.

A successful selection returns a monotonically increasing `workspace_revision`
and `authority: "read-only"`. `GET /v1/code/workspace` returns that revision,
selected relative paths and whether current model consent exists. It returns no
absolute root. `DELETE /v1/code/workspace`, with the control credential, drops
the grant and advances the revision. A failed selection preserves the old grant.
Old revisions are refused; a request at the current revision with no selection
gets `code.workspace.choice-required`. Grants/settings cannot change while an
operation is active. Cancel the operation, wait for its terminal event, then poll
`GET /v1/status` until `busy: false` before revoking or starting another
operation. A response can finish just before operation cleanup releases the
lease; `409 api.busy` means the new operation did not start.

This consent is an exact **path** allowlist, not a content freeze: a person can
change a selected source and the next read observes its current bytes. The root
directory's descriptor and identity are held throughout the grant. Renaming it
does not transfer authority to a replacement directory at its former pathname.
No grant survives process restart. These are explicit read scopes for this API,
not production platform activation or kernel write/command grants. Existing
production authority, trust and sandbox entry points remain unchanged.

## Native path boundary and budgets

Choose one to sixteen relative paths. They use ASCII letters, digits, `.`, `_`,
`-` and `/`, at most 256 bytes each. Kernel canonical path validation also
refuses empty/traversal components, encoded separators and ambiguous suffixes.
Git metadata components and case-colliding selections are refused. The absolute
root is at most 4,096 bytes and 256 components; symlink ancestors are refused.

Every file is resolved afresh beneath the held root through the native
`LinuxPathAdapter`, retaining its no-symlink, no-magic-link, no-mount-crossing,
hardlink, special-file, exact preimage and identity checks. Whole-file reads
revalidate together and re-resolve their paths before returning. Selection first
validates all files. A later boundary failure refuses the whole operation.

Files must be nonempty UTF-8 without NUL, at most 128 KiB each and in aggregate.
The scope never recursively walks a directory, follows repository instructions,
loads history or reads files outside the selected list. Read subsets must be
nonempty, unique and exactly match selected names. Errors contain fixed codes,
never an absolute root, source text or OS diagnostic. File and HTTP budgets do
not establish a filesystem I/O deadline; a stalled filesystem can delay a native
read. This is not an arbitrary untrusted command sandbox.

## Metadata-only file map

The trusted control credential can request a one-shot scan:

```text
POST /v1/code/map
```

```json
{"root":"<absolute-selected-directory>","query":"calc tests","approve_discovery":true}
```

False consent or the ordinary client credential refuses. Unknown/duplicate
fields and positional arrays refuse. This separate metadata permission opens
directories and inspects object metadata through the native path adapter; it
reads no file content and invokes no command, Git, model or instruction file.
It does not change any existing source scope or pending preview. Its
`selected: false` and `model_context_approved: false` describe the map's lack of
authority, not the state of an existing scope. Requests are visible in activity.

The response contains `root_sha256`, all eligible `{path, score}` entries and
counts of directories, names, exclusions and unsafe entries. Ranking is
deterministic: split the query into distinct lowercase ASCII alphanumeric
terms; each term contributes one point for a path substring match and two
more for a basename match. Sort by descending score, then ascending path.
Zero scores remain in the map. Only the terminal client's `map-select` proposes
the first sixteen positive-score entries, subject to fresh read/disclosure
consent. File contents, binary validity and source relevance are not inferred
from names. Existing exact reads and context excerpt selection run afterward.

Hidden names and names `target`, `build`, `dist`, `node_modules`, `venv` and
`__pycache__` are excluded without opening, case-insensitively for the listed
names. Links, multiply-linked files, special objects and non-selectable ASCII
paths are omitted. This is a directory view, not Git status, tracked-file
inventory, Git-ignore handling, or a secret classifier. Review paths before
sharing source; visible names can still identify sensitive files.

Limits are fixed: query 1,024 bytes and at most 64 distinct searchable terms;
256 directories, depth sixteen, 8,192 observed names, 4,096 eligible files;
4,096 names and 256 KiB of names per directory. Paths use the same canonical
256-byte ASCII selection format. A cooperative two-second traversal deadline
and cancellation checks run between native operations; a blocked filesystem
call can delay either. Traversal or identity failures return a content-free
error, never a usable partial map. Use a smaller root if the scan exceeds its
budget. Enumeration rechecks traversed directory identities and the root
pathname before publication. It is not an atomic filesystem snapshot: changes
after mapping require fresh native validation during exact source selection.

## Read, search and propose

An ordinary client credential may request:

```text
POST /v1/code/workspace/read
```

```json
{"workspace_revision": 1, "paths": ["src/calc.py"]}
```

The response contains complete selected `files` with relative `path` and
`content`, plus the revision and read-only authority. For a bounded literal,
case-sensitive search over a selected subset:

```text
POST /v1/code/workspace/search
```

```json
{"workspace_revision": 1, "paths": ["src/calc.py"], "query": "return"}
```

The query is nonempty, at most 128 UTF-8 bytes, without NUL. Results have relative
`path`, `start`/`end` UTF-8 byte offsets and exact matched `text`. Matches do not
overlap. At most 128 matches are returned with an explicit `truncated` boolean;
there is no regex or command interpretation. Offsets refer to that read's bytes,
not a persistent citation or a guarantee that the file remains unchanged.

With separate model-disclosure consent, request:

```text
POST /v1/code/workspace/proposals
```

```json
{
  "workspace_revision": 1,
  "paths": ["src/calc.py"],
  "model": "configured-model",
  "task": "Fix add so it adds the numbers.",
  "max_message_bytes": 6000,
  "max_tokens": 2048
}
```

The request cannot provide a root, file content, endpoint, command or authority
override. The server collects only the requested selected files and calls the
same bounded context and strict-edit machinery as snapshot proposals. Only
relative names and source enter model context. The NDJSON context/proposal
events include `workspace_revision`. Proposal results remain `applied: false`,
`tested: false` and `authority: "proposal-only"`; they cannot authorize any write.
The existing cancellation, answer cap and transport limits still apply.

Unknown/duplicate fields, positional arrays and stale revisions are refused.
All routes retain bearer, numeric Host, browser-origin and body-limit checks.
Workspace operations share the single active-operation budget; status and
cancellation stay available. The latest 128 activity events record action,
outcome and workspace/settings revisions, without root, source, model answer or
credentials. They have process-only retention; durable runs/replay remain open.
