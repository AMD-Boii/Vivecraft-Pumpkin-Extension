# Sessions preview verification

## Recorded results — updated 2026-10-30

| Check | Result |
| --- | --- |
| Native core tests | Passed: 15 tests (3 internal historical/codec, 5 configuration/import, 7 sessions/relay/lifecycle) |
| Strict native Clippy | Passed, all targets, warnings denied |
| Strict WASM adapter Clippy | Passed, warnings denied |
| WASM release build | Passed for `wasm32-wasip2` |
| SDK full version / checksum | Matches the selected published package |
| Independent Python fixture oracle | Matches the checked-in corpus |
| Pumpkin host load/unload | Passed on two starts and clean shutdowns; [smoke-test report](smoke-test-2026-10-29.md) |
| Command dispatch / configuration writes inside WASI | Passed: all aliases, persistence/reset, YAML import/overwrite refusal, malformed-reload rollback |
| Maintainer-reported in-game check | Passed without observed issues, reported 2026-10-30; detailed client/observer/vanilla/FBT cases were not enumerated |
| Release/repository helper tests | 16 checks: 15 passed, 1 OS-level symlink-creation test skipped; platform-independent symlink-rejection test passed |
| Local release packaging | Passed: binary/checksums, licenses, locked vendored source and Rust runtime notices; local dirty package is not publishable |
| GitHub CI/OIDC release / Market signing | Prepared locally; not run remotely |
| Complete gameplay/integration parity | **Not implemented; later milestones** |

Artifact: `target/wasm32-wasip2/release/vivecraft_pumpkin.wasm`.
These results establish native regression coverage, real host/console behavior and maintainer-reported
in-game stability for the implemented sessions set, **not complete Java-plugin parity**. The report
does not enumerate every FBT, player-permission or plugin-only hot-reload case; retain those as detailed
acceptance scenarios rather than claiming them passed. The original automated host runs created only
fresh disposable worlds; no preexisting server/world was touched. No new in-game run was performed
as part of the release-preparation work.

The initial build failed because MSVC `link.exe` was absent. With explicit user approval,
Rust 1.96.0, its WASI/format/lint components and Microsoft C++ Build Tools were installed.
Rust's global PATH was not modified. SDK signature mismatches and lint errors found during the first
compilation attempts were corrected before the successful runs. The resulting verification used
cached dependencies with `--offline --locked`.

The pinned official Windows Pumpkin binary was downloaded to its own ignored `.local/downloads/`
directory and matched the release's SHA-256:

```text
13daca2a4ee351ca4ad91c9ed94ffc8e7d9e3eb41cdc96e107accf9d1f522812
```

Execution was initially denied by the environment's external-code permission control. On 2026-10-29
the user explicitly authorized the isolated smoke test. The real run exposed and fixed permission
namespace registration; the rebuilt plugin then passed both starts, commands and shutdowns.
No server source changes were needed. Source inspection showed that this server reads configuration
from its working directory before parsing its limited CLI; **do not treat `--help` as a harmless
no-start probe**. A safe run requires an isolated, preconfigured working directory.

## Repeatable automated checks

On Windows:

```bash
powershell -NoProfile -File scripts/verify.ps1
```

Or run each Cargo command in the root README. To keep verification disconnected after fetching
dependencies, add `--offline` to Cargo invocations. Never replace the lockfile with a moving SDK version.

Check fixture/pin provenance (Python 3.11+):

```bash
python -I scripts/check_pins.py
```

The fixtures are independently generated synthetic bytes, not live client captures. The oracle
prints vectors and does not overwrite fixtures. Tests use the public sessions/configuration
interfaces; the private historical-assembly tests cover compatibility code not admitted by this preview.
Malformed-frame coverage includes deterministic bounded pseudo-random inputs, truncation, invalid
enums/booleans/UTF-8/version ranges, non-finite values, stale generations, visibility/range edges,
VR-off and duplicate handshakes. This is regression coverage, not an exhaustive fuzzing claim.

Current defensive limits: 64 KiB payloads; 512-byte VERSION contents; at most 1,024 sessions;
201 history entries; scales in 0.01..100; finite pose coordinates within 32 million blocks and
quaternion squared norm in 1e-12..4 (normal clients emit unit rotations). Invalid data is rejected,
not normalized/clamped. YAML has 1 MiB, 32-level and 20,000-event limits, with stream validation
before aliases could resolve or duplicate keys could overwrite each other.

## Host load / command smoke gate — passed

The [2026-10-29 report](smoke-test-2026-10-29.md) records the exact hashes, socket checks,
commands and raw evidence. The reproducible harness is `scripts/smoke_pumpkin.py`; the steps
below describe its safety constraints and the remaining player/hot-reload checks.

For a new isolated run:

1. Create a **new disposable server directory**, separate from the downloaded binary directory,
   the repository root, and any existing server installation. Put authored harness scripts elsewhere.
2. Inspect the pinned server's `pumpkin.toml` schema before launch. It is derived from
   `crates/pumpkin-config/src/lib.rs`, with `networking/java.rs`, `networking/mod.rs`, `plugins.rs`
   and `telemetry.rs` at source commit `224f9f8144ca53a8d0800acb87607d714196e2a6`.
   Bind Java to `127.0.0.1:25575`; disable Bedrock, query/management, LAN announcements and telemetry;
   use a tiny disposable world/view distance. Keep online authentication/encryption on for real-client
   testing. Inspect actual listeners after launch. Do not accept an EULA or alter firewall policy automatically.
