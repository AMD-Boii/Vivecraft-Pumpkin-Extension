# Vivecraft → Pumpkin: implementation handoff

## Accepted direction

Rewrite the Spigot Vivecraft server extension for Pumpkin. The user accepted Rust/WASM;
a pinned Minecraft Java 26.3 / Pumpkin / Vivecraft pair; full parity through milestones;
historical wire coverage without older-Minecraft claims; TOML with non-destructive YAML
import; stable commands/permissions; and native integration equivalents without Java binary
compatibility. Changes to Pumpkin itself still require separate approval.

The authorized host/console/import/restart/shutdown smoke test passed on 2026-10-29 after a
permission-registration fix; see [the report](smoke-test-2026-10-29.md). On 2026-10-30 the
maintainer reported successful in-game testing without observed issues and asked that the
implemented sessions set be treated as stable. Do not reopen those decisions or invent a
per-scenario client matrix that was not supplied. The full Java parity port is not complete.

- [ADR 0001](../adr/0001-pumpkin-migration-direction.md): direction and constraints.
- [Target/capability matrix](target-and-capabilities.md): source pins and unfinished parity.
- [Testing](testing.md): recorded evidence and reproducible checks.
- [Release guide](../releasing.md): GitHub setup, provenance and manual Market submission.
- [Contributing](../../CONTRIBUTING.md): retaining/synchronizing the Java parent.

## Implemented

- Root Rust workspace/lockfile/toolchain pin alongside unchanged Java/Gradle behavior source.
- `vivecraft-core`: protocol/pose/legacy helpers behind `Sessions`, explicit wire values,
  snapshots/effects, live versions 7..10, FBT, history, relay filtering and lifecycle cleanup.
- Configuration: four effective controls, recognized legacy keys retained as inactive, unknown
  import data retained as unmapped, TOML edits and bounded event-validated YAML import. Import
  never activates or overwrites the staged source/active settings automatically.
- `vivecraft-pumpkin`: published SDK WIT v0.1, lifecycle/transport/tick/cleanup handlers,
  aliases/permissions, own-data-folder persistence and a read-only preview IPC sessions query.
- Independent Python fixture oracle and native regression tests; Windows verification wrapper.
- Publication inventory checks, legal/provenance texts, source/notice packaging helpers/tests,
  SHA-pinned read-only CI, gated OIDC-attested draft release workflow, README and maintenance docs.

Test through consumer interfaces; do not export internals just for tests. Never call Pumpkin
methods or IPC while holding session locks. Host handles remain outside sessions. Outgoing
poses carry lifetime information so teardown invalidates pending effects.

## Verified state

The native suite has 15 passing tests (3 internal historical/codec, 5 config/import,
7 sessions/relay/lifecycle). Formatting and strict native/WASI Clippy pass, and the release
component builds at `target/wasm32-wasip2/release/vivecraft_pumpkin.wasm`. SDK version/checksum
and fixtures match the independent oracle. The 16 Python release/repository checks completed:
15 passed and one Windows symlink-creation integration case was skipped by OS permission;
the platform-independent symlink-rejection unit test passed. Local dirty packaging succeeded,
including locked vendored sources, dependency/runtime notices and checksums; it is **not a
publishable release**. Offline native tests and WASM build from the archive passed with an
initially empty Cargo home. actionlint 1.7.12 passed for all three workflows. Exact final
verification results belong in testing.md.

Fixtures are synthetic, not real-client captures. Preserve the distinction between the
maintainer's in-game success report and unenumerated FBT/player-permission/hot-reload cases.
Historical/asymmetric and outgoing gameplay codec breadth remain future work. Do not advertise
unsupported gameplay or admit legacy versions merely because a decoder exists.

## Historical host test — passed 2026-10-29

The official server hash was reverified. A fresh directory under `.local/smoke-runs/` bound
Java to 127.0.0.1:25575; OS inspection found no UDP endpoint or other owned TCP listener.
Telemetry, Bedrock, management/query and LAN discovery were disabled. Authentication,
encryption, global signature checks and permission confirmation stayed enabled. Only this
unsigned plugin's own-data-folder permissions were preapproved. No EULA/license prompt was
answered and no existing server/world changed.

