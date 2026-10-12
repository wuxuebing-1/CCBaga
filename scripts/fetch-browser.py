#!/usr/bin/env python3
# [INPUT]: 固定上游 Fingerprint Chromium macOS DMG/Linux tar.xz/Windows ZIP、发布者 SHA-256 与平台工具。
# [OUTPUT]: 校验来源、版本与架构后恢复被 Git 排除的各平台引擎，不读取用户资料；Windows ZIP 拒绝路径逃逸/链接。
# [POS]: 干净检出的引擎准备边界；保留上游完整资源与许可，不下载随时间变化的 latest。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
import hashlib
import platform
import tarfile
import shutil
from pathlib import Path
import plistlib
import subprocess
import tempfile
import zipfile
import stat
import struct
import json
from pathlib import PurePosixPath

ROOT = Path(__file__).resolve().parent.parent
VERSION = '148.0.7778.215'
URL = f'https://github.com/adryfish/fingerprint-chromium/releases/download/{VERSION}/ungoogled-chromium_{VERSION}-1.1_macos.dmg'
SHA256 = 'b72f091e2e1a7583eed389c4b8e3534ed355e568af8c8bbf8fc30a25e23ca679'
WINDOWS_URL = f'https://github.com/adryfish/fingerprint-chromium/releases/download/{VERSION}/ungoogled-chromium_{VERSION}-1.1_windows_x64.zip'
WINDOWS_SHA256 = '9ef3f471b7a6641b4224532522b29141ce3746e27d55788d88e2fd951f362579'
WINDOWS_MANIFEST = 'ccbaga-engine-manifest.json'


def sha256(path):
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(chunk)
    return digest.hexdigest()


def extract_windows_zip(archive, destination):
    """Never trust ZIP names, even after verifying the publisher's fixed digest."""
    with zipfile.ZipFile(archive) as zipped:
        names = set()
        for member in zipped.infolist():
            path = PurePosixPath(member.filename)
            key = member.filename.rstrip('/').lower()
            if (path.is_absolute() or '\\' in member.filename or ':' in member.filename
                    or any(part in ('..', '.') or part.rstrip(' .') != part for part in path.parts)
                    or any(part.split('.')[0].upper() in {'CON', 'PRN', 'AUX', 'NUL',
                           *('COM' + str(i) for i in range(1, 10)),
                           *('LPT' + str(i) for i in range(1, 10))} for part in path.parts)
                    or stat.S_ISLNK(member.external_attr >> 16) or key in names):
                raise SystemExit('Unsafe or duplicate path in Windows browser ZIP')
            names.add(key)
        if sum(member.file_size for member in zipped.infolist()) > 3 * 1024**3:
            raise SystemExit('Unexpected Windows browser archive size')
        zipped.extractall(destination)


def windows_file_version(binary):
    import ctypes
    from ctypes import wintypes
    version = ctypes.WinDLL('version', use_last_error=True)
    version.GetFileVersionInfoSizeW.argtypes = [wintypes.LPCWSTR, ctypes.POINTER(wintypes.DWORD)]
    version.GetFileVersionInfoSizeW.restype = wintypes.DWORD
    version.GetFileVersionInfoW.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, ctypes.c_void_p]
    version.GetFileVersionInfoW.restype = wintypes.BOOL
    version.VerQueryValueW.argtypes = [ctypes.c_void_p, wintypes.LPCWSTR, ctypes.POINTER(ctypes.c_void_p), ctypes.POINTER(wintypes.UINT)]
    version.VerQueryValueW.restype = wintypes.BOOL
    ignored = wintypes.DWORD()
    size = version.GetFileVersionInfoSizeW(str(binary), ctypes.byref(ignored))
    if not size:
        raise SystemExit('Missing Windows browser version resource')
    data = ctypes.create_string_buffer(size)
    if not version.GetFileVersionInfoW(str(binary), 0, size, data):
        raise SystemExit('Cannot read Windows browser version resource')
    value = ctypes.c_void_p()
    length = wintypes.UINT()
    if not version.VerQueryValueW(data, '\\', ctypes.byref(value), ctypes.byref(length)) or length.value < 52:
        raise SystemExit('Invalid Windows browser version resource')
    fields = ctypes.cast(value, ctypes.POINTER(wintypes.DWORD))
    if fields[0] != 0xFEEF04BD:
        raise SystemExit('Invalid Windows browser version signature')
    return '.'.join(str(n) for n in (fields[2] >> 16, fields[2] & 0xffff, fields[3] >> 16, fields[3] & 0xffff))


