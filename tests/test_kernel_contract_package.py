from __future__ import annotations

import copy
import unittest

from scripts.kernel_contract_package import (
    DIRECT_DEPENDENCIES,
    PREFIX,
    PackageValidationError,
    safe_member_path,
    validate_manifest,
)


def valid_manifest() -> dict[str, object]:
    return {
        "package": {
            "name": "agentmage-kernel-contracts",
            "version": "0.0.0",
            "edition": "2024",
            "rust-version": "1.95.0",
            "license": "Apache-2.0",
            "repository": "https://github.com/AaronNHorvitz/AgentMage",
            "publish": False,
            "build": False,
        },
        "dependencies": copy.deepcopy(DIRECT_DEPENDENCIES),
        "lints": {"rust": {"unsafe_code": "forbid"}},
    }


class KernelContractPackageTests(unittest.TestCase):
    def test_member_paths_reject_escape_absolute_and_wrong_prefix(self) -> None:
        self.assertTrue(safe_member_path(f"{PREFIX}/src/lib.rs"))
        self.assertFalse(safe_member_path(f"{PREFIX}/../secret"))
        self.assertFalse(safe_member_path("/absolute/member"))
        self.assertFalse(safe_member_path("other-package/src/lib.rs"))

    def test_manifest_rejects_publish_build_script_and_dependency_broadening(self) -> None:
        for mutation in ("publish", "build", "dependency"):
            with self.subTest(mutation=mutation):
                candidate = valid_manifest()
                if mutation == "publish":
                    candidate["package"]["publish"] = True  # type: ignore[index]
                elif mutation == "build":
                    candidate["package"]["build"] = "build.rs"  # type: ignore[index]
                else:
                    candidate["dependencies"]["network-client"] = {  # type: ignore[index]
                        "version": "=1.0.0"
                    }
                with self.assertRaises(PackageValidationError):
                    validate_manifest(candidate)

    def test_manifest_accepts_only_the_closed_package_contract(self) -> None:
        validate_manifest(valid_manifest())


if __name__ == "__main__":
    unittest.main()
