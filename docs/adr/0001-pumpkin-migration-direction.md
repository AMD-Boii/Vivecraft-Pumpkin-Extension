# ADR 0001: Pumpkin migration direction

- Date: 2026-10-27
- Status: Accepted direction and first-milestone implementation plan; implementation underway

## Context

This repository contains a Spigot-family Vivecraft server extension. The user wants a Pumpkin plugin rewrite, not merely a demonstration of plugin loading or pose relay. Pumpkin is a different host and plugin runtime, so copying Bukkit/NMS abstractions would preserve implementation dependencies rather than behavior.

## Decision

The user explicitly approved:

1. Select and pin an initially compatible Pumpkin/Minecraft/Vivecraft target pair.
2. Prefer Rust/WebAssembly.
3. Pursue full parity in testable milestones.
4. Require explicit approval before changing Pumpkin itself.
5. Start architectural work with the Vivecraft sessions module.
6. Retain historical Vivecraft wire behavior with fixture coverage while initially validating Minecraft Java 26.3 only.
7. Use TOML with explicit non-destructive YAML import and preserve command/permission names.
8. Use Pumpkin-native integration equivalents, without Java binary compatibility.

The detailed sessions-milestone implementation plan was subsequently approved. Preview admission requires wire versions 7–10 so unsupported teleport can be explicitly disabled; older formats remain codec/fixture coverage rather than a live compatibility claim.

The selected release/source pins and their initial discovery evidence are maintained in [target-and-capabilities.md](../migration/target-and-capabilities.md), not duplicated here. Later host checks and the maintainer's 2026-10-30 in-game success report are recorded in [testing.md](../migration/testing.md); neither makes unimplemented parity complete.

The first implementation slice will establish protocol negotiation and VR session/pose exchange through a Pumpkin adapter. It does not fulfill or remove the remaining gameplay and operator parity requirements.

## Consequences

- Concentrate wire compatibility and session state in a host-independent module tested through its consumer interface.
- Keep Pumpkin resource handles and execution hooks in the adapter rather than exposing them throughout session logic.
- Preserve Java source as reference until replacement behavior is verified; this decision does not authorize deleting it.
- Missing hooks must be reported and investigated. No server fork or silent feature removal is authorized.
- Client capability advertisement must reflect implemented behavior, not the existence of packet types.
- Historical wire codecs, TOML/import behavior, and native integration direction are settled; full parity and real-server/client acceptance remain required. Preview-only queries do not complete native integration parity.
- Rust/WASM is a preference, not permission to claim proven feasibility of all NMS-dependent behavior.

## Alternatives not selected

- A Java compatibility-layer deployment instead of a rewrite: not the requested direction.
- A native server-internal plugin coupled to moving Pumpkin master: not the chosen initial packaging/target approach.
- Treating pose relay as the completed port: conflicts with the parity goal.
