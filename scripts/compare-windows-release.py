#!/usr/bin/env python3
# [INPUT]: Downloaded Release EXE and the original Actions artifact ZIP.
# [OUTPUT]: JSON byte-equality evidence; exit 1 on mismatch, 2 on invalid input.
# [POS]: Read-only provenance check, independent of Windows installation acceptance.
import argparse
from datetime import datetime, timezone
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import zipfile

spec = importlib.util.spec_from_file_location('verify_windows', Path(__file__).with_name('verify-windows-package.py'))
verify = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verify)


def compare(release, artifact, asset_name):
    if Path(asset_name).name != asset_name or not asset_name.endswith('.exe'):
        raise ValueError('Expected a plain EXE asset filename')
    with zipfile.ZipFile(artifact) as archive:
        candidates = [p for p in archive.infolist() if not p.is_dir() and p.filename.endswith('.exe')]
        if len(candidates) != 1 or Path(candidates[0].filename).name != asset_name:
            raise ValueError('Artifact must contain exactly the expected installer EXE')
        # Stream only the selected member; never extract archive-controlled paths.
        with tempfile.TemporaryDirectory(prefix='ccbaga-compare-') as temporary:
            installer = Path(temporary) / asset_name
            with archive.open(candidates[0]) as source, installer.open('wb') as target:
                shutil.copyfileobj(source, target)
            ci = {'sha256': verify.digest(installer), 'size': installer.stat().st_size,
                  'member': candidates[0].filename}
    public = {'sha256': verify.digest(release), 'size': release.stat().st_size}
    return {'schema': 1, 'checked_at': datetime.now(timezone.utc).isoformat(),
            'asset_name': asset_name, 'release': public, 'ci_installer': ci,
            'artifact_zip_sha256': verify.digest(artifact),
            'identical': public['sha256'] == ci['sha256'] and public['size'] == ci['size']}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--release', type=Path, required=True)
    parser.add_argument('--artifact', type=Path, required=True)
    parser.add_argument('--asset-name', default='CCBaga-A_0.2.0_x64-setup.exe')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    try:
        result = compare(args.release, args.artifact, args.asset_name)
    except (OSError, ValueError, zipfile.BadZipFile, RuntimeError) as error:
        parser.exit(2, str(error) + '\n')
    args.output.write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(result, indent=2))
    return 0 if result['identical'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
