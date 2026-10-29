# Vivecraft Pumpkin Extension

A Rust/WebAssembly port of the [Vivecraft Spigot Extension](https://github.com/Vivecraft/Vivecraft-Spigot-Extension)
for [Pumpkin](https://github.com/Pumpkin-MC/Pumpkin). This is a community port, not an official
Vivecraft, Pumpkin, Mojang, or Microsoft product.

**Current version: 0.1.0 — sessions feature set, stable in maintainer testing.** The maintainer
reports successful in-game testing with no observed issues (2026-10-30). Automated native
checks and the recorded real-server smoke test also pass. **Stable does not mean full parity:**
controller interactions, locomotion, combat, and other Java-plugin features listed below are
not implemented. The original Java/Gradle project remains intact as the behavior reference
and for continued upstream maintenance.

## AI-assisted development

This Pumpkin port was created with **AI assistance from Claude / Claude Code**, including
its Rust/WASM implementation, tests, build/release tooling, and documentation. The maintainer
directs the project and reports in-game validation. AI-assisted contributions remain subject
to review, testing, and the same licensing requirements as other contributions.

This disclosure concerns the Pumpkin port, not how the original upstream Java project was
authored. Upstream authorship, copyright notices, and licenses remain unchanged.

## Supported target

| Item | Pin |
| --- | --- |
| Pumpkin / SDK | `0.2.1+26.3-26.51` |
| Minecraft | Java `26.3` |
| Vivecraft reference client | `26.3-1.3.17` (Fabric) |
| Rust | `1.96.0`, edition 2024 |
| Plugin format | WASI component, `wasm32-wasip2`, Pumpkin WIT v0.1 |
| Live Vivecraft wire negotiation | Versions **7–10** |

The reference Fabric environment uses Java 25, Fabric Loader 0.19.5 and Fabric API
0.160.4+26.3. These are **client** requirements, not Rust build dependencies. Older wire
formats have regression fixtures, but older Minecraft/live legacy clients are not claimed
as supported. [Exact source pins, checksums and capability matrix](docs/migration/target-and-capabilities.md).

## What works — and what does not

Implemented:

- VERSION negotiation, VR/non-VR switching, standing/seated/handedness and full-body pose state.
- World/height scales, active body part, draw/aim state and bounded pose history.
- Per-tick pose relay with same-world/range/visibility filtering and configurable owner echo.
- Session cleanup on VR-off, leave, duplicate handshake, respawn/world change and unload.
- Bounded payload parsing and rate-limited rejection diagnostics.
- TOML configuration, explicit non-destructive YAML import, existing command aliases and permission names.
- A read-only versioned IPC sessions query; this is not complete Java VRAPI compatibility.

**Not implemented:** controller execution/aim correction; teleport/climbing/crawl physics;
projectile/combat/mob changes; dual wielding/roomscale block execution; custom recipes;
access restrictions/messages/debug behavior; full native integration equivalents.
Recognized legacy settings for those features are stored under `[inactive]`, **not applied**.
The handshake explicitly disables unsupported gameplay capabilities. Full parity remains the
project goal; [contributing/upstream maintenance](CONTRIBUTING.md) explains how it is tracked.

## Build from source

Run commands from the repository root. Git, [rustup](https://rustup.rs/) and a native linker
are required. The checked-in [toolchain file](rust-toolchain.toml) selects Rust 1.96.0,
Clippy, rustfmt and the WASI target. Do not set a global/default WASM target: core tests run natively.

- **Windows:** [Microsoft Visual Studio 2022 Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/), “Desktop development with C++”
  workload, including MSVC x64/x86 and a Windows SDK. A standalone C++ compiler is not enough
  if `link.exe`/SDK libraries are missing. Reopen the shell after installation if necessary.
- **Linux:** a native C linker/toolchain (for example, Ubuntu's `build-essential`) and normal
  rustup prerequisites. The GitHub runner is Ubuntu 24.04.
- **Python 3.11+** is needed for the pin/fixture, repository and packaging checks, not for Rust compilation itself.
- No Java, Minecraft client, Pumpkin source checkout, or running server is needed to build the WASM.

If rustup was installed without changing PATH, expose its proxies in the **current Bash shell**:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

Install/confirm the pinned tools and fetch the exact locked dependencies:

```bash
rustup toolchain install 1.96.0 --profile minimal --component rustfmt,clippy --target wasm32-wasip2
```

```bash
cargo fetch --locked
```

Build the component:

```bash
cargo build --locked --release -p vivecraft-pumpkin --target wasm32-wasip2
```

Output: `target/wasm32-wasip2/release/vivecraft_pumpkin.wasm`. This is an **unsigned local
component**, not a JAR or native DLL. Commit `Cargo.lock`; Cargo ignores SemVer build metadata
in version requirements, so the lockfile and pin check establish the exact SDK build/checksum.
Crates are not published to crates.io (`publish = false`). The retained JVM build is separate;
see [the reference-build instructions](CONTRIBUTING.md#java-reference-build).

## Test and verify

On Windows, run all repository/Python/native/WASM checks:

```bash
powershell -NoProfile -File scripts/verify.ps1
```

After fetching dependencies, add `-Offline` to that invocation for disconnected Cargo checks.
`-CoreOnly` omits adapter lint/build; it is not the full release gate.

On other systems, or to run checks individually:

```bash
python -I scripts/check_repository.py
```

```bash
python -I scripts/check_pins.py
```

```bash
python -I -B -m unittest discover -s scripts/tests -v
```

```bash
cargo fmt --all -- --check
```

```bash
cargo test --locked -p vivecraft-core
```

```bash
cargo clippy --locked -p vivecraft-core --all-targets -- -D warnings
```

```bash
cargo clippy --locked -p vivecraft-pumpkin --target wasm32-wasip2 -- -D warnings
```

Then run the release build shown above. Cargo commands support `--offline` once all locked
sources are cached. [Testing records and the real-client checklist](docs/migration/testing.md)
distinguish maintainer-reported in-game success, synthetic fixture tests, host smoke tests and
unrecorded detailed scenarios. Do not substitute compilation for in-game testing of a new behavior.

## Install on Pumpkin

1. Back up your server/world and first try a new build in a disposable test server. Use the
   pinned Pumpkin release from its official source and verify the published checksum.
2. Put **one** matching `.wasm` file in the server's `plugins/` directory. Remove old duplicate
   versions of this plugin; never copy a `.zip`, `.tar.gz`, `.sigstore.json` or `.jar` there.
3. Keep global signature verification and permission confirmation enabled. Grant only this
   plugin's **`fs.read.data`** and **`fs.write.data`** permissions. It needs no network, environment,
   or unrestricted filesystem access.
4. Start/restart Pumpkin. Confirm `Loaded Vivecraft Pumpkin (0.1.0)` and use console input
   `vive general.sendData get` to confirm successful command registration before joining.
5. Join with the supported Vivecraft client. Full server restart is the verified lifecycle;
   plugin-only hot reload is not a promised replacement.

For an unsigned **local or GitHub build**, merge this into the corresponding sections of the
server-generated `pumpkin.toml` (do not duplicate existing tables):

```toml
[plugins]
verify_signatures = true
ask_permission_confirmation = true
allow_unsigned = false

[plugins.overrides."Vivecraft Pumpkin"]
allow_unsigned = true
allowed_permissions = ["fs.read.data", "fs.write.data"]
```

This is a per-plugin development exception, not a trusted Market signature. A Market-delivered
signed component should not need it: verify that download and remove the exception when using
it. Do not disable global safeguards to hide a signature/permission failure. The pinned host's
policy and the difference between [GitHub provenance and Market signing](docs/releasing.md#signing-what-each-signature-means)
are important; a checksum alone does not authenticate a publisher.

A reproducible Windows localhost-only smoke harness is available in
[the host smoke report](docs/migration/smoke-test-2026-10-29.md). It creates its own disposable
world, leaves authentication enabled and does not reuse an existing server directory.

## Configuration and commands

The plugin creates `config.toml` in Pumpkin's per-plugin data directory.
[config.example.toml](config.example.toml) documents the stored values:

- `[settings]`: the four effective controls — `general.requestData`, `general.sendData`,
  `general.sendDataToOwner`, `vrSwitching.enabled`.
- `[inactive]`: recognized Java settings preserved for later parity work, including Spigot-only values.
- `[unmapped]`: unknown imported values retained as data with warnings.

Command: `/vivecraft-server-config`, aliases `/vive` and `/vse`. Requires `vivecraft.config`
(operator level 2 by default). Pumpkin registration uses namespaced defaults such as
`Vivecraft Pumpkin:vivecraft.config` with a legacy-name permission bridge. Explicit legacy
grants/denials are preserved. PvP nodes have no gameplay effect until combat is implemented.

Examples of **Minecraft console/chat input, not shell commands** (prepend `/` in chat):

```text
vive help
vive list withVersion
vive general.sendData get
vive general.sendData set false
vive general.sendData reset
vive climbey.blocklist add stone
vive reload
```

Inactive edits say “stored only / NOT enabled.” Invalid input or malformed reload keeps the
previous live configuration. Same-directory temporary-file replacement avoids partial writes;
a stale `config.toml.pending` causes a safe failure rather than an overwrite. Stop the server
and inspect/back up the files before resolving such a conflict.

### Import a Java/YAML configuration

1. Copy the YAML yourself into this plugin's data directory as a regular file, e.g. `old.yml`.
2. As operator/console, issue `vive import-yaml old.yml`.
3. Review `config.imported.toml` and warnings. Source YAML and active TOML are unchanged.
4. Back up and deliberately replace the active TOML yourself, then issue `vive reload`.

Import never activates automatically, follows paths outside the data directory, or overwrites
an existing import output. Aliases, anchors, tags, merge keys, duplicate keys, complex mapping
keys and multiple documents are rejected. Limits: 1 MiB, 32 nesting levels.

## GitHub builds, signing and Pumpkin Market

[**Release and GitHub Actions setup**](docs/releasing.md) is the step-by-step operator guide.

- **Rust build and verification** runs on branch pushes/PRs/manual dispatch and supplies a
  `pumpkin-wasm` artifact: WASM, notices, corresponding source and SHA-256 sums. CI artifacts
  expire after 14 days and are **not** enduring public releases or Market-signed plugins.
- A `pumpkin-v<workspace version>` tag invokes the same verified build, then a protected
  `pumpkin-release` job creates **GitHub OIDC/Sigstore-signed build attestations** and a
  **draft** release. Configure that GitHub environment before the first tag.
- No private signing key, Marketplace token, Modrinth token or CurseForge token is required.
  PR jobs have no publication/signing permissions. The inherited JVM publisher is guarded
  to run only in the original parent repository.
- GitHub attestations do **not** embed Pumpkin's `wasm_signature` section or claim Market
  endorsement. Submit the verified original WASM manually using the official Market's
  current process; follow its signing/moderation requirements and verify the delivered copy.

Distribute the binary with its matching notices and complete source archive from the same
release. Source/dependency vendoring happens only in ignored/generated outputs, never Git.

## Troubleshooting and maintenance

- `link.exe`/native-link errors: install the MSVC C++ workload and SDK, then reopen the shell.
- Adapter build errors on the native target: pass `--target wasm32-wasip2`; run core tests natively.
- Missing `tomllib`: use Python 3.11 or newer. Fixture checks require no pip packages.
- Host ABI/import errors: confirm the exact server/SDK pins; do not update just one independently.
- A setting has no gameplay effect: check `[inactive]` and the capability matrix first.
- Read [CONTRIBUTING.md](CONTRIBUTING.md) before changing upstream behavior or updating pins.

`vivecraft-core` owns protocol/session/policy state; `vivecraft-pumpkin` owns host resources
and execution. Host methods and IPC are never called while holding session locks.

- [Migration decisions](docs/adr/0001-pumpkin-migration-direction.md)
- [Glossary](GLOSSARY.md)
- [Architecture review](docs/migration/architecture-review.html)
- [Progress and continuation handoff](docs/migration/HANDOFF.md)

## License

**LGPL-3.0-only**; see [LICENSE.md](LICENSE.md), the incorporated
[GPLv3 text](LICENSES/GPL-3.0-only.txt), and
[third-party notices and distribution requirements](THIRD_PARTY_NOTICES.md).
Original authorship/history is retained. No Minecraft binaries/assets, worlds, credentials,
server downloads, or private signing keys belong in the repository. This inventory is not
legal advice; publishing still requires your review of applicable licenses and Market terms.
