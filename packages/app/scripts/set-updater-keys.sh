#!/usr/bin/env bash
#
# set-updater-keys.sh — provision (or rotate) the Tauri updater signing key pair.
#
# Verified: `tauri signer generate` supports -p/--password, -w/--write-keys,
# -f/--force, and --ci (non-interactive) flags in @tauri-apps/cli ^2.2.0.
#
# The key pair is the source of truth for Tauri's autoupdater:
#   - Public key  → committed into packages/app/src-tauri/tauri.conf.json
#                   (plugins.updater.pubkey); verified by installed clients.
#   - Private key → local file at $KEY_PATH (default ~/.tauri/tokenwatch-updater.key)
#                   and GitHub secret TAURI_UPDATER_PRIVATE_KEY; used to sign
#                   release bundles in CI.
#   - Password    → GitHub secret TAURI_UPDATER_KEY_PASSWORD; used by CI to
#                   decrypt the private key at signing time.
#
# Usage:
#   ./set-updater-keys.sh            # create if absent, else reuse + resync
#   ./set-updater-keys.sh --rotate   # force-generate a new key pair
#
# Env overrides:
#   UPDATER_REPO         — GitHub repo slug (default: gsulloa/tokenwatch)
#   UPDATER_KEY_PATH     — local key file path (default: ~/.tauri/tokenwatch-updater.key)
#   UPDATER_KEY_PASSWORD — key password; if unset the script prompts once interactively
#
# Prerequisites: gh authenticated with repo admin on $REPO; pnpm install done.
set -euo pipefail

REPO="${UPDATER_REPO:-gsulloa/tokenwatch}"
KEY_PATH="${UPDATER_KEY_PATH:-$HOME/.tauri/tokenwatch-updater.key}"

# Resolve repo root robustly (script may be called from any cwd).
REPO_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
CONF="$REPO_ROOT/packages/app/src-tauri/tauri.conf.json"

rotate=false
[ "${1:-}" = "--rotate" ] && rotate=true

# ---------------------------------------------------------------------------
# 1. Resolve password (needed both for key generation and for the CI secret).
# ---------------------------------------------------------------------------
if [ -n "${UPDATER_KEY_PASSWORD:-}" ]; then
  PASSWORD="$UPDATER_KEY_PASSWORD"
else
  # On reuse, the original password cannot be recovered from the key file — require it.
  if ! $rotate && [ -f "$KEY_PATH" ]; then
    echo "Error: key file already exists at $KEY_PATH and --rotate was not passed." >&2
    echo "  On reuse the password cannot be recovered from the key file." >&2
    echo "  Supply it via UPDATER_KEY_PASSWORD=... or pass --rotate to generate a fresh pair." >&2
    exit 1
  fi
  printf 'Enter key password: '
  read -rs PASSWORD
  echo ""
fi

# ---------------------------------------------------------------------------
# 2. Key resolution: reuse or generate.
# ---------------------------------------------------------------------------
if $rotate || [ ! -f "$KEY_PATH" ]; then
  mkdir -p "$(dirname "$KEY_PATH")"
  pnpm --filter tokenwatch exec tauri signer generate \
    -w "$KEY_PATH" \
    -p "$PASSWORD" \
    --ci \
    --force
  echo "→ Generated new key pair at $KEY_PATH"
else
  echo "→ Reusing existing key at $KEY_PATH"
fi

# ---------------------------------------------------------------------------
# 3. Write public key into tauri.conf.json (inline node; preserves 2-space indent).
# ---------------------------------------------------------------------------
PUBKEY="$(cat "${KEY_PATH}.pub")"
PUBKEY="$PUBKEY" node -e '
  const fs = require("fs");
  const p = process.argv[1];
  const c = JSON.parse(fs.readFileSync(p, "utf8"));
  c.plugins.updater.pubkey = process.env.PUBKEY;
  fs.writeFileSync(p, JSON.stringify(c, null, 2) + "\n");
' "$CONF"
echo "✓ Wrote pubkey into $CONF (plugins.updater.pubkey)"

# ---------------------------------------------------------------------------
# 4. Upload GitHub secrets.
# ---------------------------------------------------------------------------
gh secret set TAURI_UPDATER_PRIVATE_KEY --repo "$REPO" < "$KEY_PATH"
echo "✓ Set GitHub secret TAURI_UPDATER_PRIVATE_KEY on $REPO"

printf '%s' "$PASSWORD" | gh secret set TAURI_UPDATER_KEY_PASSWORD --repo "$REPO"
echo "✓ Set GitHub secret TAURI_UPDATER_KEY_PASSWORD on $REPO"

echo ""
echo "Done. The private key lives only in $KEY_PATH and in GitHub secrets — it is"
echo "never committed. Back it up securely; losing it requires rotation (which"
echo "breaks update verification for already-installed clients)."
