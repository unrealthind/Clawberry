"""Launcher checks use dummy processes/credentials and never start the real runtime."""
import contextlib
import importlib.util
import io
import os
import sys
from pathlib import Path
import tempfile
import unittest
from unittest.mock import MagicMock, patch

sys.dont_write_bytecode = True

spec = importlib.util.spec_from_file_location('dev_local', Path(__file__).with_name('dev-local.py'))
launcher = importlib.util.module_from_spec(spec)
spec.loader.exec_module(launcher)


class LauncherTests(unittest.TestCase):
    def launch(self, arguments):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for directory in ('scripts', 'target/debug', 'app/node_modules/.bin', 'app/dist'):
                (root / directory).mkdir(parents=True)
            (root / 'target/debug/tinyclaw').touch()
            (root / 'app/node_modules/.bin/vite').touch()
            mode = 'auto' if '--local' in arguments else 'paired'
            (root / 'app/dist/index.html').write_text(f'<meta name="raaviclaw-auth-mode" content="{mode}">')
            gateway, gui = MagicMock(), MagicMock()
            gateway.pid, gui.pid = 100, 101
            gateway.poll.return_value = None
            gui.poll.return_value = 7
            gui.returncode = 7
            gateway.wait.return_value = 0
            processes = [gateway, gui] if '--local' not in arguments else [gui]
            with patch.object(launcher, '__file__', str(root / 'scripts/dev-local.py')), \
                 patch('sys.argv', ['dev-local.py', *arguments]), \
                 patch.dict(os.environ, {'ANTHROPIC_API_KEY': 'dummy-provider-value', 'OPENAI_API_KEY': 'dummy-other-provider', 'ZEROCLAW_API_KEY': 'dummy-generic-provider'}, clear=True), \
                 patch.object(launcher, 'check_port') as ports, \
                 patch.object(launcher, 'lan_addresses', return_value=['192.168.1.10']), \
                 patch.object(launcher.socket, 'create_connection', return_value=MagicMock()), \
                 patch.object(launcher.subprocess, 'Popen', side_effect=processes) as popen, \
                 patch.object(launcher.subprocess, 'run') as run, \
                 patch.object(launcher.os, 'killpg') as kill, \
                 patch.object(launcher.signal, 'signal'), \
                 contextlib.redirect_stdout(io.StringIO()) as output:
                result = launcher.main()
            self.assertEqual(result, 7)
            self.assertNotIn('dummy-provider-value', output.getvalue())
            return popen.call_args_list, run.call_args_list, ports.call_args_list, kill.call_args_list, output.getvalue()

    def test_default_builds_both_and_pairs_lan_gui_without_provider_credentials(self):
        processes, builds, ports, kills, output = self.launch([])
        self.assertEqual([call.args[0] for call in builds], [['cargo', 'build', '--locked'], ['npm', 'run', 'build'], ['sudo', 'firewall-cmd', '--zone=public', '--add-port=5173/tcp', '--timeout=1h']])
        self.assertNotIn('ANTHROPIC_API_KEY', builds[-1].kwargs['env'])
        self.assertEqual(processes[0].args[0][-1], '--dev-access')
        self.assertIn('dummy-provider-value', processes[0].kwargs['env'].values())
        for key in ['ANTHROPIC_API_KEY', 'OPENAI_API_KEY', 'ZEROCLAW_API_KEY']:
            self.assertNotIn(key, processes[1].kwargs['env'])
        self.assertEqual(processes[1].kwargs['env']['VITE_GATEWAY_PAIRED'], 'true')
        self.assertIn('--strictPort', processes[1].args[0])
        self.assertIn('http://192.168.1.10:5173/', output)
        self.assertEqual(len(ports), 2)
        self.assertEqual(len(kills), 1)

    def test_local_and_no_build_preserve_loopback_mode(self):
        processes, builds, ports, _, _ = self.launch(['--local', '--no-build'])
        self.assertEqual(len(processes), 1)
        self.assertEqual(processes[0].args[0][-1], '--local-ui')
        self.assertEqual(builds, [])
        self.assertEqual(len(ports), 1)

    def test_dangling_configuration_symlink_is_rejected_before_writing(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / '.runtime').mkdir()
            target = root / 'must-not-be-created.toml'
            (root / '.runtime/config.toml').symlink_to(target)
            with patch.object(launcher, '__file__', str(root / 'scripts/dev-local.py')), \
                 patch('sys.argv', ['dev-local.py', '--no-build']), \
                 patch.dict(os.environ, {}, clear=True):
                with self.assertRaisesRegex(SystemExit, 'symlinked runtime configuration'):
                    launcher.main()
            self.assertFalse(target.exists())

    def test_no_build_rejects_a_frontend_from_the_other_login_mode(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'app/dist').mkdir(parents=True)
            (root / 'app/dist/index.html').write_text('<meta name="raaviclaw-auth-mode" content="paired">')
            with patch.object(launcher, '__file__', str(root / 'scripts/dev-local.py')), \
                 patch('sys.argv', ['dev-local.py', '--local', '--no-build']), \
                 patch.dict(os.environ, {}, clear=True), \
                 patch.object(launcher, 'check_port'), \
                 contextlib.redirect_stdout(io.StringIO()):
                with self.assertRaisesRegex(SystemExit, 'login mode does not match'):
                    launcher.main()

    def test_firewall_failure_is_reported_without_ignoring_it(self):
        for error in [FileNotFoundError(), launcher.subprocess.CalledProcessError(1, 'sudo')]:
            with self.subTest(error=type(error).__name__), \
                 patch.object(launcher.subprocess, 'run', side_effect=error), \
                 contextlib.redirect_stdout(io.StringIO()):
                with self.assertRaisesRegex(SystemExit, 'Could not open LAN firewall'):
                    launcher.open_lan_firewall({})

    def test_occupied_port_fails_without_stopping_any_process(self):
        with patch.object(launcher.socket, 'socket') as sock:
            sock.return_value.__enter__.return_value.bind.side_effect = OSError()
            with self.assertRaisesRegex(SystemExit, 'already in use'):
                launcher.check_port('127.0.0.1', 8080)


if __name__ == '__main__':
    unittest.main()
