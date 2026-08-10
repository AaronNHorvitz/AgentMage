# macOS Model-Feasibility Execution

## Scope

This runbook executes Story 0.3's fixed public-synthetic corpus on a personally controlled MacBook Pro with an Apple M5-family processor. It does not enable either candidate model, replace the Linux evidence, or establish release support.

## Preconditions

1. Use a clean AgentMage checkout at the exact commit selected for the evaluation.
2. Keep the model, projector, runtime archive, extracted runtime, and result directory outside the repository.
3. Obtain `llama-b10333-bin-macos-arm64.tar.gz` from the immutable URL recorded in `model-profiles/runtimes/llama-cpp-b10333-macos-arm64.json`.
4. Stage the exact Gemma 4 E4B GGUF and projector identified by `model-profiles/candidates/gemma-4-e4b/artifact-admission.json`.
5. Disconnect Ethernet, turn off Wi-Fi, and disconnect VPN software before execution. The runner refuses to start while a non-loopback interface or default route is active.
6. Do not remove quarantine metadata or bypass macOS execution controls merely to make the runtime start. Record any operating-system refusal as a blocked result.

## Preflight

Run the admission and unit gates while network access is still available:

```bash
npm ci --ignore-scripts
npm run model:macos-runtime-check
python3 -m unittest tests.test_macos_model_feasibility
git status --short
```

The worktree must be clean. The execution command also refuses a runner whose bytes differ from the current committed revision.

## Execution

After disconnecting every non-loopback network path, run:

```bash
python3 scripts/macos_model_feasibility.py execute \
  --runtime-archive <outside-repo>/llama-b10333-bin-macos-arm64.tar.gz \
  --runtime-root <outside-repo>/llama-runtime \
  --server <outside-repo>/llama-runtime/llama-b10333/llama-server \
  --model <outside-repo>/gemma-4-e4b-it-Q4_K_M.gguf \
  --projector <outside-repo>/mmproj-model-f16.gguf \
  --output <outside-repo>/results/gemma-4-e4b-macos-native
```

Exit code `0` means every fixed case and threshold passed. Exit code `2` means the corpus completed and retained one or more failures. Exit code `1` means execution or evidence production was invalid or blocked. A nonzero result must not be converted into a pass.

## Verification And Transfer

Reconnect only after the runner exits and `llama-server` is no longer running. Verify the result on the Mac before transfer:

```bash
python3 scripts/macos_model_feasibility.py verify \
  --result-dir <outside-repo>/results/gemma-4-e4b-macos-native
```

Transfer the complete result directory without editing it. Re-run the same `verify` command in the receiving clean checkout. Admission into `artifacts/sprints/sprint-0/` requires a separate hash-bound evidence step; raw local output is not committed directly.

From the receiving checkout, build the immutable public extension bundle with:

```bash
python3 scripts/story_0_3_macos_evidence.py --write \
  --result-dir <outside-repo>/results/gemma-4-e4b-macos-native \
  --verification-revision HEAD
```

The importer refuses an invalid source bundle or an existing output directory. It retains the byte-identical result, replaces only the machine-local evaluation root in the admitted server log, and records hashes and sizes for the untouched external source files. The external source directory remains authoritative and must be retained.

## Required Review

Confirm that the result identifies `macos-native-metal`, an Apple M5-family MacBook Pro, the exact runtime/model identities, the fixed corpus and decoder, a loopback-only listener, no active non-loopback route, all raw trials, resource samples, threshold reconciliation, and no user data. Story 0.3 remains blocked until this review and evidence admission are complete.
