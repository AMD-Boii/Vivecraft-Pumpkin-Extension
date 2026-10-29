"""Drive the pinned Pumpkin binary in a NEW disposable loopback-only server directory.
No license acceptance, auth changes, existing-world reuse, or security-policy bypass.
Run with Python -I; downloaded files are never imported or used as script cwd.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import queue
import re
import shutil
import socket
import struct
import subprocess
import threading
import time
import tomllib
import uuid

ROOT = Path(__file__).resolve().parent.parent
SERVER_HASH = '13daca2a4ee351ca4ad91c9ed94ffc8e7d9e3eb41cdc96e107accf9d1f522812'
ANSI = re.compile(r'\x1b\[[0-?]*[ -/]*[@-~]')


def digest(path):
    return hashlib.file_digest(path.open('rb'), 'sha256').hexdigest()


def varint(value):
    result = bytearray()
    while value > 127:
        result.append((value & 127) | 128)
        value >>= 7
    return bytes(result + bytes([value]))


def status_probe(port):
    def read_varint(connection):
        value = 0
        for shift in range(0, 35, 7):
            data = connection.recv(1)
            if not data:
                raise RuntimeError('status connection closed')
            value |= (data[0] & 127) << shift
            if data[0] < 128:
                return value
        raise RuntimeError('invalid status VarInt')
    with socket.create_connection(('127.0.0.1', port), timeout=5) as connection:
        host = b'localhost'
        handshake = b'\0' + varint(777) + varint(len(host)) + host + struct.pack('>H', port) + b'\1'
        connection.sendall(varint(len(handshake)) + handshake + b'\1\0')
        length = read_varint(connection)
        if length > 1_048_576 or read_varint(connection) != 0:
            raise RuntimeError('unexpected status response')
        size = read_varint(connection)
        if size > 1_048_576:
            raise RuntimeError('oversized status JSON')
        body = bytearray()
        while len(body) < size:
            data = connection.recv(size - len(body))
            if not data:
                raise RuntimeError('truncated status JSON')
            body.extend(data)
        return json.loads(body)


class Server:
    def __init__(self, binary, folder, attempt):
        self.log = (folder / f'console-{attempt}.log').open('w', encoding='utf-8')
        env = os.environ.copy()
        env['NO_COLOR'] = '1'
        self.process = subprocess.Popen([str(binary)], cwd=folder, stdin=subprocess.PIPE,
                                        stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                        text=True, encoding='utf-8', errors='replace', env=env)
        self.lines = []
        self.queue = queue.Queue()
        def reader():
            for raw in self.process.stdout:
                line = ANSI.sub('', raw).rstrip()
                self.lines.append(line)
                self.log.write(line + '\n')
                self.log.flush()
                print(line, flush=True)
                self.queue.put(line)
        self.thread = threading.Thread(target=reader, daemon=True)
        self.thread.start()

    def wait_for(self, text, timeout=45, start=0):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            recent = self.lines[start:]
            if any(text in line for line in recent):
                return
            if any('Failed to initialize plugin' in line for line in recent):
                raise RuntimeError('plugin initialization failed; inspect console log')
            # Never respond to license or permission prompts automatically.
            if any(re.search(r'(?i)(accept.*eula|agree.*license|permission.*\[y/n\]|grant.*\[y/n\])', line) for line in recent):
                raise RuntimeError('Startup requires human license/permission approval; no response sent')
            if self.process.poll() is not None:
                raise RuntimeError(f'server exited {self.process.returncode} before {text!r}')
            try:
                self.queue.get(timeout=min(0.5, max(0.01, deadline - time.monotonic())))
            except queue.Empty:
                pass
        raise RuntimeError(f'timed out waiting for {text!r}')

    def command(self, command, expected):
        print('>>> ' + command, flush=True)
        start = len(self.lines)
        self.process.stdin.write(command + '\n')
        self.process.stdin.flush()
        self.wait_for(expected, start=start)

    def stop(self):
        if self.process.poll() is None:
            self.process.stdin.write('stop\n')
            self.process.stdin.flush()
            try:
                self.process.wait(timeout=60)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=10)
                raise RuntimeError('server did not stop cleanly; terminated only this test process')
        self.thread.join(timeout=5)
        self.log.close()
        if self.process.returncode != 0:
            raise RuntimeError(f'server exit code {self.process.returncode}')


def assert_bindings(pid):
    command = (f"$p={pid}; $tcp=@(Get-NetTCPConnection -OwningProcess $p -State Listen -ErrorAction SilentlyContinue | "
               "Select-Object LocalAddress,LocalPort); $udp=@(Get-NetUDPEndpoint -OwningProcess $p -ErrorAction SilentlyContinue | "
               "Select-Object LocalAddress,LocalPort); @{tcp=$tcp;udp=$udp} | ConvertTo-Json -Depth 4 -Compress")
    result = subprocess.run(['powershell.exe', '-NoProfile', '-Command', command], capture_output=True, text=True, check=True, timeout=30)
    bindings = json.loads(result.stdout)
    assert bindings['tcp'], f'no listener found: {bindings}'
    assert all(item['LocalAddress'] == '127.0.0.1' and item['LocalPort'] == 25575 for item in bindings['tcp']), bindings
    assert not bindings['udp'], bindings
    return bindings


def check_safety(folder):
    config = tomllib.loads((folder / 'pumpkin.toml').read_text(encoding='utf-8'))
    network = config['networking']
    assert network['java']['address'] == '127.0.0.1:25575'
    assert network['java']['online_mode'] and network['java']['encryption']
    for item in ('bedrock', 'query', 'management', 'lan_broadcast'):
        assert not network[item]['enabled'], item
    assert not config['telemetry']['enabled']
    assert config['plugins']['verify_signatures'] and config['plugins']['ask_permission_confirmation']


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--artifact', type=Path, required=True)
    args = parser.parse_args()
    binary, artifact = args.binary.resolve(), args.artifact.resolve()
    assert digest(binary) == SERVER_HASH, 'server checksum mismatch'
    with socket.socket() as reservation:
        reservation.bind(('127.0.0.1', 25575))
    base = ROOT / '.local' / 'smoke-runs'
    base.mkdir(parents=True, exist_ok=True)
    folder = base / ('pumpkin-' + time.strftime('%Y%m%d-%H%M%S-') + uuid.uuid4().hex[:8])
    folder.mkdir()  # No reuse, including after failed runs.
    (folder / 'plugins').mkdir()
    shutil.copyfile(ROOT / 'scripts' / 'smoke-pumpkin.toml', folder / 'pumpkin.toml')
    shutil.copyfile(artifact, folder / 'plugins' / 'vivecraft_pumpkin.wasm')
    check_safety(folder)
    result = {'directory': str(folder), 'server_sha256': SERVER_HASH, 'artifact_sha256': digest(artifact), 'checks': {}}
    print('SMOKE DIRECTORY: ' + str(folder), flush=True)
    server = None
    try:
        server = Server(binary, folder, 1)
        server.wait_for('Server is now running.', timeout=180)
        server.wait_for('Loaded Vivecraft Pumpkin (0.1.0)', timeout=20)
        check_safety(folder)
        result['checks']['bindings'] = assert_bindings(server.process.pid)
        result['checks']['status'] = status_probe(25575)
        server.command('vive help', 'Inactive settings are retained, NOT applied.')
        server.command('vse list withVersion', '0 negotiated Vivecraft sessions')
        server.command('vivecraft-server-config general.sendData get', 'general.sendData = true')
        server.command('vive general.sendData set false', 'Updated general.sendData.')
        server.command('vse general.sendData get', 'general.sendData = false')
        configs = list((folder / 'plugins').rglob('config.toml'))
        assert len(configs) == 1, configs
        config = configs[0]
        text = config.read_text(encoding='utf-8')
        assert tomllib.loads(text)['settings']['general.sendData'] is False
        server.command('vive reload', 'Configuration reloaded.')
        server.command('vive teleport.enabled set true', 'feature is NOT enabled')
        yaml = config.parent / 'staged.yml'
        yaml.write_text('general:\n  sendData: true\nteleport:\n  enabled: true\n', encoding='utf-8')
        source_hash, active_hash = digest(yaml), digest(config)
        server.command('vive import-yaml staged.yml', 'Created config.imported.toml; NOT activated.')
        assert digest(yaml) == source_hash and digest(config) == active_hash
        imported = config.parent / 'config.imported.toml'
        assert tomllib.loads(imported.read_text(encoding='utf-8'))['settings']['general.sendData'] is True
        import_hash = digest(imported)
        start = len(server.lines)
        server.process.stdin.write('vive import-yaml staged.yml\n')
        server.process.stdin.flush()
        # Error text depends on the WASI errno rendering; require an explicit existing-file error.
        server.wait_for('exist', start=start)
        assert digest(imported) == import_hash and digest(config) == active_hash and digest(yaml) == source_hash
        config.write_text('[settings]\n"general.sendData" = 123\n', encoding='utf-8')
        server.command('vive reload', 'wrong type for general.sendData')
        server.command('vive general.sendData get', 'general.sendData = false')
        config.write_text(text, encoding='utf-8')
        server.command('vive reload', 'Configuration reloaded.')
        result['checks']['load_commands_import_rollback'] = 'passed'
        server.stop()
        server = None
        result['checks']['first_shutdown'] = 'clean exit 0'
        assert not list(config.parent.glob('*.pending'))
        server = Server(binary, folder, 2)
        server.wait_for('Server is now running.', timeout=180)
        server.wait_for('Loaded Vivecraft Pumpkin (0.1.0)', timeout=20)
        check_safety(folder)
        result['checks']['restart_bindings'] = assert_bindings(server.process.pid)
        server.command('vive general.sendData get', 'general.sendData = false')
        server.command('vive general.sendData reset', 'Updated general.sendData.')
        server.command('vive general.sendData get', 'general.sendData = true')
        server.stop()
        server = None
        result['checks']['restart_persistence_cleanup'] = 'passed, clean exit 0'
        try:
            connection = socket.create_connection(('127.0.0.1', 25575), timeout=1)
        except OSError:
            result['checks']['listener_closed'] = True
        else:
            connection.close()
            raise RuntimeError('test port still listening after shutdown')
        result['outcome'] = 'passed'
    except BaseException as error:
        result['outcome'] = 'failed'
        result['error'] = repr(error)
        raise
    finally:
        if server:
            try:
                server.stop()
            except Exception as error:
                result['cleanup_error'] = repr(error)
        (folder / 'smoke-result.json').write_text(json.dumps(result, indent=2), encoding='utf-8')
        print('RESULT: ' + json.dumps(result), flush=True)


if __name__ == '__main__':
    main()
