"""Deterministic, side-effect-free fault adapters for synthetic tests."""

from __future__ import annotations

import hashlib
import json
import re
from dataclasses import dataclass
from enum import Enum
from typing import Any, Mapping
from urllib.parse import urlsplit


RESOURCE_DIMENSIONS = (
    "cpu_steps",
    "memory_bytes",
    "disk_bytes",
    "token_count",
    "file_count",
    "output_bytes",
)
IDENTIFIER = re.compile(r"^[a-z0-9][a-z0-9._-]{0,127}$")
HASH = re.compile(r"^[0-9a-f]{64}$")


class FaultStatus(str, Enum):
    SUCCEEDED = "succeeded"
    CRASHED = "crashed"
    UNCERTAIN = "uncertain"
    DENIED = "denied"
    FAILED = "failed"
    TIMED_OUT = "timed-out"
    PARTIAL = "partial"
    UNAVAILABLE = "unavailable"
    RESOURCE_EXHAUSTED = "resource-exhausted"
    REJECTED = "rejected"
    QUARANTINED = "quarantined"
    REDACTED = "redacted"
    CANCELLED = "cancelled"


class CrashMode(str, Enum):
    HEALTHY = "healthy"
    BEFORE_OPERATION = "crash-before-operation"
    BEFORE_COMMIT = "crash-before-commit"
    AFTER_COMMIT = "crash-after-commit"
    UNCERTAIN_COMMIT = "uncertain-commit"


class NetworkMode(str, Enum):
    ALLOWLISTED_SUCCESS = "allowlisted-success"
    POLICY_DENIAL = "policy-denial"
    DNS_FAILURE = "dns-failure"
    CONNECT_TIMEOUT = "connect-timeout"
    READ_TIMEOUT = "read-timeout"
    CONNECTION_RESET = "connection-reset"
    PARTIAL_RESPONSE = "partial-response"
    OFFLINE = "offline"


class AdversarialDisposition(str, Enum):
    REJECT = "reject"
    QUARANTINE = "quarantine"
    REDACT = "redact"


@dataclass(frozen=True)
class FaultEvent:
    sequence: int
    adapter_id: str
    operation: str
    mode: str
    status: FaultStatus
    request_sha256: str
    logical_duration_ms: int
    committed: bool | None
    detail: str | None
    side_effects: tuple[str, ...] = ()

    def as_record(self) -> dict[str, Any]:
        return {
            "sequence": self.sequence,
            "adapter_id": self.adapter_id,
            "operation": self.operation,
            "mode": self.mode,
            "status": self.status.value,
            "request_sha256": self.request_sha256,
            "logical_duration_ms": self.logical_duration_ms,
            "committed": self.committed,
            "detail": self.detail,
            "side_effects": list(self.side_effects),
        }


@dataclass(frozen=True)
class FaultOutcome:
    status: FaultStatus
    event: FaultEvent
    metadata: Mapping[str, Any]


class CancellationToken:
    def __init__(self) -> None:
        self._cancelled = False

    def cancel(self) -> None:
        self._cancelled = True

    @property
    def cancelled(self) -> bool:
        return self._cancelled


def _normalized(value: Any) -> Any:
    if isinstance(value, bytes):
        return {"bytes": len(value), "sha256": hashlib.sha256(value).hexdigest()}
    if isinstance(value, Mapping):
        return {str(key): _normalized(item) for key, item in sorted(value.items())}
    if isinstance(value, (list, tuple)):
        return [_normalized(item) for item in value]
    if value is None or isinstance(value, (bool, int, float, str)):
        return value
    return {"type": type(value).__name__}


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        _normalized(value), sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("utf-8")


def request_sha256(value: Any) -> str:
    return hashlib.sha256(canonical_json(value)).hexdigest()


def _bounded_detail(value: str | None, maximum: int) -> str | None:
    if value is None:
        return None
    encoded = value.encode("utf-8")
    if len(encoded) <= maximum:
        return value
    retained = encoded[:maximum]
    while True:
        try:
            return retained.decode("utf-8")
        except UnicodeDecodeError:
            retained = retained[:-1]


class FaultAdapter:
    def __init__(
        self,
        adapter_id: str,
        *,
        max_events: int = 64,
        max_detail_bytes: int = 128,
    ) -> None:
        if max_events < 1 or max_detail_bytes < 1:
            raise ValueError("fault adapter bounds must be positive")
        self.adapter_id = adapter_id
        self.max_events = max_events
        self.max_detail_bytes = max_detail_bytes
        self.events: list[FaultEvent] = []
        self.closed = False

    def _ensure_open(self) -> None:
        if self.closed:
            raise RuntimeError(f"{self.adapter_id} is closed")
        if len(self.events) >= self.max_events:
            raise RuntimeError(f"{self.adapter_id} event bound reached")

    def _record(
        self,
        *,
        operation: str,
        mode: str,
        status: FaultStatus,
        request: Any,
        logical_duration_ms: int,
        committed: bool | None,
        detail: str | None,
        metadata: Mapping[str, Any] | None = None,
    ) -> FaultOutcome:
        self._ensure_open()
        event = FaultEvent(
            sequence=len(self.events) + 1,
            adapter_id=self.adapter_id,
            operation=operation,
            mode=mode,
            status=status,
            request_sha256=request_sha256(request),
            logical_duration_ms=logical_duration_ms,
            committed=committed,
            detail=_bounded_detail(detail, self.max_detail_bytes),
        )
        self.events.append(event)
        return FaultOutcome(status=status, event=event, metadata=metadata or {})

    def _cancelled(self, operation: str, request: Any) -> FaultOutcome:
        return self._record(
            operation=operation,
            mode="cancelled",
            status=FaultStatus.CANCELLED,
            request=request,
            logical_duration_ms=0,
            committed=False,
            detail="pre-cancelled",
        )

    def _cleanup(self) -> None:
        pass

    def close(self) -> None:
        if not self.closed:
            self._cleanup()
            self.closed = True


