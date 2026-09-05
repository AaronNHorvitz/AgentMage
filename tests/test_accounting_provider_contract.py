import json,unittest
from scripts.accounting_provider_contract import build
class T(unittest.TestCase):
 def setUp(self):self.v=json.loads(build())
 def test_dimensions(self):self.assertEqual(self.v["case_count"],13440);self.assertEqual(len(self.v["providers"]),2)
 def test_exact(self):self.assertTrue(all(c["organization_exact"]and c["period_exact"]and c["currency_exact"]and c["tax_exact"] for c in self.v["cases"]))
 def test_fail_closed(self):self.assertTrue(all(not c["accepted"] for c in self.v["cases"] if c["mutation"]!="none"or c["failure"]!="success"))
 def test_no_effects(self):self.assertEqual(sum(self.v[k]for k in ("wrong_write_count","duplicate_write_count","money_movement_count","residual_authority_count")),0)
if __name__=="__main__":unittest.main()
