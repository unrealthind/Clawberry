#!/usr/bin/env python3
"""Build and supervise the LAN GUI and paired loopback development gateway."""
import argparse
import os
import re
from pathlib import Path
import shlex
import signal
import socket
import subprocess
import sys
import time


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--no-build', action='store_true', help='Use existing Rust/frontend builds')
    parser.add_argument('--build', action='store_true', help='Build first (the default)')
    parser.add_argument('--local', action='store_true', help='Loopback-only gateway, token-free UI, no Vite server')
    parser.add_argument('--paired', action='store_true', help='Require a development key (the default)')
    return parser.parse_args()


def credential_environment(root):
    env = os.environ.copy()
    local_env = root / '.env.local'
    if local_env.exists():
        if local_env.is_symlink():
            sys.exit('Refusing symlinked local credential file')
        os.chmod(local_env, 0o600)
        for line in local_env.read_text().splitlines():
            line = line.strip()
            if not line or line.startswith('#'):
                continue
            name, separator, value = line.partition('=')
            if not separator or name.strip() not in {'ANTHROPIC_API_KEY', 'ANTHROPIC_OAUTH_TOKEN'}:
                sys.exit('Only Anthropic credential assignments are accepted in .env.local')
            values = shlex.split(value, comments=True)
            if len(values) > 1:
                sys.exit('Quote values containing spaces in .env.local')
            if not env.get(name.strip(), '').strip():
                env[name.strip()] = values[0] if values else ''
    return env


def check_port(host, port):
    with socket.socket() as listener:
        listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        try:
            listener.bind((host, port))
        except OSError:
            sys.exit(f'Port {port} is already in use. Stop the existing dev server before running this command. No process was stopped.')


def lan_addresses():
    addresses = set()
    try:
        result = subprocess.run(['hostname', '-I'], capture_output=True, text=True, check=True)
        for value in result.stdout.split():
            try:
                socket.inet_aton(value)
            except OSError:
                continue
            if not value.startswith('127.'):
                addresses.add(value)
    except (OSError, subprocess.CalledProcessError):
        pass
    return sorted(addresses)


