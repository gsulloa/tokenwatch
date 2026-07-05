# Release Setup

This document covers the Tauri updater signing key pair — what it is, where it lives, how to provision it, and how to rotate it.

## Overview

TokenWatch uses [Tauri's built-in autoupdater](https://tauri.app/plugin/updater/), which requires every release bundle to be signed with a **minisign** key pair:

- **Public key** — committed into `packages/app/src-tauri/tauri.conf.json` (`plugins.updater.pubkey`). Installed clients use this to verify that updates come from a trusted source.
- **Private key** — stored locally at `~/.tauri/tokenwatch-updater.key` and in the GitHub secret `TAURI_UPDATER_PRIVATE_KEY`. CI uses it to sign release bundles.
- **Password** — stored in the GitHub secret `TAURI_UPDATER_KEY_PASSWORD`. CI uses it to decrypt the private key at signing time.

The private key and password are **never committed** to the repository. The public key is committed because installed clients need it to verify updates.

## Key file location

| File | Description |
|------|-------------|
| `~/.tauri/tokenwatch-updater.key` | Private key — the source of truth. **Back this up securely.** |
| `~/.tauri/tokenwatch-updater.key.pub` | Public key — written into `tauri.conf.json` by the provisioning script. |

Losing the private key means you can no longer sign updates with the same key pair. You would need to rotate (see below), which breaks update verification for already-installed clients.

## Prerequisites

- `gh` CLI authenticated with an account that has **repo admin** access to `gsulloa/tokenwatch` (required for `gh secret set`).
- `pnpm install` has been run (the script invokes `tauri signer generate` via pnpm).

## Provisioning

Run the provisioning script from the repository root:

```bash
packages/app/scripts/set-updater-keys.sh
```

The script will:
1. Generate a new key pair at `~/.tauri/tokenwatch-updater.key` (skipped if the file already exists and `--rotate` is not passed).
2. Write the public key into `packages/app/src-tauri/tauri.conf.json` (`plugins.updater.pubkey`).
3. Upload `TAURI_UPDATER_PRIVATE_KEY` and `TAURI_UPDATER_KEY_PASSWORD` as GitHub Actions secrets on `gsulloa/tokenwatch`.

### Avoiding the interactive prompt

Supply `UPDATER_KEY_PASSWORD` to skip the interactive password prompt:

```bash
UPDATER_KEY_PASSWORD="your-password" packages/app/scripts/set-updater-keys.sh
```

### Env overrides

| Variable | Default | Description |
|----------|---------|-------------|
| `UPDATER_REPO` | `gsulloa/tokenwatch` | GitHub repo slug for `gh secret set` |
| `UPDATER_KEY_PATH` | `~/.tauri/tokenwatch-updater.key` | Local key file path |
| `UPDATER_KEY_PASSWORD` | _(prompted)_ | Key password; avoids interactive prompt when set |

After running the script, commit the updated `tauri.conf.json` (which now contains the real public key instead of the placeholder).

## Rotation

To replace the current key pair with a fresh one:

```bash
packages/app/scripts/set-updater-keys.sh --rotate
```

> **WARNING — rotation has a permanent client-side impact.**
> The public key is baked into every installed binary at the time of its release. Rotating the key replaces the public key in `tauri.conf.json`, which means **already-installed clients that were built with the old public key cannot verify updates signed with the new private key**. Those clients will fail to apply future updates and must be reinstalled manually.
>
> Only rotate when absolutely necessary (e.g., private key compromise or loss) and communicate the impact to users.

After rotation:
1. Commit the updated `tauri.conf.json` with the new public key.
2. The next release will be signed with the new private key.
3. Users running binaries built before the rotation must reinstall the app.

## Verification

After provisioning or rotation, confirm:

```bash
# Both secrets should appear:
gh secret list --repo gsulloa/tokenwatch

# plugins.updater.pubkey should be a real base64 minisign key (not the placeholder):
node -e 'const c=require("./packages/app/src-tauri/tauri.conf.json"); console.log(c.plugins.updater.pubkey);'
```
