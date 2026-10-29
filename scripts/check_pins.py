"""Read-only consistency checks for release pins and the independent fixture oracle."""
import contextlib
import importlib.util
import io
import json
import sys
from pathlib import Path
import tomllib

sys.dont_write_bytecode = True
root = Path(__file__).resolve().parent.parent
lock = tomllib.loads((root / 'Cargo.lock').read_text(encoding='utf-8'))
workspace = tomllib.loads((root / 'Cargo.toml').read_text(encoding='utf-8'))
toolchain = tomllib.loads((root / 'rust-toolchain.toml').read_text(encoding='utf-8'))['toolchain']
assert toolchain['channel'] == '1.96.0', toolchain
assert workspace['workspace']['package']['rust-version'] == '1.96', workspace
assert 'wasm32-wasip2' in toolchain['targets'], toolchain
for name in ('vivecraft-core', 'vivecraft-pumpkin'):
    package = next(p for p in lock['package'] if p['name'] == name)
    assert package['version'] == workspace['workspace']['package']['version'], package
sdk = next(p for p in lock['package'] if p['name'] == 'pumpkin-plugin-api')
assert sdk['version'] == '0.2.1+26.3-26.51', sdk
assert sdk['checksum'] == '20a6a0d01590c31f1c55082acac8d23e83b136b056bc44ce2aaff1b18094f5ed', sdk
spec = importlib.util.spec_from_file_location('oracle', root / 'scripts/protocol_vectors.py')
module = importlib.util.module_from_spec(spec)
output = io.StringIO()
with contextlib.redirect_stdout(output):
    spec.loader.exec_module(module)
expected = json.loads(output.getvalue())
actual = json.loads((root / 'crates/vivecraft-core/tests/fixtures/vectors.json').read_text(encoding='utf-8'))
assert actual == expected, 'Checked-in fixtures differ from the independent oracle'
print('Rust/workspace pins, SDK full version/checksum and independent protocol fixture corpus match.')
