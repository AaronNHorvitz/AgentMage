"""Frozen Rust controls for the coding evaluator, never generated-candidate admission.

Original evaluation adapter. Historical source, regression and corpus bytes are
verified before execution; the existing evaluator owns admission and lifecycle.
"""
from __future__ import annotations

import json
import os
from pathlib import Path
import resource
import subprocess


TASKS = [{
    "id": "dev-rust-001",
    "language": "rust",
    "split": "development",
    "base": "b22a2c46f8b0f324daea6ec92af3746e7a49d763",
    "reference": "72d9448afbf9ce9e76eaa61c4dc40108964a7584",
    "source": "kernel/contracts/src/path.rs",
    "hidden_revision": "1ff13fe592ddbf93aa2f37b46790e8b065efa6e5",
    "hidden_test": "kernel/contracts/tests/path_corpus.rs",
    "fixture": "fixtures/paths/v1/corpus.json",
    "expected_tests": 1,
    "base_sha256": "ce942f5ee5cf9e662fdfe3a49c4b56b43b9e81d637ad5e42d0f2f7a07936edab",
    "reference_sha256": "1534abe465aadc759c95d0de2c8d3dc1681652f7f3c6cad35131bcb8802041a9",
    "hidden_sha256": "11e4ef3977d46ff21b767cc7bdac8022c8ccda7e088673af9090a9580523cbca",
    "fixture_sha256": "75fb2f2b235fee715cd41d9c3a562321744868b8594e48fa8289cba0d5d869ea",
    "projection_sha256": "a3fa4c18dccb112067a3979741413c766bc5694c24f7faaaec251570bdcf6b92",
    "statement": (
        "Repair ambiguous components in the existing WorkspacePath contract, editing only "
        "kernel/contracts/src/path.rs. Preserve workspace identity, component/count/byte "
        "bounds, deserialization revalidation, component indices and ordinary case-sensitive "
        "names. Add WorkspacePathErrorKind variants RootedComponent, ColonInComponent, "
        "EncodedPathSyntax, AmbiguousSuffix and InvisibleFormat with codes "
        "path.component.rooted, path.component.colon, path.component.encoded_syntax, "
        "path.component.ambiguous_suffix and path.component.invisible_format respectively. "
        "In validation order retain empty, dot and parent-component refusals; then reject "
        "leading ASCII slash/backslash as rooted, any colon, then separators anywhere "
        "(ASCII slash/backslash and U+2044/U+2215/U+29F8/U+FF0F/U+FF3C) using the existing "
        "separator code. Next refuse case-insensitive percent encodings %2e, %2f and %5c; "
        "retain NUL, control and byte-size checks; then reject trailing ASCII dot or space, "
        "invisible formatting U+200B..U+200F/U+202A..U+202E/U+2060..U+2069/U+FEFF, and "
        "text that differs under either NFC or NFKC using the existing noncanonical code. "
        "Construction and serde input must share the same refusals. Do not normalize an "
        "ambiguous input into an accepted path, fold case, access the filesystem or acquire "
        "authority. No new dependency or manifest changes."
    ),
}]

TOOLCHAIN = "1.95.0"


def projection(task: dict, common) -> dict[str, bytes]:
    entries = common.git("ls-tree", "-r", task["base"]).splitlines()
    paths = []
    for entry in entries:
        metadata, name = entry.split(b"\t", 1)
        path = name.decode("utf-8")
        if not (path in {"Cargo.toml", "Cargo.lock"} or path.endswith("/Cargo.toml")
                or "/src/" in path and path.endswith(".rs")):
            continue
        if not metadata.startswith(b"100644 blob "):
            raise common.InputIntegrityError("evaluation.rust-projection-entry-refused")
        paths.append(path)
    files = {path: common.git("show", task["base"] + ":" + path) for path in paths}
    digest = common.sha(json.dumps({p: common.sha(b) for p, b in files.items()},
                                   sort_keys=True, separators=(",", ":")).encode())
    if digest != task["projection_sha256"] or common.sha(files[task["source"]]) != task["base_sha256"]:
        raise common.InputIntegrityError("evaluation.projection-hash-mismatch")
    if any(len(b) > common.MAX_FILE for b in files.values()) or sum(map(len, files.values())) > common.MAX_TREE:
        raise common.EvaluationError("evaluation.projection-too-large")
    # Keep the complete original Cargo resolution context so --locked stays
    # meaningful. Only contracts is compiled; no other crate's code is executed.
    # The entire projection remains frozen except the one permitted source edit.
    return files


def materialize(workspace: Path, task: dict, common) -> None:
    common.write_file(workspace / "kernel/contracts/tests/eval_hidden.rs", common.pinned_blob(
        task["hidden_revision"], task["hidden_test"], task["hidden_sha256"]))
    common.write_file(workspace / task["fixture"], common.pinned_blob(
        task["hidden_revision"], task["fixture"], task["fixture_sha256"]))


