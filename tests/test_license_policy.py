"""Licence drift must fail, including complete texts and excluded manifests."""

import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from scripts import license_policy as policy


class LicensePolicyTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.names = subprocess.check_output(['git', 'ls-files', '-z'], cwd=policy.ROOT).decode().split('\0')
        for name in self.names:
            if name.endswith('Cargo.toml') or name in {
                'kernel/contracts/LICENSE', 'Agent-Scaffolding-Inventory.md', 'package.json',
                'shells/vscode/package.json', 'package-lock.json', 'NOTICE', 'README.md',
                'packaging/linux/agentmage.spec.in', 'packaging/linux/agentmage-release.spec.in',
            }:
                self.copy(name)
        for name in [*policy.PINNED_TEXTS, 'CONTRIBUTING.md']:
            self.copy(name)
        (self.root / 'source.rs').write_text('// SPDX-License-Identifier: MIT OR Apache-2.0\n')
        self.names = [n for n in self.names if n.endswith('Cargo.toml')] + ['source.rs']
        self.discovery = patch.object(policy.subprocess, 'check_output', return_value='\0'.join(self.names).encode())
        self.discovery.start()
        self.addCleanup(self.discovery.stop)

    def copy(self, name):
        target = self.root / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(policy.ROOT / name, target)

    def test_current_policy_passes(self):
        self.assertEqual(policy.check(self.root), [])

    def test_truncated_or_modified_licence_and_historical_texts_fail(self):
        for name in [*policy.PINNED_TEXTS, 'kernel/contracts/LICENSE', 'Agent-Scaffolding-Inventory.md']:
            with self.subTest(name=name):
                path = self.root / name
                original = path.read_bytes()
                path.write_bytes(original[:-30])
                self.assertTrue(policy.check(self.root))
                path.write_bytes(original)

    def test_excluded_crate_workspace_and_editor_must_agree(self):
        for name in ['Cargo.toml', 'experimental/model-lab/Cargo.toml', 'shells/vscode/package.json',
                     'package.json', 'package-lock.json', 'source.rs']:
            with self.subTest(name=name):
                path = self.root / name
                original = path.read_text()
                self.assertIn(policy.EXPRESSION, original)
                path.write_text(original.replace(policy.EXPRESSION, 'Apache-2.0'))
                self.assertTrue(policy.check(self.root))
                path.write_text(original)

    def test_inherited_crate_cannot_override_licence(self):
        path = self.root / 'kernel/engine/Cargo.toml'
        path.write_text(path.read_text().replace('license.workspace = true', 'license = "BUSL-1.1"'))
        self.assertTrue(policy.check(self.root))

    def test_missing_signoff_or_history_is_rejected(self):
        for name, marker in [('CONTRIBUTING.md', 'Signed-off-by:'), ('NOTICE', 'relicensing commit')]:
            path = self.root / name
            path.write_text(path.read_text().replace(marker, 'removed'))
            self.assertTrue(policy.check(self.root))


if __name__ == '__main__':
    unittest.main()
