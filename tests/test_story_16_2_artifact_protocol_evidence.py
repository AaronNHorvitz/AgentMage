import importlib.util
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts/story_16_2_artifact_protocol_evidence.py"
REPORT = ROOT / "artifacts/sprints/sprint-16/story-16.2/artifact-protocol-report.json"


def module():
    spec = importlib.util.spec_from_file_location("story_16_2_evidence", SCRIPT)
    assert spec and spec.loader
    loaded = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(loaded)
    return loaded


def test_report_is_source_bound_and_truthful():
    report = json.loads(REPORT.read_text(encoding="utf-8"))
    assert report["status"] == "PASS_LOCAL_PROTOCOL_AND_FAKE_BACKEND"
    assert report["catalog"] == [
        "artifact.list", "artifact.metadata", "artifact.read", "artifact.range",
        "artifact.sections", "artifact.search",
    ]
    truth = report["product_truth"]
    assert truth["workspace_mutation_count"] == 0
    assert truth["network_access_count"] == 0
    assert truth["parser_launch_count"] == 0
    assert truth["future_extension_registered_count"] == 0
    assert not truth["production_source_artifact_execution"]
    assert not truth["installed_worker_execution"]
    for artifact in report["artifacts"]:
        path = ROOT / artifact["path"]
        assert path.stat().st_size == artifact["byte_length"]
        assert module().digest(path) == artifact["sha256"]


def test_generator_declares_all_required_local_gates():
    loaded = module()
    assert len(loaded.COMMANDS) == 4
    assert len(loaded.MARKERS) == 9
    assert "--all-targets" in loaded.COMMANDS[3]
    assert "-D" in loaded.COMMANDS[3]
