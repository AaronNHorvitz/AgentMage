"""Offline synthetic model preparation, not AgentMage's coding runtime.

Run with python3 -m scripts.coding_model_lab. No model output executes a tool.
"""
from __future__ import annotations

import argparse
from contextlib import contextmanager
import fcntl
import hashlib
import json
import os
from pathlib import Path
import secrets
import signal
import stat
import subprocess
import sys
import threading
import time
import urllib.request

from scripts.demo import UnixHTTP

ROOT = Path(__file__).resolve().parents[1]
CONFIG = ROOT / "model-profiles/development/coding-model-lab.json"
READ_ONLY = (
    "/usr/lib64", "/usr/share/vulkan", "/usr/share/glvnd",
    "/usr/share/nvidia", "/etc/vulkan", "/etc/glvnd", "/etc/nvidia",
    "/sys", "/proc/driver/nvidia",
)
DEVICES = (
    "/dev/dri", "/dev/nvidia0", "/dev/nvidiactl", "/dev/nvidia-uvm",
    "/dev/nvidia-uvm-tools", "/dev/nvidia-modeset", "/dev/nvidia-caps",
)


def digest(path: Path) -> str:
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def private_directory(path: Path) -> Path:
    path.mkdir(parents=True, exist_ok=True, mode=0o700)
    info = path.lstat()
    if not stat.S_ISDIR(info.st_mode) or info.st_uid != os.getuid() or info.st_mode & 0o077:
        raise RuntimeError(f"Expected an owned private directory: {path}")
    return path


def save_json(path: Path, value: object) -> None:
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, "w") as target:
        json.dump(value, target, indent=2, sort_keys=True)
        target.write("\n")


def load_config() -> dict:
    cfg = json.loads(CONFIG.read_text())
    if cfg["product_enabled"] is not False or cfg["scope"] != "synthetic-development-preparation-only":
        raise RuntimeError("Only the explicit development preparation profile is allowed")
    if cfg["parallel_slots"] != 1 or cfg["threads"] > 4:
        raise RuntimeError("This lab permits one slot and at most four threads")
    if not 8192 < cfg["context_tokens"] <= 32768:
        raise RuntimeError("Development context must be above 8K and at most 32K")
    if not 2048 <= cfg["output_tokens"] < cfg["context_tokens"]:
        raise RuntimeError("Invalid reasoning plus final-output reservation")
    return cfg


def verify_file(path: Path, expected_size: int, expected_hash: str) -> None:
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    with os.fdopen(fd, "rb") as source:
        info = os.fstat(source.fileno())
        if not stat.S_ISREG(info.st_mode) or info.st_size != expected_size:
            raise RuntimeError(f"Incomplete or non-regular artifact: {path}")
        if source.read(4) != b"GGUF":
            raise RuntimeError("Expected GGUF, not an error page or LFS pointer")
        source.seek(0)
        if hashlib.file_digest(source, "sha256").hexdigest() != expected_hash:
            raise RuntimeError(f"Artifact SHA-256 mismatch: {path}")


def promote_download(cfg: dict) -> None:
    model = cfg["models"]["gpt-oss"]
    destination = Path(model["path"]).expanduser()
    private_directory(destination.parent)
    if destination.exists():
        verify_file(destination, model["size_bytes"], model["sha256"])
        print("Existing GPT-OSS artifact verified", flush=True)
        return
    partial = Path(cfg["data_root"]).expanduser() / "quarantine/gpt-oss-20b-MXFP4.gguf.part"
    verify_file(partial, model["size_bytes"], model["sha256"])
    os.chmod(partial, 0o600)
    # Hard-link promotion cannot overwrite an existing destination on a race.
    os.link(partial, destination, follow_symlinks=False)
    partial.unlink()
    print(f"Verified and promoted {destination}", flush=True)


