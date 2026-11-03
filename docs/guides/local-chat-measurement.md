# Record a local chat connection

T-302.4 uses `scripts/local_chat_smoke.py` to record two fixed, synthetic chat
turns through the [version 1 local API](local-chat-api.md). This is a connection
check, not a coding evaluation, a task CLI or model qualification. It starts no
model, executes no model output and downloads nothing. Model text stays in
private files and is never printed to the terminal.

Start a **dedicated** API process using the API guide, with a new authentication
directory and numeric loopback listener. The check refuses an already configured
or busy API instance. Separately start the chosen existing local model under the
lane's GPU reservation and wait for its readiness check to succeed. A listening
socket alone does not establish that a model has loaded. Do not stop another
worker, take its endpoint or change its limits.

Run the recording client while that reservation remains held:

```sh
python3 scripts/local_chat_smoke.py \
  --api-address 127.0.0.1:43127 \
  --auth-directory <private-auth-directory> \
  --endpoint http://127.0.0.1:8080/v1 \
  --model <listed-model-id> \
  --endpoint-key-file <private-model-key-file> \
  --output-directory <new-private-record-directory> \
  --approve-conversations
```

The angle-bracket values are placeholders. Omit `--endpoint-key-file` for an
endpoint without a key. The key file contains exactly the bearer value, with no
newline. Keys and API tokens must be private owned regular files; symlinks,
hard links and group/other permissions are refused. Never pass a key value on
the command line. The API and model must use distinct keys. The client uses the
control token only for endpoint choice/revocation and the client token for
listing, conversation, status and cancellation. It does not use proxies or
follow redirects. Only the API contacts the chosen model endpoint.

`--approve-conversations` explicitly authorizes the fixed prompts at this
endpoint. The authenticated settings request binds that choice before model
listing or chat. The second request includes the first actual answer in its
history. There is no hidden prompt, tool call, prompt tuning or repository data.

The new output directory has mode 0700, with mode-0600 `events.ndjson` and
`report.json`. Existing output directories are refused. Records contain the
actual model text and are untrusted; inspect them as data, never execute them or
render them as active markup. Keep raw records private until checked for public
sharing. Python credential allocations are not guaranteed to be erased.

Each event records its turn and elapsed wall time, including request setup.
The report records first-text and total times, event/text counts and a SHA-256
of the events file. Success requires a nonempty answer, an explicit normal-stop
event and stream EOF for **both** turns. Missing completion, error, length limit,
malformed event, trailing event, excessive record size or deadline fails. Text
fragments are flushed as they arrive, including partial records on failure.
This does not grade the answer's semantic correctness or measure coding ability.
Token counts are null because this API does not expose upstream usage. No GPU
measurement is inferred by this client; collect reservation/resource observations
in the owning launcher.

The client requests cancellation, waits for idle, and revokes its settings on
ordinary success/failure. Failed revocation makes the result FAIL. After a
forced process kill, stop the dedicated API to erase its memory-only settings.
Always stop and reap the API, model, and any transport bridge the launcher owns,
and remove ephemeral credential files before releasing the reservation. The
recording client owns none of those processes and cannot stop another service.
It uses the API's existing bounds and does not change its permissions.

The lane's shared reservation is acquired by the owning launcher, for example:

```sh
bash /tools/build-slot bash /tools/gpu-slot queue-run agentmage 600 python3 <private-reservation-runner>
```

That runner supplies the machine-local read-only model/runtime paths and manages
their confinement and cleanup; they do not belong in this guide. The measured
[T-302.4 record](../evidence/t302-4-real-local-chat-2026-11-03.md) describes the
actual launcher and its limitations. No public release or owner acceptance
follows from a connection check.

The client tests require only the standard Python library:

```sh
python3 -m unittest tests.test_local_chat_smoke -v
```

They include a synthetic API on actual local HTTP sockets. They are not model
measurements. The separate API contracts still cover the real Rust executable,
upstream protocol, authority separation and streaming failures.