def stop(children):
    for child in reversed(children):
        if child.poll() is None:
            try:
                os.killpg(child.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
    for child in children:
        try:
            child.wait(timeout=5)
        except subprocess.TimeoutExpired:
            try:
                os.killpg(child.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            child.wait()


def open_lan_firewall(env):
    print('Opening public-zone TCP port 5173 for one hour (sudo may ask for your password).', flush=True)
    try:
        subprocess.run(
            ['sudo', 'firewall-cmd', '--zone=public', '--add-port=5173/tcp', '--timeout=1h'],
            env=env, check=True,
        )
    except (OSError, subprocess.CalledProcessError):
        sys.exit('Could not open LAN firewall port 5173. Check sudo/firewalld and retry. No dev servers were started.')
    print('Temporary firewall opening requested; it expires after one hour. Restart the launcher to renew it.', flush=True)


def main():
    args = arguments()
    if args.local and args.paired:
        sys.exit('Choose --local or --paired, not both.')
    root = Path(__file__).resolve().parent.parent
    env = credential_environment(root)
    state = root / '.runtime'
    if state.is_symlink():
        sys.exit('Refusing symlinked runtime directory')
    state.mkdir(mode=0o700, exist_ok=True)
    os.chmod(state, 0o700)
    config = state / 'config.toml'
    if config.is_symlink():
        sys.exit('Refusing symlinked runtime configuration')
    if not config.exists():
        import json
        config.write_text('default_provider = "anthropic"\ndefault_model = "claude-sonnet-5-5"\ndefault_temperature = 0.7\n[gateway]\nlocal_ui = false\napp_dir = ' + json.dumps(str(root / 'app/dist')) + '\n')
        os.chmod(config, 0o600)
    check_port('127.0.0.1', 8080)
    if not args.local:
        check_port('0.0.0.0', 5173)
    configured = any(env.get(k, '').strip() for k in ['ANTHROPIC_API_KEY', 'ANTHROPIC_OAUTH_TOKEN'])
    print('Claude credential: ' + ('configured (private environment)' if configured else 'missing; add ANTHROPIC_API_KEY to .env.local, then restart'), flush=True)
    # Only the gateway receives provider credentials. Vite and npm do not need them.
    gui_names = {
        'PATH', 'HOME', 'USER', 'LOGNAME', 'SHELL', 'LANG', 'LANGUAGE', 'TZ',
        'TERM', 'COLORTERM', 'NO_COLOR', 'FORCE_COLOR', 'TMPDIR', 'TMP', 'TEMP',
        'NODE_OPTIONS', 'NODE_EXTRA_CA_CERTS', 'SSL_CERT_FILE', 'SSL_CERT_DIR',
        'npm_config_cache', 'NPM_CONFIG_CACHE',
    }
    gui_env = {key: value for key, value in env.items()
               if key in gui_names or key.startswith('LC_')}
    gui_env['VITE_GATEWAY_PAIRED'] = 'false' if args.local else 'true'
    if args.no_build:
        index = root / 'app/dist/index.html'
        expected_mode = 'auto' if args.local else 'paired'
        html = index.read_text() if index.is_file() else ''
        marker = re.search(r'<meta\s+name="raaviclaw-auth-mode"\s+content="(paired|auto)"', html)
        if not marker or marker.group(1) != expected_mode:
            sys.exit('Frontend login mode does not match this launch. Run once without --no-build to rebuild the correct GUI.')
    if not args.no_build:
        subprocess.run(['cargo', 'build', '--locked'], cwd=root, env=env, check=True)
        if not (root / 'app/node_modules').exists():
            subprocess.run(['npm', 'ci', '--ignore-scripts'], cwd=root / 'app', env=gui_env, check=True)
        subprocess.run(['npm', 'run', 'build'], cwd=root / 'app', env=gui_env, check=True)
    binary = root / 'target/debug/tinyclaw'
    if not binary.exists():
        sys.exit('Rust build missing. Run again without --no-build.')
    if not args.local and not (root / 'app/node_modules/.bin/vite').exists():
        sys.exit('GUI dependencies missing. Run again without --no-build.')
    if not args.local:
        open_lan_firewall(gui_env)
    command = [str(binary), '--config-dir', str(state), 'gateway', '--host', '127.0.0.1', '--port', '8080', '--local-ui' if args.local else '--dev-access']
    children = []
    def interrupted(_signum, _frame):
        raise KeyboardInterrupt
    signal.signal(signal.SIGTERM, interrupted)
    try:
        children.append(subprocess.Popen(command, cwd=root, env=env, start_new_session=True))
        # Wait for gateway readiness before exposing the GUI. Never read/capture its key.
        deadline = time.monotonic() + 60
        while time.monotonic() < deadline:
            if children[0].poll() is not None:
                return children[0].returncode or 1
            try:
                with socket.create_connection(('127.0.0.1', 8080), timeout=0.2):
                    break
            except OSError:
                time.sleep(0.1)
        else:
            sys.exit('Gateway did not start within 60 seconds.')
        if args.local:
            print('Local GUI: http://127.0.0.1:8080/ — direct loopback access', flush=True)
        else:
            children.append(subprocess.Popen(['npm', 'run', 'dev', '--', '--host', '0.0.0.0', '--port', '5173', '--strictPort'], cwd=root / 'app', env=gui_env, start_new_session=True))
            print('\nGUI: http://localhost:5173/', flush=True)
            for address in lan_addresses():
                print(f'LAN GUI: http://{address}:5173/', flush=True)
            print('Paste the DEVELOPMENT ACCESS KEY printed by the gateway into the GUI login.\nLAN development uses HTTP: use a trusted LAN. Ctrl+C stops both servers.', flush=True)
        while True:
            for child in children:
                if child.poll() is not None:
                    return child.returncode or 1
            time.sleep(0.25)
    except KeyboardInterrupt:
        return 0
    finally:
        stop(children)


if __name__ == '__main__':
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        sys.exit(130)
