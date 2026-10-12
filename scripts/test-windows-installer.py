#!/usr/bin/env python3
"""Compile/run real NSIS pre-install and uninstall hooks on disposable data.

No POST hooks, registry writes, shortcuts, installed files or user processes are
modified. Locally compiled sleep programs are the only executable fixtures.
"""
import argparse
import ctypes as c
from ctypes import wintypes as w
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import unittest

ROOT = Path(__file__).resolve().parents[1]
SHELL = Path(os.environ.get('SystemRoot', 'C:/Windows')) / 'System32/WindowsPowerShell/v1.0/powershell.exe'
MAKENSIS = None


def find_makensis(explicit=None):
    candidates = [explicit, os.environ.get('MAKENSIS_EXE'), shutil.which('makensis')]
    for base in [os.environ.get('LOCALAPPDATA'), os.environ.get('ProgramFiles'), os.environ.get('ProgramFiles(x86)')]:
        if base:
            candidates.extend([str(Path(base) / 'tauri/NSIS/makensis.exe'), str(Path(base) / 'NSIS/makensis.exe')])
    return next((Path(p).resolve() for p in candidates if p and Path(p).is_file()), None)


def nsis_string(value):
    value = str(value)
    if '\n' in value or '\r' in value:
        raise ValueError('Invalid NSIS fixture path')
    return value.replace('$', '$$').replace('"', '$\\"')


class ProcessObserver:
    """Observe only harness-descendant PowerShell images, without WMI or secrets."""
    class Entry(c.Structure):
        _fields_ = [('size', w.DWORD), ('usage', w.DWORD), ('pid', w.DWORD),
                    ('heap', c.c_size_t), ('module', w.DWORD), ('threads', w.DWORD),
                    ('parent', w.DWORD), ('priority', w.LONG), ('flags', w.DWORD), ('name', w.WCHAR * 260)]

    def __init__(self, parent):
        self.parent, self.observed = parent, set()
        self.k = c.WinDLL('kernel32', use_last_error=True)
        declarations = {
            'CreateToolhelp32Snapshot': (w.HANDLE, [w.DWORD, w.DWORD]),
            'Process32FirstW': (w.BOOL, [w.HANDLE, c.POINTER(self.Entry)]),
            'Process32NextW': (w.BOOL, [w.HANDLE, c.POINTER(self.Entry)]),
            'OpenProcess': (w.HANDLE, [w.DWORD, w.BOOL, w.DWORD]),
            'QueryFullProcessImageNameW': (w.BOOL, [w.HANDLE, w.DWORD, w.LPWSTR, c.POINTER(w.DWORD)]),
            'IsWow64Process': (w.BOOL, [w.HANDLE, c.POINTER(w.BOOL)]),
            'CloseHandle': (w.BOOL, [w.HANDLE]),
        }
        for name, (result, args) in declarations.items():
            fn = getattr(self.k, name)
            fn.restype, fn.argtypes = result, args

    def sample(self):
        snapshot = self.k.CreateToolhelp32Snapshot(2, 0)
        if snapshot == w.HANDLE(-1).value:
            raise c.WinError(c.get_last_error())
        entries = []
        try:
            entry = self.Entry()
            entry.size = c.sizeof(entry)
            found = self.k.Process32FirstW(snapshot, c.byref(entry))
            while found:
                entries.append((entry.pid, entry.parent, entry.name))
                found = self.k.Process32NextW(snapshot, c.byref(entry))
        finally:
            self.k.CloseHandle(snapshot)
        descendants = {self.parent}
        while True:
            added = {pid for pid, parent, _ in entries if parent in descendants} - descendants
            if not added:
                break
            descendants.update(added)
        for pid, _, name in entries:
            if pid not in descendants or name.lower() != 'powershell.exe':
                continue
            handle = self.k.OpenProcess(0x1000, False, pid)
            if not handle:
                continue  # Process may have exited since the snapshot.
            try:
                path, length, wow64 = c.create_unicode_buffer(32768), w.DWORD(32768), w.BOOL()
                if (self.k.QueryFullProcessImageNameW(handle, 0, path, c.byref(length))
                        and self.k.IsWow64Process(handle, c.byref(wow64))):
                    self.observed.add((path.value.lower(), bool(wow64.value)))
            finally:
                self.k.CloseHandle(handle)