class CrashTestAdapter(FaultAdapter):
    def __init__(self, **bounds: int) -> None:
        super().__init__("crash-test-adapter", **bounds)

    def inject(
        self,
        checkpoint: str,
        mode: CrashMode,
        token: CancellationToken | None = None,
    ) -> FaultOutcome:
        if not IDENTIFIER.fullmatch(checkpoint):
            raise ValueError("crash checkpoint is invalid")
        request = {"checkpoint": checkpoint, "mode": mode.value}
        if token is not None and token.cancelled:
            return self._cancelled("inject", request)
        mapping = {
            CrashMode.HEALTHY: (FaultStatus.SUCCEEDED, False, None),
            CrashMode.BEFORE_OPERATION: (
                FaultStatus.CRASHED,
                False,
                "synthetic-crash-before-operation",
            ),
            CrashMode.BEFORE_COMMIT: (
                FaultStatus.CRASHED,
                False,
                "synthetic-crash-before-commit",
            ),
            CrashMode.AFTER_COMMIT: (
                FaultStatus.CRASHED,
                True,
                "synthetic-crash-after-commit",
            ),
            CrashMode.UNCERTAIN_COMMIT: (
                FaultStatus.UNCERTAIN,
                None,
                "synthetic-commit-state-unknown",
            ),
        }
        status, committed, detail = mapping[mode]
        return self._record(
            operation="inject",
            mode=mode.value,
            status=status,
            request=request,
            logical_duration_ms=1,
            committed=committed,
            detail=detail,
            metadata={"checkpoint": checkpoint},
        )


class NetworkTestAdapter(FaultAdapter):
    def __init__(self, allowlist: list[str], **bounds: int) -> None:
        super().__init__("network-test-adapter", **bounds)
        if not allowlist or any(not self._valid_endpoint(item) for item in allowlist):
            raise ValueError("network test allowlist is invalid")
        self.allowlist = frozenset(allowlist)

    @staticmethod
    def _valid_endpoint(endpoint: str) -> bool:
        parsed = urlsplit(endpoint)
        return (
            parsed.scheme == "https"
            and parsed.hostname is not None
            and parsed.hostname.endswith(".fixture.invalid")
            and parsed.username is None
            and parsed.password is None
            and not parsed.query
            and not parsed.fragment
        )

    def request(
        self,
        endpoint: str,
        mode: NetworkMode,
        token: CancellationToken | None = None,
    ) -> FaultOutcome:
        if not self._valid_endpoint(endpoint):
            raise ValueError("network test endpoint must use the synthetic invalid domain")
        request = {"endpoint": endpoint, "mode": mode.value}
        if token is not None and token.cancelled:
            return self._cancelled("request", request)
        if mode is NetworkMode.ALLOWLISTED_SUCCESS and endpoint not in self.allowlist:
            mode = NetworkMode.POLICY_DENIAL
        mapping = {
            NetworkMode.ALLOWLISTED_SUCCESS: (FaultStatus.SUCCEEDED, "synthetic-response"),
            NetworkMode.POLICY_DENIAL: (FaultStatus.DENIED, "endpoint-not-allowlisted"),
            NetworkMode.DNS_FAILURE: (FaultStatus.FAILED, "synthetic-dns-failure"),
            NetworkMode.CONNECT_TIMEOUT: (FaultStatus.TIMED_OUT, "synthetic-connect-timeout"),
            NetworkMode.READ_TIMEOUT: (FaultStatus.TIMED_OUT, "synthetic-read-timeout"),
            NetworkMode.CONNECTION_RESET: (FaultStatus.FAILED, "synthetic-connection-reset"),
            NetworkMode.PARTIAL_RESPONSE: (FaultStatus.PARTIAL, "synthetic-partial-response"),
            NetworkMode.OFFLINE: (FaultStatus.UNAVAILABLE, "synthetic-offline"),
        }
        status, detail = mapping[mode]
        metadata: dict[str, Any] = {"response_bytes": 0}
        if mode is NetworkMode.ALLOWLISTED_SUCCESS:
            metadata = {
                "response_bytes": 18,
                "response_sha256": request_sha256("synthetic-response"),
            }
        elif mode is NetworkMode.PARTIAL_RESPONSE:
            metadata = {
                "received_chunks": 1,
                "expected_chunks": 2,
                "response_bytes": 9,
            }
        return self._record(
            operation="request",
            mode=mode.value,
            status=status,
            request=request,
            logical_duration_ms=2,
            committed=False,
            detail=detail,
            metadata=metadata,
        )


