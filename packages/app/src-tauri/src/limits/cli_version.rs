//! Detects the installed Claude Code CLI version to build the User-Agent sent to
//! the usage endpoint. Only directory-entry NAMES are read; no file is opened.

use std::path::Path;
use std::sync::OnceLock;

pub(crate) const FALLBACK_CLI_VERSION: &str = "2.1.328";

/// Highest valid `X.Y.Z` (all numeric) among the entry names in `dir`.
/// Entries of any type (file, dir, symlink) count; names that are not exactly
/// three dot-separated u64 components are ignored. `None` if the dir is missing,
/// unreadable or has no valid name.
pub(crate) fn detect_cli_version(dir: &Path) -> Option<String> {
    let entries = std::fs::read_dir(dir).ok()?;
    entries
        .filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name().to_str()?.to_owned();
            let ver = parse_semver(&name)?;
            Some((ver, name))
        })
        .max_by_key(|(ver, _)| *ver)
        .map(|(_, name)| name)
}

/// Parse "X.Y.Z" into (u64,u64,u64). Rejects "1.2", "1.2.3.4", "1.2.3-beta", "v1.2.3", "".
fn parse_semver(name: &str) -> Option<(u64, u64, u64)> {
    let mut parts = name.split('.');
    let mut next = || {
        let p = parts.next()?;
        if p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        p.parse::<u64>().ok()
    };
    let a = next()?;
    let b = next()?;
    let c = next()?;
    if parts.next().is_some() {
        return None;
    }
    Some((a, b, c))
}

/// `"claude-cli/<ver> (external, cli)"`
pub(crate) fn user_agent_for(version: &str) -> String {
    format!("claude-cli/{version} (external, cli)")
}

/// Cached once per process: `~/.local/share/claude/versions` → detect → fallback.
pub(crate) fn claude_cli_user_agent() -> &'static str {
    static UA: OnceLock<String> = OnceLock::new();
    UA.get_or_init(|| {
        let v = crate::ingest::home_dir()
            .map(|h| h.join(".local/share/claude/versions"))
            .and_then(|d| detect_cli_version(&d))
            .unwrap_or_else(|| FALLBACK_CLI_VERSION.to_owned());
        tracing::debug!("limits: claude-cli UA version {v}");
        user_agent_for(&v)
    })
    .as_str()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir, File};

    #[test]
    fn detect_picks_highest_semver() {
        let d = tempfile::tempdir().unwrap();
        create_dir(d.path().join("2.1.9")).unwrap();
        File::create(d.path().join("2.1.10")).unwrap();
        create_dir(d.path().join("2.0.99")).unwrap();
        assert_eq!(detect_cli_version(d.path()), Some("2.1.10".to_owned()));
    }

    #[test]
    fn detect_ignores_invalid_names() {
        let d = tempfile::tempdir().unwrap();
        for n in [
            "latest",
            "2.1",
            "2.1.3-beta",
            "v3.0.0",
            ".DS_Store",
            "1.0.0",
        ] {
            File::create(d.path().join(n)).unwrap();
        }
        assert_eq!(detect_cli_version(d.path()), Some("1.0.0".to_owned()));
    }

    #[test]
    fn detect_missing_dir_returns_none() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(detect_cli_version(&d.path().join("nope")), None);
    }

    #[test]
    fn detect_empty_dir_returns_none() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(detect_cli_version(d.path()), None);
    }

    #[test]
    fn user_agent_format() {
        assert_eq!(
            user_agent_for("2.1.328"),
            "claude-cli/2.1.328 (external, cli)"
        );
    }
}