@unittest.skipUnless(os.name == 'nt', 'Windows installer guard')
class InstallerGuardTests(unittest.TestCase):
    def setUp(self):
        self.assertIsNotNone(MAKENSIS, 'NSIS compiler required: use --makensis or MAKENSIS_EXE')
        self.temp = tempfile.TemporaryDirectory(prefix='ccbaga-installer-test-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.install = self.root / "install 中文 ' quoted path"
        self.engine = self.install / 'browser-runtime/WindowsBrowser'
        self.compile_harness()

    def compile_harness(self):
        script = r'''Unicode true
Name "CCBaga disposable installer guard test"
OutFile "@OUTFILE@"
InstallDir "@INSTALLDIR@"
RequestExecutionLevel user
SilentInstall silent
SilentUnInstall silent
AutoCloseWindow true
LoadLanguageFile "${NSISDIR}\Contrib\Language files\English.nlf"
LoadLanguageFile "${NSISDIR}\Contrib\Language files\SimpChinese.nlf"
!define MAINBINARYNAME "fixture"
!include "@HOOKS@"
Section
  FileOpen $R8 "$EXEDIR\nsis-max-string.txt" w
  FileWrite $R8 "${NSIS_MAX_STRLEN}"
  FileClose $R8
  !insertmacro NSIS_HOOK_PREINSTALL
  WriteUninstaller "$EXEDIR\guard-uninstall.exe"
  FileOpen $R8 "$EXEDIR\install-pass.txt" w
  FileWrite $R8 "guard returned"
  FileClose $R8
SectionEnd
Section "Uninstall"
  !insertmacro NSIS_HOOK_PREUNINSTALL
  FileOpen $R8 "$EXEDIR\uninstall-pass.txt" w
  FileWrite $R8 "guard returned"
  FileClose $R8
SectionEnd
'''
        for key, value in {'OUTFILE': self.root / 'guard-install.exe', 'INSTALLDIR': self.install,
                           'HOOKS': ROOT / 'packaging/windows/hooks.nsh'}.items():
            script = script.replace('@' + key + '@', nsis_string(value))
        source = self.root / 'harness.nsi'
        source.write_text(script, encoding='utf-8-sig')
        result = subprocess.run([str(MAKENSIS), '/V3', str(source)], cwd=self.root,
            capture_output=True, text=True, errors='replace', timeout=30, creationflags=subprocess.CREATE_NO_WINDOW)
        self.assertEqual(result.returncode, 0, 'Actual NSIS compilation failed:\n' + result.stdout + result.stderr)

    def run_guard(self, uninstall=False, expected=0):
        kind = 'uninstall' if uninstall else 'install'
        marker = self.root / (kind + '-pass.txt')
        marker.unlink(missing_ok=True)
        command = [str(self.root / ('guard-' + kind + '.exe')), '/S']
        if uninstall:
            # NSIS parses the raw tail after _?=, including spaces; quoting this
            # as a regular argv item prevents its no-self-copy switch matching.
            # Popen executes directly without a shell, so this is path data.
            command = subprocess.list2cmdline(command) + ' _?=' + str(self.install)
        process = subprocess.Popen(command, cwd=self.root, creationflags=subprocess.CREATE_NO_WINDOW)
        observer = ProcessObserver(process.pid)
        deadline = time.monotonic() + 30
        try:
            while process.poll() is None:
                observer.sample()
                if time.monotonic() > deadline:
                    self.fail('Actual NSIS guard exceeded its deadline')
                time.sleep(.01)
        finally:
            if process.poll() is None:
                process.kill()
            process.wait(timeout=5)
        self.assertEqual(process.returncode, expected, f'{kind} guard returned {process.returncode}, expected {expected}')
        self.assertEqual(marker.exists(), expected == 0, 'Post-hook instruction must execute only when the guard passes')
        self.assertTrue(observer.observed, 'Did not observe the actual guard PowerShell process')
        for image, wow64 in observer.observed:
            self.assertFalse(wow64, 'Actual NSIS hook launched 32-bit PowerShell: ' + image)
            self.assertIn('\\system32\\windowspowershell\\', image)

    def test_real_install_and_uninstall_guards(self):
        self.run_guard()  # First install: directory does not exist.
        self.assertFalse(self.install.exists(), 'Harness must not install application files')
        self.assertEqual((self.root / 'nsis-max-string.txt').read_text(), '1024')
        source = (ROOT / 'packaging/windows/browser-guard.ps1').read_text(encoding='utf-8-sig')
        self.assertGreater(len(source.encode('utf-16le')) * 4 // 3, 1024,
                           'Guard payload must exceed the former encoded-command limit')
        self.run_guard(uninstall=True)
        self.engine.mkdir(parents=True)
        adjacent = self.engine.with_name('WindowsBrowser-other')
        adjacent.mkdir()
        program = self.engine / 'chrome.exe'
        result = subprocess.run([str(SHELL), '-NoProfile', '-NonInteractive', '-Command',
            'Add-Type -TypeDefinition "public class CCBagaWait { public static void Main() { System.Threading.Thread.Sleep(120000); } }" -OutputAssembly $env:CCBAGA_FIXTURE_EXE -OutputType ConsoleApplication'],
            env={**os.environ, 'CCBAGA_FIXTURE_EXE': str(program)}, capture_output=True,
            timeout=30, creationflags=subprocess.CREATE_NO_WINDOW)
        self.assertEqual(result.returncode, 0, result.stderr.decode(errors='replace'))
        other = adjacent / 'chrome.exe'
        shutil.copyfile(program, other)
        process = subprocess.Popen([str(other)], creationflags=subprocess.CREATE_NO_WINDOW)
        try:
            self.run_guard()  # Adjacent unrelated browser must not block.
            own = subprocess.Popen([str(program)], creationflags=subprocess.CREATE_NO_WINDOW)
            try:
                self.run_guard(expected=12)
                self.run_guard(uninstall=True, expected=12)
                self.assertIsNone(own.poll(), 'The guard must not stop the browser')
            finally:
                own.kill()
                own.wait(timeout=5)
            self.assertIsNone(process.poll(), 'The unrelated browser must remain running')
            self.run_guard()
            self.run_guard(uninstall=True)
        finally:
            process.kill()
            process.wait(timeout=5)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--makensis', help='Absolute path to makensis.exe')
    args, remaining = parser.parse_known_args()
    MAKENSIS = find_makensis(args.makensis)
    unittest.main(argv=[sys.argv[0], *remaining])
