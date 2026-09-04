import copy,json,unittest
from unittest.mock import patch
from scripts.desktop_lifecycle_contract import CORPUS,validate

class DesktopLifecycleContractTests(unittest.TestCase):
    def test_contract(self): self.assertEqual(validate(),[])
    def test_truth_mutations_fail(self):
        source=json.loads(CORPUS.read_text()); original=type(CORPUS).read_text
        for field,value in (("signed_package_present",True),("native_accessibility_complete",True),("platform_support",True),("substitution_set",["contract"]),("case_count",37)):
            mutated=copy.deepcopy(source); mutated[field]=value
            def read_text(path,*args,**kwargs):
                if path==CORPUS: return json.dumps(mutated)
                return original(path,*args,**kwargs)
            with patch("pathlib.Path.read_text",read_text): self.assertTrue(validate())

if __name__=="__main__": unittest.main()
