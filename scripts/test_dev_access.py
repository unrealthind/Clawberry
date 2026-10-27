"""Exercise dev-key authentication using temporary state; never print the key."""
import os
from pathlib import Path
import queue
import socket
import subprocess
import tempfile
import threading
import time
import urllib.error
import urllib.request

root = Path(__file__).resolve().parent.parent
with tempfile.TemporaryDirectory() as directory:
    state = Path(directory)
    (state / 'config.toml').write_text('default_provider = "anthropic"\ndefault_temperature = 0.7\n[gateway]\nlocal_ui = true\nrequire_pairing = false\n')
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        port = sock.getsockname()[1]
    env = {key: value for key, value in os.environ.items() if not key.startswith(('ANTHROPIC_', 'OPENAI_', 'OPENROUTER_'))}
    command = [str(root / 'target/debug/tinyclaw'), '--config-dir', str(state), 'gateway', '--host', '127.0.0.1', '--port', str(port), '--dev-access']
    process = subprocess.Popen(command, env=env, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    lines = queue.Queue()
    def read_output():
        for line in process.stdout:
            # Output stays only in process memory, never a log or test artifact.
            if line.startswith('zc_'):
                lines.put(line.strip())
    reader = threading.Thread(target=read_output, daemon=True)
    reader.start()
    try:
        token = lines.get(timeout=20)
        def status(key=None):
            request = urllib.request.Request(f'http://127.0.0.1:{port}/api/agents')
            if key:
                request.add_header('Authorization', f'Bearer {key}')
            try:
                with urllib.request.urlopen(request, timeout=1) as response:
                    return response.status
            except urllib.error.HTTPError as error:
                return error.code
        deadline = time.monotonic() + 20
        while True:
            try:
                assert status() == 401, 'Development gateway must require authentication'
                break
            except urllib.error.URLError:
                if time.monotonic() > deadline:
                    raise RuntimeError('Fixture gateway did not become ready') from None
                time.sleep(0.1)
        assert status('invalid-fixture-key') == 401, 'Invalid key must fail'
        assert status(token) == 200, 'Terminal key must authenticate'
        assert token not in (state / 'config.toml').read_text(), 'Raw key must not be persisted'
        print('Development gateway fixture passed: unauthenticated/invalid requests denied, terminal key accepted, raw key not persisted.')
    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
        reader.join(timeout=1)
    denied = subprocess.run([str(root / 'target/debug/tinyclaw'), '--config-dir', str(state), 'gateway', '--host', '0.0.0.0', '--dev-access'], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    assert denied.returncode != 0
    assert 'Development access requires a loopback gateway' in denied.stderr
    assert 'DEVELOPMENT ACCESS KEY' not in denied.stdout