def record_provenance(cfg: dict) -> None:
    model = cfg["models"]["gpt-oss"]
    root = private_directory(Path(cfg["data_root"]).expanduser() / "provenance")
    run = private_directory(root / time.strftime("%Y%m%dT%H%M%S"))
    conversion = f"https://huggingface.co/{model['source_repository']}/resolve/{model['source_revision']}"
    upstream = f"https://huggingface.co/{model['base_repository']}/resolve/{model['base_revision']}"
    urls = {
        "conversion-source.txt": conversion + "/.src_sha",
        "conversion-README.md": conversion + "/README.md",
        "conversion.log": conversion + "/convert.log",
        "upstream-LICENSE": upstream + "/LICENSE",
        "upstream-USAGE_POLICY": upstream + "/USAGE_POLICY",
        "upstream-README.md": upstream + "/README.md",
        "upstream-config.json": upstream + "/config.json",
        "upstream-chat-template.jinja": upstream + "/chat_template.jinja",
        "conversion-metadata.json": f"https://huggingface.co/api/models/{model['source_repository']}/revision/{model['source_revision']}?blobs=true",
    }
    records = []
    for name, url in urls.items():
        with urllib.request.urlopen(url, timeout=60) as response:
            payload = response.read(4 * 1024 * 1024 + 1)
            if len(payload) > 4 * 1024 * 1024:
                raise RuntimeError("Provenance response exceeds bound")
        if name == "conversion-source.txt" and f"PRIMARY={model['base_revision']}" not in payload.decode():
            raise RuntimeError("Conversion source revision differs")
        if name == "conversion-metadata.json":
            metadata = json.loads(payload)
            artifact = next(x for x in metadata["siblings"] if x["rfilename"] == "gpt-oss-20b-MXFP4.gguf")
            if metadata["sha"] != model["source_revision"] or artifact["lfs"]["sha256"] != model["sha256"] or artifact["size"] != model["size_bytes"]:
                raise RuntimeError("Published conversion metadata differs from pinned profile")
        with (run / name).open("xb") as target:
            target.write(payload)
        records.append({"file": name, "url": url, "sha256": hashlib.sha256(payload).hexdigest(), "bytes": len(payload)})
    save_json(run / "manifest.json", {"model": model, "files": records, "qualification": "identity evidence only; conversion equivalence not established"})
    print(f"Recorded pinned provenance: {run}", flush=True)


def gpu_memory() -> tuple[int, int]:
    result = subprocess.run(
        ["/usr/bin/nvidia-smi", "--query-gpu=memory.used,memory.free", "--format=csv,noheader,nounits"],
        check=True, capture_output=True, text=True, timeout=10,
    )
    lines = result.stdout.strip().splitlines()
    if len(lines) != 1:
        raise RuntimeError("This preparation profile requires exactly one GPU")
    used, free = lines[0].split(",")
    return int(used), int(free)


def scope_resources() -> dict:
    group = Path("/proc/self/cgroup").read_text().strip().split("::", 1)[1]
    root = Path("/sys/fs/cgroup") / group.lstrip("/")
    return {name: (root / name).read_text().strip() for name in
            ("memory.high", "memory.max", "memory.swap.max", "memory.peak", "cpu.max")}


def verify_runtime(cfg: dict) -> Path:
    manifest = json.loads((ROOT / cfg["runtime_manifest"]).read_text())
    runtime = Path(manifest["runtime_root"])
    files = {str(p.relative_to(runtime)) for p in runtime.rglob("*") if p.is_file() and not p.is_symlink()}
    links = {str(p.relative_to(runtime)): str(p.readlink()) for p in runtime.rglob("*") if p.is_symlink()}
    if files != set(manifest["runtime_files_sha256"]) or links != manifest["runtime_symlinks"]:
        raise RuntimeError("Runtime inventory differs from its verified demo package")
    for name, expected in manifest["runtime_files_sha256"].items():
        if digest(runtime / name) != expected:
            raise RuntimeError(f"Runtime digest differs: {name}")
    return runtime


