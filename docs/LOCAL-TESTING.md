# AgentMage local Linux testing

## Standalone coding workflow

The coding workflow uses the actual `agentmage` CLI, `agentmage-host`, native tools,
protected operation approvals and verifier in a disposable repository. Read the
[development guide](guides/standalone-coding-development.md) for the complete
interface and the [restart reconciliation](verification/lane-reconciliation-2026-09-27.md)
for current results and blockers. The document demonstration below is a separate workflow.

In the configured implementation lane, build through its shared reservation:

```sh
bash /tools/build-slot env CARGO_BUILD_JOBS=1 cargo build --locked --offline \
  -p agentmage-host -p agentmage-capability-read-only --bins
python3 -m scripts.coding_harness setup --root /lane/state/coding-demo-1
python3 -m scripts.coding_harness diagnose --root /lane/state/coding-demo-1
```

Use a fresh, short, private root for each campaign. These commands require the
pinned Rust toolchain and locked dependencies to be available. Outside this lane,
use the operator's configured build reservation and private scratch directory.
The doctor distinguishes executable trust and user-manager availability from
binary presence. `confinement_prerequisites.scope=prerequisites-only` is a
preflight observation; native Rust owners still verify confinement at dispatch.
`lifecycle=ready` means no recorded session is running, not platform admission.
The wrapper's schema 2 diagnosis names the fixture as `fixture_profile_id` and reports
`model_request` separately. A requested selection comes only from an exactly owned
process's arguments; absent or ambiguous observations remain unavailable. Serving and
qualification remain unobserved and unassessed, including when the lifecycle is running.

When the exact native prerequisites are available, the scripted fail/repair
workflow is:

```sh
bash /tools/build-slot python3 -m scripts.coding_harness start \
  --root /lane/state/coding-demo-1 --model scripted --scenario failed-test-repair \
  --objective 'Repair the failing synthetic add test and rerun validation.' \
  --approve-this-run --log-dir /lane/state/logs/coding-demo-1
python3 -m scripts.coding_harness status --root /lane/state/coding-demo-1
```

Omit `--approve-this-run` to inspect and answer each exact protected challenge.
Use `python3 -m scripts.coding_harness stop --root /lane/state/coding-demo-1`
from another terminal to request cancellation of that session. Preserve its
result and streams; a stop request alone does not prove cleanup.
Cancellation requires Linux process descriptors and an exact recorded process identity.
If diagnosis reports `reserved`, `stale-record` or an invalid legacy record, preserve that
root and use a fresh disposable root. The wrapper never adopts or automatically removes
an uncertain record. See the development guide for lifecycle and cleanup limitations.