class ResourceTestAdapter(FaultAdapter):
    def __init__(self, limits: Mapping[str, int], **bounds: int) -> None:
        super().__init__("resource-test-adapter", **bounds)
        if set(limits) != set(RESOURCE_DIMENSIONS) or any(
            isinstance(value, bool) or not isinstance(value, int) or value < 0
            for value in limits.values()
        ):
            raise ValueError("resource test limits are invalid")
        self.limits = {key: limits[key] for key in RESOURCE_DIMENSIONS}
        self.used = {key: 0 for key in RESOURCE_DIMENSIONS}

    @staticmethod
    def _valid_demand(demand: Mapping[str, int]) -> bool:
        return set(demand) == set(RESOURCE_DIMENSIONS) and all(
            not isinstance(value, bool) and isinstance(value, int) and value >= 0
            for value in demand.values()
        )

    def consume(
        self,
        case_id: str,
        demand: Mapping[str, int],
        token: CancellationToken | None = None,
    ) -> FaultOutcome:
        if not IDENTIFIER.fullmatch(case_id) or not self._valid_demand(demand):
            raise ValueError("resource test demand is invalid")
        normalized = {key: demand[key] for key in RESOURCE_DIMENSIONS}
        request = {"case_id": case_id, "demand": normalized}
        if token is not None and token.cancelled:
            return self._cancelled("consume", request)
        exhausted = next(
            (
                key
                for key in RESOURCE_DIMENSIONS
                if self.used[key] + normalized[key] > self.limits[key]
            ),
            None,
        )
        if exhausted is not None:
            return self._record(
                operation="consume",
                mode=f"resource-exhausted-{exhausted}",
                status=FaultStatus.RESOURCE_EXHAUSTED,
                request=request,
                logical_duration_ms=1,
                committed=False,
                detail=exhausted,
                metadata={"exhausted_dimension": exhausted},
            )
        for key in RESOURCE_DIMENSIONS:
            self.used[key] += normalized[key]
        return self._record(
            operation="consume",
            mode="within-budget",
            status=FaultStatus.SUCCEEDED,
            request=request,
            logical_duration_ms=1,
            committed=True,
            detail=None,
            metadata={
                "remaining": {
                    key: self.limits[key] - self.used[key] for key in RESOURCE_DIMENSIONS
                }
            },
        )

    def _cleanup(self) -> None:
        self.used = {key: 0 for key in RESOURCE_DIMENSIONS}


class AdversarialTestAdapter(FaultAdapter):
    def __init__(self, max_payload_metadata_bytes: int = 1048576, **bounds: int) -> None:
        super().__init__("adversarial-test-adapter", **bounds)
        if max_payload_metadata_bytes < 1:
            raise ValueError("adversarial metadata bound must be positive")
        self.max_payload_metadata_bytes = max_payload_metadata_bytes

    def inspect(
        self,
        *,
        case_id: str,
        archive_path: str,
        classification: str,
        disposition: AdversarialDisposition,
        payload_sha256: str,
        payload_bytes: int,
        token: CancellationToken | None = None,
    ) -> FaultOutcome:
        if not IDENTIFIER.fullmatch(case_id) or not IDENTIFIER.fullmatch(classification):
            raise ValueError("adversarial case identity is invalid")
        if not HASH.fullmatch(payload_sha256):
            raise ValueError("adversarial payload hash is invalid")
        if (
            isinstance(payload_bytes, bool)
            or not isinstance(payload_bytes, int)
            or payload_bytes < 0
            or payload_bytes > self.max_payload_metadata_bytes
        ):
            raise ValueError("adversarial payload metadata size is invalid")
        if archive_path.startswith("/") or ".." in archive_path.split("/"):
            raise ValueError("adversarial archive path is invalid")
        request = {
            "case_id": case_id,
            "archive_path": archive_path,
            "classification": classification,
            "disposition": disposition.value,
            "payload_sha256": payload_sha256,
            "payload_bytes": payload_bytes,
        }
        if token is not None and token.cancelled:
            return self._cancelled("inspect", request)
        status = {
            AdversarialDisposition.REJECT: FaultStatus.REJECTED,
            AdversarialDisposition.QUARANTINE: FaultStatus.QUARANTINED,
            AdversarialDisposition.REDACT: FaultStatus.REDACTED,
        }[disposition]
        return self._record(
            operation="inspect",
            mode=disposition.value,
            status=status,
            request=request,
            logical_duration_ms=1,
            committed=False,
            detail=f"synthetic-{disposition.value}",
            metadata={
                "classification": classification,
                "payload_bytes": payload_bytes,
                "payload_sha256": payload_sha256,
                "raw_payload_retained": False,
            },
        )


def trace_sha256(adapters: list[FaultAdapter]) -> str:
    records = [event.as_record() for adapter in adapters for event in adapter.events]
    return hashlib.sha256(canonical_json(records)).hexdigest()
