# Research Transport: Isolated DNS and TLS Diagnostics

Date: 2026-09-27. Source parent: `d54b35af6b7222b5a6562b6436e8bb1790104fed`.
This unit began at `edcd6d19a05216970925dd7b3c0832093174ca59`; the owner's intervening
IP-policy/agent-instruction documentation commit is preserved. It changed no runtime
source or staged fixture bytes. The final 543-document scan included that policy.
Scope: AMR-02.3.2 transport prerequisites. Status: **Accepted under owner delegation,
2026-09-20**. Authority: Decision 0054, within Decision 0084. Acceptance is separate.

This increment adds Rust test-only fixtures and a developer launcher. It does not
activate research, change production transport/trust, issue grants, qualify a model,
close a task row or perform independent review. The containing commit pins its sources.

## Actual boundary exercised

The tests invoke the existing public-target validation, real synchronous NSS resolution,
pinned TCP connector, Rustls client, response measurement and complete-request loop.
They run in disposable rootless user, network, mount and PID namespaces. Only loopback
and one synthetic public-address alias exist; there is no external default route.
The public-address alias is local to that namespace and never assigned on the host.

Fresh namespace-only `/etc` and `/run` mounts hide host resolver configuration and
name-service sockets. The test asserts that `/etc` contains only its exact two fixture
files and `/run` is empty before opening a socket. Fixed `hosts: dns` and the local UDP
fixture serve absolute names. Host files, trust roots, sysctls and services are untouched.
Earlier passing runs projected only resolver/NSS files; inspection identified the
potential host name-service-socket path and prompted this additional isolation.
Their observed local queries remain retained, not generalized into proof of that gap.

The TLS server is an owned, supervised OpenSSL fixture in the disposable PID namespace,
not a provider, alternate product transport or model server. A fresh synthetic CA/leaf
pair is generated into private test state. No keys are committed or installed as host
trust. The unchanged production WebPKI path must reject that CA. Test-only assembly of
the existing transport components admits the synthetic CA for positive-response and
hostname-negative tests, with certificate verification and SNI still enabled. That
test-only trust choice is not production admission or evidence for a real public CA.

## Cases

Three explicitly selected ignored tests cover 25 scenario observations:

- Six complete DNS sets: private-only, public/private in either order, link-local,
  mixed IPv4/IPv6 loopback and 17 public answers. All refuse before TCP.
- A real pinned connection: one DNS snapshot, an unused synthetic proxy, a Rustls
  handshake record and refusal of a plaintext peer. A hypothetical second lookup
  returns private data; no such lookup occurs before this connection.
- Default-root refusal of the synthetic CA and test-root refusal of a hostname mismatch.
- Sixteen HTTP/TLS cases: exact success, inert retrieved instructions, same-origin
  redirect success, redirect-time rebinding refusal, aggregate redirect-byte exhaustion,
  cross-origin and zero-hop refusal, content encoding, declared size, truncation,
  header size, informational flooding, conflicting framing, duplicate media,
  executable media and chunked-body bounds. Rebinding must fail as a destination
  refusal. Successful frames re-decode against their exact original packet.

The 20 ordinary worker tests remain separate. Developer-launcher regressions refuse
missing or expanded job/thread settings and entry into an unchanged host namespace;
fixture files contain no credentials. No fake TLS flag replaces the real handshakes
in these namespace tests. Existing in-memory parser fixtures remain separately labelled.

## Reproduction and retained attempts

Use the shared heavy reservation **before** the existing capped scope, with at least
16 GiB available RAM, one Cargo job and one test thread. No GPU or inference is needed.
Inside that scope, build the existing optional worker test executable:

```bash
cargo test -p agentmage-platform-linux --features public-research-worker \
  --bin agentmage-public-research-worker --locked --offline --no-run --message-format=json
```

Select the exact `compiler-artifact.executable` for that target's test profile, then
invoke `bash scripts/research_transport_fixtures.sh EXACT_TEST_BINARY PRIVATE_STATE_DIR`
in the same reserved capped scope. The launcher is not a replacement for that reservation
or scope. It logs executable/tool/fixture hashes, uses two 20-second namespace deadlines,
waits for its owned server and retains generated certificates and server logs privately.
The synthetic fixture directory is never a publication artifact.

