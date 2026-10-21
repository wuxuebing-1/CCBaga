#!/usr/bin/env python3
# [INPUT]: Synthetic installer bytes and temporary ZIPs, no network or Windows.
# [OUTPUT]: Equality, mismatch, ambiguous/missing member and traversal regressions.
# [POS]: Provenance checker tests; never executes an installer.
import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import zipfile

SCRIPT = Path(__file__).with_name('compare-windows-release.py')
spec = importlib.util.spec_from_file_location('compare', SCRIPT)
compare = importlib.util.module_from_spec(spec)
spec.loader.exec_module(compare)


class ComparisonTests(unittest.TestCase):
    def test_comparison_and_exit_status(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            release, artifact = root / 'release.exe', root / 'ci.zip'
            release.write_bytes(b'MZsame')
            with zipfile.ZipFile(artifact, 'w') as archive:
                archive.writestr('nested/setup.exe', b'MZsame')
            self.assertTrue(compare.compare(release, artifact, 'setup.exe')['identical'])
            for content, status in [(b'MZsame', 0), (b'MZchanged', 1)]:
                release.write_bytes(content)
                result = subprocess.run([sys.executable, str(SCRIPT), '--release', str(release), '--artifact', str(artifact), '--asset-name', 'setup.exe', '--output', str(root / 'result.json')], capture_output=True)
                self.assertEqual(result.returncode, status, result.stderr)
                self.assertTrue((root / 'result.json').is_file())

    def test_invalid_members_and_no_path_extraction(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            release, artifact = root / 'release.exe', root / 'ci.zip'
            release.write_bytes(b'MZ')
            for members in [[], ['other.exe'], ['setup.exe', 'other.exe'], ['setup.exe', 'setup.exe']]:
                with zipfile.ZipFile(artifact, 'w') as archive:
                    for name in members:
                        archive.writestr(name, b'MZ')
                with self.assertRaises(ValueError):
                    compare.compare(release, artifact, 'setup.exe')
            with zipfile.ZipFile(artifact, 'w') as archive:
                archive.writestr('../../setup.exe', b'MZ')
            self.assertTrue(compare.compare(release, artifact, 'setup.exe')['identical'])
            self.assertFalse((root.parent / 'setup.exe').exists())


if __name__ == '__main__':
    unittest.main()
