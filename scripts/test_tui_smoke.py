"""Exercise legacy TUI rendering, input editing, and clean exit in a temporary PTY."""
import argparse
import fcntl
import os
from pathlib import Path
import pty
import re
import select
import struct
import subprocess
import tempfile
import termios
import time

parser = argparse.ArgumentParser()
parser.add_argument('--binary', default='target/debug/tinyclaw')
binary = str(Path(parser.parse_args().binary).resolve())
with tempfile.TemporaryDirectory(prefix='raaviclaw-tui-smoke-') as directory:
    Path(directory, 'config.toml').write_text(
        'default_provider = "custom:http://127.0.0.1:9"\n'
        'default_model = "offline-fixture"\ndefault_temperature = 0.0\napi_key = "fixture-placeholder"\n')
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 30, 120, 0, 0))
    process = subprocess.Popen([binary, '--config-dir', directory, 'tui'],
                               stdin=slave, stdout=slave, stderr=slave,
                               env={**os.environ, 'TERM': 'xterm-256color',
                                    'ZEROCLAW_OPEN_SKILLS_ENABLED': 'false'})
    os.close(slave)
    output = bytearray()

    def drain(seconds):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            ready, _, _ = select.select([master], [], [], .05)
            if ready:
                try:
                    output.extend(os.read(master, 65536))
                except OSError:
                    break

    try:
        drain(1)
        assert process.poll() is None, output.decode(errors='replace')
        os.write(master, b'baseline-edit-marker')
        drain(.4)
        os.write(master, b'\x7f')  # Backspace, then restore the final character.
        os.write(master, b'r')
        drain(.3)
        os.write(master, b'\x03')  # Ctrl+C is handled by the TUI.
        drain(.4)
        assert process.wait(timeout=5) == 0, output.decode(errors='replace')
        plain = re.sub(rb'\x1b\[[0-9;?]*[A-Za-z]', b'', bytes(output))
        assert b'baseline-edit-marker' in plain, output.decode(errors='replace')
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
        os.close(master)
print('PASS: legacy TUI renders, edits input, and exits cleanly without provider calls')
