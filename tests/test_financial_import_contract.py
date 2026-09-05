import json,unittest
from scripts.financial_import_contract import build
class FinancialImportContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_dimensions(self):self.assertEqual(self.value["case_count"],8400);self.assertEqual(self.value["formats"],["csv","ofx","qfx"])
 def test_ambiguous_inputs_never_guess(self):self.assertTrue(all(case["guessed_field_count"]==0 for case in self.value["cases"]if case["failure"]!="success"))
 def test_reimport_is_stable_and_duplicate_free(self):self.assertTrue(all(case["byte_stable"] for case in self.value["cases"]));self.assertEqual(self.value["duplicate_record_count"],0)
 def test_sources_and_partial_state_are_immutable(self):self.assertEqual(self.value["source_mutation_count"]+self.value["partial_state_count"],0)
 def test_failures_are_visible(self):self.assertTrue(all(case["conflict_visible"] for case in self.value["cases"]if case["failure"]!="success"))
if __name__=="__main__":unittest.main()
