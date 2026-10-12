#!/usr/bin/env python3
# [INPUT]: Prepared genuine Windows fingerprint engine; disposable profile and loopback HTML.
# [OUTPUT]: Actual platform/version/CPU/language/timezone readings plus seeded Canvas stability/rotation evidence.
# [POS]: Opt-in runtime smoke test, no user profile or Windows settings/default associations are modified.
import hashlib
import http.server
import json
import os
from pathlib import Path
import platform
import queue
import subprocess
import tempfile
import threading

ROOT = Path(__file__).resolve().parent.parent
VERSION = '148.0.7778.215'
PAGE = b'''<!doctype html><meta charset="utf-8"><canvas id="c" width="360" height="80"></canvas><script>
(async()=>{
const ctx=c.getContext('2d');ctx.font='22px Arial';ctx.fillStyle='#ef7b29';ctx.fillRect(5,5,300,65);
ctx.fillStyle='#221c12';ctx.fillText('CCBaga fingerprint 012345',12,45);
const high=await navigator.userAgentData.getHighEntropyValues(['platformVersion','uaFullVersion']);
const result={platform:navigator.platform,version:high.platformVersion,browser:high.uaFullVersion,
cores:navigator.hardwareConcurrency,language:navigator.language,timezone:Intl.DateTimeFormat().resolvedOptions().timeZone,
canvas:c.toDataURL()};
await fetch('/result',{method:'POST',body:JSON.stringify(result)});document.body.textContent='CCBAGA_SMOKE_OK';
})().catch(e=>document.body.textContent=String(e));</script>'''


def main():
    if platform.system() != 'Windows':
        raise SystemExit('This smoke test requires Windows.')
    binary = ROOT / 'browser-runtime/WindowsBrowser/chrome.exe'
    if not binary.is_file():
        raise SystemExit('Run python scripts/fetch-browser.py first.')
    reports = queue.Queue()
    class Handler(http.server.BaseHTTPRequestHandler):
        def do_GET(self):
            self.send_response(200)
            self.send_header('Content-Type', 'text/html; charset=utf-8')
            self.end_headers()
            self.wfile.write(PAGE)

        def do_POST(self):
            length = int(self.headers['Content-Length'])
            if length > 100000:
                self.send_error(413)
                return
            reports.put(json.loads(self.rfile.read(length)))
            self.send_response(200)
            self.end_headers()

        def log_message(self, *_):
            pass

    server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    observed = []
    actual_version = platform.version()
    try:
        for seed in (1234567, 1234567, 7654321):
            with tempfile.TemporaryDirectory(prefix='ccbaga-browser-smoke-') as profile:
                args = [str(binary), '--headless=new', '--no-first-run', '--no-default-browser-check',
                        '--disable-background-networking', '--disable-component-update',
                        '--disable-sync', '--disable-extensions', '--disable-quic',
                        '--user-data-dir=' + profile, '--profile-directory=Default',
                        '--fingerprint=' + str(seed), '--fingerprint-platform=windows',
                        '--fingerprint-platform-version=' + actual_version,
                        '--fingerprint-brand=Chrome', '--fingerprint-brand-version=' + VERSION,
                        '--fingerprint-hardware-concurrency=' + str(os.cpu_count()),
                        '--disable-spoofing=font,audio,gpu', '--lang=en-US', '--accept-lang=en-US,en',
                        '--timezone=America/New_York', '--dump-dom', '--virtual-time-budget=3000',
                        'http://127.0.0.1:' + str(server.server_port) + '/']
                result = subprocess.run(args, capture_output=True, timeout=40, creationflags=subprocess.CREATE_NO_WINDOW)
                if result.returncode:
                    raise SystemExit('Browser smoke launch failed: ' + result.stderr.decode('utf-8', errors='replace')[-2000:])
                try:
                    report = reports.get(timeout=5)
                except queue.Empty:
                    raise SystemExit('Browser did not submit measured readings: ' + result.stdout.decode('utf-8', errors='replace')[-2000:])
                for key, expected in {'platform': 'Win32', 'version': actual_version, 'browser': VERSION,
                                      'cores': os.cpu_count(), 'language': 'en-US', 'timezone': 'America/New_York'}.items():
                    if report[key] != expected:
                        raise SystemExit(f'Actual {key}: {report[key]!r}; expected {expected!r}')
                report['canvas_sha256'] = hashlib.sha256(report.pop('canvas').encode()).hexdigest()
                report['seed'] = seed
                observed.append(report)
        if observed[0]['canvas_sha256'] != observed[1]['canvas_sha256']:
            raise SystemExit('Identical fingerprint seeds did not produce stable Canvas readings.')
        if observed[0]['canvas_sha256'] == observed[2]['canvas_sha256']:
            raise SystemExit('Changing the fingerprint seed did not rotate Canvas readings.')
        print(json.dumps({'verified': True, 'observed': observed}, indent=2))
    finally:
        server.shutdown()
        server.server_close()


if __name__ == '__main__':
    main()
