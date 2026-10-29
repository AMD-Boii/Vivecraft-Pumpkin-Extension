# GitHub Actions, releases, signing and Pumpkin Market

This guide prepares **GitHub release artifacts and a manual Market submission**. It does not
create your Market account, accept legal terms, guarantee moderation, or claim that an OIDC
attestation is Pumpkin's embedded plugin signature. The workflows never upload to Market.

## 1. Configure this GitHub repository once

Target repository: [AMD-Boii/Vivecraft-Pumpkin-Extension](https://github.com/AMD-Boii/Vivecraft-Pumpkin-Extension).
The workflows expect the reviewed branch to be `master` and the release prefix to be `pumpkin-v`.

1. Review the local change and licenses. Commit source, `Cargo.lock`, docs, scripts and workflows;
   do **not** commit `target/`, `.local/`, `dist/`, vendored dependencies, worlds or secrets.
   Run the [README verification commands](../README.md#test-and-verify) before committing.
2. In **Settings → Actions → General**, enable Actions and permit the SHA-pinned official
   `actions/checkout`, `setup-python`, `upload-artifact`, `download-artifact`,
   `attest-build-provenance` actions and their official composite dependencies. Keep the
   default `GITHUB_TOKEN` read-only; only the release job explicitly requests write/OIDC access.
   Organization policies must allow those job-level permissions.
3. In **Settings → Environments**, create **`pumpkin-release` before pushing a release tag**.
   Under deployment branches/tags, select a rule for **tags** `pumpkin-v*`. Add a required
   reviewer/prevent self-review where your plan supports it. Without configuration, GitHub
   can create an unprotected environment automatically; the YAML name alone is not a review gate.
4. Protect `master` with your normal review/check policy. Select the successful Rust verification
   check in branch protection/rulesets after its first run. Restrict creation/deletion of
   `pumpkin-v*` tags to maintainers with a tag ruleset where available. Do not reuse release tags.
5. Use a public repository for the public source/distribution workflow described here. Confirm
   GitHub artifact-attestation eligibility if using a private repository/organization policy;
   do not silently omit failed signing and label an artifact signed.

**Secrets/keys to add: none.** `github.token` is issued automatically for each run. GitHub OIDC
and Sigstore produce a short-lived signing identity. Do not store a private Ed25519, GPG,
Authenticode, or Market signing key in this repository. No Market/Modrinth/CurseForge token
is used. The inherited JVM publisher is guarded to its original upstream repository and
cannot publish this fork to the parent's project IDs.

## 2. Check the first branch build

Push the reviewed source commit to `master`, or use **Actions → Rust build and verification →
Run workflow** after that workflow has reached GitHub. PRs run read-only verification too;
never use `pull_request_target` to build untrusted source with release credentials.

[`.github/workflows/rust.yml`](../.github/workflows/rust.yml) runs:

- publication inventory/high-confidence secret-pattern and Gradle-wrapper provenance checks;
- toolchain/SDK/lockfile/independent-fixture checks and Python release-helper tests;
- Rust 1.96.0 formatting, native tests and strict native/WASI Clippy;
- a locked `wasm32-wasip2` release build;
- binary-size/header and license checks, source vendoring and packaging.

Open the completed run and download **Artifacts → `pumpkin-wasm`**. Extract it into a new,
empty directory. It contains:

| File | Purpose |
| --- | --- |
| `vivecraft-pumpkin-0.1.0.wasm` | Original component, suitable as the Market upload input |
| `vivecraft-pumpkin-0.1.0-source.tar.gz` | Project/build source, locked vendored dependencies, offline Cargo config and notices |
| `vivecraft-pumpkin-0.1.0-notices.zip` | Project licenses, dependency license inventory/texts and Rust runtime notices |
| `release.json` | Source commit, toolchain/SDK version, original binary hash/size and local/dirty status |
| `SHA256SUMS` | SHA-256 of every packaged output except the checksum file itself |

Version names change with `[workspace.package].version`. Branch/PR artifacts have **no signed
provenance bundle**, are not Market-signed, and expire after 14 days. A GitHub artifact's
transport digest is not a replacement for publisher verification or corresponding source.

## 3. Create a tagged, attested draft release

The current Pumpkin version is **0.1.0**. It is independent of `gradle.properties` and the
parent's Java version/tags. For a later version, update `[workspace.package].version`,
regenerate/inspect `Cargo.lock` with Cargo, and update version-specific docs. Then verify,
commit and push that source to `master` before tagging it. The version/pin check must pass.

After reviewing what is about to be pushed:

```bash
git push origin master
```

```bash
git tag -a pumpkin-v0.1.0 -m "Vivecraft Pumpkin 0.1.0 sessions release"
```

```bash
git push origin refs/tags/pumpkin-v0.1.0
```

These are instructions for the maintainer, not actions performed during project preparation.
The annotated Git tag is not a plugin signature. Git-signed tags are optional and require
your own separately configured signing identity.

[`.github/workflows/pumpkin-release.yml`](../.github/workflows/pumpkin-release.yml):

1. Reuses the full Rust verification/packaging workflow. It rejects a tag/version mismatch
   and a tagged commit that is not an ancestor of `origin/master`.
2. Waits for the configured `pumpkin-release` environment approval. Only this job has
   `contents: write`, `id-token: write`, `attestations: write`; PR/build jobs have read-only access.
3. Downloads outputs from **that same run**, checks their hashes, signs provenance for the
   checksummed outputs and exports **`provenance.sigstore.json`**.
4. Verifies the binary and source against that bundle and this repository's release workflow.
5. Creates a **draft GitHub release**, with all outputs attached. It never publishes it automatically.

Before publishing the draft: verify the run, inspect `release.json`, review feature limitations
and license/source assets, download and verify the artifacts, and write accurate release notes.
If needed, test the downloaded component on the pinned server. Then deliberately publish the
draft in GitHub's UI. Publishing it does not upload anything to Pumpkin Market.

Do not move or replace an existing release tag. A rerun will refuse to create a second release
with the same tag; inspect any existing draft/attestation before deciding how to recover. Use
an appropriately incremented version for changed released bytes. The pipeline does not promise
bit-for-bit reproducibility across operating systems/toolchain environments.

## Signing: what each signature means

### GitHub build provenance — generated here

GitHub OIDC/Sigstore signs an attestation binding artifact hashes to the source repository,
commit and workflow identity. The binary bytes are **not modified**. There is no private
publisher key to manage. For public repositories, provenance/identity data may be recorded
in a permanent public transparency log; never put secrets or customer information in artifacts
or signed metadata. Attestation proves provenance, **not safety, feature completeness or a legal review**.

Install a current [GitHub CLI](https://cli.github.com/) with `gh attestation` support. From a
new directory containing downloaded release files, check hashes:

```bash
sha256sum --check SHA256SUMS
```

Then verify **each subject separately** (the bundle may contain multiple subject digests):

```bash
gh attestation verify vivecraft-pumpkin-0.1.0.wasm --repo AMD-Boii/Vivecraft-Pumpkin-Extension --signer-workflow AMD-Boii/Vivecraft-Pumpkin-Extension/.github/workflows/pumpkin-release.yml --bundle provenance.sigstore.json
```

```bash
gh attestation verify vivecraft-pumpkin-0.1.0-source.tar.gz --repo AMD-Boii/Vivecraft-Pumpkin-Extension --signer-workflow AMD-Boii/Vivecraft-Pumpkin-Extension/.github/workflows/pumpkin-release.yml --bundle provenance.sigstore.json
```

Check the reported source commit/ref against your intended tag and `release.json`. A same-named
file or a checksum from an untrusted source is insufficient. `SHA256SUMS` covers the original
four package outputs; the subsequently generated provenance bundle is authenticated by its
own signature and is intentionally not inside that checksum file. GitHub's CLI may need
connectivity/trust roots even when given a downloaded bundle; these commands do not claim a
fully air-gapped verifier.

### Pumpkin's embedded Market signature — not generated here

The [pinned loader implementation](https://github.com/Pumpkin-MC/Pumpkin/blob/224f9f8144ca53a8d0800acb87607d714196e2a6/crates/pumpkin-wasm-host-common/src/signature.rs)
recognizes `pumpkin.metadata` and `wasm_signature` sections with an Ed25519 envelope. It is
not a generic `.sig` file, Windows Authenticode signature, GPG tag, or GitHub attestation.

The [official signing documentation](https://docs.pumpkinmc.org/developer/plugins/wasm-signing)
describes Market/developer signing, but does not supply a supported public publisher CLI.
Its current description is not identical to all details of the pinned loader: do not invent
trusted-key configuration or copy a self-signing workaround from another server version.
This project does **not** exploit permissive verification paths, invent Market claims, or
pretend a self-signed binary carries official approval.

The official Market frontend accepts the original `.wasm` without requiring a developer
signature; its developer terms authorize the operator to sign/distribute binaries. Follow
the live Market's signing/delivery process and any staff instructions. The backend's actual
signing/moderation policy has **not** been verified here. If it requests a publisher key,
additional metadata or a different format, obtain its official instructions before changing
the pipeline; a GitHub bundle alone cannot satisfy an embedded-signature requirement.

Keep global signature verification on when checking a Market download. Inspect the resulting
metadata/signature and loader status. Adding Market/customer metadata can legitimately change
the distributed bytes/hash: keep separate hashes for the original GitHub upload and the
Market-delivered file. GitHub's attestation covers the **original input only**, not a later
Market-rewritten copy. Do not strip a Market signature and redistribute it as still verified.

## 4. Submit to the official Pumpkin Market manually

Official site: [market.pumpkinmc.org](https://market.pumpkinmc.org/). The site itself was not
reachable from the preparation environment; the steps/limits below were checked against
[the official frontend source at `d49eca0`](https://github.com/Pumpkin-MC/Market-Frontend/tree/d49eca0fc541ee073b728a8377322c313f0b70e0).
The live UI/backend can change, so its current readiness result takes precedence.

1. Sign in with **your** account and complete developer onboarding. Read the current terms,
   Minecraft/Mojang rules and applicable open-source licenses yourself. No agreement was
   accepted and no account or listing was created as part of this preparation.
2. Enable the account security required by the live site. The source readiness checklist
   supports **2FA or a passkey**; the upload page specifically warns about 2FA. Resolve any
   backend security requirement rather than bypassing it.
3. In the developer/studio area create a plugin listing. Supply a name, category, description
   in at least one language, an original/appropriately licensed icon, supported target,
   support/repository links and pricing configuration. If paid, the frontend also requires
   a positive price and a connected payout account; do not add DRM/terms inconsistent with LGPL.
   A free open-source listing is the simplest fit for this project.
4. Describe **the stable sessions feature set**, not full Spigot parity. List disabled gameplay,
   exact Pumpkin/Minecraft/Vivecraft pins, data-folder permissions and config behavior. State
   that the plugin itself adds no telemetry, outbound updater or remote license check; the
   host/Market may have their own behavior. Do not imply official project endorsement.
5. Link the public repository, matching tagged source/notices assets, LGPL and third-party
   notices. If the Market cannot host those archives, keep the GitHub source assets publicly
   accessible alongside the binary. Never submit Mojang/Pumpkin executables, account tokens,
   worlds, logs containing player data, or material you lack distribution rights for.
6. Open the listing's **Publish Update** area and choose the appropriate release track
   (**Production (Stable)**, **Beta**, or **Alpha / Testing**). Use the same plugin version
   (`0.1.0`) and accurate release notes. Upload **only** the verified `.wasm` binary, not the
   whole GitHub artifact or signature bundle. The checked frontend validates `.wasm`, component
   headers and a **5 MiB (5,242,880 bytes)** maximum; this is also a local packaging gate.
7. Use the **publish-readiness** checklist to fix identity/category/description/icon/binary/
   pricing/account-security failures, then submit/publish only when you are ready. Follow any
   current moderation or publisher-signing instructions. No approval is implied by CI passing.
8. Download a delivered copy, preserve its hash separately, inspect signature/metadata and test
   host loading with signature verification on. Keep the public source/license links working.
   Do not announce “officially Market-signed/approved” until that actual path is verified.

Relevant official source evidence:

- [Upload fields/tracks](https://github.com/Pumpkin-MC/Market-Frontend/blob/d49eca0fc541ee073b728a8377322c313f0b70e0/src/features/studio/manage/PublishUpdate.tsx)
- [Binary/component/size checks](https://github.com/Pumpkin-MC/Market-Frontend/blob/d49eca0fc541ee073b728a8377322c313f0b70e0/src/lib/fileValidation.ts)
- [Publication readiness checks](https://github.com/Pumpkin-MC/Market-Frontend/blob/d49eca0fc541ee073b728a8377322c313f0b70e0/src/features/studio/manage/PublishReadinessModal.tsx)
- [Developer terms source](https://github.com/Pumpkin-MC/Market-Frontend/blob/d49eca0fc541ee073b728a8377322c313f0b70e0/src/features/legal/DeveloperTermsPage.tsx)

## 5. Check corresponding-source rebuildability

Each generated source archive contains the selected repository files (no `.git` or local
runtime data), **all locked registry sources** in `vendor/`, `.cargo/config.toml` pointing to
that vendor directory, and verbatim license notices. Download the matching archive, verify
provenance/checksums, inspect its paths, and extract it into its own new empty directory.
From the extracted `vivecraft-pumpkin-0.1.0-source/` directory with the pinned tools installed:

```bash
cargo test --locked --offline -p vivecraft-core
```

```bash
cargo build --locked --offline --release -p vivecraft-pumpkin --target wasm32-wasip2
```

No Git checkout, registry fetch or original signing key is required for those two rebuild
commands. rustup may still need to install the compiler/target first; vendor source is not a
bundled compiler or Windows/Linux toolchain. Git-dependent publication checks/scripts are for
the repository checkout, not a `.git`-less source archive.

For **local pre-commit packaging verification only**, after a successful build and `cargo fetch --locked`:

```bash
python -I scripts/release.py package --output .local/release-check-0.1.0 --allow-dirty
```

The output directory must not already exist. This mode includes the Git-visible working tree,
marks the manifest as local/possibly modified, and is forbidden in GitHub Actions. **Never
publish that verification package.** Normal release packaging requires a clean committed source
checkout; the reusable workflow uses that strict mode. Dependency/runtime notices are collected
from actual locked sources/toolchain, and missing license material fails the package.

## Verification boundaries

Local tooling/tests/WASM build and source-archive rebuild results are recorded in
[testing.md](migration/testing.md). Remote GitHub execution, OIDC issuance, draft creation,
account setup, Market signing/moderation and downloaded-Market loading can only be verified
after the corresponding real run. Source preparation is not a claim that any of those external
steps has already happened.

GitHub references: [artifact attestations](https://docs.github.com/en/actions/concepts/security/artifact-attestations),
[provenance setup](https://docs.github.com/en/actions/security-for-github-actions/using-artifact-attestations/using-artifact-attestations-to-establish-provenance-for-builds),
[CLI verification](https://cli.github.com/manual/gh_attestation_verify).