The original `vivecraft.*` permission registration failed because Pumpkin requires names to
begin with `<plugin metadata name>:`. The adapter registers namespaced nodes and bridges legacy
queries through `PlayerPermissionCheckEvent`, honoring explicit grants/denials. Commands and
permissions register before tick/lifecycle handlers. The smoke used console, not player-based
permission attachments/defaults; do not claim those edge cases from console success.

Both server starts then loaded the WASM, exercised aliases/get/set/reset/list/reload, inactive
feedback, actual WASI YAML import/overwrite refusal, source/active hash preservation, malformed
reload rollback and persistence. Both clean shutdowns exited 0 with no listener/process left.
The historical run did not test player pose traffic or plugin-only hot reload. The subsequent
maintainer in-game report is separate evidence, not a retroactive change to that report.

`scripts/smoke_pumpkin.py` plus `scripts/smoke-pumpkin.toml` recreate a new disposable run,
drive stdin, probe the status socket, check bindings and stop the owned processes. Do not
use the pinned server's `--help` as a no-start probe: configuration/startup happens first.
Raw logs/results remain ignored local evidence, not broken public repository links.

## Environment and publication boundaries

- Git MCP is connected and was used for status/history/diff inspection. `origin` is the user's
  GitHub fork; `upstream` is `Vivecraft/Vivecraft-Spigot-Extension`. Keep both histories.
- With explicit approval on 2026-10-30, missing Rust 1.96.0/WASI/rustfmt/Clippy and VS 2022
  C++ Build Tools were installed. Rust's global PATH was not changed. Use the README shell
  setup or Windows wrapper; do not retain a developer's absolute user-specific tool paths.
- `target/`, `.local/`, `dist/`, local harness state and release/vendor outputs stay ignored.
  Downloaded executables/archives use separate fresh directories; authored scripts stay in `scripts/`.
- The maintainer requested public AI-assistance disclosure and authorized a local commit of
  the prepared project. Pushing, PRs, release uploads, Market submissions and Pumpkin edits
  are not authorized by that commit request and remain unperformed.
- GitHub releases use `pumpkin-v<workspace version>`, separate from parent Java tags/version.
  The inherited JVM publisher is guarded to its original upstream repository.
- Keyless GitHub OIDC/Sigstore provenance is not Pumpkin's embedded Ed25519 Market signature.
  No long-lived signing secret or fake Market identity is added. See the release guide before
  claiming signed/approved Market distribution or accepting developer terms.

## Next actions

1. Review the prepared source and push `master` only on the maintainer's instruction.
   Preserve Java reference, licenses, lockfile and source-provenance material.
2. Configure the `pumpkin-release` environment/tag rules, observe CI, and deliberately create
   `pumpkin-v0.1.0`. Review the attested draft and matching source/notices before publishing.
   GitHub/OIDC and Market operations have not been run from the local preparation work.
3. Follow the manual Market process using the verified original WASM and source/license links;
   verify a delivered copy and its actual embedded signature before claiming Market approval.
4. For continued development, record detailed VR/observer/vanilla/FBT/permission cases when the
   setup is available, with real captures separated from synthetic fixture provenance.
5. Extend historical/asymmetric and outgoing gameplay coverage, then locomotion/interaction,
   combat/world, native integrations/operator parity and the full parity gate. Every outstanding
   row remains implemented/tested work or needs an explicit user-approved disposition.

## Requested skills / workflow

The user previously requested mattpocock `grill-me`, `handoff`, and
`improve-codebase-architecture`. They were not installed as callable skills; their instructions
were read directly during the original migration. Do not invent tool skill names or claim
that this handoff installs those skills. Preserve settled decisions and ask only genuinely
unresolved questions. Initial delegated exploration could not start in the prior environment;
no agent findings exist.

The available `run` skill was consulted for the original server smoke test. Console/unit-test
success is not a substitute for a detailed actual-player matrix. Use available `code-review`
when requested. No review agent or separate session was started for release preparation;
keep this cohesive work here unless the user asks otherwise.
