"""Typed deterministic adapters for synthetic AgentMage tests."""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass
from enum import Enum
from typing import Any


class FakeMode(str, Enum):
    SUCCESS = "success"
    DENIAL = "denial"
    MALFORMED = "malformed-result"
    CANCELLATION = "cancellation"
    TIMEOUT = "timeout"
    CRASH_BEFORE = "crash-before-commit"
    CRASH_AFTER = "crash-after-commit"
    UNCERTAIN = "uncertain-result"


class FakeStatus(str, Enum):
    SUCCEEDED = "succeeded"
    DENIED = "denied"
    MALFORMED = "malformed"
    CANCELLED = "cancelled"
    TIMED_OUT = "timed-out"
    CRASHED = "crashed"
    UNCERTAIN = "uncertain"


@dataclass(frozen=True)
class FakeEvent:
    sequence: int
    adapter_id: str
    operation: str
    mode: FakeMode
    status: FakeStatus
    request_sha256: str
    committed: bool | None
    reason: str | None

    def as_record(self) -> dict[str, Any]:
        return {
            "sequence": self.sequence,
            "adapter_id": self.adapter_id,
            "operation": self.operation,
            "mode": self.mode.value,
            "status": self.status.value,
            "request_sha256": self.request_sha256,
            "committed": self.committed,
            "reason": self.reason,
        }


@dataclass(frozen=True)
class FakeOutcome:
    status: FakeStatus
    event: FakeEvent
    payload: Any = None


class SyntheticSecret:
    """A test-only value whose representation never reveals its bytes."""

    def __init__(self, value: str) -> None:
        if not value.startswith("AM_SYNTHETIC_SECRET_"):
            raise ValueError("fake secret store accepts synthetic secret values only")
        self.__value = value

    def reveal_for_test(self) -> str:
        return self.__value

    def sha256(self) -> str:
        return hashlib.sha256(self.__value.encode("utf-8")).hexdigest()

    def __repr__(self) -> str:
        return "<SyntheticSecret redacted>"

    __str__ = __repr__


def _normalized(value: Any) -> Any:
    if isinstance(value, SyntheticSecret):
        return {"synthetic_secret_sha256": value.sha256()}
    if isinstance(value, bytes):
        return {"bytes_sha256": hashlib.sha256(value).hexdigest(), "length": len(value)}
    if isinstance(value, dict):
        return {str(key): _normalized(item) for key, item in sorted(value.items())}
    if isinstance(value, (list, tuple)):
        return [_normalized(item) for item in value]
    if value is None or isinstance(value, (bool, int, float, str)):
        return value
    return {"type": type(value).__name__}