def verify_windows_pe(binary):
    with binary.open('rb') as stream:
        header = stream.read(64)
        if len(header) != 64 or header[:2] != b'MZ':
            raise SystemExit('Expected a Windows PE executable')
        offset = struct.unpack_from('<I', header, 60)[0]
        if not 64 <= offset <= 1024 * 1024:
            raise SystemExit('Invalid PE header offset')
        stream.seek(offset)
        if stream.read(6) != b'PE\0\0\x64\x86':
            raise SystemExit('Expected an x86_64 Windows PE executable')


def windows_engine_files(engine):
    if engine.is_symlink() or getattr(engine.lstat(), 'st_file_attributes', 0) & 0x400:
        raise SystemExit('Windows engine root must be a real directory')
    files = {}
    for path in sorted(engine.rglob('*')):
        metadata = path.lstat()
        if path.is_symlink() or getattr(metadata, 'st_file_attributes', 0) & 0x400:
            raise SystemExit('Links/reparse points are not allowed in the Windows engine')
        if path.is_file() and path != engine / WINDOWS_MANIFEST:
            files[path.relative_to(engine).as_posix()] = sha256(path)
    return files


def write_windows_manifest(engine):
    data = {'source': WINDOWS_URL, 'archive_sha256': WINDOWS_SHA256,
            'version': VERSION, 'files': windows_engine_files(engine)}
    (engine / WINDOWS_MANIFEST).write_text(json.dumps(data, indent=2, sort_keys=True) + '\n', encoding='utf-8')


def verify_windows_manifest(engine):
    data = json.loads((engine / WINDOWS_MANIFEST).read_text(encoding='utf-8'))
    if (data.get('source') != WINDOWS_URL or data.get('archive_sha256') != WINDOWS_SHA256
            or data.get('version') != VERSION or data.get('files') != windows_engine_files(engine)):
        raise SystemExit('Windows browser engine integrity mismatch; fetch the pinned engine again')
    verify_windows_pe(engine / 'chrome.exe')
    if windows_file_version(engine / 'chrome.exe') != VERSION:
        raise SystemExit('Windows browser executable version mismatch')
    return data


def fetch_windows():
    if platform.machine().lower() not in ('x86_64', 'amd64'):
        raise SystemExit('The pinned upstream Windows engine only supports x86_64')
    target = ROOT / 'browser-runtime/WindowsBrowser'
    if target.exists():
        raise SystemExit('WindowsBrowser already exists; refusing to overwrite a prepared engine')
    with tempfile.TemporaryDirectory(prefix='ccbaga-engine-') as temporary:
        archive = Path(temporary) / 'browser.zip'
        subprocess.run(['curl.exe', '--fail', '--location', '--retry', '3', '--output', str(archive), WINDOWS_URL], check=True)
        if sha256(archive) != WINDOWS_SHA256:
            raise SystemExit('Upstream browser SHA-256 mismatch')
        extracted = Path(temporary) / 'extracted'
        extract_windows_zip(archive, extracted)
        engines = list(extracted.glob('**/chrome.exe'))
        if len(engines) != 1:
            raise SystemExit('Expected exactly one Windows Chromium executable')
        binary = engines[0]
        verify_windows_pe(binary)
        if windows_file_version(binary) != VERSION:
            raise SystemExit('Upstream browser version mismatch')
        (binary.parent / 'version.txt').write_text(VERSION + '\n', encoding='utf-8')
        write_windows_manifest(binary.parent)
        shutil.copytree(binary.parent, target)
    print(f'Fingerprint Chromium {VERSION}: Windows x86_64 SHA-256, PE and version verified')