Retained diagnostic logs, before the final isolation assertions:

| Attempt | Disposition | SHA-256 |
| --- | --- | --- |
| Initial DNS/TCP | 20 ordinary and two explicit namespace tests; Clippy/audits passed | `01a87be75de3a185fd47a6fc2d5c0761f3389f823e674e3da79ddd703a33f61f` |
| Initial TLS | Added real certificate and nine response cases; owned server reaped | `30cbca9e695f2fd3d7c07388fafcb4b4891e7b71ff3a51bee7b3b030b5ba4260` |
| Public launcher | Reproduced the then-current three namespace tests and cleanup | `dbf53b4487b9389eacf52f567b41ac3969a33987ec3137da22a6646f201ae0cb` |
| Expanded matrix | All 25 observations, 12 Python regressions, Clippy/audits passed | `67fed9ad67a3275aad2acbf1b977cbde0e6f7ca180fb46bff320a72c7148311b` |

No failed attempt is replaced with a fixture success. The final-source run, including
the isolated `/etc` and `/run` assertions and exact rebinding refusal, exited zero:
20 ordinary tests, all three selected namespace tests, 12 Python regressions, strict
optional-worker Clippy, effect/source/dependency checks, formatting and the full
543-document/policy scan passed. The owned TLS server was reaped. Log SHA-256:
`f6fd75c91b1c27b6259523dd26f507148dca56877b3c6d4958fcfb8a13758893`.
Test executable SHA-256:
`44787bb83ffcd72d6adb320758d2fef99ebab61d8719a1efc29c4cd3814a6e62`.
Transport source SHA-256:
`4ad7e290785a2aa728e85e595aa8bb5c1cf5f5e433ce62336a096fce8c6872e7`.
The log also retains exact launcher/fixture and generated public-certificate hashes;
private keys and machine-local transcripts remain outside public Git. Batch evidence
freshness remains separate from these passing component observations.

## Batch freshness disposition

The first evidence attempt stopped at its exact-source-parent guard when it detected
the owner's documentation commit, before any SBOM write. Its retained log SHA-256 is
`eae3664b9818a12e55d7974baf2d62df4d710197636267efcb1f01426515593a`.
After inspecting that commit and the binding IP policy, the reconciled attempt exited
zero with exactly one SBOM write for this fixture unit. Supply-chain validation and
11 regressions, the actual contract-boundary checks (including 52 runtime-schema tests),
and six applicable artifact builders with 47 regressions passed. Log SHA-256:
`0ea37b6eab7f5780104c35adab1f09792a4f8442ff9cd591df76f73d83fab9cb`.
The regenerated reports retain inactive profile registration and unperformed product,
platform and release gates; no input set or acceptance threshold was reduced.

The direct named whole-file inventory compared with the pre-policy source parent:
750 JSON artifacts, 429 bindings to changed inputs, 23 current bindings and 406 already
stale or historical bindings. No newly stale direct binding remains in that limited
inventory. It excludes transitive, unnamed and 1,602 line-span bindings, so it is not a
whole-repository freshness claim. The changed agent instructions and new IP policy had
no matching named whole-file bindings in that inventory; the current document scan
covered them. Older planning/privacy, model and acceptance evidence remains historical,
not silently reaccepted by this pass.

## Still open

These are transport-component observations, not the actual confined-worker/grant/
coordinator workflow. The [native boundary](research-native-boundary-2026-09-27.md)
separately proves bounded launch, output and cleanup with real negative worker cases.
Neither layer alone completes native research admission. Native connected-worker
composition, canonical full-source/receipt/event binding, configured-provider execution
and real-model research-to-code campaigns remain owned work. No fetched hash or supplied
producer description is retrieval evidence by itself.

Earlier Muse evidence remains pinned to its tested coding source; GPT-OSS is still
unqualified. Independent review, human acceptance, platform and release gates remain open.
