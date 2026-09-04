import unittest
from scripts import scheduled_authority_contract as contract
class ScheduledAuthorityContractTests(unittest.TestCase):
 def test_current(self):self.assertEqual(contract.validate(),[])
 def test_missing(self):
  original=contract.REQUIRED
  try:contract.REQUIRED=(*original,"MissingScheduledAuthorityBoundary");self.assertIn("scheduled-authority boundary absent: MissingScheduledAuthorityBoundary",contract.validate())
  finally:contract.REQUIRED=original
if __name__=="__main__":unittest.main()
