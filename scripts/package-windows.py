#!/usr/bin/env python3
# [INPUT]: Verified Windows Fingerprint Chromium tree and built dual-entry UI.
# [OUTPUT]: Validated resources consumed directly by the Windows NSIS bundle.
# [POS]: Windows packaging preflight; never downloads a substitute browser.
import importlib.util
import platform
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

def validate():
    if platform.system() != "Windows":
        raise SystemExit("Windows packaging must run on Windows x64.")
    spec = importlib.util.spec_from_file_location("fetch_browser", ROOT / "scripts/fetch-browser.py")
    fetch = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(fetch)
    engine = ROOT / "browser-runtime/WindowsBrowser"
    for relative in ("chrome.exe", "chrome.dll", "icudtl.dat", "resources.pak", "version.txt"):
        if not (engine / relative).is_file():
            raise SystemExit(f"Missing engine resource {relative}; run python scripts/fetch-browser.py")
    if (engine / "version.txt").read_text().strip() != (ROOT / "browser-runtime/version.txt").read_text().strip():
        raise SystemExit("Browser engine version does not match the pinned project version")
    fetch.verify_windows_manifest(engine)
    for relative in ("dist/index.html", "dist/browser.html", "src-tauri/icons/icon.ico"):
        if not (ROOT / relative).is_file():
            raise SystemExit(f"Missing build resource: {relative}")
    for path in engine.rglob("*"):
        if path.is_symlink() or getattr(path.lstat(), "st_file_attributes", 0) & 0x400:
            raise SystemExit("Browser package may not contain links or reparse points")
    print("Windows browser and embedded UI resources verified.")

if __name__ == "__main__":
    validate()
