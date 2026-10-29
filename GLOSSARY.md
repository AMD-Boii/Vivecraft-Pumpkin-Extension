# Migration glossary

These terms describe the migration direction and the first Rust sessions-preview interfaces. Native tests and compilation do not establish live host/client acceptance.

- **Inactive setting** — A recognized legacy value retained for migration but not applied by the preview. It must not be reported as an enabled feature.
- **Connection generation** — The adapter-assigned lifetime token carried in snapshots/effects to reject stale work for a reused player UUID.
- **Fixture coverage** — Byte-level compatibility evidence from documented synthetic or historical reference vectors, distinct from running an actual client.

- **Vivecraft sessions** — The module responsible for negotiated Vivecraft wire compatibility and per-player VR session state, independent of Pumpkin handles. It is the first architectural focus.
- **Pumpkin adapter** — The module translating host events and player snapshots into sessions input, and applying outputs through verified Pumpkin methods. Host execution timing and resource lifetimes stay here.
- **Wire protocol version** — Vivecraft's negotiated feature version. It is distinct from the Minecraft protocol number, the plugin ABI version, and the packet payload identifier.
- **Payload identifier** — The byte identifying a Vivecraft message on `vivecraft:data`. Its numeric value is independent of the negotiated wire protocol version.
- **Capability policy** — The set of VR behaviors the server actually implements and enables, used to decide what may be advertised to the client. Decoding a message alone does not establish a capability.
- **Parity milestone** — A verified slice of the intended full rewrite. Completing one milestone does not imply completion or removal of outstanding behavior.

- **Stable sessions feature set** — The implemented scope reported working in game by the maintainer without observed issues; not a statement that missing Java gameplay has been ported.
- **Build attestation** — A GitHub OIDC/Sigstore-signed claim connecting artifact hashes to a repository, commit and workflow. It does not alter the WASM or establish safety/legal approval.
- **Market signature** — Pumpkin-specific embedded Ed25519 metadata/signature sections on a distributed plugin, separate from GitHub attestations, checksums or Git tags.
- **Corresponding-source archive** — The release's project/build source, exact locked dependency source and notices, with an offline Cargo source configuration. Generated outside Git so recipients can modify/rebuild the binary.

See [ADR 0001](docs/adr/0001-pumpkin-migration-direction.md), the [target/capability matrix](docs/migration/target-and-capabilities.md), and [release guide](docs/releasing.md).
