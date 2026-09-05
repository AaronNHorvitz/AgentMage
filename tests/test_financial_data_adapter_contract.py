import json,unittest
from scripts.financial_data_adapter_contract import build
class FinancialDataAdapterContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_dimensions(self):self.assertEqual(self.value["case_count"],4320);self.assertEqual(len(self.value["classes"]),6)
 def test_only_exact_reads_occur(self):self.assertTrue(all(case["read_count"]==0 for case in self.value["cases"]if case["mutation"]!="none"or case["failure"]!="success"))
 def test_prohibited_families_are_closed(self):self.assertEqual(len(self.value["prohibited"]),13);self.assertEqual(self.value["prohibited_representation_count"],0)
 def test_coverage_is_always_visible(self):self.assertTrue(all(case["coverage_visible"]for case in self.value["cases"]))
 def test_no_write_disclosure_credential_or_residue(self):self.assertEqual(sum(self.value[key]for key in ("write_count","disclosure_count","credential_exposure_count","removal_authority_count")),0)
if __name__=="__main__":unittest.main()
