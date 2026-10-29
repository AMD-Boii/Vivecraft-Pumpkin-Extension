# Contributing and keeping upstream parity

## Project boundaries

This repository deliberately contains **both** the Rust/Pumpkin implementation and the
Java/Gradle parent reference. The parent is
[Vivecraft/Vivecraft-Spigot-Extension](https://github.com/Vivecraft/Vivecraft-Spigot-Extension).
The starting Java baseline is
[`81d5bf7edd041e72756c775335a1844930a97f8f`](https://github.com/Vivecraft/Vivecraft-Spigot-Extension/commit/81d5bf7edd041e72756c775335a1844930a97f8f)
(1.3.17-0); the first fork commit changed only its README. Preserve that history and upstream
copyright/license notices; do not squash the parent history into a new unrelated repository.

The approved route is **sessions → locomotion/interaction → combat/world → native
integrations/operator parity → full parity release gate**. Maintainer-reported stability of
the current sessions feature set does not cancel any remaining row of the
[capability matrix](docs/migration/target-and-capabilities.md).

- Do not delete/reorganize Java source, generated-code inputs, Gradle files or the wrapper
  just to make the repository look smaller. The ignored generated stubs/output are different
  from their checked-in generators.
- Port observable behavior, not Bukkit/NMS implementation machinery. Keep host-independent
  policy/state in `vivecraft-core` and host handles/execution in `vivecraft-pumpkin`.
- Do not call Pumpkin methods or IPC while holding session locks. Keep bounded decoding,
  explicit wire values, lifecycle generations and inactive-capability policy intact.
- A decodable packet or stored config key is not an implemented feature. Only advertise
  capabilities that have authoritative execution and regression/live acceptance coverage.
- Any needed Pumpkin-server change requires separate approval. A missing hook is an explicit
  investigation item, not permission to remove behavior or introduce a server fork silently.

## Sync changes from the parent

The local clone uses `origin` for this port and `upstream` for the parent. Check your remotes:

```bash
git remote -v
```

Only if `upstream` is missing, add the original project (do not replace `origin`):

```bash
git remote add upstream https://github.com/Vivecraft/Vivecraft-Spigot-Extension.git
```

With a clean working tree, create a maintenance branch and inspect incoming changes:

```bash
git switch -c maintenance/upstream-sync
```

```bash
git fetch upstream --tags
```

```bash
git log --oneline HEAD..upstream/master
```

```bash
git diff HEAD...upstream/master -- core versions gen build.gradle buildSrc gradle
```

For a full source sync, merge **without auto-committing**, inspect conflicts and keep both
histories (do not force-push or silently overwrite the Rust workspace/docs):

```bash
git merge --no-commit --no-ff upstream/master
```

Review parent changes feature by feature before completing the merge. Retain the guard in
`.github/workflows/publish-on-release.yml`; otherwise the fork could target the parent's
Modrinth/CurseForge IDs. Keep Pumpkin workflows and versioning separate. In an app-managed
worktree, use the app's base-branch sync facility when updating from that worktree's base;
parent comparison and deliberate upstream maintenance are separate operations.

For every upstream behavior/config/protocol change:

1. Record the exact source commit and affected matrix rows. Compare the pinned **client**
   too: Java enum ordinals and actual negotiated wire versions are not interchangeable.
2. Preserve or explicitly migrate settings/commands/permissions. An unimplemented setting
   remains visibly inactive; unknown import data must not disappear.
3. Add independent fixtures/interface-level regression tests, update the Rust behavior and
   verify the host path. Record limits or hook gaps rather than using unproven substitutes.
4. Run all [README checks](README.md#test-and-verify), then relevant real-server/client cases
   for changed gameplay. No in-game test may be claimed merely because compilation passes.
5. Update the capability matrix, test evidence and handoff. Review and commit the maintenance
   merge/port changes only when the intended semantics and notices are intact.

## Pins, fixtures and versioning

- `rust-toolchain.toml`, manifests, **committed `Cargo.lock`** and `scripts/check_pins.py`
  define the reproducible dependency target. Do not follow a moving SDK Git branch.
- Keep ordinary tests native; compile only the adapter as `wasm32-wasip2`. Do not add a
  workspace-wide WASM target that makes native tests silently stop running.
- The Python fixture oracle is independent of the Rust codec. Fixtures are synthetic, not
  recordings. Preserve [provenance](crates/vivecraft-core/tests/fixtures/README.md); sanitized
  real-client captures need their own provenance and must contain no tokens/player identifiers.
- Pumpkin versions live in `[workspace.package].version`; use **`pumpkin-v0.1.0`**-style tags.
  Parent Java `gradle.properties`/tags remain independent. Never reuse an upstream tag or move
  a released Pumpkin tag to changed bytes.
- Update pinned GitHub action SHAs deliberately, verify they belong to the official action
  repositories, review major-version behavior, and revalidate workflows. Do not relax PR
  permissions or sign PR-controlled output.

## Java reference build

This is **not** the Pumpkin build. It remains available for behavioral comparison and future
parent maintenance; it is not part of the Rust CI gate. The Java 26.x subprojects request
Java 25 bytecode, and the wrapper selects Gradle 9.1.0. Use an appropriate JDK (25 for those
subprojects), native tools/prerequisites required by the parent, and network access to its
external dependency/mapping repositories.

The parent workflow records this generator/build sequence:

```bash
./gradlew generateAccessors
```

```bash
./gradlew rebuildStubs
```

```bash
./gradlew release
```

Use `gradlew.bat` from a native Windows shell if needed. This sequence was **not rerun** as
part of Pumpkin release preparation. It may download Mojang/server/mapping material into
caches and generate Minecraft-derived stubs; those outputs must stay ignored and must not be
bundled with the Pumpkin source/binary release. Inspect applicable third-party terms before
redistributing anything beyond this repository's own source.

## Publication hygiene and legal review

Run `python -I scripts/check_repository.py` before staging. It checks Git-visible candidates,
not ignored files or historical Git objects, and is not an exhaustive secret scanner. Review
`git diff --cached` yourself. A private file accidentally staged despite ignore rules must be
removed from the index; do not publish secrets and assume deleting them later undoes exposure.
If a real secret ever reaches Git history, revoke it and agree a remediation plan before any
history rewrite.

Build artifacts/downloads, server worlds/logs, personal configuration and signing credentials
stay in ignored local storage. Preserve [license notices](THIRD_PARTY_NOTICES.md); release
vendoring belongs only in generated outputs. The only checked-in binary is the checksum-pinned
Gradle wrapper. There is no permission to redistribute Minecraft/Pumpkin executables or
third-party artwork by virtue of this port's license.

Use [the release guide](docs/releasing.md) for GitHub environment setup, keyless attestations,
source packaging and manual Market submission. Committing, pushing, accepting terms and
publishing are deliberate maintainer actions, not side effects of local verification.
