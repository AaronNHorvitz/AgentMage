import copy,json,unittest
from unittest.mock import patch
from scripts.capability_package_runtime_contract import CORPUS,validate
class CapabilityPackageRuntimeContractTests(unittest.TestCase):
    def test_contract(self):self.assertEqual(validate(),[])
    def test_truth_mutations_fail(self):
        source=json.loads(CORPUS.read_text());original=type(CORPUS).read_text
        for field,value in (("native_lifecycle_complete",True),("safe_mode_startup_observed",True),("independent_review_present",True),("installed_package_count",1),("enabled_package_count",1),("substitution_set",["contract"]),("case_count",47)):
            mutated=copy.deepcopy(source);mutated[field]=value
            def read_text(path,*args,**kwargs):
                if path==CORPUS:return json.dumps(mutated)
                return original(path,*args,**kwargs)
            with patch("pathlib.Path.read_text",read_text):self.assertTrue(validate())
if __name__=="__main__":unittest.main()
