"""Check that LAN-mode Vite requires a gateway key even at localhost."""
import os
from pathlib import Path
import signal
import socket
import subprocess
import time
import urllib.error
import urllib.request

root = Path(__file__).resolve().parent.parent
with socket.socket() as sock:
    sock.bind(('127.0.0.1', 0))
    port = sock.getsockname()[1]
env = {key: value for key, value in os.environ.items() if not key.startswith(('ANTHROPIC_', 'VITE_'))}
env['VITE_GATEWAY_PAIRED'] = 'true'
process = subprocess.Popen(['npm', 'run', 'dev', '--', '--host', '127.0.0.1', '--port', str(port), '--strictPort'], cwd=root / 'app', env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True)
try:
    deadline = time.monotonic() + 20
    while True:
        if process.poll() is not None:
            raise RuntimeError('Fixture GUI failed to start')
        try:
            with urllib.request.urlopen(f'http://127.0.0.1:{port}/', timeout=1):
                break
        except urllib.error.URLError:
            if time.monotonic() > deadline:
                raise RuntimeError('Fixture GUI readiness timeout') from None
            time.sleep(0.1)
    env['DEV_GUI_FIXTURE_URL'] = f'http://127.0.0.1:{port}/'
    subprocess.run(['node', '--input-type=module', '-e', '''
import { chromium } from '@playwright/test';
const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage();
  let privateCalls = 0;
  await page.route('**/api/**', route => { privateCalls++; return route.abort(); });
  await page.goto(process.env.DEV_GUI_FIXTURE_URL);
  await page.getByLabel('Paired gateway token', { exact: true }).waitFor();
  if (privateCalls !== 0) throw new Error('GUI must not fetch private state before login');
} finally { await browser.close(); }
'''], cwd=root / 'app', env=env, check=True)
    print('LAN GUI fixture passed: localhost requires the key and no private API is called before login.')
finally:
    os.killpg(process.pid, signal.SIGTERM)
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        process.wait()