def sandbox_command(cfg: dict, model_id: str, runtime: Path, state: Path) -> list[str]:
    model = cfg["models"][model_id]
    cmd = ["/usr/bin/bwrap", "--unshare-all", "--unshare-user", "--disable-userns",
           "--new-session", "--die-with-parent", "--clearenv", "--cap-drop", "ALL",
           "--proc", "/proc", "--dev", "/dev", "--tmpfs", "/tmp"]
    for directory in ("/runtime", "/model", "/run", "/run/agentmage", "/usr", "/usr/share", "/etc", "/proc/driver"):
        cmd += ["--dir", directory]
    cmd += ["--ro-bind", str(runtime), "/runtime", "--ro-bind", str(Path(model["path"]).expanduser()), "/model/model.gguf",
            "--bind", str(state), "/run/agentmage"]
    for path in READ_ONLY:
        cmd += ["--ro-bind", path, path]
    cmd += ["--symlink", "usr/lib64", "/lib64"]
    for path in DEVICES:
        cmd += ["--dev-bind", path, path]
    for name, value in {"LD_LIBRARY_PATH": "/runtime/lib:/usr/lib64", "PATH": "/runtime/bin",
                        "HOME": "/nonexistent", "TMPDIR": "/tmp", "XDG_RUNTIME_DIR": "/tmp", "LANG": "C"}.items():
        cmd += ["--setenv", name, value]
    cmd += ["--chdir", "/runtime/lib", "/runtime/bin/llama-server",
            "--model", "/model/model.gguf", "--alias", model_id,
            "--host", "/run/agentmage/model.sock", "--api-key-file", "/run/agentmage/api-key",
            "--ctx-size", str(cfg["context_tokens"]), "--parallel", "1",
            "--n-gpu-layers", "999", "--fit", "off", "--flash-attn", "on",
            "--cache-type-k", cfg["cache_type_k"], "--cache-type-v", cfg["cache_type_v"],
            "--threads", str(cfg["threads"]), "--threads-batch", str(cfg["threads"]),
            "--threads-http", "2", "--batch-size", str(cfg["batch_tokens"]),
            "--ubatch-size", str(cfg["microbatch_tokens"]), "--poll", "0",
            "--n-predict", str(cfg["output_tokens"]), "--temp", str(cfg["temperature"]),
            "--seed", str(cfg["seed"]), "--jinja", "--reasoning", "auto",
            "--chat-template-kwargs", json.dumps(model["chat_template_kwargs"]),
            "--no-context-shift", "--no-cache-prompt", "--cache-ram", "0",
            "--offline", "--no-webui", "--no-agent", "--no-slots"]
    return cmd


class ModelServer:
    def __init__(self, cfg: dict, model_id: str, state: Path):
        self.cfg, self.model_id, self.state = cfg, model_id, state
        self.key = secrets.token_urlsafe(32)
        self.process = None
        self.peak_gpu_mib = 0
        self.guard_error = None

    def call(self, route: str, payload: dict | None = None, timeout: int = 600) -> dict:
        connection = UnixHTTP(self.state / "model.sock", timeout)
        try:
            connection.request("GET" if payload is None else "POST", route,
                               None if payload is None else json.dumps(payload),
                               {"Authorization": "Bearer " + self.key, "Content-Type": "application/json"})
            response = connection.getresponse()
            data = response.read(8 * 1024 * 1024 + 1)
            if len(data) > 8 * 1024 * 1024 or response.status != 200:
                raise RuntimeError(f"Model endpoint {route}: HTTP {response.status}; {data[:300]!r}")
            return json.loads(data)
        finally:
            connection.close()

    def complete(self, messages: list[dict], **kwargs) -> dict:
        result = self.call("/v1/chat/completions", dict(
            model=self.model_id, messages=messages, max_tokens=self.cfg["output_tokens"],
            temperature=self.cfg["temperature"], seed=self.cfg["seed"], **kwargs,
        ))
        if result["choices"][0]["finish_reason"] not in ("stop", "tool_calls"):
            raise RuntimeError("Incomplete model response; not a passing result")
        return result