def runtime_paths(common) -> tuple[Path, Path]:
    # `which` only locates the already installed toolchain; no install/update.
    rustc = Path(subprocess.check_output(
        ["rustup", "which", "--toolchain", TOOLCHAIN, "rustc"],
        stderr=subprocess.DEVNULL, timeout=10,
    ).decode().strip()).resolve(strict=True)
    toolchain = rustc.parent.parent
    registry = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo")) / "registry"
    registry = registry.resolve(strict=True)
    if not (toolchain / "bin/cargo").is_file() or not registry.is_dir():
        raise common.EvaluationError("evaluation.rust-runtime-unavailable")
    return toolchain, registry


def sandbox_command(workspace: Path, grader: Path, output: Path, common) -> list[str]:
    toolchain, registry = runtime_paths(common)
    command = common.sandbox_command(workspace, grader, output)
    index = command.index("/usr/bin/python3")
    command[index:index] = [
        "--ro-bind", str(toolchain), "/toolchain", "--tmpfs", "/cargo",
        "--ro-bind", str(registry), "/cargo/registry",
        "--setenv", "CARGO_HOME", "/cargo", "--setenv", "CARGO_NET_OFFLINE", "true",
        "--setenv", "PATH", "/toolchain/bin:/usr/bin",
        "--setenv", "RUSTFLAGS", "-Ccodegen-units=1 -Clink-arg=-Wl,--threads=2",
    ]
    return command


def limits() -> None:
    # Compiler limits are per process, NOT aggregate quotas. Generated Rust is
    # blocked by the same parent guard as generated Python.
    resource.setrlimit(resource.RLIMIT_AS, (2 * 1024**3, 2 * 1024**3))
    resource.setrlimit(resource.RLIMIT_FSIZE, (128 * 1024**2, 128 * 1024**2))
    resource.setrlimit(resource.RLIMIT_CPU, (60, 60))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    os.sched_setaffinity(0, sorted(os.sched_getaffinity(0))[:4])


# The child driver is trusted infrastructure, not an independent result oracle.
# Its binary output can be forged by arbitrary Rust; parent admission therefore
# accepts only byte-identical historical controls, including the entire tree.
DRIVER = b'''import json
import os
import re
import subprocess
import sys

print("rust-worker-cpus=" + str(len(os.sched_getaffinity(0))), file=sys.stderr, flush=True)
command = ["/toolchain/bin/cargo", "test", "--locked", "--offline", "--jobs", "2",
           "--target-dir", "/tmp/target", "--manifest-path", "/workspace/kernel/contracts/Cargo.toml",
           "--test", "eval_hidden", "--no-run", "--message-format=json"]
with open("/tmp/compiler.jsonl", "xb") as log:
    compiled = subprocess.run(command, stdout=log)
with open("/tmp/compiler.jsonl", "rb") as log:
    raw = log.read(2 * 1024 * 1024 + 1)
if len(raw) > 2 * 1024 * 1024:
    raise SystemExit(2)
sys.stdout.buffer.write(raw)
sys.stdout.buffer.flush()
if compiled.returncode != 0:
    raise SystemExit(2)
executables = []
for line in raw.splitlines():
    message = json.loads(line)
    if (message.get("reason") == "compiler-artifact" and message.get("target", {}).get("name") == "eval_hidden"
            and message.get("executable")):
        executables.append(message["executable"])
if len(executables) != 1 or not executables[0].startswith("/tmp/target/"):
    raise SystemExit(2)
with open("/tmp/test-output", "xb") as log:
    tested = subprocess.run([executables[0], "--format=pretty", "--test-threads=2"], stdout=log)
with open("/tmp/test-output", "rb") as log:
    raw = log.read(2 * 1024 * 1024 + 1)
if len(raw) > 2 * 1024 * 1024:
    raise SystemExit(2)
sys.stdout.buffer.write(raw)
sys.stdout.buffer.flush()
summary = re.findall(rb"^test result: (ok|FAILED)\\. ([0-9]+) passed; ([0-9]+) failed; ([0-9]+) ignored; ([0-9]+) measured; ([0-9]+) filtered out;[^\\n]*$", raw, re.M)
if len(summary) != 1:
    raise SystemExit(2)
status, passed, failed, ignored, measured, filtered = summary[0]
passed, failed, ignored, measured, filtered = map(int, (passed, failed, ignored, measured, filtered))
if measured or filtered or (status == b"ok") != (tested.returncode == 0 and failed == 0):
    raise SystemExit(2)
report = {"tests": passed + failed + ignored, "failures": failed, "errors": 0,
          "skipped": ignored, "expected_failures": 0, "unexpected_successes": 0}
with open("/output/tests.json", "x") as stream:
    json.dump(report, stream, sort_keys=True)
raise SystemExit(0 if tested.returncode == 0 and not ignored and passed > 0 else 1)
'''
