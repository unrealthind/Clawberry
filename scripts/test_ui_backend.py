"""Unmocked browser -> Vite -> Rust contract audit, with isolated state/model only."""
import collections
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import tempfile
import threading
import time
import urllib.error
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

root = Path(__file__).resolve().parent.parent
responses = collections.deque()
class Model(BaseHTTPRequestHandler):
    secret_leak = False
    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        if 'ui-audit-fixture-token' in json.dumps(body) or '314159' in json.dumps(body):
            Model.secret_leak = True
        fixture = responses.popleft() if responses else {}
        if fixture.get('delay'):
            time.sleep(fixture['delay'])
        data = {'choices': [{'message': {'role': 'assistant', 'content': fixture.get('content', f"Verified local fixture reply ({body.get('model')})")}}], 'usage': {'prompt_tokens': 2, 'completion_tokens': 2, 'total_tokens': 4}}
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.end_headers()
        try:
            self.wfile.write(json.dumps(data).encode())
        except (BrokenPipeError, ConnectionResetError):
            pass
    def log_message(self, *_args):
        pass

def port():
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        return sock.getsockname()[1]

def ready(url, process):
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError('Isolated fixture failed to start')
        try:
            with urllib.request.urlopen(url, timeout=1):
                return
        except urllib.error.URLError:
            time.sleep(0.1)
    raise RuntimeError('Isolated fixture readiness timeout')

model = ThreadingHTTPServer(('127.0.0.1', 0), Model)
threading.Thread(target=model.serve_forever, daemon=True).start()
children = []
probe = None
try:
    with tempfile.TemporaryDirectory(prefix='raaviclaw-ui-audit-') as temporary:
        state = Path(temporary)
        gateway_port, gui_port = port(), port()
        provider = f'custom:http://127.0.0.1:{model.server_port}'
        (state / 'config.toml').write_text(f'default_provider = "{provider}"\ndefault_model = "fixture-model"\ndefault_temperature = 0.0\napi_key = "fixture-only"\n[gateway]\nlocal_ui = false\npaired_tokens = ["ui-audit-fixture-token"]\nwebhook_rate_limit_per_minute = 5000\n')
        profiles = 'agent_daily_microusd = 5000000\ndeep_daily_microusd = 2000000\ntask_microusd = 2000000\n'
        for profile in ['fast', 'normal', 'deep']:
            profiles += f'\n[profiles.{profile}]\nprovider = "{provider}"\nmodel = "fixture-{profile}"\ninput_microusd_per_million = 1000000\noutput_microusd_per_million = 1000000\nmax_output_tokens = 512\n'
        (state / 'model-profiles.toml').write_text(profiles)
        env = {key: value for key, value in os.environ.items() if not key.startswith(('ANTHROPIC_', 'VITE_', 'OPENAI_', 'OPENROUTER_'))}
        gateway = subprocess.Popen([str(root / 'target/debug/tinyclaw'), '--config-dir', str(state), 'gateway', '--host', '127.0.0.1', '--port', str(gateway_port)], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True)
        children.append(gateway)
        ready(f'http://127.0.0.1:{gateway_port}/health', gateway)
        env['VITE_GATEWAY_PAIRED'] = 'true'
        env['RAAVICLAW_DEV_GATEWAY_PORT'] = str(gateway_port)
        gui = subprocess.Popen(['npm', 'run', 'dev', '--', '--host', '127.0.0.1', '--port', str(gui_port), '--strictPort'], cwd=root / 'app', env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True)
        children.append(gui)
        ready(f'http://127.0.0.1:{gui_port}/', gui)
        probe = tempfile.NamedTemporaryFile(prefix='.env.audit-', dir=root / 'app', mode='w', delete=False)
        probe.write('PRIVATE_AUDIT_SENTINEL')
        probe.close()
        env['UI_AUDIT_PROBE_NAME'] = Path(probe.name).name
        env['UI_AUDIT_URL'] = f'http://127.0.0.1:{gui_port}'
        subprocess.run(['node', 'e2e/ui-backend-contract.mjs'], cwd=root / 'app', env=env, check=True, timeout=180)
        assert not Model.secret_leak, 'Gateway key or TOTP must not enter model input'
finally:
    if probe is not None:
        Path(probe.name).unlink(missing_ok=True)
    for child in reversed(children):
        if child.poll() is None:
            os.killpg(child.pid, signal.SIGTERM)
        try:
            child.wait(timeout=5)
        except subprocess.TimeoutExpired:
            os.killpg(child.pid, signal.SIGKILL)
            child.wait()
    model.shutdown()
