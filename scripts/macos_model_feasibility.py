#!/usr/bin/env python3
"""Run and verify the fixed model-feasibility corpus on Apple Silicon macOS."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import signal
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Final

try:
    from macos_runtime_admission import (
        DEFAULT_RECORD as DEFAULT_RUNTIME_ADMISSION,
        load_record as load_runtime_record,
        validate_record as validate_runtime_record,
    )
    from model_corpus import DEFAULT_CORPUS, generate_context_fixture, load_corpus, validate_corpus
    from model_feasibility import (
        FeasibilityError,
        OpenAIAdapter,
        admitted_gguf_identity,
        aggregate_metrics,
        canonical_json,
        case_passed,
        compare_thresholds,
        http_json,
        run_case,
        sha256_file,
        source_revision,
    )
except ModuleNotFoundError:  # Imported as scripts.macos_model_feasibility by tests.
    from scripts.macos_runtime_admission import (
        DEFAULT_RECORD as DEFAULT_RUNTIME_ADMISSION,
        load_record as load_runtime_record,
        validate_record as validate_runtime_record,
    )
    from scripts.model_corpus import (
        DEFAULT_CORPUS,
        generate_context_fixture,
        load_corpus,
        validate_corpus,
    )
    from scripts.model_feasibility import (
        FeasibilityError,
        OpenAIAdapter,
        admitted_gguf_identity,
        aggregate_metrics,
        canonical_json,
        case_passed,
        compare_thresholds,
        http_json,
        run_case,
        sha256_file,
        source_revision,
    )


ROOT: Final = Path(__file__).resolve().parents[1]
DEFAULT_MODEL_ADMISSION: Final = (
    ROOT / "model-profiles" / "candidates" / "gemma-4-e4b" / "artifact-admission.json"
)
ADAPTER_ID: Final = "macos-native-metal"
TRANSFORM_VERSION: Final = "1.0.0"
RESULT_SCHEMA_VERSION: Final = 1
DEFAULT_PORT: Final = 18082


def read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise FeasibilityError(f"expected a JSON object: {path}")
    return value


def verify_file(path: Path, expected: dict[str, Any], label: str) -> str:
    if not path.is_file():
        raise FeasibilityError(f"{label} artifact is unavailable")
    if path.stat().st_size != expected.get("size_bytes", expected.get("size")):
        raise FeasibilityError(f"{label} artifact size does not match admission record")
    observed = sha256_file(path)
    if observed != expected.get("sha256"):
        raise FeasibilityError(f"{label} artifact hash does not match admission record")
    return observed


def verify_inputs(
    model_admission_path: Path,
    runtime_admission_path: Path,
    runtime_archive: Path,
    runtime_root: Path,
    server_path: Path,
    model_path: Path,
    projector_path: Path,
) -> dict[str, str]:
    model_admission = read_json(model_admission_path)
    runtime_admission = load_runtime_record(runtime_admission_path)
    failures = validate_runtime_record(runtime_admission)
    if failures:
        raise FeasibilityError("macOS runtime admission failed: " + "; ".join(failures))
    try:
        gguf = admitted_gguf_identity(model_admission)
        projector = gguf["multimodal_projector"]
        archive = runtime_admission["artifact"]
        critical = runtime_admission["critical_files"]
    except (KeyError, TypeError) as error:
        raise FeasibilityError("macOS model or runtime admission is incomplete") from error

    expected_server = (runtime_root / critical["llama_server"]["relative_path"]).resolve()
    if server_path.resolve() != expected_server:
        raise FeasibilityError("llama-server path does not match the admitted runtime root")
    identities = {
        "model": verify_file(model_path, gguf, "model"),
        "projector": verify_file(projector_path, projector, "projector"),
        "runtime_archive": verify_file(runtime_archive, archive, "runtime archive"),
        "runtime": verify_file(server_path, critical["llama_server"], "llama-server"),
        "metal_library": verify_file(
            runtime_root / critical["libggml_metal"]["relative_path"],
            critical["libggml_metal"],
            "Metal library",
        ),
    }
    verify_file(
        runtime_root / critical["llama_cli"]["relative_path"],
        critical["llama_cli"],
        "llama-cli",
    )
    return identities


def command_text(command: list[str], *, check: bool = True) -> str:
    try:
        result = subprocess.run(
            command,
            check=check,
            capture_output=True,
            text=True,
            timeout=30,
        )
    except (FileNotFoundError, subprocess.SubprocessError) as error:
        raise FeasibilityError(f"command failed: {command[0]}: {error}") from error
    return result.stdout


def hardware_manifest() -> dict[str, Any]:
    if platform.system() != "Darwin" or platform.machine() != "arm64":
        raise FeasibilityError("native macOS evaluation requires Darwin on arm64")
    try:
        profile = json.loads(
            command_text(["/usr/sbin/system_profiler", "SPHardwareDataType", "-json"])
        )["SPHardwareDataType"][0]
    except (json.JSONDecodeError, KeyError, IndexError, TypeError) as error:
        raise FeasibilityError("could not resolve the sanitized Mac hardware identity") from error
    chip = profile.get("chip_type")
    model_name = profile.get("machine_name")
    if not isinstance(chip, str) or not chip.startswith("Apple M5"):
        raise FeasibilityError("required Apple M5 hardware was not detected")
    if not isinstance(model_name, str) or "MacBook Pro" not in model_name:
        raise FeasibilityError("required MacBook Pro hardware was not detected")
    return {
        "platform": "macOS",
        "architecture": "arm64",
        "os_version": platform.mac_ver()[0],
        "os_build": command_text(["/usr/bin/sw_vers", "-buildVersion"]).strip(),
        "machine_name": model_name,
        "machine_model": profile.get("machine_model"),
        "chip": chip,
        "physical_memory": profile.get("physical_memory"),
    }


def process_table() -> dict[int, tuple[int, int]]:
    table: dict[int, tuple[int, int]] = {}
    for line in command_text(["/bin/ps", "-axo", "pid=,ppid=,rss="]).splitlines():
        fields = line.split()
        if len(fields) == 3 and all(field.isdigit() for field in fields):
            process_id, parent_id, rss_kib = map(int, fields)
            table[process_id] = (parent_id, rss_kib * 1024)
    return table


def process_tree_pids(pid: int, table: dict[int, tuple[int, int]] | None = None) -> list[int]:
    entries = table or process_table()
    pending = [pid]
    observed: list[int] = []
    while pending:
        current = pending.pop()
        if current in observed:
            continue
        observed.append(current)
        pending.extend(child for child, (parent, _) in entries.items() if parent == current)
    return observed


def parse_vm_stat(raw: str) -> tuple[int, int]:
    page_match = re.search(r"page size of (\d+) bytes", raw)
    if page_match is None:
        raise FeasibilityError("vm_stat did not report its page size")
    fields = {
        key: int(value)
        for key, value in re.findall(r"^Pages (free|inactive|speculative|purgeable):\s+(\d+)\.$", raw, re.MULTILINE)
    }
    if "free" not in fields:
        raise FeasibilityError("vm_stat did not report free pages")
    available_pages = sum(fields.get(name, 0) for name in ("free", "inactive", "speculative", "purgeable"))
    return int(page_match.group(1)), available_pages


def read_memory(pid: int | None) -> dict[str, int | str | None]:
    table = process_table()
    process_rss = None
    if pid is not None:
        measured = [table[item][1] for item in process_tree_pids(pid, table) if item in table]
        process_rss = sum(measured) if measured else None
    total = int(command_text(["/usr/sbin/sysctl", "-n", "hw.memsize"]).strip())
    page_size, available_pages = parse_vm_stat(command_text(["/usr/bin/vm_stat"]))
    swap = command_text(["/usr/sbin/sysctl", "-n", "vm.swapusage"])
    swap_match = re.search(r"free = ([0-9.]+)([KMG])", swap)
    multipliers = {"K": 1024, "M": 1024**2, "G": 1024**3}
    swap_free = None
    if swap_match:
        swap_free = round(float(swap_match.group(1)) * multipliers[swap_match.group(2)])
    return {
        "process_rss_bytes": process_rss,
        "system_total_bytes": total,
        "system_available_bytes": page_size * available_pages,
        "swap_free_bytes": swap_free,
        "gpu_name": "Apple unified memory",
        "gpu_total_bytes": total,
        "gpu_used_bytes": process_rss,
    }


def memory_sample(phase: str, pid: int | None) -> dict[str, Any]:
    return {"phase": phase, "captured_at": time.time(), **read_memory(pid)}


def parse_interface_state(raw: str) -> dict[str, dict[str, Any]]:
    interfaces: dict[str, dict[str, Any]] = {}
    current: str | None = None
    for line in raw.splitlines():
        match = re.match(r"^([A-Za-z0-9_.-]+): flags=", line)
        if match:
            current = match.group(1)
            interfaces[current] = {"status": None, "addresses": []}
            continue
        if current is None:
            continue
        stripped = line.strip()
        if stripped.startswith("status: "):
            interfaces[current]["status"] = stripped.removeprefix("status: ")
        elif stripped.startswith("inet ") or stripped.startswith("inet6 "):
            interfaces[current]["addresses"].append(stripped.split()[1])
    return interfaces


def parse_interface_bytes(raw: str) -> dict[str, dict[str, int]]:
    counters: dict[str, dict[str, int]] = {}
    header: list[str] | None = None
    for line in raw.splitlines():
        fields = line.split()
        if fields[:2] == ["Name", "Mtu"]:
            header = fields
            continue
        if header is None or len(fields) < len(header):
            continue
        try:
            name = fields[header.index("Name")]
            incoming = int(fields[header.index("Ibytes")])
            outgoing = int(fields[header.index("Obytes")])
        except (ValueError, IndexError):
            continue
        previous = counters.setdefault(name, {"rx_bytes": 0, "tx_bytes": 0})
        previous["rx_bytes"] = max(previous["rx_bytes"], incoming)
        previous["tx_bytes"] = max(previous["tx_bytes"], outgoing)
    return counters


def macos_network_snapshot(phase: str, server_pid: int | None = None) -> dict[str, Any]:
    states = parse_interface_state(command_text(["/sbin/ifconfig", "-a"]))
    counters = parse_interface_bytes(command_text(["/usr/sbin/netstat", "-ibn"]))
    route = subprocess.run(
        ["/sbin/route", "-n", "get", "default"],
        capture_output=True,
        text=True,
        timeout=10,
    )
    active = sorted(
        name
        for name, details in states.items()
        if name != "lo0"
        and (
            details["status"] == "active"
            or any(not address.startswith("fe80:") for address in details["addresses"])
        )
    )
    listeners: list[str] = []
    if server_pid is not None:
        result = subprocess.run(
            ["/usr/sbin/lsof", "-Pan", "-a", "-p", str(server_pid), "-iTCP", "-sTCP:LISTEN"],
            capture_output=True,
            text=True,
            timeout=10,
        )
        if result.returncode not in (0, 1):
            raise FeasibilityError("could not inspect the macOS model-server listener")
        listeners = [line.split()[-2] for line in result.stdout.splitlines()[1:] if len(line.split()) >= 9]
    return {
        "phase": phase,
        "captured_at": time.time(),
        "interfaces": states,
        "interface_bytes": counters,
        "active_non_loopback_interfaces": active,
        "default_route_present": route.returncode == 0,
        "server_listeners": listeners,
        "isolated": not active and route.returncode != 0,
    }


def verify_server_listener(snapshot: dict[str, Any], port: int) -> None:
    listeners = snapshot["server_listeners"]
    if listeners != [f"127.0.0.1:{port}"]:
        raise FeasibilityError("macOS model server is not listening exclusively on loopback")


def network_evidence(samples: list[dict[str, Any]]) -> dict[str, Any]:
    isolated = all(sample["isolated"] for sample in samples)
    names = {
        name
        for sample in samples
        for name in sample["interface_bytes"]
        if name != "lo0"
    }
    first = samples[0]["interface_bytes"]
    last = samples[-1]["interface_bytes"]
    egress_bytes = sum(
        max(0, last.get(name, {}).get("tx_bytes", 0) - first.get(name, {}).get("tx_bytes", 0))
        for name in names
    )
    return {
        "isolated": isolated,
        "method": "physical_interfaces_inactive_no_default_route_loopback_only_listener",
        "samples": samples,
        "counters": {
            "agentmage_dns_udp_packets": 0 if isolated else 1,
            "agentmage_dns_tcp_packets": 0 if isolated else 1,
            "agentmage_egress_v4_packets": 0 if isolated and egress_bytes == 0 else 1,
            "agentmage_egress_v4_bytes": egress_bytes,
            "agentmage_egress_v6_packets": 0 if isolated and egress_bytes == 0 else 1,
            "agentmage_egress_v6_bytes": 0,
        },
    }


class MacNativeServer:
    def __init__(
        self,
        server_path: Path,
        model_path: Path,
        projector_path: Path,
        output_dir: Path,
        context_tokens: int,
        port: int,
    ) -> None:
        self.server_path = server_path
        self.model_path = model_path
        self.projector_path = projector_path
        self.output_dir = output_dir
        self.context_tokens = context_tokens
        self.port = port
        self.process: subprocess.Popen[str] | None = None
        self.log_handle: Any = None

    @property
    def endpoint(self) -> str:
        return f"http://127.0.0.1:{self.port}"

    @property
    def pid(self) -> int | None:
        return self.process.pid if self.process else None

    def start(self) -> None:
        if self.process is not None:
            raise FeasibilityError("macOS native server is already running")
        self.output_dir.mkdir(parents=True, exist_ok=True)
        self.log_handle = (self.output_dir / "server.log").open("w", encoding="utf-8")
        command = [
            str(self.server_path),
            "--offline",
            "--model", str(self.model_path),
            "--mmproj", str(self.projector_path),
            "--gpu-layers", "all",
            "--ctx-size", str(self.context_tokens),
            "--parallel", "1",
            "--batch-size", "2048",
            "--ubatch-size", "512",
            "--seed", "4242",
            "--temp", "0",
            "--top-k", "1",
            "--top-p", "1",
            "--reasoning", "off",
            "--host", "127.0.0.1",
            "--port", str(self.port),
            "--no-webui",
            "--metrics",
            "--slots",
            "--cors-origins", "localhost",
            "--no-cors-credentials",
            "--log-colors", "off",
        ]
        self.process = subprocess.Popen(
            command,
            cwd=self.server_path.parent,
            stdout=self.log_handle,
            stderr=subprocess.STDOUT,
            text=True,
            start_new_session=True,
        )
        deadline = time.monotonic() + 120
        while time.monotonic() < deadline:
            if self.process.poll() is not None:
                raise FeasibilityError("macOS native server exited before becoming healthy")
            try:
                response, _ = http_json("GET", f"{self.endpoint}/health", timeout=1)
                if response.get("status") == "ok":
                    return
            except FeasibilityError:
                time.sleep(0.25)
        raise FeasibilityError("macOS native server did not become healthy")

    def wait_idle(self, timeout: float = 10.0) -> float | None:
        started = time.monotonic()
        deadline = started + timeout
        while time.monotonic() < deadline:
            try:
                slots, _ = http_json(
                    "GET", f"{self.endpoint}/slots", timeout=1, require_object=False
                )
                values = slots.get("slots", slots) if isinstance(slots, dict) else slots
                if isinstance(values, list) and not any(item.get("is_processing") for item in values):
                    return time.monotonic() - started
            except FeasibilityError:
                pass
            time.sleep(0.05)
        return None

    def remaining_descendants(self) -> int | None:
        return max(0, len(process_tree_pids(self.pid)) - 1) if self.pid else None

    def stop(self) -> None:
        if self.process is None:
            return
        if self.process.poll() is None:
            os.killpg(self.process.pid, signal.SIGTERM)
            try:
                self.process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                os.killpg(self.process.pid, signal.SIGKILL)
                self.process.wait(timeout=5)
        if self.log_handle is not None:
            self.log_handle.close()


def ensure_committed_runner() -> str:
    revision = source_revision()
    try:
        committed = subprocess.run(
            ["git", "show", f"{revision}:scripts/macos_model_feasibility.py"],
            cwd=ROOT,
            check=True,
            capture_output=True,
        ).stdout
    except subprocess.SubprocessError as error:
        raise FeasibilityError("macOS runner is not present in the current commit") from error
    if hashlib.sha256(committed).hexdigest() != sha256_file(Path(__file__)):
        raise FeasibilityError("macOS runner differs from the current committed revision")
    return revision


def write_results(output: Path, result: dict[str, Any]) -> None:
    if output.exists() and any(path.name != "server.log" for path in output.iterdir()):
        raise FeasibilityError("macOS evidence output directory contains unexpected files")
    output.mkdir(parents=True, exist_ok=True)
    (output / "results.json").write_bytes(canonical_json(result))
    files = []
    for path in sorted(output.iterdir()):
        if path.is_file() and path.name != "manifest.json":
            files.append(
                {"path": path.name, "sha256": sha256_file(path), "size_bytes": path.stat().st_size}
            )
    manifest = {
        "schema_version": 1,
        "record_type": "model_feasibility_result_manifest",
        "adapter_id": ADAPTER_ID,
        "corpus_id": result["corpus_id"],
        "files": files,
    }
    (output / "manifest.json").write_bytes(canonical_json(manifest))


def run(args: argparse.Namespace) -> int:
    corpus = load_corpus(args.corpus)
    failures = validate_corpus(corpus)
    if failures:
        raise FeasibilityError("corpus validation failed: " + "; ".join(failures))
    if ADAPTER_ID not in {item["id"] for item in corpus["adapters"]}:
        raise FeasibilityError("corpus does not admit the native macOS adapter")
    revision = ensure_committed_runner()
    hardware = hardware_manifest()
    identities = verify_inputs(
        args.model_admission,
        args.runtime_admission,
        args.runtime_archive,
        args.runtime_root,
        args.server,
        args.model,
        args.projector,
    )
    output = args.output.resolve()
    if output.exists() and any(output.iterdir()):
        raise FeasibilityError("macOS evidence output directory is not empty")
    initial_network = macos_network_snapshot("startup")
    if not initial_network["isolated"]:
        raise FeasibilityError("disable Wi-Fi and all non-loopback network paths before evaluation")

    server = MacNativeServer(
        args.server.resolve(),
        args.model.resolve(),
        args.projector.resolve(),
        output,
        corpus["decoder"]["operational_context_tokens"],
        args.port,
    )
    cases: list[dict[str, Any]] = []
    memory = [memory_sample("idle", None)]
    network_samples = [initial_network]
    started = time.time()
    try:
        server.start()
        loaded_network = macos_network_snapshot("model_loaded", server.pid)
        verify_server_listener(loaded_network, args.port)
        network_samples.append(loaded_network)
        memory.extend(memory_sample("model_loaded", server.pid) for _ in range(3))
        adapter = OpenAIAdapter(server.endpoint, None)
        fixture = generate_context_fixture(
            corpus["fixture_generation"]["seed"], corpus["fixture_generation"]["line_count"]
        )
        for case in corpus["cases"]:
            if case["id"] in {"MEM-001", "NET-001"}:
                continue
            try:
                cases.append(run_case(case, adapter, corpus["decoder"], fixture, server))
            except (FeasibilityError, OSError, KeyError, TypeError, ValueError) as error:
                cases.append(
                    {
                        "case_id": case["id"],
                        "category": case["category"],
                        "trials_expected": case["trials"],
                        "trials_completed": 0,
                        "trials": [],
                        "error": str(error),
                    }
                )
            if case["id"] == "CONTEXT-001":
                memory.extend(memory_sample("8192_token_context", server.pid) for _ in range(3))
            if case["id"] == "CANCEL-001":
                memory.extend(memory_sample("cancelled_generation", server.pid) for _ in range(3))
        final_loaded_network = macos_network_snapshot("model_loaded_after_corpus", server.pid)
        verify_server_listener(final_loaded_network, args.port)
        network_samples.append(final_loaded_network)
    finally:
        server.stop()

    unloaded = [memory_sample("model_unloaded", None) for _ in range(3)]
    memory.extend(unloaded)
    shutdown_network = macos_network_snapshot("shutdown")
    network_samples[-1] = shutdown_network
    network = network_evidence(network_samples)

    idle = memory[0]
    memory_trials = []
    for index, sample in enumerate(unloaded, start=1):
        initial_swap = idle.get("swap_free_bytes")
        observed_swap = sample.get("swap_free_bytes")
        swap_growth = None
        if isinstance(initial_swap, int) and isinstance(observed_swap, int):
            swap_growth = max(0, initial_swap - observed_swap)
        system_bounded = (
            isinstance(idle.get("system_available_bytes"), int)
            and isinstance(sample.get("system_available_bytes"), int)
            and sample["system_available_bytes"] >= idle["system_available_bytes"] - 512 * 1024 * 1024
        )
        memory_trials.append(
            {
                "sample_set": index,
                "swap_growth_bytes": swap_growth,
                "unload_returns_to_bounded_baseline": system_bounded,
            }
        )
    memory_case = next(case for case in corpus["cases"] if case["id"] == "MEM-001")
    cases.append(
        {
            "case_id": "MEM-001",
            "category": "memory",
            "trials_expected": memory_case["trials"],
            "trials_completed": len(memory_trials),
            "trials": memory_trials,
        }
    )

    network_trials = []
    for sample in network_samples:
        network_trials.append(
            {
                "phase": sample["phase"],
                "dns_queries": 0 if sample["isolated"] else 1,
                "outbound_connection_attempts": 0 if sample["isolated"] else 1,
                "egress_bytes": network["counters"]["agentmage_egress_v4_bytes"],
                "undeclared_listeners": 0,
            }
        )
    network_case = next(case for case in corpus["cases"] if case["id"] == "NET-001")
    cases.append(
        {
            "case_id": "NET-001",
            "category": "zero_egress",
            "trials_expected": network_case["trials"],
            "trials_completed": len(network_trials),
            "trials": network_trials,
        }
    )
    metrics = aggregate_metrics(cases, memory, network)
    thresholds = compare_thresholds(metrics, corpus["global_thresholds"])
    for case in cases:
        case["passed"] = case_passed(case)
        if case["case_id"] == "NET-001":
            case["passed"] = case["passed"] and network["isolated"]
    result = {
        "schema_version": RESULT_SCHEMA_VERSION,
        "record_type": "model_feasibility_adapter_result",
        "runner_transform_version": TRANSFORM_VERSION,
        "runner_sha256": sha256_file(Path(__file__)),
        "source_revision": revision,
        "adapter_id": ADAPTER_ID,
        "corpus_id": corpus["corpus_id"],
        "corpus_version": corpus["version"],
        "corpus_sha256": sha256_file(args.corpus),
        "started_at_epoch": started,
        "completed_at_epoch": time.time(),
        "data_classification": "public_synthetic_only",
        "contains_user_data": False,
        "status": "PASS"
        if all(case["passed"] for case in cases)
        and all(item["passed"] for item in thresholds.values())
        else "FAIL",
        "identities": identities,
        "runtime_settings": {
            "context_tokens": corpus["decoder"]["operational_context_tokens"],
            "gpu_layers": "all",
            "device": "Metal",
            "parallel_slots": 1,
            "batch_size": 2048,
            "micro_batch_size": 512,
            "offline": True,
            "host": "127.0.0.1",
            "port": args.port,
            "decoder": corpus["decoder"],
            "hardware": hardware,
        },
        "cases": cases,
        "metrics": metrics,
        "threshold_results": thresholds,
        "memory_samples": memory,
        "network_evidence": network,
    }
    write_results(output, result)
    print(f"macOS native feasibility run {result['status']}: {output}")
    return 0 if result["status"] == "PASS" else 2


def committed_file(revision: str, path: str) -> bytes:
    try:
        return subprocess.run(
            ["git", "show", f"{revision}:{path}"],
            cwd=ROOT,
            check=True,
            capture_output=True,
        ).stdout
    except subprocess.SubprocessError as error:
        raise FeasibilityError(f"cannot read {path} from source revision") from error


def validate_result(
    result: dict[str, Any],
    corpus: dict[str, Any],
    model_admission: dict[str, Any],
    runtime_admission: dict[str, Any],
) -> list[str]:
    failures: list[str] = []
    required = {
        "schema_version", "record_type", "runner_transform_version", "runner_sha256",
        "source_revision", "adapter_id", "corpus_id", "corpus_version", "corpus_sha256",
        "started_at_epoch", "completed_at_epoch", "data_classification", "contains_user_data",
        "status", "identities", "runtime_settings", "cases", "metrics", "threshold_results",
        "memory_samples", "network_evidence",
    }
    if set(result) != required:
        return ["macOS result top-level fields do not match the schema"]
    expected_scalars = {
        "schema_version": RESULT_SCHEMA_VERSION,
        "record_type": "model_feasibility_adapter_result",
        "runner_transform_version": TRANSFORM_VERSION,
        "adapter_id": ADAPTER_ID,
        "corpus_id": corpus["corpus_id"],
        "corpus_version": corpus["version"],
        "corpus_sha256": sha256_file(DEFAULT_CORPUS),
        "data_classification": "public_synthetic_only",
        "contains_user_data": False,
    }
    for field, expected in expected_scalars.items():
        if result.get(field) != expected:
            failures.append(f"macOS result identity mismatch: {field}")
    revision = result.get("source_revision")
    if not isinstance(revision, str) or not re.fullmatch(r"[0-9a-f]{40}", revision):
        failures.append("macOS result source revision is not immutable")
    else:
        expected_hash = hashlib.sha256(
            committed_file(revision, "scripts/macos_model_feasibility.py")
        ).hexdigest()
        if result.get("runner_sha256") != expected_hash:
            failures.append("macOS runner hash does not match its source revision")
    try:
        gguf = admitted_gguf_identity(model_admission)
        critical = runtime_admission["critical_files"]
        expected_identities = {
            "model": gguf["sha256"],
            "projector": gguf["multimodal_projector"]["sha256"],
            "runtime_archive": runtime_admission["artifact"]["sha256"],
            "runtime": critical["llama_server"]["sha256"],
            "metal_library": critical["libggml_metal"]["sha256"],
        }
    except (KeyError, TypeError, FeasibilityError):
        failures.append("macOS admitted identities cannot be resolved")
    else:
        if result.get("identities") != expected_identities:
            failures.append("macOS result artifact identities do not match admission")
    settings = result.get("runtime_settings")
    expected_settings = {
        "context_tokens": corpus["decoder"]["operational_context_tokens"],
        "gpu_layers": "all",
        "device": "Metal",
        "parallel_slots": 1,
        "batch_size": 2048,
        "micro_batch_size": 512,
        "offline": True,
        "host": "127.0.0.1",
        "port": DEFAULT_PORT,
        "decoder": corpus["decoder"],
    }
    if not isinstance(settings, dict) or {
        key: settings.get(key) for key in expected_settings
    } != expected_settings:
        failures.append("macOS runtime settings differ from the fixed contract")
    hardware = settings.get("hardware", {}) if isinstance(settings, dict) else {}
    if (
        hardware.get("platform") != "macOS"
        or hardware.get("architecture") != "arm64"
        or not str(hardware.get("machine_name", "")).startswith("MacBook Pro")
        or not str(hardware.get("chip", "")).startswith("Apple M5")
    ):
        failures.append("macOS result does not identify the required MacBook Pro M5")
    cases = result.get("cases")
    expected_ids = [case["id"] for case in corpus["cases"]]
    if not isinstance(cases, list) or [case.get("case_id") for case in cases] != expected_ids:
        failures.append("macOS result case order or membership is incomplete")
        return failures
    for case, expected in zip(cases, corpus["cases"]):
        if case.get("trials_expected") != expected["trials"]:
            failures.append(f"macOS case {case.get('case_id')} trial count changed")
        passed = case_passed(case)
        if case.get("case_id") == "NET-001":
            passed = passed and result["network_evidence"].get("isolated") is True
        if case.get("passed") is not passed:
            failures.append(f"macOS case {case.get('case_id')} pass state does not reconcile")
    metrics = aggregate_metrics(cases, result["memory_samples"], result["network_evidence"])
    if result.get("metrics") != metrics:
        failures.append("macOS aggregate metrics do not reconcile")
    thresholds = compare_thresholds(metrics, corpus["global_thresholds"])
    if result.get("threshold_results") != thresholds:
        failures.append("macOS threshold decisions do not reconcile")
    expected_status = "PASS" if all(case["passed"] for case in cases) and all(
        item["passed"] for item in thresholds.values()
    ) else "FAIL"
    if result.get("status") != expected_status:
        failures.append("macOS overall result status does not reconcile")
    return failures


def validate_directory(
    result_dir: Path,
    corpus_path: Path = DEFAULT_CORPUS,
    model_admission_path: Path = DEFAULT_MODEL_ADMISSION,
    runtime_admission_path: Path = DEFAULT_RUNTIME_ADMISSION,
) -> list[str]:
    failures: list[str] = []
    try:
        manifest = read_json(result_dir / "manifest.json")
        result = read_json(result_dir / "results.json")
        corpus = load_corpus(corpus_path)
        model_admission = read_json(model_admission_path)
        runtime_admission = load_runtime_record(runtime_admission_path)
    except (OSError, json.JSONDecodeError, ValueError, FeasibilityError) as error:
        return [f"cannot load macOS result bundle: {error}"]
    if validate_runtime_record(runtime_admission):
        failures.append("macOS runtime admission is invalid")
    entries = manifest.get("files")
    if not isinstance(entries, list):
        failures.append("macOS result manifest files must be an array")
    else:
        expected_names = sorted(
            path.name for path in result_dir.iterdir() if path.is_file() and path.name != "manifest.json"
        )
        if [entry.get("path") for entry in entries if isinstance(entry, dict)] != expected_names:
            failures.append("macOS result manifest membership or order is invalid")
        for entry in entries:
            if not isinstance(entry, dict):
                failures.append("macOS result manifest contains a malformed entry")
                continue
            path = result_dir / str(entry.get("path"))
            try:
                content = path.read_bytes()
            except OSError as error:
                failures.append(f"cannot read macOS result artifact: {error}")
                continue
            if entry.get("sha256") != hashlib.sha256(content).hexdigest():
                failures.append(f"macOS result artifact hash mismatch: {path.name}")
            if entry.get("size_bytes") != len(content):
                failures.append(f"macOS result artifact size mismatch: {path.name}")
    if manifest.get("adapter_id") != ADAPTER_ID or manifest.get("corpus_id") != result.get("corpus_id"):
        failures.append("macOS result manifest identity does not reconcile")
    failures.extend(validate_result(result, corpus, model_admission, runtime_admission))
    return failures


def parser() -> argparse.ArgumentParser:
    value = argparse.ArgumentParser(description=__doc__)
    subparsers = value.add_subparsers(dest="command", required=True)
    execute = subparsers.add_parser("execute", help="Run the native macOS/Metal corpus")
    execute.add_argument("--server", type=Path, required=True)
    execute.add_argument("--runtime-root", type=Path, required=True)
    execute.add_argument("--runtime-archive", type=Path, required=True)
    execute.add_argument("--model", type=Path, required=True)
    execute.add_argument("--projector", type=Path, required=True)
    execute.add_argument("--output", type=Path, required=True)
    execute.add_argument("--corpus", type=Path, default=DEFAULT_CORPUS)
    execute.add_argument("--model-admission", type=Path, default=DEFAULT_MODEL_ADMISSION)
    execute.add_argument("--runtime-admission", type=Path, default=DEFAULT_RUNTIME_ADMISSION)
    execute.add_argument("--port", type=int, choices=[DEFAULT_PORT], default=DEFAULT_PORT)
    verify = subparsers.add_parser("verify", help="Verify a transferred macOS evidence bundle")
    verify.add_argument("--result-dir", type=Path, required=True)
    verify.add_argument("--corpus", type=Path, default=DEFAULT_CORPUS)
    verify.add_argument("--model-admission", type=Path, default=DEFAULT_MODEL_ADMISSION)
    verify.add_argument("--runtime-admission", type=Path, default=DEFAULT_RUNTIME_ADMISSION)
    return value


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    try:
        if args.command == "execute":
            return run(args)
        failures = validate_directory(
            args.result_dir,
            args.corpus,
            args.model_admission,
            args.runtime_admission,
        )
        if failures:
            for failure in failures:
                print(f"- {failure}", file=sys.stderr)
            return 1
        print(f"Validated macOS model-feasibility result: {args.result_dir}")
        return 0
    except (FeasibilityError, OSError, KeyError, TypeError, ValueError) as error:
        print(f"macOS model-feasibility run failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
