use std::path::PathBuf;

pub const RTK_DATA_DIR: &str = "rtk";
pub const HISTORY_DB: &str = "history.db";
pub const CONFIG_TOML: &str = "config.toml";
pub const FILTERS_TOML: &str = "filters.toml";
pub const TRUSTED_FILTERS_JSON: &str = "trusted_filters.json";
pub const DEFAULT_HISTORY_DAYS: i64 = 90;

/// RTK-only subcommands that should never fall back to raw execution.
/// When adding a new RTK-only subcommand to `Commands`, add its clap name here.
pub const RTK_META_COMMANDS: &[&str] = &[
    "gain",
    "discover",
    "learn",
    "init",
    "config",
    "proxy",
    "run",
    "hook",
    "hook-audit",
    "pipe",
    "cc-economics",
    "verify",
    "trust",
    "untrust",
    "session",
    "rewrite",
    "telemetry",
    "smart",
    "deps",
    "json",
];

/// Environment variable naming the directory that holds `config.toml` and the
/// global `filters.toml`, in place of `<config_dir>/rtk`. Distinct from
/// [`RTK_DATA_DIR`], which is the path segment appended to the platform default.
pub const RTK_CONFIG_DIR_ENV: &str = "RTK_CONFIG_DIR";

/// Where rtk's user configuration lives: `config.toml`, the global
/// `filters.toml` and `filters/*.toml`.
///
/// `RTK_CONFIG_DIR` wins when set and non-empty, mirroring `RTK_DB_PATH` for
/// the tracking database; otherwise this is the platform config directory plus
/// rtk's own segment (`~/.config/rtk` on Linux, `~/Library/Application
/// Support/rtk` on macOS). `None` only when the variable is unset or empty and
/// the platform gives no answer.
pub fn config_dir() -> Option<PathBuf> {
    config_dir_from(std::env::var_os(RTK_CONFIG_DIR_ENV).map(PathBuf::from))
}

/// The resolution behind [`config_dir`], taking the environment value as a
/// parameter so it can be tested without mutating process-global state.
fn config_dir_from(override_dir: Option<PathBuf>) -> Option<PathBuf> {
    match override_dir {
        Some(dir) if !dir.as_os_str().is_empty() => Some(dir),
        _ => dirs::config_dir().map(|d| d.join(RTK_DATA_DIR)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn platform_default() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join(RTK_DATA_DIR))
    }

    #[test]
    fn test_config_dir_env_override_wins() {
        let custom = std::env::temp_dir().join("rtk_test_config_dir_override");
        assert_eq!(config_dir_from(Some(custom.clone())), Some(custom));
    }

    #[test]
    fn test_config_dir_unset_uses_platform_default() {
        assert_eq!(config_dir_from(None), platform_default());
    }

    #[test]
    fn test_config_dir_empty_behaves_like_unset() {
        assert_eq!(config_dir_from(Some(PathBuf::new())), platform_default());
    }
}
