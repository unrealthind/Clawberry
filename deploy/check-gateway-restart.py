#!/usr/bin/env python3
"""Local-only restart acceptance; suppress startup secrets and private output."""
import argparse
import json
import socket
import sqlite3
import subprocess
import time
import urllib.request
from pathlib import Path

binary = Path(__file__).resolve().parents[1] / 'target/debug/tinyclaw'
state = Path.home() / '.tinyclaw/workspace/assistants'
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--request-id', required=True, help='Consumed request to check')
args = parser.parse_args()
request_id = args.request_id

def approval_snapshot():
    with sqlite3.connect('file:' + str(state / 'execution.db') + '?mode=ro', uri=True) as db:
        return db.execute("SELECT e.decision,a.remaining_uses FROM execution_audit e LEFT JOIN execution_approvals a ON a.id=e.approval_id WHERE e.request_id=? AND e.decision='admitted'", (request_id,)).fetchall()

before = approval_snapshot()
assert before == [('admitted', 0)], 'Expected one consumed correlated approval'
for cycle in range(2):
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        port = sock.getsockname()[1]
    process = subprocess.Popen([str(binary), 'gateway', '--host', '127.0.0.1', '--port', str(port)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        ready = False
        for _ in range(100):
            assert process.poll() is None, 'Gateway exited before health acceptance'
            try:
                with urllib.request.urlopen(f'http://127.0.0.1:{port}/health', timeout=0.5) as response:
                    ready = json.load(response).get('status') == 'ok'
                if ready:
                    break
            except OSError:
                time.sleep(0.1)
        assert ready, 'Gateway did not become healthy'
        assert approval_snapshot() == before, 'Approval state changed across gateway start'
        print(f'Gateway start {cycle + 1}: healthy; consumed approval preserved')
    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
assert approval_snapshot() == before
print('Restart acceptance passed; temporary localhost gateways stopped')
