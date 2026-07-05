## 1. Provisioning script

- [x] 1.1 Verify `@tauri-apps/cli` supports a non-interactive password flag for `tauri signer generate` (`-p`/`--password`/`--ci`); note the resolved approach at the top of the script.
- [x] 1.2 Create `packages/app/scripts/set-updater-keys.sh` mirroring `packages/infra/scripts/set-feedback-app-key.sh`: `set -euo pipefail`, header comment (purpose, usage, env overrides), `UPDATER_REPO`/`UPDATER_KEY_PATH`/`UPDATER_KEY_PASSWORD` overrides, `--rotate` flag.
- [x] 1.3 Implement key resolution: reuse existing key file unless `--rotate` or file absent; otherwise run `tauri signer generate -w "$KEY_PATH" --force` sourcing one password for both key and secret.
- [x] 1.4 Implement pubkey write into `plugins.updater.pubkey` in `packages/app/src-tauri/tauri.conf.json` via inline node (preserve 2-space indent + trailing newline).
- [x] 1.5 Upload secrets: `gh secret set TAURI_UPDATER_PRIVATE_KEY --repo "$REPO" < "$KEY_PATH"` and `printf '%s' "$PWD" | gh secret set TAURI_UPDATER_KEY_PASSWORD --repo "$REPO"`; print only confirmations, never secret values.
- [x] 1.6 `chmod +x` the script.

## 2. Provision keys

- [x] 2.1 Run `packages/app/scripts/set-updater-keys.sh` locally to generate the key pair and wire everything.
- [x] 2.2 Confirm `packages/app/src-tauri/tauri.conf.json` `pubkey` is a real key (no `REPLACE_WITH_TAURI_UPDATER_PUBKEY`).
- [x] 2.3 Confirm `gh secret list --repo gsulloa/tokenwatch` lists `TAURI_UPDATER_PRIVATE_KEY` and `TAURI_UPDATER_KEY_PASSWORD`.

## 3. Documentation

- [x] 3.1 Create `docs/RELEASE_SETUP.md` covering: key file location (`~/.tauri/tokenwatch-updater.key`), prerequisites (`gh` auth, repo admin), how to run the script, and the rotation procedure + its impact on installed clients.
- [x] 3.2 Add a reference/link to `docs/RELEASE_SETUP.md` from `CONTRIBUTING.md`.

## 4. Verification

- [x] 4.1 Confirm no private key or password is written to any tracked file (`git status` / `git diff` show only the pubkey, script, and docs).
- [x] 4.2 Run project checks (`pnpm typecheck && pnpm lint`) and confirm the config change is valid JSON.
