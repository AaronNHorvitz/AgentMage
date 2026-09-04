import copy,json,unittest
from unittest.mock import patch
from scripts.desktop_experience_contract import CORPUS,validate

class DesktopExperienceContractTests(unittest.TestCase):
    def test_contract(self): self.assertEqual(validate(),[])
    def test_truth_mutations_fail(self):
        source=json.loads(CORPUS.read_text())
        original=type(CORPUS).read_text
        for field,value in (("native_desktop_evidence",True),("platform_support",True),("substitution_set",["linux-contract"]),("case_count",41)):
            mutated=copy.deepcopy(source); mutated[field]=value
            def read_text(path,*args,**kwargs):
                if path==CORPUS: return json.dumps(mutated)
                return original(path,*args,**kwargs)
            with patch("pathlib.Path.read_text",read_text): self.assertTrue(validate())

if __name__=="__main__": unittest.main()
