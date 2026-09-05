import json,unittest
from scripts.financial_document_contract import build
class FinancialDocumentContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_dimensions(self):self.assertEqual(self.value["case_count"],6720);self.assertEqual(len(self.value["kinds"]),5);self.assertEqual(len(self.value["copies"]),7)
 def test_source_evidence_is_always_visible(self):self.assertTrue(all(case["source_hash_visible"]and case["coordinates_visible"]and case["confidence_visible"]and case["parser_identity_visible"]for case in self.value["cases"]))
 def test_mutation_and_hostility_fail_closed(self):self.assertTrue(all(case["accepted_count"]==0 for case in self.value["cases"]if case["mutation"]!="none"or case["failure"]!="success"))
 def test_matching_remains_distinct_and_reversible(self):self.assertTrue(all(case["records_distinct"]and case["reversible"]for case in self.value["cases"]));self.assertEqual(self.value["silent_merge_count"],0)
 def test_no_authority_escape_residue_or_citation_loss(self):self.assertEqual(sum(self.value[key]for key in ("authority_count","escape_count","residue_count","citation_loss_count")),0)
if __name__=="__main__":unittest.main()
