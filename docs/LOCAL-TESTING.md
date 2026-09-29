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
The [review-fix and runtime component verification](verification/review-fixes-and-runtime-components-2026-09-29.md)
retains the latest source, rebuilt binary identities and startup results, with its logs
under `artifacts/verification-logs/`; the
[coding recoverability verification](verification/coding-recoverability-2026-09-29.md),
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

- Apply: type `yes` and Enter at each prompt; the run should show the failing test,
  the exact patch, the rerun and a verified terminal report. Below each write prompt the
  CLI prints a hunk review of the change the exact arguments make to the current file
  (Decision 0112). It says so when the file changed or the change is not reviewable.
  Approving still allows the whole call. After each run, standard error shows the run's
  progress: its terminal state, whether the runtime's verifier accepted it, counts
  against the declared ceilings, and that independent review and delivery are not
  established by the runtime.
- Deny: in a fresh root, answer anything other than `yes`; no file may change.
- Cancel: in a fresh root, run `python3 -m scripts.coding_harness stop --root "$demo"`
  from a second terminal while a prompt or command is active.
- Preservation: add a staged, unstaged or untracked file in `$demo/disposable/worktree`
  before `start`; the wrapper must refuse and leave every byte and the Git index unchanged.

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
