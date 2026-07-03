//! Single source of truth for the application's name and brand identifiers.

/// Human-facing application name.
pub const APP_DISPLAY_NAME: &str = "TokenWatch";

/// Tauri bundle identifier. MUST match `bundle.identifier` in tauri.conf.json.
pub const BUNDLE_IDENTIFIER: &str = "com.tokenwatch.app";

/// OS keychain service name under which secrets are stored.
pub const KEYCHAIN_SERVICE: &str = "tokenwatch";

/// SQLite database filename inside the app data dir (if/when used).
pub const DB_FILENAME: &str = "tokenwatch.db";

/// Log file stem inside the app log dir.
pub const LOG_FILE_STEM: &str = "tokenwatch.log";

/// Cargo binary name (see [package]/[lib] name in Cargo.toml).
pub const CARGO_BIN_NAME: &str = "tokenwatch";

/// Prefix for power-user environment-variable overrides (e.g. TOKENWATCH_*).
pub const ENV_VAR_PREFIX: &str = "TOKENWATCH";
