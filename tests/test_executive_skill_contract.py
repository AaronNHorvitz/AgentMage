from __future__ import annotations

import copy
import unittest

from scripts import executive_skill_contract as contract


def fixture() -> dict[str, object]:
    manifests = []
    for skill in contract.SKILLS:
        workflow = f"executive-{skill.replace('_', '-')}"
        manifests.append(
            {
                "skill_id": f"skill-{workflow}",
                "trust_state": "admitted",
                "version": "0.6.0",
                "files": [
                    {"kind": "prompt"},
                    {"kind": "template"},
                ],
            }
        )
    return {
        "schema_version": 1,
        "record_type": "agentmage-executive-skill-pack",
        "version": "0.6.0",
        "skills": list(contract.SKILLS),
        "manifests": manifests,
        "prompt_count": len(contract.SKILLS),
        "template_count": len(contract.SKILLS),
        "authority": {
            "filesystem": False,
            "shell": False,
            "secrets": False,
            "network": False,
            "connectors": False,
            "approvals": False,
            "grant_creation": False,
            "tool_registration": False,
            "code_execution": False,
            "workspace_expansion": False,
            "memory_promotion": False,
            "writes": False,
        },
        **{field: False for field in contract.ZERO_EFFECT_FIELDS},
    }


class ExecutiveSkillContractTests(unittest.TestCase):
    def test_exact_pack_is_valid(self) -> None:
        self.assertEqual(contract.validate(fixture()), [])

    def test_every_effect_and_authority_overclaim_fails(self) -> None:
        for field in contract.ZERO_EFFECT_FIELDS:
            changed = copy.deepcopy(fixture())
            changed[field] = True
            self.assertTrue(contract.validate(changed), field)
        changed = copy.deepcopy(fixture())
        changed["authority"]["network"] = True
        self.assertTrue(contract.validate(changed))

    def test_inventory_manifest_and_file_drift_fails(self) -> None:
        mutations = (
            lambda value: value["skills"].pop(),
            lambda value: value["manifests"].pop(),
            lambda value: value["manifests"][0].update({"trust_state": "disabled"}),
            lambda value: value["manifests"][0]["files"].reverse(),
            lambda value: value.update({"template_count": 7}),
        )
        for mutate in mutations:
            changed = copy.deepcopy(fixture())
            mutate(changed)
            self.assertTrue(contract.validate(changed))


if __name__ == "__main__":
    unittest.main()
