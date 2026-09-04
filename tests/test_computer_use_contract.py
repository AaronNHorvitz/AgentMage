import unittest
from scripts import computer_use_contract as contract
class ComputerUseContractTests(unittest.TestCase):
 def test_current(self): self.assertEqual(contract.validate(),[])
 def test_missing(self):
  original=contract.REQUIRED
  try:
   contract.REQUIRED=(*original,"MissingComputerUseBoundary"); self.assertIn("computer-use boundary absent: MissingComputerUseBoundary",contract.validate())
  finally: contract.REQUIRED=original
if __name__=="__main__": unittest.main()
