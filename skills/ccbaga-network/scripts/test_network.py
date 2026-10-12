#!/usr/bin/env python3
# [INPUT]: 本 Skill 执行层、临时密钥/配置和可丢弃本机子进程。
# [OUTPUT]: URL、配对/权限、注入边界、有界传输、超时和订阅来源离线回归。
# [POS]: 安装后即可运行的标准库验收；不读取用户配置或连接 VPS。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
import importlib.util
import json
import io
import os
import shlex
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest

spec = importlib.util.spec_from_file_location('network', Path(__file__).with_name('network.py'))
n = importlib.util.module_from_spec(spec); spec.loader.exec_module(n)


class NetworkTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='ccbaga-skill-test-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()

    def assertPrivate(self, path):
        with path.open() as stream:
            n.private_check(stream)

    def test_fixed_provider_destinations(self):
        self.assertEqual(n.provider_url('dmit'), 'https://www.dmit.io/aff.php?aff=28800')
        self.assertEqual(n.provider_url('webshare'), 'https://www.webshare.io/?referral_code=qlm1mt9xibc9')

    def test_real_key_pairing_private_copy_and_cleanup(self):
        pem = self.root/'id_rsa.pem'; pub = self.root/'id_rsa.pub'; ppk = self.root/'id_rsa.ppk'
        subprocess.run(['ssh-keygen','-q','-t','ed25519','-N','','-f',str(pem)],check=True)
        (self.root/'id_rsa.pem.pub').rename(pub)
        algorithm, blob = pub.read_text().split()[:2]
        ppk.write_text(f'PuTTY-User-Key-File-3: {algorithm}\nPublic-Lines: 1\n{blob}\nPrivate-Lines: 1\nfixture\n')
        conf = {'pem':str(pem),'ppk':str(ppk),'pub':str(pub)}
        pem.chmod(0o644)
        with n.Key(conf) as key:
            self.assertPrivate(key)
            self.assertEqual(key.read_bytes(),pem.read_bytes())
            copy = key
        self.assertFalse(copy.exists())
        if os.name != 'nt':
            self.assertEqual(pem.stat().st_mode & 0o777,0o644)
        ppk.write_text(f'PuTTY-User-Key-File-3: {algorithm}\nPublic-Lines: 1\nAAAA\n')
        with self.assertRaises(ValueError): n.Key(conf)
        with self.assertRaises(ValueError): n.key_data(None,str(ppk),str(pub))

    def config(self):
        return {'vps':'vps.example.com','home_host':'203.0.113.1','home_port':5328,
                'expected_ip':'203.0.113.1','home_user':'fixture-user','home_pass':'fixture-secret',
                'pem':'/tmp/id_rsa.pem','ppk':'/tmp/id_rsa.ppk','pub':'/tmp/id_rsa.pub'}

    def test_config_is_private_and_rejects_link_or_command_host(self):
        conf = self.config(); path = self.root/'connection.json'
        n.private_write(path,json.dumps(conf)); self.assertEqual(n.read_config(path),conf)
        with self.assertRaises(FileExistsError): n.private_write(path,'overwrite')
        if os.name == 'nt':
            insecure = self.root/'insecure.json'
            insecure.write_text(json.dumps(conf))
            with self.assertRaises(ValueError): n.read_config(insecure)
        else:
            path.chmod(0o644)
            with self.assertRaises(ValueError): n.read_config(path)
        for host in ['-oProxyCommand=id','user@host','host;id','ssh://host','host\nwhoami']:
            self.assertFalse(n.host_valid(host))
        self.assertTrue(n.host_valid('2001:db8::1'))

    def test_linked_config_is_rejected(self):
        path = self.root/'connection.json'
        n.private_write(path, json.dumps(self.config()))
        link = self.root/'linked.json'
        try:
            link.symlink_to(path)
        except OSError as error:
            if os.name == 'nt' and error.winerror == 1314:
                self.skipTest('Creating symlinks requires Windows Developer Mode or privilege')
            raise
        with self.assertRaises(ValueError): n.read_config(link)

    def test_agent_configuration_needs_no_terminal_and_never_prints_secrets(self):
        path = self.root/'private/connection.json'
        n.configure(path, io.StringIO(json.dumps(self.config())))
        self.assertEqual(n.read_config(path), self.config())
        self.assertPrivate(path)
        with self.assertRaises(FileExistsError): n.configure(path, io.StringIO(json.dumps(self.config())))
        for conf in [{}, {**self.config(), 'home_port': 0}, {**self.config(), 'pem': None}]:
            invalid = self.root/'invalid.json'
            with self.assertRaises(ValueError): n.configure(invalid, io.StringIO(json.dumps(conf)))
            self.assertFalse(invalid.exists())
        with self.assertRaises(ValueError): n.configure(path, io.StringIO(' ' * 65537))
        repo = self.root/'project'; repo.mkdir(); (repo/'.git').mkdir()
        with self.assertRaises(ValueError): n.configure(repo/'connection.json', io.StringIO(json.dumps(self.config())))
        result = subprocess.run([sys.executable, str(n.SKILL/'scripts/network.py'), 'configure', '--config', str(self.root/'cli/connection.json')],
                                input=json.dumps(self.config()), text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn(self.config()['home_pass'], result.stdout + result.stderr)

    def test_credentials_travel_only_in_quoted_stdin(self):
        conf = self.config(); marker = self.root/'injected'
        conf['home_pass'] = "p'$(touch " + str(marker) + ')'
        self.assertIn('export HOME_PASS=' + shlex.quote(conf['home_pass']), n.exports(conf))
        if os.name != 'nt':
            out = n.execute(['/bin/bash','-s'],n.exports(conf)+'printf %s "$HOME_PASS"\n')
            self.assertEqual(out,conf['home_pass'])
        else:
            out = n.execute([sys.executable, '-c', 'import sys; sys.stdout.write(sys.stdin.read())'], conf['home_pass'])
            self.assertEqual(out, conf['home_pass'])
        self.assertFalse(marker.exists())
        args = n.ssh_args(conf,Path('/tmp/key'),self.root/'connection.json')
        self.assertNotIn(conf['home_pass'],' '.join(args))
        self.assertIn('StrictHostKeyChecking=yes',args)
        self.assertIn('IdentityAgent=none',args)
        self.assertIn(os.devnull, args)
        self.assertPrivate(self.root/'known_hosts')
        curl = n.curl_config(conf)
        self.assertIn(json.dumps(conf['home_user']+':'+conf['home_pass']),curl)

    def test_failed_or_timed_out_process_never_returns_success(self):
        with self.assertRaises(RuntimeError): n.execute([sys.executable,'-c','raise SystemExit(1)'])
        with self.assertRaises(RuntimeError): n.execute([sys.executable,'-c','import time;time.sleep(5)'],timeout=.05)
        output = n.execute([sys.executable,'-c','print("x"*100000,end="")'])
        self.assertEqual(len(output),65536)

    def test_timeout_ends_descendants(self):
        marker = self.root/'descendant-survived'
        child = 'import time,pathlib;time.sleep(1);pathlib.Path(' + repr(str(marker)) + ').write_text("unexpected")'
        parent = 'import subprocess,sys,time;subprocess.Popen([sys.executable,"-c",' + repr(child) + ']);time.sleep(30)'
        with self.assertRaises(RuntimeError):
            n.execute([sys.executable, '-c', parent], timeout=.25)
        time.sleep(1.1)
        self.assertFalse(marker.exists())

    def test_subscription_requires_expected_host_and_complete_success_marker(self):
        link = 'http://example.com:8443/'+'a'*32+'/clash.yaml'
        self.assertEqual(n.subscription('CC_SUB_URL='+link+'\n','example.com'),link)
        for output in ['partial setup', 'CC_SUB_URL='+link.replace('example.com','other.com'), 'CC_SUB_URL='+link+'?injected=1']:
            with self.assertRaises(ValueError): n.subscription(output,'example.com')
        ipv6 = 'http://[2001:db8::1]:8443/'+'b'*32+'/clash.yaml'
        self.assertEqual(n.subscription('CC_SUB_URL='+ipv6,'2001:0db8::1'),ipv6)


if __name__ == '__main__':
    unittest.main()
