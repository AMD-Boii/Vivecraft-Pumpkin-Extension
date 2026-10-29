# Migration target and capability matrix

Status: stable-in-maintainer-tested-use sessions feature set, updated 2026-10-30. **The maintainer reports successful in-game testing without observed issues.** Native tests, WASM build and recorded real host/console/restart checks pass. This is not full Java-plugin parity; the report does not enumerate every client/FBT/permission/hot-reload case. The matrix below retains discovery evidence; current acceptance results are in [testing.md](testing.md) and the [host smoke report](smoke-test-2026-10-29.md).

## Implementation status

- `vivecraft-core`: bounded protocol/session state, FBT poses, historical decoder/assembly fixtures, history, relay/visibility/lifecycle cleanup, capability policy and native configuration/import tests.
- `vivecraft-pumpkin`: SDK-backed lifecycle, custom payload/tick/cleanup handlers, command aliases/permissions, data-directory configuration, and a read-only preview IPC query.
- Live negotiation is restricted to wire versions 7–10, which can explicitly disable teleport. Historical codec coverage does not assert old-client admission or old Minecraft support.
- Four effective settings: requestData, sendData, sendDataToOwner and VR switching. Other recognized settings remain visibly inactive; unknown imported values remain unmapped data.
- Remaining evidence/implementation work: detailed VR/observer/vanilla/FBT scenarios not enumerated in the maintainer report, player-permission edge cases and plugin-only hot reload, full outgoing gameplay packet repertoire, refined historical/asymmetric fixture coverage, and all remaining gameplay/integration milestones.
- The official binary checksum was reverified and the user-authorized isolated smoke test passed on 2026-10-29. Namespaced permission registration plus a legacy query bridge fixed the initial load failure. No existing server/world was touched. On 2026-10-30 the maintainer reported in-game testing with no issues; treat the implemented feature set as stable without claiming unimplemented parity.
- Publication preparation adds isolated GitHub build/release workflows, keyless provenance attestations, source/license packaging and [Market instructions](../releasing.md). Remote signing/publication has not run; it does not change gameplay capabilities.

## Selected initial target

The user authorized selection and pinning of one compatible target pair, Rust/WASM, full parity through milestones, approval before Pumpkin changes, and Vivecraft sessions as the first architectural focus.

| Item | Selected pin | Evidence / constraint |
| --- | --- | --- |
| Pumpkin | `0.2.1+26.3-26.51` | Released 2026-10-24; not nightly/canary. |
| Pumpkin source | `224f9f8144ca53a8d0800acb87607d714196e2a6` | Resolved from the release tag. |
| Minecraft edition/version | Java `26.3` | Server's current packet format and client's declared game version match. Java protocol number is `777`. |
| Vivecraft | `26.3-1.3.17` | Released 2026-10-08. |
| Vivecraft source | `7e610caf672c67e4501ba8675ba0d620ffeb17ee` | Resolved from the release tag. |
| Reference client artifact | `vivecraft-26.3-1.3.17-fabric.jar` | Select Fabric for the first test environment, not as a permanent exclusion of other loaders. |
| Fabric reference dependencies | Loader `0.19.5`; Fabric API `0.160.4+26.3`; Java `25` | Declared in pinned Vivecraft `gradle.properties`; not installed or launched here. |
| Rust plugin SDK | `pumpkin-plugin-api` `0.2.1+26.3-26.51` | Published 2026-10-25, not yanked. |
| SDK crate checksum | `20a6a0d01590c31f1c55082acac8d23e83b136b056bc44ce2aaff1b18094f5ed` | crates.io version metadata. Validate the resolved Cargo.lock package before using it. |
| SDK ABI | `pumpkin:plugin` WIT `v0.1` | The selected SDK's `src/lib.rs:205-214` generates bindings from `../pumpkin-plugin-wit/v0.1`. The server also supports v0.2; that does not make this SDK v0.2. |
| Build format | Rust `cdylib`, `wasm32-wasip2` | Current Rust plugin instructions. Validate the resulting component with the selected server. |
| Rust compiler | `1.96.0`, edition `2024` | Pinned in `rust-toolchain.toml`; installed with user approval and used for native tests/WASM build. |