**Current lane result:** setup and diagnosis ran; the actual scripted CLI/host
launch returned exit 5 before tools because native prerequisites are unavailable.
The [memory and extension revocation verification](verification/memory-and-extension-revocation-2026-10-01.md)
retains the latest source, rebuilt binary identities, the startup refusal, and the actual
catalog host's memory and extension operations, with its logs under
`artifacts/verification-logs/`; the
[review-fix and documentation pack verification](verification/review-fixes-and-documentation-packs-2026-10-01.md)
retains the catalog host's documentation pack operations and ended-run read; the
[durable run action history verification](verification/durable-run-action-histories-2026-09-30.md)
retains an actual support bundle export and the earlier refused ended-run read; the
[review-fix, local-only route and support bundle verification](verification/review-fixes-local-only-routes-support-bundles-2026-09-30.md),
the
[review-fix and run action history verification](verification/review-fixes-and-run-action-history-2026-09-30.md),
the
[review-fix, offline documentation, language server codec and recipe verification](verification/review-fixes-and-amr-05-codec-packs-recipes-2026-09-30.md),
the [review-fix and AMR-05 component verification](verification/review-fixes-and-amr-05-components-2026-09-30.md),
the [review-fix and suspension verification](verification/review-fixes-and-suspension-2026-09-30.md),
the [review-fix and host job control verification](verification/review-fixes-and-job-control-2026-09-30.md),
the [review-fix and durable job ledger verification](verification/review-fixes-and-durable-job-ledgers-2026-09-30.md),
the [review-fix and run declaration verification](verification/review-fixes-and-run-declarations-2026-09-29.md),
the [review-fix and hunk selection verification](verification/review-fixes-and-hunk-selection-2026-09-29.md),
the [CLI hunk review and run progress verification](verification/cli-inspection-2026-09-29.md), the
[review-fix and runtime component verification](verification/review-fixes-and-runtime-components-2026-09-29.md),
the [coding recoverability verification](verification/coding-recoverability-2026-09-29.md),
the [selective hunk change verification](verification/selective-hunk-changes-2026-09-29.md),
the [research disclosure-binding verification](verification/research-disclosure-binding-2026-09-29.md),
the [preparation review-fix verification](verification/preparation-review-fixes-2026-09-29.md),
the [controlled preparation verification](verification/controlled-native-preparation-2026-09-28.md),
the [runtime phase deadline verification](verification/runtime-phase-deadlines-2026-09-28.md),
the [socket deadline verification](verification/model-socket-deadlines-2026-09-28.md),
the [failed-start verification](verification/failed-start-consumption-2026-09-28.md),
the [model request diagnostic verification](verification/harness-model-diagnostics-2026-09-28.md),
the [research completion verification](verification/research-completion-2026-09-28.md),
the [artifact preparation verification](verification/runtime-artifact-preparation-2026-09-27.md),
the [inference cleanup verification](verification/native-inference-cleanup-2026-09-27.md),
the [confirmation verification](verification/development-confirmation-2026-09-27.md),
the [development lifecycle verification](verification/development-host-lifecycle-2026-09-27.md),
the [native diagnostic verification](verification/native-development-diagnostics-2026-09-27.md) and
the [preceding observation](verification/linux-current-startup-2026-09-27.md)
keep their original source pins. The clean fixture was unchanged. A separate fixture with staged, unstaged and untracked work
was refused by the wrapper before launch, with its file bytes and Git state preserved.
The lane does not expose root-trusted native executables or a user-systemd bus.
The corrected doctor reports this refusal. Do not alter confinement to run the demo.
A successful edit/test/correction, denial/cancellation matrix and real-model demo
have not been reproduced in this lane. Historical Muse coding results and the
interrupted scripted run are documented separately in the reconciliation record.

Startup stderr preserves content-free native causes. In this lane,
`linux.repository.git_artifact.invalid` precedes `linux.development.launch-envelope.failed`:
the native Git executable was refused before the host could serve IPC. An envelope
failure alone does not establish an authentication failure. Keep the complete local
stderr and consult the prerequisite diagnosis.

| Startup code | Meaning and next step |
| --- | --- |
| `linux.development.host-executable.unsafe` | The exact sibling host is missing, aliased or unsafe. Rebuild the CLI and host together and inspect their local placement. |
| `linux.development.host-launch.failed` | The operating system could not spawn the sibling host. Retain the failure and verify the executable format and environment. |
| `linux.development.launch-envelope.failed` | No valid one-use launch envelope was received. Inspect any preceding host diagnostic. |
| `linux.development.launch-envelope.timed-out` | The original 120-second startup deadline expired. Retain the host diagnostics; fragments do not renew the deadline. |
| `linux.development.startup.cancelled` | SIGINT or SIGTERM cancelled startup; exit 6 follows verified direct-child reaping. This is not a native-tool cancellation result. |
| `linux.development.host-exit.timed-out` | The host missed its ten-second graceful exit deadline. Forced or late exit does not become success. |
| `linux.development.owner.unavailable` / `linux.development.cleanup.uncertain` | An existing or uncertain direct host owner prevents replacement within this calling process. Preserve the failure and inspect owned processes before retrying. |
| `linux.command.manifest.invalid` / `linux.sandbox.manifest.invalid` | Native tool composition rejected its exact prerequisites. Check native trust and confinement; do not bypass them. |

Startup stop handlers are installed before host spawn. Error and destructor cleanup
allow three seconds for directly owned child termination and reaping. These cooperative
bounds cannot preempt kernel-stalled syscalls, and direct-host cleanup does not prove
native/model descendants stopped. Actual CLI tests with a synthetic incomplete-envelope
host verified timeout, SIGINT and SIGTERM cleanup; they do not qualify native coding.

