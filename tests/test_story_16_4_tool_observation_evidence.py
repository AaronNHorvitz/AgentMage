import hashlib
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
REPORT = ROOT / "artifacts/sprints/sprint-16/story-16.4/tool-observation-report.json"


def test_story_16_4_report_is_source_bound_and_truthful():
    report = json.loads(REPORT.read_text(encoding="utf-8"))
    assert report["status"] == "PASS_LOCAL_CLOSED_OBSERVATION"
    assert report["terminal_disposition_count"] == 10
    assert report["output_artifact_families"] == ["stdout", "stderr", "binary", "structured"]
    truth = report["product_truth"]
    assert truth["exactly_once_terminal_contract_complete"]
    assert truth["atomic_content_addressed_output_contract_complete"]
    assert truth["live_local_process_executed"]
    assert truth["false_completion_count"] == 0
    assert not truth["installed_worker_campaign_executed"]
    for artifact in report["artifacts"]:
        path = ROOT / artifact["path"]
        assert path.stat().st_size == artifact["byte_length"]
        assert hashlib.sha256(path.read_bytes()).hexdigest() == artifact["sha256"]


def test_story_16_4_retains_all_terminal_boundary_markers():
    markers = json.loads(REPORT.read_text(encoding="utf-8"))["required_markers"]
    assert len(markers) == 10
    assert any("all_ten_terminal_states" in marker for marker in markers)
    assert any("resource_ceiling" in marker for marker in markers)
    assert any("live_local_process" in marker for marker in markers)
    assert any("incomplete_cleanup" in marker for marker in markers)
    assert any("restart_restore" in marker for marker in markers)
