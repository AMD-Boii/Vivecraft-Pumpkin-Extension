# Pumpkin host smoke test — 2026-10-29

**Result: passed after fixing permission-node registration in the adapter.** This verifies real
host loading, console commands, WASI configuration/import behavior and full server restart/shutdown.
It does not verify player login, player permission decisions, custom-payload exchange with an actual
Vivecraft client, VR rendering/FBT, or full feature parity.

## Exact artifacts

- Pumpkin release: `0.2.1+26.3-26.51`, Java protocol `777`.
- Windows server SHA-256: `13daca2a4ee351ca4ad91c9ed94ffc8e7d9e3eb41cdc96e107accf9d1f522812` (rechecked before execution).
- Tested WASM SHA-256: `b918b0e80f1c9d5b834834c6be0f657322e5c0ade379814a416b8f46c708411f`.
- Native tests: 15 passed; formatting, native/adapter Clippy with warnings denied and release WASM build passed after the fix.
- Test world: freshly created `smoke-world`, seed `1`, overworld only. The second start reused **only this newly created test world** to check persistence.

The self-contained harness is [scripts/smoke_pumpkin.py](../../scripts/smoke_pumpkin.py), with
[the pinned-server configuration overlay](../../scripts/smoke-pumpkin.toml). It creates a new
unique directory under ignored `.local/smoke-runs/` on every invocation; no existing server is reused.

## Failure found and fixed

The original component compiled but failed `on_load`:

```text
Permission vivecraft.config must use the plugin's namespace (Vivecraft Pumpkin)
```

Pumpkin's context requires registered nodes to begin with `<metadata.name>:`. The adapter now
registers `Vivecraft Pumpkin:vivecraft.config` and corresponding namespaced PvP permissions.
A blocking `PlayerPermissionCheckEvent` bridge preserves queries for the original `vivecraft.*`
names: explicit legacy grant/denial is honored, otherwise an existing grant is retained or the
namespaced permission supplies the default. Native queries are not remapped recursively. Command
names and the legacy `vivecraft.config` gate remain unchanged. Actual player attachment/default
behavior still needs a player-based test; console success is not evidence of those cases.

Command registration now precedes tick/lifecycle event registration, so a permission-registration
failure cannot leave those callbacks attached. No Pumpkin source change or global permission
bypass was used. The first interrupted terminal attempt stopped cleanly; the successful harness
run used its own controlled process and stdin rather than the terminal startup detector.

## Passing checks

1. Plugin loaded on both starts:

   ```text
   INFO Loaded Vivecraft Pumpkin (0.1.0)
   WARN Plugin "Vivecraft Pumpkin" uses the following permissions: ["fs.read.data", "fs.write.data"]
   ```

2. OS socket inspection showed **only TCP `127.0.0.1:25575` and no UDP endpoints** for the server
   process on both starts. A real Minecraft status handshake returned version `26.3`, protocol
   `777`, max players `2`, online `0`.
3. Console command dispatch passed through `/vivecraft-server-config`, `/vive`, and `/vse`:
   help, list, get, set, reload and reset. This was not merely a successful process launch.
4. Setting `general.sendData=false` changed the on-disk TOML, survived a real server restart,
   and could be reset to `true` after restarting.
5. Editing `teleport.enabled` reported **stored only / NOT enabled**, consistent with preview policy.
6. `import-yaml staged.yml` created `config.imported.toml` inside the actual WASI data directory.
   The staged YAML and active TOML hashes stayed unchanged. A repeated import failed with
   `File exists`, and the imported output hash stayed unchanged.
7. A deliberately malformed active TOML caused reload to report `wrong type for general.sendData`;
   a subsequent live query still returned `false`. The valid test TOML was restored afterward.
8. Both shutdowns logged `All plugins unloaded successfully`, completed world saving and exited
   with code `0`. No `.pending` configuration file remained. Independent OS inspection after the
   test found no Pumpkin process or listener on port `25575`.

## Safety constraints observed

- The downloaded executable directory was not used as cwd. Harness source stayed in `scripts/`.
- Bindings were loopback-only. Bedrock, query, management and LAN discovery were disabled.
- Telemetry was explicitly disabled before the first launch and rechecked after config default filling.
- Online-mode and encryption remained enabled. No offline player-login bypass was used.
- Signature verification and permission confirmation remained enabled globally. The test configuration
  permitted only this local unsigned development plugin and preapproved its two data-folder permissions.
- No license prompt was answered; no EULA file was created. No firewall/system security settings changed.
- No existing server/world, Java source, or Pumpkin source was touched. Nothing was published or committed.

## Evidence files

Successful test directory (ignored local evidence, not a repository path):
`.local/smoke-runs/pumpkin-20261029-181934-eefe72a1/`.

The machine-readable result and two console logs remain on the verification machine under that
ignored directory. They are intentionally not linked as repository files: they contain raw host
output and may not exist for another checkout. This report records the relevant hashes, commands,
checks and safety constraints without publishing runtime logs or player/server data.

## Repeat the host test

After building the WASM, run from the project root with Python 3.11+ and a checksum-matching
server binary. The script refuses a wrong server hash, a busy test port and directory reuse.
It has finite startup/command/shutdown timeouts and records failures as well as successes.

```bash
python -I scripts/smoke_pumpkin.py --binary .local/downloads/pumpkin-0.2.1-x64/pumpkin-X64-Windows.exe --artifact target/wasm32-wasip2/release/vivecraft_pumpkin.wasm
```

The script currently uses Windows PowerShell socket inspection. It does not claim Linux smoke-test
coverage or plugin-only hot reload. See [testing.md](testing.md) for the remaining real-client gates.