At a protected operation prompt, type `yes` and press Enter to confirm the displayed
operation. Session preauthorization requires `preauthorize` and Enter. Each answer
must fit within 256 UTF-8 bytes including its newline; EOF without a newline never
confirms. Invalid encoding, excessive input or descriptor errors fail closed.
Use the dedicated CLI as the sole stdin reader. Its bounded readiness polling and
optional approval delay observe the existing cancellation flag without changing
terminal settings or descriptor flags. Component checks show cancellation takes
precedence over sending an approval response; native prompt cancellation remains
unverified in this lane. See the confirmation verification for the cooperative
read limitation and separate results.

Real-model runs additionally require the exact admitted development profile and
its unchanged memory, CPU, GPU and confinement requirements. In this lane any GPU
command must be nested under `bash /tools/build-slot bash /tools/gpu-slot queue-run
agentmage SECONDS ...` with `SECONDS` at most 3600 and owned cleanup inside the
reservation. Current missing native prerequisites preclude such a run; do not
launch inference just to rediscover them. No model download or host installation
is part of these instructions.

The native inference driver arms its existing private lease before possible model
spawn. Only that live owner clears the reservation after verified process and file
cleanup. An uncertain or abandoned reservation refuses later admission; it is not
a stale lock to delete. Preserve the state and obtain external reconciliation.
There is no automatic reset, and a reboot or recreated runtime directory does not
prove cleanup. CPU component checks cover bounded cleanup and preservation; actual
isolated model cleanup remains unverified in this lane. See the inference cleanup
verification for the exact source and remaining limitations.

### Running the coding demo on your own Linux host

The implementation lane cannot run the positive workflow: its user namespace does
not map real root, so `/usr/bin/git`, `bwrap`, `systemctl` and `systemd-run` appear
owned by an unmapped user and fail the root-owned executable check, and it has no
user systemd manager. A normal Fedora login session on the development machine has
both. These commands have not been run positively at the current revision; the
last positive scripted matrix is historical (see the native command-control record).

From the repository root, as your normal user, with the pinned toolchain available:

```sh
cargo build --locked --offline -p agentmage-host -p agentmage-capability-read-only --bins
demo="$XDG_RUNTIME_DIR/am-demo-1"   # fresh, short, private; must not exist yet
python3 -m scripts.coding_harness setup --root "$demo"
python3 -m scripts.coding_harness diagnose --root "$demo"
```

`diagnose` must report `confinement_prerequisites.ready: true` before a start. Then run
the failed-test repair scenario and answer each protected prompt yourself:

```sh
python3 -m scripts.coding_harness start --root "$demo" --model scripted \
  --scenario failed-test-repair \
  --objective 'Repair the failing synthetic add test and rerun validation.' \
  --log-dir "$demo-logs"
```

With `--log-dir`, the CLI's standard output and standard error, which carry every
prompt, go to `stdout.jsonl` and `stderr.log` in that directory. Omit `--log-dir` to read
and answer the prompts on the terminal.

- Apply: type `yes` and Enter at each prompt; the run should show the failing test,
  the exact patch, the rerun and a verified terminal report. Below each write prompt the
  CLI prints a hunk review of the change the exact arguments make to the current file
  (Decision 0112). It says so when the file changed or the change is not reviewable.
  Approving still allows the whole call. After each run, standard error shows the run's
  progress: its terminal state, whether the runtime's verifier accepted it, counts
  against the declared ceilings, and that independent review and delivery are not
  established by the runtime. It then shows the host's declarations about that run
  (Decision 0116): which of the run's effects a fresh approved inverse write could
  restore, which need your reconciliation, and a content-free view of each context
  the model received. A part the host cannot declare completely says so. It also
  shows the run's two action histories (Decision 0127): one line for each call whose
  grant was used, and for each call you refused, with its outcome, grant or decision
  and reason; and one line for each job control request the host decided. It then
  shows the run's model route (Decision 0128): the host routes every model request of
  the run in local-only mode, so the line names the one selected `strict_local` route,
  the data classes sent to it and the router's receipt digest, followed by the route's
  own one-entry history. No remote route is offered, and hybrid routing is still open
  (AMR-05.9.7). Last, it shows the run's job state from the host's durable job ledger,
  and the host's answer to each cancellation request (Decision 0120).
