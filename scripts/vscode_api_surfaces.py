#!/usr/bin/env python3
"""Validate the recorded Visual Studio Code API surfaces for Decision 0042.

Decision 0042 section 4 requires that proposed and private APIs are never required
for the supported path, and that any experiment using one carries a feature flag, a
version guard, a fallback, and a separate non-support claim. This validator makes both
requirements checkable rather than aspirational.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
SURFACES_PATH: Final = ROOT / "architecture" / "vscode-api-surfaces.json"

SUPPORTED_PATH: Final = "agentmage-chat-participant"
# Exactly one channel may carry the supported path.
PERMITTED_SUPPORTED_PATH_CHANNELS: Final = {"stable"}
EXPECTED_CHANNELS: Final = ["stable", "preview", "proposed", "private"]
EXPECTED_EXPERIMENT_CONTROLS: Final = [
    "feature-flag",
    "version-guard",
    "fallback",
    "separate-non-support-claim",
]
SUPPORT_CLASSES: Final = {"guaranteed", "best-effort"}
EXPECTED_NATIVE_COMPATIBILITY: Final = {
    "minimum_vscode_version": "1.125.0",
    "extension_engine": "^1.125.0",
    "stable_input_roles": ["user", "assistant"],
    "stable_input_parts": ["text"],
    "explicitly_unsupported_parts": [
        "data",
        "image",
        "tool_call",
        "tool_result",
        "unknown",
    ],
    "external_tool_proposals": "unsupported-transition-to-verified-chat",
    "model_options": "unsupported-transition-to-verified-chat",
    "route_disclosure": "requested-strict-local-profile-runtime-no-fallback",
    "usage_disclosure": "exact-byte-and-part-counts-token-usage-unavailable",
    "token_count": "conservative-utf8-byte-upper-bound",
    "persistent_lifecycle": "verified-chat-only",
    "complete_inspector": "verified-chat-only",
    "transition_command": "agentmage.openVerifiedChat",
    "provider_claim": "stable-text-only-compatibility-not-agent-host",
}
# The stable participant path AgentMage guarantees.
GUARANTEED_SUPPORTED_PATH_SURFACES: Final = {
    "chat-participant",
    "chat-request-references",
    "language-model-chat-provider",
    "chat-progress-and-cancellation",
}
# Compatibility AgentMage offers on a best-effort basis only.
BEST_EFFORT_SURFACES: Final = {"label-provider"}
SURFACE_KEYS: Final = {
    "id",
    "channel",
    "support",
    "supported_path_member",
    "degradation",
    "absence_disclosed",
}


class SurfaceError(RuntimeError):
    """Raised when the recorded surfaces are malformed."""


def load_surfaces(path: Path = SURFACES_PATH) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _validate_channels(record: Any, failures: list[str]) -> dict[str, Any]:
    channels = record.get("channels")
    if not isinstance(channels, list):
        failures.append("channels must be an array")
        return {}

    indexed: dict[str, Any] = {}
    for channel in channels:
        if not isinstance(channel, dict) or not isinstance(channel.get("id"), str):
            failures.append("every channel must have a string id")
            continue
        if channel["id"] in indexed:
            failures.append(f"duplicate channel id: {channel['id']}")
        indexed[channel["id"]] = channel

    if list(indexed) != EXPECTED_CHANNELS:
        failures.append(
            "channels must record exactly stable, preview, proposed, and private in order"
        )

    for channel_id, channel in indexed.items():
        permitted = channel.get("permitted_for_supported_path")
        expected_permitted = channel_id in PERMITTED_SUPPORTED_PATH_CHANNELS
        if permitted is not expected_permitted:
            failures.append(
                f"channel {channel_id} permitted_for_supported_path must be {expected_permitted}"
            )
        # Every non-stable channel is an experiment and needs the full control set.
        if channel.get("requires_experiment_controls") is not (not expected_permitted):
            failures.append(
                f"channel {channel_id} requires_experiment_controls must be "
                f"{not expected_permitted}"
            )
    return indexed


def _validate_surfaces(record: Any, channels: dict[str, Any], failures: list[str]) -> None:
    surfaces = record.get("surfaces")
    if not isinstance(surfaces, list):
        failures.append("surfaces must be an array")
        return

    indexed: dict[str, Any] = {}
    for surface in surfaces:
        if not isinstance(surface, dict) or not isinstance(surface.get("id"), str):
            failures.append("every surface must have a string id")
            continue
        surface_id = surface["id"]
        if surface_id in indexed:
            failures.append(f"duplicate surface id: {surface_id}")
        indexed[surface_id] = surface

        unknown = set(surface) - SURFACE_KEYS
        if unknown:
            failures.append(
                f"surface {surface_id} has unknown keys: " + ", ".join(sorted(unknown))
            )
        if surface.get("channel") not in channels:
            failures.append(f"surface {surface_id} names an unrecorded channel")
        if surface.get("support") not in SUPPORT_CLASSES:
            failures.append(f"surface {surface_id} must declare guaranteed or best-effort support")
        if surface.get("absence_disclosed") is not True:
            failures.append(f"surface {surface_id} must disclose its absence")

        # A guarantee can only rest on a stable API.
        if surface.get("support") == "guaranteed":
            if surface.get("channel") not in PERMITTED_SUPPORTED_PATH_CHANNELS:
                failures.append(
                    f"surface {surface_id} cannot be guaranteed on a non-stable channel"
                )
            if surface.get("degradation") != "none":
                failures.append(f"guaranteed surface {surface_id} must not declare degradation")

        # Best-effort compatibility must say what happens when it is unavailable.
        if surface.get("support") == "best-effort":
            if surface.get("supported_path_member") is not False:
                failures.append(
                    f"best-effort surface {surface_id} must not be on the supported path"
                )
            degradation = surface.get("degradation")
            if not isinstance(degradation, str) or degradation in {"", "none"}:
                failures.append(
                    f"best-effort surface {surface_id} must declare its degradation"
                )

        # A member of the supported path can never sit on an experimental channel.
        if surface.get("supported_path_member") is True:
            channel = channels.get(surface.get("channel"), {})
            if channel.get("permitted_for_supported_path") is not True:
                failures.append(
                    f"surface {surface_id} cannot carry the supported path on its channel"
                )

    guaranteed = {
        surface_id
        for surface_id, surface in indexed.items()
        if surface.get("support") == "guaranteed" and surface.get("supported_path_member") is True
    }
    if guaranteed != GUARANTEED_SUPPORTED_PATH_SURFACES:
        failures.append("the guaranteed stable participant path is incomplete or widened")

    best_effort = {
        surface_id
        for surface_id, surface in indexed.items()
        if surface.get("support") == "best-effort"
    }
    if best_effort != BEST_EFFORT_SURFACES:
        failures.append("the best-effort compatibility set is incomplete or widened")


def validate_surfaces(record: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(record, dict):
        return ["vscode api surfaces must be an object"]

    if record.get("schema_version") != 2:
        failures.append("schema_version must equal 2")
    if record.get("decision_id") != "ADR-0042":
        failures.append("decision_id must equal ADR-0042")
    if record.get("status") != "accepted":
        failures.append("status must equal accepted")
    if record.get("supported_path") != SUPPORTED_PATH:
        failures.append(f"supported_path must equal {SUPPORTED_PATH}")
    if record.get("experiment_controls") != EXPECTED_EXPERIMENT_CONTROLS:
        failures.append(
            "experiment_controls must record a feature flag, version guard, fallback, "
            "and separate non-support claim"
        )
    if record.get("native_compatibility") != EXPECTED_NATIVE_COMPATIBILITY:
        failures.append(
            "native_compatibility must retain the exact stable text-only capability and limitation matrix"
        )

    channels = _validate_channels(record, failures)
    _validate_surfaces(record, channels, failures)
    return failures


def main() -> int:
    try:
        record = load_surfaces()
    except (OSError, json.JSONDecodeError) as error:
        print(f"vscode api surface validation failed: {error}", file=sys.stderr)
        return 1

    failures = validate_surfaces(record)
    if failures:
        for failure in failures:
            print(f"vscode api surface validation failed: {failure}", file=sys.stderr)
        return 1

    print("vscode api surface validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
