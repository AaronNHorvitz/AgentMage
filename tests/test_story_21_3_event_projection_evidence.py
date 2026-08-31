import hashlib
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
REPORT = ROOT / "artifacts/sprints/sprint-21/story-21.3/event-projection-report.json"


def test_story_21_3_report_is_source_bound_and_truthful():
    report = json.loads(REPORT.read_text(encoding="utf-8"))
    assert report["status"] == "PASS_LOCAL_EVENT_PROJECTION"
    assert report["new_correctness_event_families"] == 13
    assert report["process_stop_cases"] == 30
    truth = report["product_truth"]
    assert truth["closed_schema_complete"]
    assert truth["deterministic_projection_complete"]
    assert truth["duplicate_or_invented_event_count"] == 0
    assert truth["large_or_sensitive_inline_payload_count"] == 0
    assert not truth["external_platform_campaign_executed"]
    assert not truth["independent_review_executed"]
    for artifact in report["artifacts"]:
        path = ROOT / artifact["path"]
        assert path.stat().st_size == artifact["byte_length"]
        assert hashlib.sha256(path.read_bytes()).hexdigest() == artifact["sha256"]


def test_story_21_3_retains_schema_replay_pressure_and_crash_markers():
    markers = json.loads(REPORT.read_text(encoding="utf-8"))["required_markers"]
    assert len(markers) == 12
    assert any("every_new_correctness_event" in marker for marker in markers)
    assert any("unsupported_kind" in marker for marker in markers)
    assert any("lagging_subscriber" in marker for marker in markers)
    assert any("projection event families" in marker for marker in markers)