- Select hunks: when a patch review lists two or more numbered hunks, answer
  `select` followed by hunk numbers, for example `select 2`, instead of `yes`. The
  whole call is refused, and the next prompt asks separately for a write of only the
  selected hunks (Decision 0114); its review shows only those hunks. The scripted
  failed-test repair patch has a single hunk, so it never offers this. The
  actual-process proof on a native host is still open (AMR-04.2.3).
- Deny: in a fresh root, answer anything other than `yes`; no file may change.
- Cancel: in a fresh root, run `python3 -m scripts.coding_harness stop --root "$demo"`
  from a second terminal while a prompt or command is active. The CLI sends the
  cancellation to the host as a job control request that names the job revision it
  observed; standard error then shows the host's decision and the job ending as
  cancelled, or as completed if the work finished first.
- Pause and resume: while a prompt is displayed, run
  `python3 -m scripts.coding_harness pause --root "$demo"` from a second terminal,
  then answer the prompt. The run stops at the next safe boundary after that step
  (Decision 0122); standard error shows the suspension and the checkpoint it stopped
  at, and nothing runs until you run `python3 -m scripts.coding_harness resume --root
  "$demo"` (or `stop` to cancel). The host then continues the same run from that
  checkpoint through a new composition, and the job state after the run lists the
  suspension and resumption answers. A run that finishes before a safe boundary is
  never suspended. Closing the CLI while the run is suspended leaves the job
  suspended; reconnecting to it is still open (AMR-04.6.3).
- Preservation: add a staged, unstaged or untracked file in `$demo/disposable/worktree`
  before `start`; the wrapper must refuse and leave every byte and the Git index unchanged.
- Export: add `--action-history-export effects:1:3` (or `job-control:1:1`, or
  `routes:1:1`) to `start`. After each run the stream log gets a redacted export of
  that range of the run's history, written before the outcome row. The export includes
  its digest and exact document text, and nothing is written elsewhere. A range the run
  does not have prints a notice with the reason `range` on standard error instead
  (`reason=range` in text, `"reason": "range"` in JSON).
- Ended run: the host also keeps each run's three histories in its encrypted
  operational store, closed when the run is released (Decision 0129). After the host
  has ended, run `python3 -m scripts.coding_harness ended-run --root "$demo" --run
  RUN_ID`, where `RUN_ID` is the `run_id` of the outcome row. The CLI launches the
  catalog host (Decision 0130), which composes no run and needs no native Git. It
  reads the stored chains back, and the CLI prints each one with whether it was
  closed and whether it is complete. Add `--action-history-export effects:1:3` for a
  redacted export of a range. A chain whose host ended before the run did, or that was
  resumed after a restart, is shown as never closed or incomplete, never as a complete
  record. A run recorded before schema 22 has no stored chain; when it is resumed
  after a restart, its chains begin there, marked incomplete.
- Support bundle: create a private directory (`mkdir -m 700 "$demo-bundles"`) and add
  `--support-bundle "$demo-bundles"` to `start` (Decision 0128). After the invocation
  ends, whether it succeeded, failed or could not start, standard error shows a preview
  of a content-free support bundle (its payload digest, byte count, field families,
  redactions and confirmation digest) and asks you to type `yes`. Only `yes` writes the
  one file `agentmage-support-bundle-<preview id>.json` into that directory; anything
  else, or end of input, writes nothing. The bundle holds a doctor report of what this
  CLI invocation observed, component versions and the digests the host declared; every
  component the CLI did not observe is reported missing. Nothing is uploaded, and
  `--approve-this-run` does not answer this question. A cancelled invocation asks
  nothing. Omit `--log-dir` to see the preview and question on the terminal.

### Documentation packs through the catalog host

Documentation packs work in the implementation lane too, because the catalog host
(Decision 0130) builds no repository composition and needs no native Git, `bwrap` or
user manager. It validates the same disposable activation, serves the authenticated
session of the CLI that launched it, keeps the packs in its encrypted operational
store and ends with the CLI. Nothing is downloaded: a pack is a directory of Markdown
or plain text files with a sealed `manifest.json` that you already hold. To try it
with a small synthetic pack (its text was written for this project):

