#!/usr/bin/env python3
# [INPUT]: Temporary ZIP/PE fixtures and the pinned browser preparation module.
# [OUTPUT]: Offline Windows engine archive/executable boundary regressions.
# [POS]: No browser launch, real profile, credentials or system associations are touched.
import importlib.util
from pathlib import Path
import stat
import struct
import tempfile
import unittest
import zipfile

spec = importlib.util.spec_from_file_location('fetch_browser', Path(__file__).with_name('fetch-browser.py'))
fetch = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fetch)


class BrowserArchiveTests(unittest.TestCase):
    def test_zip_preserves_full_engine_and_rejects_unsafe_names(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            archive = root / 'browser.zip'
            with zipfile.ZipFile(archive, 'w') as zipped:
                zipped.writestr('engine/chrome.exe', b'fixture')
                zipped.writestr('engine/locales/en-US.pak', b'locale')
                zipped.writestr('engine/LICENSE', b'keep attribution')
            fetch.extract_windows_zip(archive, root / 'good')
            self.assertEqual((root / 'good/engine/locales/en-US.pak').read_bytes(), b'locale')
            for unsafe in ('../escape', '/absolute', 'C:/escape', 'a\\..\\escape', 'file:stream', 'CON', 'a/NUL.txt', 'trailing. /x'):
                with self.subTest(unsafe=unsafe):
                    with zipfile.ZipFile(archive, 'w') as zipped:
                        zipped.writestr(unsafe, b'bad')
                    with self.assertRaises(SystemExit):
                        fetch.extract_windows_zip(archive, root / 'bad')
                    self.assertFalse((root / 'bad').exists())

    def test_case_collisions_and_links_are_rejected_before_extraction(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            archive = root / 'browser.zip'
            with zipfile.ZipFile(archive, 'w') as zipped:
                zipped.writestr('Chrome.exe', b'first')
                zipped.writestr('chrome.exe', b'second')
            with self.assertRaises(SystemExit):
                fetch.extract_windows_zip(archive, root / 'extracted')
            with zipfile.ZipFile(archive, 'w') as zipped:
                link = zipfile.ZipInfo('engine')
                link.external_attr = (stat.S_IFLNK | 0o777) << 16
                zipped.writestr(link, b'../outside')
            with self.assertRaises(SystemExit):
                fetch.extract_windows_zip(archive, root / 'extracted')
            self.assertFalse((root / 'extracted').exists())

    def test_pe_requires_amd64_and_checks_header(self):
        with tempfile.TemporaryDirectory() as root:
            binary = Path(root) / 'chrome.exe'
            data = bytearray(70)
            data[:2] = b'MZ'
            struct.pack_into('<I', data, 60, 64)
            data[64:] = b'PE\0\0\x64\x86'
            binary.write_bytes(data)
            fetch.verify_windows_pe(binary)
            data[68:] = b'\x4c\x01'
            binary.write_bytes(data)
            with self.assertRaises(SystemExit):
                fetch.verify_windows_pe(binary)
            binary.write_bytes(b'not a program')
            with self.assertRaises(SystemExit):
                fetch.verify_windows_pe(binary)


if __name__ == '__main__':
    unittest.main()
