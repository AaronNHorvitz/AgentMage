import unittest
from scripts import connector_mutation_contract as contract
class ConnectorMutationContractTests(unittest.TestCase):
 def test_current(self):self.assertEqual(contract.validate(),[])
 def test_missing(self):
  original=contract.REQUIRED
  try:contract.REQUIRED=(*original,"MissingConnectorMutationBoundary");self.assertIn("connector mutation boundary absent: MissingConnectorMutationBoundary",contract.validate())
  finally:contract.REQUIRED=original
if __name__=="__main__":unittest.main()