```sh
python3 -m scripts.coding_harness setup --root "$demo"      # if not already set up
python3 -m scripts.coding_harness doc-pack-sample --directory "$demo-pack"
python3 -m scripts.coding_harness doc-pack --root "$demo" \
  --import "$demo-pack" --allow-license LicenseRef-agentmage-sample
python3 -m scripts.coding_harness doc-pack --root "$demo" --list
python3 -m scripts.coding_harness doc-pack --root "$demo" --search 'remote cache'
python3 -m scripts.coding_harness doc-pack --root "$demo" --inspect agentmage-sample-guide
python3 -m scripts.coding_harness doc-pack --root "$demo" --delete agentmage-sample-guide
```

The wrapper passes each operation to the CLI's `--doc-pack-import`,
`--doc-pack-list`, `--doc-pack-inspect`, `--doc-pack-delete PACK[@VERSION]` and
`--doc-pack-search TERMS [--doc-pack PACK] [--include-history]`, and the CLI prints one
JSON row per result (the CLI alone prints text lines without `--json`).

- An import names every license you accept for it with `--allow-license`; any other
  license is refused with `doc-pack.license-not-allowed` (exit 4). The CLI reads the
  manifest and every listed file without following links, sends them to the host in
  chunks of at most 1 MiB, and keeps the host's receipt only when it names the manifest
  it sent and used no network.
- A refresh is an import of a newer version with `--refresh-version` naming the
  current one. The old version is then searched only with `--include-history`, and it
  is deleted 30 days after it was superseded; each operation applies that retention
  first and reports what it deleted.
- A search returns at most 20 cited fragments of current versions, each with its
  pack, version, path, lines and citation digest; its text is escaped for the terminal.
- A refusal prints one content-free code on standard error and exits with its class
  (2 for malformed input, 4 for a policy refusal or an unknown pack, 5 when the store
  or clock is unavailable, 7 at a bound).

**Current lane result:** the actual rebuilt CLI and catalog host imported, listed,
searched, inspected and deleted the sample pack, refused another license, and read
back an ended run; the
[review-fix and documentation pack verification](verification/review-fixes-and-documentation-packs-2026-10-01.md)
lists the exact results. The search is the
deterministic knowledge retrieval through the CLI; no coding run's model reads a pack
yet. Language server observations and recipe plans remain AMR-05.9.5.2 and
AMR-05.9.5.3.

### Memory through the catalog host

The catalog host also keeps one memory catalog in the same encrypted store
(Decision 0131). You state each memory yourself and cite a kept documentation pack
file as its evidence. Your invocation is the approval, and the existing memory policy
can still refuse it. With the sample pack imported as above:

```sh
python3 -m scripts.coding_harness memory --root "$demo" \
  --remember 'Clear the build cache when the toolchain changes.' \
  --workspace calculator --cite agentmage-sample-guide@1.0.0:guide/cache.md
python3 -m scripts.coding_harness memory --root "$demo" --list --workspace calculator
python3 -m scripts.coding_harness memory --root "$demo" \
  --revoke-source doc-pack:agentmage-sample-guide:1.0.0 --workspace calculator
python3 -m scripts.coding_harness memory --root "$demo" --revoke MEMORY_ID
python3 -m scripts.coding_harness memory --root "$demo" --delete MEMORY_ID
```

The wrapper passes each operation to the CLI's `--memory-remember TEXT` (with
`--memory-workspace`, `--memory-cite PACK@VERSION:PATH` and an optional
`--memory-type semantic|preference|procedural|episodic`), `--memory-list` (with an
optional `--memory-workspace`), `--memory-revoke-source SOURCE` (with
`--memory-workspace` and an optional `--memory-object`), `--memory-revoke ID` and
`--memory-delete ID`.

- An item's evidence names the source `doc-pack:PACK:VERSION`, the object `path:`
  followed by the file's path components joined by colons, and the file's digest. A
  citation of a file the catalog does not keep is refused with
  `memory.citation-not-found` (exit 4). A path with capitals, spaces or `..` is
  refused with `memory.citation-not-portable` (exit 2).
- Text that looks like a credential, or that names a home directory, is refused with
  `memory.candidate.prohibited` (exit 4), and nothing is stored.
- A source revocation reaches every revocable item of the one workspace you name that
  cites the source, or only the object you name. Items of other workspaces are never
  touched. A revoked item keeps its text for inspection and later deletion. A deleted
  item keeps only a tombstone.
