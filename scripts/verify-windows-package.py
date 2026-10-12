#!/usr/bin/env python3
# [INPUT]: Actual NSIS installer, 7-Zip and the verified source browser/UI resources.
# [OUTPUT]: Extracted-payload equality verification and SHA256SUMS.txt.
# [POS]: Read-only installer acceptance; never installs, opens UI or changes registry.
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]

def digest(path):
    value = hashlib.sha256()
    with path.open('rb') as stream:
        for part in iter(lambda: stream.read(1024 * 1024), b''):
            value.update(part)
    return value.hexdigest()

def verify(directory):
    seven = shutil.which('7z') or next((str(Path(base) / '7-Zip/7z.exe') for base in
        [os.environ.get('ProgramFiles', ''), os.environ.get('ProgramFiles(x86)', '')]
        if (Path(base) / '7-Zip/7z.exe').is_file()), None)
    if not seven:
        raise SystemExit('7-Zip is required to inspect the actual NSIS payload.')
    installers = sorted(directory.glob('*.exe'))
    if not installers:
        raise SystemExit('No Windows installer found')
    sums = []
    for installer in installers:
        with tempfile.TemporaryDirectory(prefix='ccbaga-inspect-') as temporary:
            extracted = Path(temporary)
            subprocess.run([seven, 'x', '-y', '-o' + str(extracted), str(installer.resolve())], check=True, stdout=subprocess.DEVNULL)
            guards = list(extracted.rglob('ccbaga-browser-guard.ps1'))
            if not guards or any(digest(path) != digest(ROOT / 'packaging/windows/browser-guard.ps1') for path in guards):
                raise SystemExit('Installer browser guard script is missing or truncated')
            hosts = list(extracted.rglob('requirement-french.exe'))
            expected_host = (ROOT / 'src-tauri/target/release/requirement-french.exe').read_bytes()
            # Tauri injects the NSIS bundle marker into the packaged copy, then
            # restores the build output to UNK. Every other byte must match.
            marker = b'__TAURI_BUNDLE_TYPE_VAR_UNK'
            if expected_host.count(marker) == 1:
                expected_host = expected_host.replace(marker, b'__TAURI_BUNDLE_TYPE_VAR_NSS', 1)
            if len(hosts) != 1 or hosts[0].read_bytes() != expected_host:
                raise SystemExit('Installer host is missing or differs from the built application')
            bases = [p.parent.parent.parent for p in extracted.rglob('WindowsBrowser/chrome.exe')]
            if len(bases) != 1:
                raise SystemExit('Installer must contain exactly one companion engine')
            base = bases[0]
            for source in (ROOT / 'browser-runtime/WindowsBrowser').rglob('*'):
                if source.is_file():
                    target = base / 'browser-runtime/WindowsBrowser' / source.relative_to(ROOT / 'browser-runtime/WindowsBrowser')
                    if not target.is_file() or digest(source) != digest(target):
                        raise SystemExit('Installer browser payload differs: ' + str(source.name))
            for relative in ['browser-ui/index.html', 'browser-ui/browser.html', 'browser-runtime/LICENSE.fingerprint-chromium', 'licenses/LICENSE.lobehub.txt', 'licenses/LICENSE.CCBaga.txt', 'licenses/NOTICE.md']:
                if not (base / relative).is_file():
                    raise SystemExit('Installer resource missing: ' + relative)
            for source in (ROOT / 'dist').rglob('*'):
                if source.is_file():
                    target = base / 'browser-ui' / source.relative_to(ROOT / 'dist')
                    if not target.is_file() or digest(source) != digest(target):
                        raise SystemExit('Installer UI differs: ' + str(source.name))
            for source in (ROOT / 'skills/ccbaga-network').rglob('*'):
                if source.is_file() and '__pycache__' not in source.parts and source.suffix != '.pyc':
                    target = base / 'network-skill' / source.relative_to(ROOT / 'skills/ccbaga-network')
                    if not target.is_file() or digest(source) != digest(target):
                        raise SystemExit('Installer network Skill differs: ' + str(source.name))
        sums.append(digest(installer) + '  ' + installer.name)
    (directory / 'SHA256SUMS.txt').write_text('\n'.join(sums) + '\n', encoding='utf-8')
    print('Windows installer: host, complete browser bytes, dual UI and licenses verified.')

if __name__ == '__main__':
    verify(Path(sys.argv[1]))
