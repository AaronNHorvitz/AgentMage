import json,unittest
from scripts.release_decision_contract import CLASSES,STATES,build
class T(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_matrix(self):self.assertEqual((len(CLASSES),len(STATES),self.value["case_count"]),(20,9,180))
 def test_fail_closed(self):self.assertTrue(all(not x["publication_allowed"]and not x["ga_closed"]for x in self.value["cases"]));self.assertEqual(sum(x["substitution_count"]for x in self.value["cases"]),0)
 def test_truth(self):self.assertEqual(self.value["current_release_decision"],"BLOCKED")
if __name__=="__main__":unittest.main()