def fetch_linux():
    if platform.machine() not in ('x86_64', 'AMD64'):
        raise SystemExit('The pinned upstream Linux engine only supports x86_64')
    target = ROOT / 'browser-runtime/LinuxBrowser'
    if target.exists():
        raise SystemExit('LinuxBrowser already exists; refusing to overwrite a prepared engine')
    url = f'https://github.com/adryfish/fingerprint-chromium/releases/download/{VERSION}/ungoogled-chromium-{VERSION}-1-x86_64_linux.tar.xz'
    digest = '70d239830332e5820aa34dfcb284161cac0429eee25da642830afe04bda717f4'
    with tempfile.TemporaryDirectory(prefix='ccbaga-engine-') as temporary:
        archive = Path(temporary) / 'browser.tar.xz'
        subprocess.run(['curl', '--fail', '--location', '--retry', '3', '--output', str(archive), url], check=True)
        if sha256(archive) != digest:
            raise SystemExit('Upstream browser SHA-256 mismatch')
        extracted = Path(temporary) / 'extracted'
        with tarfile.open(archive) as tar:
            tar.extractall(extracted, filter='data')
        engines = list(extracted.glob('**/chrome'))
        if len(engines) != 1:
            raise SystemExit('Expected exactly one Linux Chromium executable')
        binary = engines[0]
        with binary.open('rb') as stream:
            elf = stream.read(20)
        if elf[:5] != b'\x7fELF\x02' or elf[18:20] != b'\x3e\x00':
            raise SystemExit('Expected an x86_64 ELF executable')
        actual = subprocess.check_output([str(binary), '--version'], text=True)
        if VERSION not in actual:
            raise SystemExit('Upstream browser version mismatch')
        (binary.parent / 'version.txt').write_text(VERSION + '\n')
        shutil.copytree(binary.parent, target, symlinks=True)
    print(f'Fingerprint Chromium {VERSION}: Linux x86_64 SHA-256 verified')


def main():
    if platform.system() == 'Windows':
        return fetch_windows()
    if platform.system() == 'Linux':
        return fetch_linux()
    if platform.system() != 'Darwin':
        raise SystemExit('Only macOS, Linux and Windows are supported')
    target = ROOT / 'browser-runtime/Browser.app'
    if target.exists():
        raise SystemExit('Browser.app already exists; refusing to overwrite a prepared engine')
    with tempfile.TemporaryDirectory(prefix='cctool-engine-') as temporary:
        image = Path(temporary) / 'browser.dmg'
        subprocess.run(['curl', '--fail', '--location', '--retry', '3', '--output', str(image), URL], check=True)
        if sha256(image) != SHA256:
            raise SystemExit('Upstream browser SHA-256 mismatch')
        mounted = plistlib.loads(subprocess.check_output(['hdiutil', 'attach', '-readonly', '-nobrowse', '-plist', str(image)]))
        volumes = [Path(item['mount-point']) for item in mounted['system-entities'] if 'mount-point' in item]
        try:
            apps = [app for volume in volumes for app in volume.glob('*.app')]
            if len(apps) != 1:
                raise SystemExit('Expected exactly one browser app in upstream DMG')
            metadata = plistlib.loads((apps[0] / 'Contents/Info.plist').read_bytes())
            if metadata['CFBundleShortVersionString'] != VERSION:
                raise SystemExit('Upstream browser version mismatch')
            binary = apps[0] / 'Contents/MacOS' / metadata['CFBundleExecutable']
            if 'arm64' not in subprocess.check_output(['lipo', '-archs', str(binary)], text=True).split():
                raise SystemExit('Upstream browser does not contain Apple Silicon code')
            subprocess.run(['ditto', str(apps[0]), str(target)], check=True)
        finally:
            for volume in volumes:
                subprocess.run(['hdiutil', 'detach', str(volume)], check=True)
    print(f'Fingerprint Chromium {VERSION}: SHA-256 and arm64 verified')


if __name__ == '__main__':
    main()
