# AgentMage local Linux demo

This Fedora Kinoite demo is separate from production preview/full-GA qualification.
It uses the real AgentMage Rust host and canonical knowledge retrieval/rendering,
with a protected localhost browser shell and a locally running Muse model.

## Launch and stop on this computer

From the repository:

```sh
cd /var/home/aaronnhorvitz/dev/01_repos/AgentMage
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