@contextmanager
def running_model(cfg: dict, model_id: str, report: dict):
    state_root = private_directory(Path(cfg["state_root"]).expanduser())
    lock_fd = os.open(state_root / "gpu.lock", os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
    process = None
    stopped = threading.Event()
    monitor = None
    state = None
    try:
        fcntl.flock(lock_fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        memory = dict(line.split(":", 1) for line in Path("/proc/meminfo").read_text().splitlines())
        if int(memory["MemAvailable"].split()[0]) < cfg["minimum_available_ram_gib"] * 1024**2:
            raise RuntimeError("Less than 16 GiB available RAM; leave other work untouched")
        used, free = gpu_memory()
        if free < cfg["minimum_free_vram_mib"]:
            raise RuntimeError("GPU already occupied; stop nothing and retry later")
        print(f"Verifying {model_id} and the pinned runtime", flush=True)
        model = cfg["models"][model_id]
        verify_file(Path(model["path"]).expanduser(), model["size_bytes"], model["sha256"])
        runtime = verify_runtime(cfg)
        # Recheck after hashing; another application may have acquired GPU memory.
        if gpu_memory()[1] < cfg["minimum_free_vram_mib"]:
            raise RuntimeError("GPU became occupied during verification")
        state = private_directory(state_root / (time.strftime("%Y%m%dT%H%M%S") + "-" + model_id))
        server = ModelServer(cfg, model_id, state)
        key_fd = os.open(state / "api-key", os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(key_fd, "w") as target:
            target.write(server.key + "\n")
        command = sandbox_command(cfg, model_id, runtime, state)
        report.update(model_id=model_id, model=model, profile=cfg, profile_sha256=digest(CONFIG),
                      preparation_script_sha256=digest(Path(__file__)),
                      runtime_manifest_sha256=digest(ROOT / cfg["runtime_manifest"]),
                      state_directory=str(state), command=command, baseline_gpu_used_mib=used)
        with (state / "server.log").open("xb") as log:
            process = subprocess.Popen(command, stdout=log, stderr=log, start_new_session=True)
            server.process = process

            def guard():
                while not stopped.wait(1):
                    try:
                        current, _ = gpu_memory()
                        server.peak_gpu_mib = max(server.peak_gpu_mib, current)
                        if current > cfg["maximum_total_vram_used_mib"]:
                            raise RuntimeError("GPU desktop headroom exceeded")
                    except Exception as error:
                        server.guard_error = str(error)
                        if process.poll() is None:
                            process.terminate()
                        return

            monitor = threading.Thread(target=guard, daemon=True)
            monitor.start()
            start = time.monotonic()
            while time.monotonic() - start < 180:
                if process.poll() is not None:
                    raise RuntimeError(f"Server exited; inspect {state / 'server.log'}; {server.guard_error or ''}")
                try:
                    if server.call("/health", timeout=1).get("status") == "ok":
                        break
                except (OSError, RuntimeError, ValueError):
                    pass
                time.sleep(.25)
            else:
                raise RuntimeError("Model startup exceeded 180 seconds")
            report["load_seconds"] = round(time.monotonic() - start, 3)
            print(f"{model_id} ready in {report['load_seconds']} s; private socket {state / 'model.sock'}", flush=True)
            try:
                yield server
                if server.guard_error:
                    raise RuntimeError(server.guard_error)
            finally:
                report["peak_total_gpu_used_mib_sampled"] = server.peak_gpu_mib
                report["resource_guard_error"] = server.guard_error
    except BaseException as error:
        report.update(status="failed-or-interrupted", error=str(error) or type(error).__name__)
        raise
    finally:
        if process is not None:
            report["shutdown_requested_by_lab"] = process.poll() is None
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=10)
            report["server_exit_code"] = process.returncode
        stopped.set()
        if monitor is not None:
            monitor.join(timeout=12)
        if state is not None:
            (state / "api-key").unlink(missing_ok=True)
            (state / "model.sock").unlink(missing_ok=True)
            report["scope_resources"] = scope_resources()
            save_json(state / "report.json", report)
        os.close(lock_fd)


def token_count(server: ModelServer, messages: list[dict]) -> int:
    rendered = server.call("/apply-template", {"messages": messages})["prompt"]
    return len(server.call("/tokenize", {"content": rendered, "add_special": True})["tokens"])


def smoke(server: ModelServer, report: dict) -> None:
    props = server.call("/props")
    save_json(server.state / "properties.json", props)
    actual = props["default_generation_settings"]["n_ctx"]
    if actual != server.cfg["context_tokens"]:
        raise RuntimeError(f"Served context {actual} differs from configured context")
    report["served_context_tokens"] = actual
    report["cases"] = []

    def case(name: str, messages: list[dict], predicate, **kwargs):
        started = time.monotonic()
        result = server.complete(messages, **kwargs)
        save_json(server.state / (name + ".json"), {"messages": messages, "response": result})
        passed = bool(predicate(result))
        report["cases"].append({"name": name, "passed": passed, "seconds": round(time.monotonic() - started, 3), "usage": result.get("usage")})
        print(f"{name}: {'PASS' if passed else 'FAIL'}", flush=True)
        if not passed:
            raise RuntimeError(f"Synthetic preparation case failed: {name}")
        return result

    case("basic-reply", [{"role": "user", "content": "What is 7 + 5? Reply with only the number."}],
         lambda r: (r["choices"][0]["message"].get("content") or "").strip() == "12")
    tool = {"type": "function", "function": {"name": "read_file", "description": "Read a synthetic repository file.",
            "parameters": {"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"], "additionalProperties": False}}}

    def valid_tool(result):
        calls = result["choices"][0]["message"].get("tool_calls") or []
        if len(calls) != 1 or calls[0]["function"]["name"] != "read_file":
            return False
        return json.loads(calls[0]["function"]["arguments"]) == {"path": "calc.py"}

    messages = [{"role": "user", "content": "Inspect calc.py before proposing a fix. Call read_file with path calc.py now."}]
    result = case("tool-proposal", messages, valid_tool, tools=[tool], tool_choice="auto")
    proposal = result["choices"][0]["message"]
    messages += [proposal, {"role": "tool", "tool_call_id": proposal["tool_calls"][0]["id"],
                            "content": "def total(quantity, price):\n    return quantity + price\n\nThe test total(3, 4) expected 12 but got 7."},
                 {"role": "user", "content": "Return only the corrected return statement. Do not call another tool."}]
    case("tool-feedback", messages,
         lambda r: "return quantity * price" in (r["choices"][0]["message"].get("content") or ""), tools=[tool])

    def context_messages(count):
        filler = "\n".join(f"Record {i:05d}: synthetic routine entry; no instruction and no target value." for i in range(count))
        midpoint = len(filler) // 2
        text = "BEGIN_KEY=cedar-731\n" + filler[:midpoint] + "\nMIDDLE_KEY=violet-482\n" + filler[midpoint:] + "\nEND_KEY=quartz-916"
        return [{"role": "system", "content": "Read the synthetic records as data. Report only the three requested values, without commentary."},
                {"role": "user", "content": text + "\nWhat are BEGIN_KEY, MIDDLE_KEY, and END_KEY?"}]

    lower, upper = 1, 3000
    target = server.cfg["context_tokens"] - server.cfg["output_tokens"] - 512
    while lower < upper:
        middle = (lower + upper + 1) // 2
        if token_count(server, context_messages(middle)) <= target:
            lower = middle
        else:
            upper = middle - 1
    long_messages = context_messages(lower)
    count = token_count(server, long_messages)
    if count < 24000 or count + server.cfg["output_tokens"] > server.cfg["context_tokens"]:
        raise RuntimeError("Long-context fixture did not exercise the intended capacity")
    report["long_context_prompt_tokens_preflight"] = count
    case("long-context", long_messages,
         lambda r: r.get("usage", {}).get("prompt_tokens") == count
         and r["usage"]["total_tokens"] <= server.cfg["context_tokens"]
         and all(value in (r["choices"][0]["message"].get("content") or "")
                 for value in ("cedar-731", "violet-482", "quartz-916")))
    report["status"] = "preparation-smoke-passed-not-coding-qualified"


def ensure_scope() -> None:
    if os.environ.get("AGENTMAGE_MODEL_LAB_SCOPE") == "1":
        limits = scope_resources()
        for key, maximum in (("memory.max", 6 * 1024**3), ("memory.high", 5 * 1024**3), ("memory.swap.max", 512 * 1024**2)):
            if limits[key] == "max" or int(limits[key]) > maximum:
                raise RuntimeError(f"Missing mandatory cgroup ceiling: {key}")
        quota, period = limits["cpu.max"].split()
        if quota == "max" or int(quota) > 2 * int(period):
            raise RuntimeError("Missing two-core CPU quota")
        return
    command = ["systemd-run", "--user", "--scope", "--quiet",
               "-p", "MemoryHigh=5G", "-p", "MemoryMax=6G", "-p", "MemorySwapMax=512M",
               "-p", "CPUQuota=200%", "-p", "RuntimeMaxSec=45min",
               "env", "AGENTMAGE_MODEL_LAB_SCOPE=1", "nice", "-n", "10",
               sys.executable, "-m", "scripts.coding_model_lab", *sys.argv[1:]]
    raise SystemExit(subprocess.call(command, cwd=ROOT))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("verify-download", "provenance", "probe", "serve"))
    parser.add_argument("model", nargs="?", choices=("muse", "gpt-oss"))
    args = parser.parse_args()
    os.umask(0o077)
    ensure_scope()
    cfg = load_config()
    if args.action == "verify-download":
        promote_download(cfg)
        return 0
    if args.action == "provenance":
        record_provenance(cfg)
        return 0
    if args.model is None:
        parser.error("probe/serve requires a model")
    report = {"status": "failed-or-interrupted", "scope": "synthetic model preparation only", "utc_started": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())}

    def interrupt(_signal, _frame):
        raise KeyboardInterrupt

    signal.signal(signal.SIGTERM, interrupt)
    try:
        with running_model(cfg, args.model, report) as server:
            try:
                if args.action == "probe":
                    smoke(server, report)
                else:
                    report["status"] = "development-service-no-qualification"
                    server.process.wait()
                    if server.process.returncode:
                        raise RuntimeError("Development server exited unsuccessfully")
            except BaseException as error:
                report["error"] = str(error) or type(error).__name__
                raise
    except (RuntimeError, OSError, ValueError, KeyboardInterrupt) as error:
        print(f"Preparation stopped: {error or type(error).__name__}", file=sys.stderr)
        return 1
    print("Preparation finished; no coding or production model gate was closed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
