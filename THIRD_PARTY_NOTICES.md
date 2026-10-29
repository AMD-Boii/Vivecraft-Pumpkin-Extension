# Copyright, licensing and provenance

## This project and the Java reference

Vivecraft Pumpkin Extension is a modified/ported version of
[Vivecraft/Vivecraft-Spigot-Extension](https://github.com/Vivecraft/Vivecraft-Spigot-Extension),
retained here as the Java/Gradle behavior reference. Copyright remains with the original
Vivecraft authors and contributors for their work; Pumpkin-port contributions remain with
their respective contributors. Git history records the original authors and changes.

The project is distributed under **LGPL-3.0-only**. The unmodified upstream LGPLv3 text is
[LICENSE.md](LICENSE.md); its incorporated GPLv3 text is
[LICENSES/GPL-3.0-only.txt](LICENSES/GPL-3.0-only.txt). This notice does not relicense any
third-party dependency or claim ownership of the original work.

**Modification notice (2026-10-30):** this fork adds a Rust/WASI sessions implementation,
Pumpkin host adapter, TOML configuration and YAML import, regression fixtures, build/release
tooling, and documentation. It does not implement all Java-plugin behavior. The original
Java/Gradle source and historical attribution are retained. The parent baseline and open
parity work are recorded in [the capability matrix](docs/migration/target-and-capabilities.md).

Protocol behavior was also inspected in the LGPLv3
[Vivecraft client at `7e610caf672c67e4501ba8675ba0d620ffeb17ee`](https://github.com/Vivecraft/VivecraftMod/tree/7e610caf672c67e4501ba8675ba0d620ffeb17ee).
Fixtures are independently generated synthetic data, not recordings of players or copied
client binaries; see [fixture provenance](crates/vivecraft-core/tests/fixtures/README.md).
No Minecraft client/server binary, account credential, world, game asset pack, or downloaded
Pumpkin executable is distributed in this repository or in the generated source archive.

## AI-assisted Pumpkin-port contributions

The Pumpkin port's implementation, tests, build/release tooling, and documentation were
developed with AI assistance from **Claude / Claude Code**, under the maintainer's direction.
This is a disclosure of the port's development process, not a claim about how the original
Vivecraft project was authored, a change to upstream attribution, or a new license grant.
Original and third-party copyright notices and license obligations remain in force.

## Compiled Rust dependencies and runtime

The pinned `pumpkin-plugin-api` SDK is **MIT OR Apache-2.0**, not the license of the whole
Pumpkin server. Other Cargo packages retain their own licenses. `Cargo.lock` identifies
exact resolved versions and checksums; license expressions are not inferred from package names.

The packaging tool runs `cargo vendor --locked --versioned-dirs` into a **generated source
archive**, never into tracked source. The accompanying notices archive contains:

- `DEPENDENCIES.md`: every resolved non-workspace package, version, declared SPDX expression,
  and matching path in the source archive;
- verbatim dependency license/copyright/notice files found in those vendored packages;
- the installed pinned Rust toolchain's runtime copyright report and license texts;
- this notice, the project licenses, the README and example configuration.

Preserving an SPDX field alone is not a substitute for license texts or copyright notices.
A package with missing license metadata or missing license files stops packaging for review.
The inventory includes build-only/platform-specific dependencies conservatively; inclusion
in the inventory is not a claim that every package is linked into the WASM.

Rust's standard library and compiler-supplied runtime have their own permissive and
third-party terms. Their notices are copied from the actual pinned toolchain rather than
replaced with this project's LGPL notice. The Rust compiler itself is not bundled.

## Gradle wrapper retained from upstream

`gradle/wrapper/gradle-wrapper.jar` is the only permitted checked-in compiled binary: it is
necessary source-build tooling, not a plugin/server binary. It is from Gradle 9.1.0,
Copyright Gradle contributors, under Apache-2.0; see
[LICENSES/Apache-2.0.txt](LICENSES/Apache-2.0.txt). Its SHA-256 is:

```text
76805e32c009c0cf0dd5d206bddc9fb22ea42e84db904b764f3047de095493f3
```

This matches the [official Gradle wrapper checksum](https://services.gradle.org/distributions/gradle-9.1.0-wrapper.jar.sha256).
`gradle-wrapper.properties` selects the upstream Gradle distribution. Java dependencies and
Minecraft-derived generated stubs are not vendored into the repository or Pumpkin release.

## Binary distribution and modification rights

Distribute each binary with its matching notices and corresponding-source archive from the
**same release**, and retain those assets. The source archive includes project/build scripts,
lockfile, and dependency source plus an offline Cargo source configuration. It permits a
recipient to modify and rebuild the plugin; do not make the original publisher's signing key
or a paid service necessary to build a modified plugin. Standard Rust/native platform build
tools must be installed separately. Installation of a locally rebuilt unsigned component is
documented without disabling verification globally.

Keep source and license links in the Pumpkin Market listing and release notes. Do not add
terms that prohibit the rights granted by the applicable open-source licenses, and do not
promise rights over upstream material that you do not hold. Signing authenticates an
artifact; it does not replace license compliance or Marketplace moderation. Review the
current Market/Mojang terms yourself before accepting them or publishing; this engineering
inventory is **not a legal opinion or a guarantee of approval**.

## Names and assets

Vivecraft, Pumpkin, Minecraft, and their associated names/marks belong to their respective
owners. This is a community port, not an official endorsement by those projects or Mojang/
Microsoft. No third-party logo is included. Use only an original or appropriately licensed
icon/screenshots when creating the Market listing. The inherited funding link supports
Vivecraft upstream; it is not a claim that upstream maintains this port.