- A listing shows each item's text escaped for the terminal. A refusal prints one
  content-free code on standard error and exits with its class (2, 4, 5 or 7 as for
  documentation packs).

**Current lane result:** the actual rebuilt CLI and catalog host remembered, listed,
revoked and deleted memory items in two workspaces; the
[memory and extension revocation verification](verification/memory-and-extension-revocation-2026-10-01.md)
lists the exact results. No coding run reads memory yet.

### Extensions and revocation lists through the catalog host

The catalog host also keeps one extension catalog in the same encrypted store
(Decision 0132). Extensions live in workspace scopes. Each scope holds the keys you
trust in it, the extensions you installed in it and the revocation list it accepted
last, and nothing crosses scopes. The harness copies a synthetic sample: trust
statements, three signed packages and five signed lists. The sample's keys come from
fixed, published seeds, so trust them only inside a disposable demonstration root:

```sh
python3 -m scripts.coding_harness extension-sample --directory "$demo-extensions"
ext() { python3 -m scripts.coding_harness extension --root "$demo" "$@"; }
ext --trust "$demo-extensions/signer-trust.json" --workspace calculator
ext --trust "$demo-extensions/issuer-trust.json" --workspace calculator
for package in sample-formatter sample-linter sample-report; do
  ext --install "$demo-extensions/packages/$package" \
    --allow-license LicenseRef-agentmage-sample --workspace calculator
done
ext --revocations "$demo-extensions/revocations/1.json" --workspace calculator
ext --list --workspace calculator
ext --revocations "$demo-extensions/revocations/2.json" --workspace calculator
ext --revocations "$demo-extensions/revocations/1.json" --workspace calculator
```

The wrapper passes each operation to the CLI's `--extension-trust FILE`,
`--extension-distrust KEY_SHA256`, `--extension-install DIRECTORY` with
`--extension-allow-license LICENSE`, `--extension-uninstall PACKAGE_ID`,
`--extension-revocations FILE` or `--extension-list`. Each takes
`--extension-workspace LABEL`, which is optional only for the list.

- The first list revokes `sample-formatter`. `sample-report` depends on it and goes
  inactive too, and `sample-linter` stays active. The second list also revokes the
  linter's exact manifest. Applying the first list again is then refused with
  `extension.revocations-stale` (exit 3), as is `2-fork.json`. `foreign.json` is
  refused with `extension.revocations-untrusted`, and `unsigned.json` with
  `extension.revocations-invalid`. The same list twice changes nothing.
- Each scope is separate. A second scope that trusts `foreign-issuer-trust.json`
  accepts `foreign.json`, which then revokes the linter only there.
- An installation checks the manifest seal, the source file's digest, the license you
  allow, a trusted signer key of the scope, each dependency at its exact version, the
  host's contract versions and the signature. It also checks the scope's accepted
  list. A revoked package is refused with `extension.revoked` (exit 4). Distrusting the
  signer of an installed extension, or the issuer of the accepted list, is refused with
  `extension.key-in-use`.
- Files are read from absolute paths only, without following links, as bounded
  regular files. A file that cannot be read is refused with `extension.file-invalid`
  (exit 2) before any host starts.

**Current lane result:** the actual rebuilt CLI and catalog host ran these steps in two
scopes, with the results listed in the
[memory and extension revocation verification](verification/memory-and-extension-revocation-2026-10-01.md).

An installed extension provides nothing to a coding run, and nothing it declares is
granted or run. The host keeps no package source. It trusts the source digest your
own authenticated CLI observed.

To run the whole declared scripted matrix in one step instead (edits, denial,
cancellation, pause and resume, stale approvals, rollback and the other cases, each in
a fresh root), use
the [acceptance runner](guides/standalone-coding-development.md) with short, new roots:

```sh
python3 scripts/coding_harness_acceptance.py \
  --work-root "$XDG_RUNTIME_DIR/am-matrix-1" --log-root "$XDG_RUNTIME_DIR/am-matrix-1-logs"
```

Its report hashes both binaries and every retained stream and stays
`executable-scripted-only`.

Inspect `git -C "$demo/disposable/worktree" diff`, `status` and the log directory after
each run. Keep failed and refused runs. A scripted run proves the executable path
only; it is not model qualification. Real-model runs additionally need the exact
admitted development profile and its resource and confinement checks.