The pins are represented by `rust-toolchain.toml`, workspace manifests and `Cargo.lock`; include all of them when committing/pushing the port. The SDK requirement is `=0.2.1`; Cargo ignores SemVer build metadata in requirements, so the recorded full version and checksum in Cargo.lock are checked by `scripts/check_pins.py`. No dependency tracks a moving Git branch.

The optional `pumpkin-plugin-utils` crate currently has only the older `0.1.0-dev+26.2-26.45` publication. Do not add it merely because a template does; the sessions milestone does not need its update/licensing functionality.

### Reproduction sources

- [Pumpkin release](https://github.com/Pumpkin-MC/Pumpkin/releases/tag/0.2.1%2B26.3-26.51)
- [Pumpkin pinned workspace](https://github.com/Pumpkin-MC/Pumpkin/blob/224f9f8144ca53a8d0800acb87607d714196e2a6/Cargo.toml)
- [Selected SDK binding generation](https://github.com/Pumpkin-MC/Pumpkin/blob/224f9f8144ca53a8d0800acb87607d714196e2a6/crates/pumpkin-plugin-api/src/lib.rs#L203-L218)
- [Published SDK version metadata](https://crates.io/api/v1/crates/pumpkin-plugin-api/0.2.1%2B26.3-26.51)
- [Vivecraft release](https://github.com/Vivecraft/VivecraftMod/releases/tag/26.3-1.3.17)
- [Client requirements](https://github.com/Vivecraft/VivecraftMod/blob/7e610caf672c67e4501ba8675ba0d620ffeb17ee/gradle.properties)
- [Pumpkin Java protocol mapping](https://github.com/Pumpkin-MC/Pumpkin/blob/224f9f8144ca53a8d0800acb87607d714196e2a6/crates/pumpkin-util/src/version.rs#L143-L146)

## Protocol compatibility finding

Do not copy the local `NetworkVersion` enum's ordinals into Rust.

| Protocol feature | Existing Java reference | Selected Vivecraft client |
| --- | --- | --- |
| New networking | `0` | `0` |
| Aim override | `8` | `8` |
| Climbey jump version gate | No separate `CLIMBEY_JUMP` entry | `9` |
| Roomscale attack | `9` | `10` |

Both inspected payload-identifier lists still put `JUMPING` at byte ID `27` and `ROOMSCALE_ATTACK` at byte ID `28`. **Payload ID and negotiated network version are separate namespaces.** The selected client advertises modern network versions `0..10`; the local Java reference advertises `0..9` plus legacy handling. Client fixtures and explicit wire values define the new codec. Historical behavior remains retained/tested without claiming live old-Minecraft admission; additional historical fixture breadth remains tracked acceptance work.

Evidence:

- Local [NetworkVersion](../../core/src/main/java/org/vivecraft/network/NetworkVersion.java)
- Local [PayloadIdentifier](../../core/src/main/java/org/vivecraft/network/packet/PayloadIdentifier.java)
- [Pinned client NetworkVersion](https://github.com/Vivecraft/VivecraftMod/blob/7e610caf672c67e4501ba8675ba0d620ffeb17ee/common/src/main/java/org/vivecraft/common/network/NetworkVersion.java)
- [Pinned client payload identifiers](https://github.com/Vivecraft/VivecraftMod/blob/7e610caf672c67e4501ba8675ba0d620ffeb17ee/common/src/main/java/org/vivecraft/common/network/packet/PayloadIdentifier.java)
- [Pinned client version payload](https://github.com/Vivecraft/VivecraftMod/blob/7e610caf672c67e4501ba8675ba0d620ffeb17ee/common/src/main/java/org/vivecraft/common/network/packet/c2s/VersionPayloadC2S.java)

## Capability / parity matrix

The table preserves the original **discovery/acceptance plan**, not a claim that every row was retested. The maintainer's 2026-10-30 in-game report supersedes the earlier blanket “no real-client testing” status for the sessions build, but does not provide a per-row scenario matrix. Host loading and console/WASI behavior have the separate recorded smoke evidence. A missing method in the inspected surface is an investigation blocker, not proof that all plugin-only approaches are impossible. No row authorizes dropping a feature.

| Behavior | Java reference | Selected Pumpkin evidence | Assessment / acceptance gate |
| --- | --- | --- | --- |
| Plugin load/unload | `ViveMain` | SDK `Plugin` lifecycle, WIT v0.1 host | Source-backed. Load real WASM, unload/reload without stale state. |
| Receive `vivecraft:data` | `NetworkHandler` | `JavaClient::handle_play_packet`, `net/java/mod.rs:1075-1083`, decodes `SCustomPayload` and fires blocking `PlayerCustomPayloadEvent`; v0.1 host event conversion exists | Source-backed end-to-end path. Test channel filtering, registration, handshake receipt, ordering. |
| Send custom payload | `NetworkHandler.sendPacket` | v0.1 `player.wit` Java-player `send-custom-payload`; v0.1 host `player.rs:3500` sends `CCustomPayload` | Source-backed. Confirm client accepts handshake and reply bytes. |
| Handshake / negotiation | `NetworkHandler.handleVersion` | Transport primitives exist; version policy belongs in sessions | Pure logic plus live test. Resolve protocol version difference above. Do not claim unsupported versions or features. |
| Pose, scales, active body part, full-body tracking | `VivePlayer`, `VrPlayerState`, packet classes | No host-specific math needed for codec/state | Golden client bytes and module-interface tests; confirm head/hands/FBT in real clients. |
| Pose relay and visibility | `ViveMain.tick`, `sendVrPlayerStateToClients` | Tick events, player/world access, custom sender | Partial. World membership is not entity tracking. Verify visibility/range/dimension rules and no pose leaks to unintended recipients. |
| VR switching / leave / respawn | `NetworkHandler.handleIsActive`, `PlayerEvents` | Player lifecycle and tick event interfaces | Source-backed event surface; verify session cleanup, inactive notifications, respawn/world transitions, unload. |
| Teleport + food exhaustion | `handleTeleport`, server limits config | Player teleport/exhaustion functions and host implementations | Partial. Verify server authority, collision/world constraints, configured limits and client correction; do not blindly trust client coordinates. |
| Climbing / jump fall handling | `handleClimbing`, `handleJumping`, `NMSHelper.resetFallDistance` | Entity `set-fall-distance` in `world.wit`; v0.1 host writes living fall distance; player exhaustion methods | Partial. Java resets an airborne counter too; equivalent movement/floating behavior needs investigation and live tests. |
| Crawl pose and hitbox | `handleCrawl`, NMS pose wrapper | Entity pose read/change-event surface; no direct `set-pose` found in inspected v0.1 world interface | Potential hook gap. A swim flag or metadata packet is not proof of authoritative crawl dimensions/physics. |
| Controller-based block/item aim | `AimFixHandler`, `NMSHelper` | Blocking `PacketReceivedEvent` before Java decode; host applies packet ID/raw bytes/cancellation | Partial. No proven scoped execution override/restore equivalent. Broad teleport-and-rotate is not an accepted substitute. |
| Projectiles / roomscale bow | `ProjectileEvents` | Projectile launch/hit interfaces; entity position/velocity/rotation primitives | Partial. Verify event fire sites, timing and mutation against projectile construction, origin, draw/inaccuracy and damage semantics. |
| PvP matrix / headshots / boot damage / shields | `DamageEvents`, projectile damage logic | Damage-by-entity interface exposes damage/cancellation; host entity/world primitives | Partial. Check actual event dispatch and propagation, shield geometry/cooldowns, headshot hit tests, damage direction and haptics. |
| Dual wield / roomscale block breaking | `VivePlayer`, network handlers, game-mode helpers | Inventory and packet/event surfaces exist | Not established. Requires atomic hand/item handling, correct durability and server break progress; no parity promise yet. |
| Mob VR awareness | `EntityEvents`, creeper/enderman helpers, version-specific entity changes | Mob target/navigation/data interfaces exist | Not established. Verify custom head/eye targeting, creeper distance and melee reach in actual AI execution. |
| Crafting / Climbey items | `RecipeManager` | Recipe/item/inventory interfaces | Interface-level only in this pass. Verify custom-item identity and recipe registration/removal. |
| Config, commands, permissions, messages | `Config`, commands, `PermissionManager`, translations | SDK filesystem permissions, command registration and permission interfaces | Migration design needed. Preserve semantics; decide file format/import and compatibility names. |
| Debug particles / diagnostics / update checks | `Debug`, `UpdateChecker` | Particle/log facilities; optional SDK utilities not required for first milestone | Inventory and tests still needed. No unrelated outbound update checks by default during development. |
| Public VR data access | `api/VRAPI`, pose-history interfaces | Pumpkin IPC exists as a candidate host interface | Product decision needed. Preserve useful queries/events via a Pumpkin-native contract; Java binary compatibility is not available inside a standalone WASM artifact. |
| Vault / PlaceholderAPI | `PermissionManager`, `VivecraftPAPIExpansion` | Bukkit JVM integrations are not directly a WASM interface | Product decision needed. Define equivalent operator/integration behavior; do not silently pretend the integrations work. |
| Spigot/Paper/Folia + generated NMS compatibility | `compat`, `versions`, `gen`, platform subprojects | Different host/runtime | Keep Java source as reference. Do not recreate obsolete host machinery merely to preserve implementation shape. Agree historical Minecraft support separately from behavior parity. |

### Pinned Pumpkin implementation references

- [Java packet and custom-payload dispatch](https://github.com/Pumpkin-MC/Pumpkin/blob/224f9f8144ca53a8d0800acb87607d714196e2a6/crates/pumpkin-core/src/net/java/mod.rs#L744-L764)
- [Custom payload event fire site](https://github.com/Pumpkin-MC/Pumpkin/blob/224f9f8144ca53a8d0800acb87607d714196e2a6/crates/pumpkin-core/src/net/java/mod.rs#L1075-L1083)
- [v0.1 custom payload event conversion](https://github.com/Pumpkin-MC/Pumpkin/blob/224f9f8144ca53a8d0800acb87607d714196e2a6/crates/pumpkin-wasm-host-v0_1/src/events/player.rs#L623-L645)
- [v0.1 payload sender host](https://github.com/Pumpkin-MC/Pumpkin/blob/224f9f8144ca53a8d0800acb87607d714196e2a6/crates/pumpkin-wasm-host-v0_1/src/player.rs#L3500)
- [v0.1 packet-event mutation](https://github.com/Pumpkin-MC/Pumpkin/blob/224f9f8144ca53a8d0800acb87607d714196e2a6/crates/pumpkin-wasm-host-v0_1/src/events/server.rs#L57-L63)
- [v0.1 entity/mob methods, in world.wit](https://github.com/Pumpkin-MC/Pumpkin/blob/224f9f8144ca53a8d0800acb87607d714196e2a6/crates/pumpkin-plugin-wit/v0.1/world.wit#L405)
- [v0.1 fall-distance host implementation](https://github.com/Pumpkin-MC/Pumpkin/blob/224f9f8144ca53a8d0800acb87607d714196e2a6/crates/pumpkin-wasm-host-v0_1/src/entity.rs#L334-L342)

## First milestone acceptance outline

This outline is not a final approved implementation plan.

1. Keep host-independent sessions code buildable/testable natively; compile only the Pumpkin adapter to WASM. Do not set a workspace-wide default target that accidentally prevents native tests.
2. Exercise the sessions interface with golden bytes from the pinned client: VERSION, negotiation, VR state, scales, active body part, malformed/truncated data, unknown identifiers, and version-dependent features. Cover explicit wire constants rather than enum ordinal inference.
3. Confirm finite/bounded numeric data, bounded payload sizes, no panics on client input, and rejection of state updates before negotiation.
4. Load the component on the pinned server. Test a real VR client plus a second Vivecraft client, and a vanilla client as a non-regression check.
5. Confirm pose relay, VR on/off transitions, recipient selection, disconnect, reconnect, respawn/world change, plugin unload and reload.
6. Never advertise teleport, crawl, climbing, dual wielding or roomscale capabilities just because a packet can be decoded; advertise only implemented capability policy.
7. Keep every unfinished parity row visible. The first milestone is not the full rewrite.

## Accepted compatibility decisions

- Retain historical Vivecraft wire behavior with fixtures; initially validate Minecraft Java 26.3 only. Live preview admission is narrower than codec coverage because unsupported capabilities must be explicitly disabled.
- TOML with explicit non-destructive YAML import; preserve command aliases and permission names. Unimplemented semantics are retained as inactive, not claimed to work.
- Pumpkin-native VR data/integration equivalents without Java binary compatibility. The preview sessions query is not full integration parity.

The remaining decisions concern proven hook gaps and later-milestone interfaces, not a re-opening of these accepted defaults.

The authorizing decision and constraints are recorded in [ADR 0001](../adr/0001-pumpkin-migration-direction.md). Environment constraints and continuation notes belong in [HANDOFF.md](HANDOFF.md).