def request_sha256(value: Any) -> str:
    encoded = json.dumps(
        _normalized(value), sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


class ScenarioPlan:
    def __init__(self, scenarios: dict[str, list[FakeMode]] | None = None) -> None:
        self._scenarios = {
            key: list(modes) for key, modes in (scenarios or {}).items()
        }

    def next(self, adapter_id: str, operation: str) -> FakeMode:
        key = f"{adapter_id}.{operation}"
        modes = self._scenarios.get(key, [])
        return modes.pop(0) if modes else FakeMode.SUCCESS

    def pending(self) -> int:
        return sum(len(modes) for modes in self._scenarios.values())


class FakeAdapter:
    def __init__(self, adapter_id: str, plan: ScenarioPlan | None = None) -> None:
        self.adapter_id = adapter_id
        self.plan = plan or ScenarioPlan()
        self.events: list[FakeEvent] = []
        self.closed = False

    def _ensure_open(self) -> None:
        if self.closed:
            raise RuntimeError(f"{self.adapter_id} is closed")

    def _outcome(
        self,
        operation: str,
        request: Any,
        success_payload: Any,
        success_committed: bool = False,
    ) -> FakeOutcome:
        self._ensure_open()
        mode = self.plan.next(self.adapter_id, operation)
        mapping = {
            FakeMode.SUCCESS: (FakeStatus.SUCCEEDED, success_committed, None),
            FakeMode.DENIAL: (FakeStatus.DENIED, False, "synthetic-denial"),
            FakeMode.MALFORMED: (FakeStatus.MALFORMED, False, "synthetic-malformed-result"),
            FakeMode.CANCELLATION: (FakeStatus.CANCELLED, False, "synthetic-cancellation"),
            FakeMode.TIMEOUT: (FakeStatus.TIMED_OUT, False, "synthetic-timeout"),
            FakeMode.CRASH_BEFORE: (FakeStatus.CRASHED, False, "synthetic-crash-before"),
            FakeMode.CRASH_AFTER: (FakeStatus.CRASHED, True, "synthetic-crash-after"),
            FakeMode.UNCERTAIN: (FakeStatus.UNCERTAIN, None, "synthetic-uncertain-result"),
        }
        status, committed, reason = mapping[mode]
        event = FakeEvent(
            sequence=len(self.events) + 1,
            adapter_id=self.adapter_id,
            operation=operation,
            mode=mode,
            status=status,
            request_sha256=request_sha256(request),
            committed=committed,
            reason=reason,
        )
        self.events.append(event)
        payload = success_payload if status is FakeStatus.SUCCEEDED else None
        if status is FakeStatus.MALFORMED:
            payload = {"synthetic_malformed": True}
        return FakeOutcome(status=status, event=event, payload=payload)

    def _cleanup(self) -> None:
        pass

    def close(self) -> None:
        if not self.closed:
            self._cleanup()
            self.closed = True


class FakeModel(FakeAdapter):
    def __init__(self, plan: ScenarioPlan | None = None) -> None:
        super().__init__("fake-model", plan)

    def generate(self, prompt: str) -> FakeOutcome:
        digest = request_sha256({"prompt": prompt})
        return self._outcome(
            "generate",
            {"prompt": prompt},
            {"text": f"synthetic-model-response-{digest[:12]}", "citations": []},
        )


class FakeTool(FakeAdapter):
    def __init__(self, plan: ScenarioPlan | None = None) -> None:
        super().__init__("fake-tool", plan)

    def invoke(self, tool: str, arguments: dict[str, Any]) -> FakeOutcome:
        request = {"tool": tool, "arguments": arguments}
        return self._outcome(
            "invoke",
            request,
            {
                "tool": tool,
                "arguments_sha256": request_sha256(arguments),
                "side_effects": [],
            },
        )


class FakeInferenceRuntime(FakeAdapter):
    def __init__(self, plan: ScenarioPlan | None = None) -> None:
        super().__init__("fake-inference-runtime", plan)
        self.running = False

    def start(self, profile_id: str) -> FakeOutcome:
        outcome = self._outcome(
            "start", {"profile_id": profile_id}, {"profile_id": profile_id}, True
        )
        if outcome.status is FakeStatus.SUCCEEDED:
            self.running = True
        return outcome

    def infer(self, prompt: str) -> FakeOutcome:
        if not self.running:
            return self._forced_denial(
                "infer", {"prompt": prompt, "running": False}, "runtime-not-running"
            )
        digest = request_sha256({"prompt": prompt})
        return self._outcome(
            "infer",
            {"prompt": prompt, "running": True},
            {"text": f"synthetic-runtime-response-{digest[:12]}"},
        )

    def _forced_denial(self, operation: str, request: Any, reason: str) -> FakeOutcome:
        self._ensure_open()
        event = FakeEvent(
            sequence=len(self.events) + 1,
            adapter_id=self.adapter_id,
            operation=operation,
            mode=FakeMode.DENIAL,
            status=FakeStatus.DENIED,
            request_sha256=request_sha256(request),
            committed=False,
            reason=reason,
        )
        self.events.append(event)
        return FakeOutcome(status=FakeStatus.DENIED, event=event)

    def stop(self) -> FakeOutcome:
        outcome = self._outcome("stop", {"running": self.running}, {"stopped": True}, True)
        if outcome.status is FakeStatus.SUCCEEDED:
            self.running = False
        return outcome

    def _cleanup(self) -> None:
        self.running = False


class FakeConnector(FakeAdapter):
    def __init__(self, plan: ScenarioPlan | None = None) -> None:
        super().__init__("fake-connector", plan)
        self.records = {
            "record-alpha": {"title": "Synthetic Alpha", "classification": "public-fixture"},
            "record-beta": {"title": "Synthetic Beta", "classification": "public-fixture"},
        }

    def list(self) -> FakeOutcome:
        return self._outcome("list", {}, {"ids": sorted(self.records)})

    def read(self, record_id: str) -> FakeOutcome:
        if record_id not in self.records:
            return self._forced_missing(record_id)
        return self._outcome(
            "read", {"record_id": record_id}, dict(self.records[record_id])
        )

    def _forced_missing(self, record_id: str) -> FakeOutcome:
        self._ensure_open()
        event = FakeEvent(
            sequence=len(self.events) + 1,
            adapter_id=self.adapter_id,
            operation="read",
            mode=FakeMode.DENIAL,
            status=FakeStatus.DENIED,
            request_sha256=request_sha256({"record_id": record_id}),
            committed=False,
            reason="synthetic-record-not-found",
        )
        self.events.append(event)
        return FakeOutcome(status=FakeStatus.DENIED, event=event)


class FakeClock(FakeAdapter):
    def __init__(
        self,
        epoch: int = 1704067200,
        plan: ScenarioPlan | None = None,
    ) -> None:
        super().__init__("fake-clock", plan)
        self._epoch = epoch
        self._now = epoch

    def _record_clock(self, operation: str, request: Any) -> None:
        self._ensure_open()
        self.events.append(
            FakeEvent(
                sequence=len(self.events) + 1,
                adapter_id=self.adapter_id,
                operation=operation,
                mode=FakeMode.SUCCESS,
                status=FakeStatus.SUCCEEDED,
                request_sha256=request_sha256(request),
                committed=False,
                reason=None,
            )
        )

    def now(self) -> int:
        self._record_clock("now", {})
        return self._now

    def advance(self, seconds: int) -> int:
        if seconds < 0:
            raise ValueError("fake clock cannot move backwards")
        self._now += seconds
        self._record_clock("advance", {"seconds": seconds})
        return self._now

    def probe(self) -> FakeOutcome:
        return self._outcome("probe", {"epoch": self._now}, {"epoch": self._now})

    @property
    def current(self) -> int:
        return self._now

    def _cleanup(self) -> None:
        self._now = self._epoch


class FakeSecretStore(FakeAdapter):
    def __init__(self, plan: ScenarioPlan | None = None) -> None:
        super().__init__("fake-secret-store", plan)
        self._values: dict[str, SyntheticSecret] = {}

    def store(self, handle: str, value: SyntheticSecret) -> FakeOutcome:
        outcome = self._outcome(
            "store",
            {"handle": handle, "value": value},
            {"handle": handle, "secret_sha256": value.sha256()},
            True,
        )
        if outcome.status is FakeStatus.SUCCEEDED:
            self._values[handle] = value
        return outcome

    def load(self, handle: str) -> FakeOutcome:
        if handle not in self._values:
            return self._missing(handle, "load")
        return self._outcome(
            "load", {"handle": handle}, self._values[handle], False
        )

    def delete(self, handle: str) -> FakeOutcome:
        if handle not in self._values:
            return self._missing(handle, "delete")
        outcome = self._outcome("delete", {"handle": handle}, {"deleted": True}, True)
        if outcome.status is FakeStatus.SUCCEEDED:
            del self._values[handle]
        return outcome

    def _missing(self, handle: str, operation: str) -> FakeOutcome:
        self._ensure_open()
        event = FakeEvent(
            sequence=len(self.events) + 1,
            adapter_id=self.adapter_id,
            operation=operation,
            mode=FakeMode.DENIAL,
            status=FakeStatus.DENIED,
            request_sha256=request_sha256({"handle": handle}),
            committed=False,
            reason="synthetic-secret-not-found",
        )
        self.events.append(event)
        return FakeOutcome(status=FakeStatus.DENIED, event=event)

    @property
    def stored_count(self) -> int:
        return len(self._values)

    def _cleanup(self) -> None:
        self._values.clear()


class CrashInjector(FakeAdapter):
    def __init__(self, plan: ScenarioPlan | None = None) -> None:
        super().__init__("crash-injector", plan)

    def checkpoint(self, name: str) -> FakeOutcome:
        return self._outcome(
            "checkpoint", {"name": name}, {"checkpoint": name}, False
        )


def trace_sha256(adapters: list[Any]) -> str:
    records = [
        event.as_record()
        for adapter in adapters
        for event in adapter.events
    ]
    encoded = json.dumps(records, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def baseline_trace() -> dict[str, Any]:
    model = FakeModel()
    tool = FakeTool()
    runtime = FakeInferenceRuntime()
    connector = FakeConnector()
    clock = FakeClock()
    secrets = FakeSecretStore()
    crash = CrashInjector()
    model.generate("synthetic prompt")
    tool.invoke("fixture-read", {"path": "synthetic.txt"})
    runtime.start("fixture-profile")
    runtime.infer("synthetic prompt")
    runtime.stop()
    connector.list()
    connector.read("record-alpha")
    clock.now()
    clock.advance(60)
    secret = SyntheticSecret("AM_SYNTHETIC_SECRET_BASELINE")
    secrets.store("fixture-handle", secret)
    secrets.load("fixture-handle")
    secrets.delete("fixture-handle")
    crash.checkpoint("fixture-checkpoint")
    adapters = [model, tool, runtime, connector, clock, secrets, crash]
    records = [
        event.as_record()
        for adapter in adapters
        for event in adapter.events
    ]
    for adapter in adapters:
        adapter.close()
    return {
        "events": records,
        "event_count": len(records),
        "trace_sha256": trace_sha256(adapters),
        "raw_secret_retained": False,
        "all_adapters_closed": all(adapter.closed for adapter in adapters),
    }