## Separate document demonstration

The following historical Fedora Kinoite document demo requires its recorded native host
prerequisites and the current operator resource reservation. Its commands are not verified
in the implementation lane. This demo is separate from production preview/full-GA qualification.
It uses the real AgentMage Rust host and canonical knowledge retrieval/rendering,
with a protected localhost browser shell and a locally running Muse model.

## Launch and stop on this computer

On the separately qualified native host, from the repository:

```sh
python3 scripts/demo.py start
```

This builds the Rust document host, starts the backend at `127.0.0.1:8765`,
starts the exact verified local model and opens your default browser. Wait for
“Local model ready”. The printed URL includes a private access token in its
fragment; use the launcher to reopen it. An existing running instance is reused.
Do not share that token. To stop the application and its model:

```sh
python3 scripts/demo.py stop
```

`python3 scripts/demo.py status` prints the current access URL. For a terminal-only
launch use `python3 scripts/demo.py start --no-open`. For a different free loopback
port use `python3 scripts/demo.py start --port 8766` after stopping the instance.

## Five-minute walkthrough

1. Click **Use synthetic Aurora demo**. Check that the three text/Markdown sources
   are accepted and `unsupported.pdf` is skipped with its reason.
2. Ask **When does Aurora launch, and who leads it?** Expected facts: 18 October
   2026 and Mira Chen. Expand the citation to inspect `project.md` and source lines.
3. Ask **Who is responsible for the rehearsal, and when is it?** Expected facts:
   Theo Park, 15 October 2026 at 14:00, supported by `operations.txt`.
4. Ask **What is the project lead's favorite ice cream flavor?** The documents
   provide no evidence; the application should say so.
5. Ask a longer question and click **Cancel generation** while generating, then
   ask **What is the Aurora project budget?** Expected: 42,000 credits.
6. Use **New conversation** to reset bounded history. Model controls let you stop
   the model, observe a clear unavailable error, and start/retry it.
7. Stop and launch again with the commands above. Load the synthetic folder and
   repeat the first question.

To use your own documents, click **Browse…** or enter an absolute folder path.
The selected folder is the entire read authorization. Ingestion never writes to
source files. Citations describe immutable read-time snapshots; click **Read
folder** again to refresh changed sources. No home-directory crawl occurs.

## Model and setup

The existing artifacts on this machine are reused; launch downloads nothing.
Exact paths, hashes, runtime inventory and source revisions are in
[`demo/model.json`](../demo/model.json). Required user-space tools already present
here are Cargo/Rust 1.95, Python 3, Bubblewrap, the packaged Vulkan llama.cpp runtime
and NVIDIA Vulkan support. KDE `kdialog` supplies the optional folder picker;
enter a folder path if unavailable. No driver, firewall, VPN, SELinux or system
configuration changes are required.

- Model: first-party Meta Muse Glimmer 30B Q4_K_M, 16,756,683,904 bytes,
  SHA-256 `4cc57c0f51040a226e5a72cc47b7613f7772950e460a665f7083de89f183f60e`.
- Runtime: packaged llama.cpp b10423, commit
  `a94d563ed801d1da1b8c2432946de07d0231bb3d`, Vulkan; all GPU layers,
  flash attention, one slot, 8,192 context tokens, 2,048 maximum output tokens,
  temperature 0, seed 42. Structured local chat completions retain internal reasoning
  within the output budget; only final answer content is displayed. Startup checks model and complete runtime file/symlink hashes.
- Hardware tested: RTX 4090 with 24 GiB VRAM, 62 GiB system RAM.
- First-party model terms/provenance and historical evaluation causes are recorded
  in [`demo-model-probe.md`](verification/demo-model-probe.md). Historical rejected
  production Muse/Gemma profiles remain rejected; demo acceptance enables none
  of those profiles and makes no model-family or release-quality claim.

The system-global `llama-server` b10361 is too old for this Muse artifact; the
launcher deliberately uses the verified existing b10423 package.

## Supported inputs and limits

