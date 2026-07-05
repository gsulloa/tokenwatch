## Why

The Tauri autoupdater is wired end-to-end (plugin, `tauri.conf.json`, `release.yml`, hosting infra) but the loop never closes: `tauri.conf.json` still ships the `REPLACE_WITH_TAURI_UPDATER_PUBKEY` placeholder and the GitHub secrets `TAURI_UPDATER_PRIVATE_KEY` / `TAURI_UPDATER_KEY_PASSWORD` don't exist. Without a real signing key pair, release builds can't be verified and users never receive updates.

## What Changes

- Add a `packages/app/scripts/set-updater-keys.sh` provisioning script (mirroring `packages/infra/scripts/set-feedback-app-key.sh`) that generates/rotates the Tauri updater key pair, writes the public key into `tauri.conf.json`, and uploads the private key + password as GitHub Actions secrets.
- Replace the `pubkey` placeholder in `packages/app/src-tauri/tauri.conf.json` with the real generated public key (committed).
- Provision the `TAURI_UPDATER_PRIVATE_KEY` and `TAURI_UPDATER_KEY_PASSWORD` secrets in `gsulloa/tokenwatch` (not committed).
- Add release-signing setup documentation (where the private key lives, how to rotate) in a new `docs/RELEASE_SETUP.md`, referenced from `CONTRIBUTING.md`.

## Capabilities

### New Capabilities
- `release-signing`: Management of the Tauri updater signing key pair — how the key pair is generated/rotated, where the public key is stored (committed config), and where the private material is stored (local key file + GitHub secrets, never committed).

### Modified Capabilities
<!-- None: no existing spec's requirements change. -->

## Impact

- **Code/config**: `packages/app/src-tauri/tauri.conf.json` (pubkey), new `packages/app/scripts/set-updater-keys.sh`, new `docs/RELEASE_SETUP.md`, `CONTRIBUTING.md` (reference).
- **Secrets/infra**: GitHub Actions secrets `TAURI_UPDATER_PRIVATE_KEY`, `TAURI_UPDATER_KEY_PASSWORD` in `gsulloa/tokenwatch`; local key file at `~/.tauri/tokenwatch-updater.key`.
- **CI**: `.github/workflows/release.yml` (consumes the secrets — already wired, no change needed).
- **Dependencies**: `@tauri-apps/cli` (`tauri signer generate`), `gh` CLI, `jq`/node for JSON edit.
