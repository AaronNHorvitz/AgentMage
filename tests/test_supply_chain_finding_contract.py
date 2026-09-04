import json,unittest
from scripts.supply_chain_finding_contract import ATTACKS,DOCUMENTS,TOOLS,build
class SupplyChainFindingContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_documents_preserve_exact_evidence(self):self.assertEqual(tuple(self.value["documents"]),DOCUMENTS);self.assertTrue(all(c["producer_bound"] and c["subject_bound"] and not c["assurance_overclaim_count"] for c in self.value["document_cases"]))
 def test_findings_preserve_conflict(self):self.assertEqual(tuple(self.value["tools"]),TOOLS);self.assertTrue(all(c["conflict_preserved"] and c["severity"]!=c["conflicting_severity"] and c["text_untrusted"] for c in self.value["finding_cases"]))
 def test_attacks_fail_closed(self):self.assertEqual(tuple(self.value["attacks"]),ATTACKS);self.assertTrue(all(not c["hidden_blocking_finding_count"] and not c["accepted_stale_evidence_count"] and not c["release_state_change_count"] for c in self.value["attack_cases"]));self.assertEqual(self.value["promoted_tool_count"],0)
if __name__=="__main__":unittest.main()