3. Copy the built WASM into that directory's `plugins/` folder. Review the per-plugin unsigned-development
   policy and approve only `fs.read.data` / `fs.write.data` for `Vivecraft Pumpkin`.
   Keep signature verification and permission confirmation globally enabled; do not grant network or environment access.
4. Launch the binary by its absolute path **with the isolated directory as cwd** in a user-controlled terminal.
   Verify loader logs, no ABI/import mismatch, the expected plugin metadata, and no unintended listeners/telemetry.
5. Interact through the actual console (these are Minecraft commands, not shell commands):

   ```text
   vive help
   vive list withVersion
   vive general.sendData get
   vive general.sendData set false
   vive reload
   vive general.sendData reset
   ```

   Confirm aliases, actual permission enforcement, persistence and effective values. A malformed TOML reload must
   preserve the previous live settings. Inactive settings must explicitly say they are not enabled.
6. Stage a small YAML file inside the plugin's data directory, invoke `vive import-yaml staged.yml`, compare source
   and active-config hashes, inspect the imported TOML and warnings, and repeat to confirm overwrite refusal.
7. Stop/restart the disposable server to verify cleanup and persisted configuration. Only use the pinned server's
   documented plugin reload command if it exists; do not assume Bukkit `/reload` semantics. Check unloaded callbacks
   and resource cleanup in host logs. Stop the test server cleanly and leave no background listener running.

The launch/config harness is verified on Windows for the pinned release. It checks loopback-only
bindings, performs a real Minecraft status handshake, drives actual console commands, and stops both
server processes. Player permission enforcement and plugin-only hot reload are not covered by it.

## Detailed client acceptance checklist

The maintainer reports successful in-game testing without observed issues on 2026-10-30; the exact
client/observer/vanilla/FBT case matrix was not supplied. Treat the current sessions feature set as
stable in tested use, and retain the following as a reproducible per-scenario checklist, not a claim
that none of the plugin has been tested in game or that every item passed.

Use the pinned Fabric reference setup, a valid client account and available VR hardware. A synthetic
protocol peer may test transport bytes, but cannot establish real-client rendering or VR behavior.

- Join one Vivecraft VR client and a second Vivecraft observer, plus a vanilla client as a regression control.
- Confirm actual VERSION negotiation and explicit disabled teleport/crawl/climbing/dual-wield/blocking settings.
  The client treats VERSION itself as teleport support; absence of an enable packet is insufficient.
- Compare actual captured handshake/pose/relay bytes against the documented layouts, recording capture provenance
  separately from synthetic fixtures. Do not commit account tokens or personal identifiers.
- Verify standing/seated, handedness, calibrated scales and any available FBT trackers visually.
- Switch VR off/on; respawn; change dimensions; move across the range edge; hide/show a player using host visibility;
  disconnect/reconnect. Check inactive notifications and no stale/hidden/cross-world pose relay.
- Check owner echo with its setting both ways and that a vanilla client receives no Vivecraft payloads.
- Exercise operator and ordinary-player command permissions; reload/import through the real host.
- Verify unload/restart and no resource/callback leaks. Record unsupported gameplay as unavailable, not working by omission.

## Release-preparation checks — 2026-10-30

With explicit approval, the missing Rust 1.96.0/WASI/rustfmt/Clippy tools and Microsoft Visual Studio
2022 C++ build workload were installed on the current machine. The Rust installer checksum was
verified against its official distribution; no global PATH change was requested. The Windows
verification wrapper passed all 15 native Rust tests, formatting, strict native/WASM Clippy and the
WASM release build with locked offline Cargo sources. Gameplay source was not changed.

`python -I scripts/check_repository.py`, `python -I scripts/check_pins.py`, and the Python
release-helper suite passed. The symlink-creation test was skipped by Windows permissions, not
reported as executed. The package generator ran successfully against the local pre-commit tree;
its output is explicitly marked LOCAL and must not be published. It preserves dependency license
texts, the pinned runtime's notices and full locked vendored source outside Git.

The package included **78 locked registry packages**; checksums and required source/license entries
passed inspection. An archive extracted into a fresh ignored directory passed all 15 native tests
and rebuilt the WASM using `--locked --offline` with an **initially empty Cargo home**. Both original
and rebuilt components were below the 5 MiB Market input gate. Different build paths changed their
hashes; this is a rebuildability check, not a bit-identical reproducibility claim.

With explicit permission to run the official checksum-verified `rhysd/actionlint` binary,
**actionlint 1.7.12 passed all three workflows**. Relative Markdown file links were checked too.
The OS-level symlink integration case remains skipped, while its simulated cross-platform unit
case passes. No remote CI/signing/Market operation was run to obtain these results.

The [release guide](../releasing.md) documents clean-checkout packaging, GitHub OIDC provenance,
corresponding-source rebuilding and manual Market submission. A local package is not proof that
GitHub OIDC issuance, draft creation, Market moderation/signing, or loading a delivered Market
binary has run. Those external operations remain unperformed until the relevant real run.

## Remaining parity work

The approved route remains: sessions → locomotion/interaction → combat/world behavior → native integrations/operator
parity → full release gate. All original config keys/features must receive implemented/tested behavior or an explicit
user-approved disposition. No Pumpkin server change, Java-source removal or public release is authorized implicitly.
The [capability matrix](target-and-capabilities.md) and [handoff](HANDOFF.md) track the open gates.
