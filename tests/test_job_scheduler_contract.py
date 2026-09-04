import unittest
from scripts import job_scheduler_contract as contract
class JobSchedulerContractTests(unittest.TestCase):
 def test_current(self):self.assertEqual(contract.validate(),[])
 def test_missing(self):
  original=contract.REQUIRED
  try:contract.REQUIRED=(*original,"MissingJobSchedulerBoundary");self.assertIn("job scheduler boundary absent: MissingJobSchedulerBoundary",contract.validate())
  finally:contract.REQUIRED=original
if __name__=="__main__":unittest.main()
