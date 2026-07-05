# release-signing Specification

## Purpose
Defines the requirements for provisioning and managing the Tauri updater signing key pair, ensuring release builds are signed with a real key, that signing secrets are present in CI, and that contributors have documentation to provision and rotate the keys.

## Requirements

### Requirement: Updater key-pair provisioning script

The project SHALL provide `packages/app/scripts/set-updater-keys.sh` that generates (or rotates) the Tauri updater signing key pair and wires it end-to-end, following the pattern of `packages/infra/scripts/set-feedback-app-key.sh`.

#### Scenario: First-time provisioning

- **WHEN** the script runs and no key file exists at the configured path (default `~/.tauri/tokenwatch-updater.key`)
- **THEN** it generates a new key pair with `tauri signer generate`, writes the public key into `plugins.updater.pubkey` in `packages/app/src-tauri/tauri.conf.json`, and uploads the private key and its password as the GitHub secrets `TAURI_UPDATER_PRIVATE_KEY` and `TAURI_UPDATER_KEY_PASSWORD`

#### Scenario: Reuse existing key without rotation

- **WHEN** the script runs without `--rotate` and a key file already exists at the configured path
- **THEN** it reuses the existing key pair (does not regenerate) and re-syncs the public key into `tauri.conf.json` and the secrets into GitHub

#### Scenario: Forced rotation

- **WHEN** the script runs with the `--rotate` flag
- **THEN** it generates a fresh key pair (overwriting the existing key file), updates `tauri.conf.json` with the new public key, and overwrites both GitHub secrets

#### Scenario: Private material never committed

- **WHEN** the script completes
- **THEN** the private key and password exist only in the local key file and in GitHub secrets, are never written into any tracked repo file, and the script prints only confirmations (no secret values)

### Requirement: Real updater public key in config

`packages/app/src-tauri/tauri.conf.json` SHALL contain a valid Tauri updater public key at `plugins.updater.pubkey`, with no placeholder value.

#### Scenario: No placeholder remains

- **WHEN** `plugins.updater.pubkey` is read from `tauri.conf.json`
- **THEN** its value is a real base64 minisign public key and is not `REPLACE_WITH_TAURI_UPDATER_PUBKEY`

#### Scenario: Signed build verifies against the config key

- **WHEN** a release build is signed in CI with the private key backing `TAURI_UPDATER_PRIVATE_KEY`
- **THEN** the produced update signature verifies against the `pubkey` in `tauri.conf.json`

### Requirement: Release signing secrets present in CI

The `gsulloa/tokenwatch` GitHub repository SHALL define the secrets `TAURI_UPDATER_PRIVATE_KEY` and `TAURI_UPDATER_KEY_PASSWORD` consumed by `.github/workflows/release.yml`.

#### Scenario: Secrets exist

- **WHEN** `gh secret list --repo gsulloa/tokenwatch` is run
- **THEN** both `TAURI_UPDATER_PRIVATE_KEY` and `TAURI_UPDATER_KEY_PASSWORD` are listed

### Requirement: Release signing documentation

The project SHALL document the release-signing setup — where the private key lives, how to provision it, and how to rotate it — in `docs/RELEASE_SETUP.md`, referenced from `CONTRIBUTING.md`.

#### Scenario: Setup doc discoverable

- **WHEN** a contributor reads `CONTRIBUTING.md`
- **THEN** it links to `docs/RELEASE_SETUP.md`, which explains the key file location, how to run `set-updater-keys.sh`, and the rotation procedure