UTF-8 `.txt`, `.md`, and `.markdown` files are supported. Other formats, including
PDF/Office/image/audio files, are displayed honestly as unsupported. Hidden entries
are skipped; symbolic links and non-UTF-8 inputs are rejected. Limits: 200 inventory
entries including directories, 128 KiB per file, 1 MiB admitted bytes per folder,
eight nested directories. Inventory/total/depth overflows fail the selection closed;
narrow the folder. Selection paths with `..` or symlink components are rejected.
On Kinoite use `/var/home/...` rather than the `/home` symlink alias if necessary.

Retrieval is deterministic lexical matching, with a bounded summary route, not
semantic search. The model receives only retrieved source fragments. At most
12,000 bytes of retrieved context are admitted; omitted fragments are explicitly
reported and answers must not claim complete coverage. Source identity, digest
and exact ranges are validated; semantic entailment is checked in the synthetic
acceptance tests, not automatically proven for every generated answer.

A conversation allows six successful turns. The complete question and retained
turns reach the model; the lexical search query is a bounded term projection.
Actual tokenizer counts reserve output capacity before dispatch. Overflow rejects
the request; no silent truncation or automatic compaction occurs. Incomplete output
is discarded. Shorten the question, narrow the folder, or start a new conversation.

Conversations and source snapshots are memory-only and disappear on restart.
Application state/logs/access metadata live under
`${XDG_STATE_HOME:-$HOME/.local/state}/agentmage-demo` with private directory/file
permissions. This demo makes no encryption claim and does not change production
SQLCipher storage. Application logs omit questions and document content; model logs
contain operational timings. Inference has no cloud fallback, no tool execution,
and no model-issued shell commands. The model uses an authenticated private UNIX
socket and a dedicated network namespace. The browser backend binds loopback and
requires a random token plus Host/Origin checks; no CORS is enabled.

## Repeatable acceptance

After dependencies in `package-lock.json` are installed with `npm ci`, and a
Puppeteer Chromium is available (already cached on this machine), run:

```sh
python3 scripts/demo_smoke.py
```

This uses Chromium automation and real inference through the application, exercises
known facts/citations, follow-ups, missing evidence, boundary/unsupported inputs,
overflow, cancellation and recovery, then stops the app, tests the backend and model
inside scoped networkless namespaces, restarts and repeats a successful browser
interaction. It leaves the restarted demo running. It resets the demo conversation;
use synthetic data. Automation uses `--no-sandbox` for its Chromium process; this does
not alter the application/model sandbox or system settings.

Individual entry points:

```sh
node scripts/demo_browser_smoke.mjs
python3 scripts/demo.py stop
python3 scripts/demo_offline_smoke.py
python3 scripts/demo.py start --no-open
node scripts/demo_browser_smoke.mjs --restart-only
```

Release the GPU when an acceptance run is finished. `scripts/demo_smoke.py` deliberately
leaves the demo running so a reviewer can use it, and the resident model holds roughly 15.7 GB
of GPU memory. Other work on this machine needs that memory, so stop the demo and confirm the
GPU is released:

```sh
python3 scripts/demo.py stop
nvidia-smi --query-gpu=memory.used --format=csv
```

Used memory should fall back to the desktop baseline, around 1 GB on this host.

Verification JSON and regression results live in `docs/verification/demo-*`.
The screenshot is local application-owned test output, not backend proof.

## Troubleshooting

- **Model unavailable:** use **Start / retry model** and wait for readiness. Inspect
  `~/.local/state/agentmage-demo/model.log`. A missing/hash-changed artifact is refused;
  verify provenance before changing the configuration.
- **VRAM exhausted:** stop another model you launched, then retry. The demo does not
  stop unrelated processes automatically. Expect roughly 17 GiB total observed GPU
  usage including the desktop; this is not a certified peak bound.
- **Access denied / reloaded page:** rerun `python3 scripts/demo.py start` to open a
  fresh authorized tab. Tokens are process-scoped and cleared from the visible URL.
- **Port in use:** stop the demo or choose another loopback port with `--port`.
- **Folder rejected:** inspect the reason, choose a smaller supported folder, avoid
  symlinks/traversal, and use its canonical `/var/home/...` location.
- **Context/output limit:** start a new conversation or ask a smaller question. An
  incomplete answer is never displayed as complete.
- **Application fails to launch:** inspect
  `~/.local/state/agentmage-demo/application.log`. Bubblewrap network isolation is
  required; failure stops model startup rather than allowing a networked fallback.
