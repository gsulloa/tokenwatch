## Context

The autoupdater is fully wired except for the signing key pair. `release.yml` already reads `TAURI_UPDATER_PRIVATE_KEY` / `TAURI_UPDATER_KEY_PASSWORD` (mapped to Tauri's `TAURI_SIGNING_PRIVATE_KEY[_PASSWORD]`), and `tauri.conf.json` has a `pubkey` placeholder. The repo already establishes a secret-provisioning pattern in `packages/infra/scripts/set-feedback-app-key.sh` (bash, `set -euo pipefail`, `--rotate`, env overrides, `gh secret set`, prints only confirmations). This change mirrors that pattern for the updater keys.

Key generation is `tauri signer generate`, which prompts interactively for a password and writes `<key>` (private) and `<key>.pub` (public). The password is needed both to generate and later to upload as a secret, so the script must own the password rather than letting the CLI prompt silently.

## Goals / Non-Goals

**Goals:**
- One idempotent script that generates/rotates the key pair and wires pubkey + secrets end-to-end.
- Real pubkey committed in `tauri.conf.json`; private key + password only in local key file and GitHub secrets.
- Short, discoverable release-signing docs.

**Non-Goals:**
- Changing `release.yml` (secrets already consumed) or the hosting/manifest pipeline.
- Rotating the Apple codesigning cert or feedback app-key.
- Automating disaster recovery / key escrow beyond documenting the local key file location.

## Decisions

**1. Script owns the password (env var, not CLI prompt).** `tauri signer generate` prompts for a password; that value must also be uploaded as `TAURI_UPDATER_KEY_PASSWORD`. The script reads the password from `UPDATER_KEY_PASSWORD` if set, otherwise prompts once with `read -s` and passes it to the CLI via `-p`/stdin so the same value flows to both the key and the secret. Rationale: avoids a mismatch between the password baked into the key and the one stored in CI; alternative (let CLI prompt, ask user to re-enter for the secret) is error-prone.

**2. Edit `tauri.conf.json` with node, not `jq`.** `jq` may not be installed; node is guaranteed (pnpm/Node 22 is a project baseline). Use a tiny inline node script that reads, sets `plugins.updater.pubkey`, and writes back with 2-space indent + trailing newline to preserve formatting. Alternative (`jq`) adds a dependency and reformats.

**3. Reuse-by-default, `--rotate` to force.** Same semantics as `set-feedback-app-key.sh`: if the key file exists and `--rotate` is not passed, reuse it (only re-sync pubkey + secrets); with `--rotate` (or no existing file) generate fresh with `--force`. Rationale: safe to re-run; rotation is explicit.

**4. Key file location `~/.tauri/tokenwatch-updater.key`, overridable via `UPDATER_KEY_PATH`.** Outside the repo tree so it can never be accidentally committed. `REPO` overridable via `UPDATER_REPO` (default `gsulloa/tokenwatch`).

**5. Docs in new `docs/RELEASE_SETUP.md`.** No `docs/` dir exists yet; create it. Link from `CONTRIBUTING.md`. Rationale: keeps `CONTRIBUTING.md` lean while giving release setup a dedicated home referenced by the acceptance criteria.

## Risks / Trade-offs

- **Lost private key file** → the pubkey in released binaries can only be matched by that private key; losing it forces a rotation that breaks updates for already-installed clients (they can't verify the new key). Mitigation: doc explicitly states the key file is the source of truth and should be backed up securely; rotation implications are documented.
- **Password mismatch between key and secret** → signing fails in CI. Mitigation: script sources one password for both (decision 1).
- **Running the script requires `gh` auth + repo admin** → `gh secret set` fails otherwise. Mitigation: `set -euo pipefail` surfaces the failure; doc lists prerequisites.
- **`tauri signer generate` password non-interactive flag differences across CLI versions** → mitigate by pinning behavior to the installed `@tauri-apps/cli@^2.2.0` and verifying `-p`/`--password` and `--ci` support during implementation; fall back to documented manual password entry if unsupported.

## Migration Plan

1. Run `packages/app/scripts/set-updater-keys.sh` locally (generates key, writes pubkey, sets secrets).
2. Commit the updated `tauri.conf.json`, the new script, and docs.
3. Verify `gh secret list` shows both secrets and a test release build signs/verifies.
4. Rollback: revert the `tauri.conf.json` pubkey commit; secrets can be re-set or deleted via `gh secret`. No client impact until a signed release ships.

## Open Questions

- Does the installed `@tauri-apps/cli` version support a non-interactive password flag (`-p` / `--password` / `--ci`)? Resolve during implementation; if not, the script prompts once with `read -s` and documents it.
