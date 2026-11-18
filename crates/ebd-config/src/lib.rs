//! Persistent, deny-by-default configuration for EBD.
//!
//! Configuration is deliberately small: it covers process-level concerns
//! (logging today, plugin permissions later) and never stores credentials or
//! client session data. Unknown keys are rejected so typos fail loudly instead
//! of being silently ignored.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Directory name used under the platform configuration root.
pub const APP_DIR: &str = "ebd";
/// File name used inside [`APP_DIR`].
pub const CONFIG_FILE_NAME: &str = "config.json";
/// Environment variable that overrides the config file path.
pub const CONFIG_PATH_ENV: &str = "EBD_CONFIG";

/// Minimum severity that is written to the log.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    /// Lower-case identifier used in the config file and in output.
    pub const fn as_str(self) -> &'static str {
        match self {
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for LogLevel {
    type Err = ConfigError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "error" => Ok(LogLevel::Error),
            "warn" => Ok(LogLevel::Warn),
            "info" => Ok(LogLevel::Info),
            "debug" => Ok(LogLevel::Debug),
            "trace" => Ok(LogLevel::Trace),
            other => Err(ConfigError::InvalidLogLevel(other.to_owned())),
        }
    }
}

/// How log records are rendered.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    #[default]
    Text,
    Json,
}

/// Logging settings.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LoggingConfig {
    pub level: LogLevel,
    pub format: LogFormat,
}

/// Root configuration document.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub logging: LoggingConfig,
}

impl Config {
    /// Reads configuration from `path`.
    ///
    /// A missing file yields the defaults so a fresh install works without any
    /// setup. Unreadable or invalid files are errors: EBD does not silently
    /// discard a configuration the user edited.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        match fs::read_to_string(path) {
            Ok(contents) => Self::from_json(&contents),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(ConfigError::Io {
                path: path.to_owned(),
                source,
            }),
        }
    }

    /// Parses a JSON document.
    pub fn from_json(contents: &str) -> Result<Self, ConfigError> {
        Ok(serde_json::from_str(contents)?)
    }

    /// Renders the configuration as pretty JSON.
    pub fn to_json(&self) -> Result<String, ConfigError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Writes the configuration to `path`, creating parent directories.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|source| ConfigError::Io {
                    path: parent.to_owned(),
                    source,
                })?;
            }
        }

        fs::write(path, self.to_json()?).map_err(|source| ConfigError::Io {
            path: path.to_owned(),
            source,
        })
    }
}

/// Default config file path, honouring `EBD_CONFIG` and platform roots.
pub fn default_path() -> Result<PathBuf, ConfigError> {
    default_path_from(|key| std::env::var(key).ok())
}

/// Testable form of [`default_path`].
///
/// Resolution order is `EBD_CONFIG`, then the platform configuration root, then
/// `$HOME/.config`. No directory is created here.
pub fn default_path_from<E>(env: E) -> Result<PathBuf, ConfigError>
where
    E: Fn(&str) -> Option<String>,
{
    if let Some(explicit) = env(CONFIG_PATH_ENV) {
        if !explicit.trim().is_empty() {
            return Ok(PathBuf::from(explicit));
        }
    }

    let root = env("APPDATA")
        .or_else(|| env("XDG_CONFIG_HOME"))
        .map(PathBuf::from)
        .or_else(|| env("HOME").map(|home| PathBuf::from(home).join(".config")));

    match root {
        Some(root) => Ok(root.join(APP_DIR).join(CONFIG_FILE_NAME)),
        None => Err(ConfigError::NoConfigDir),
    }
}

/// Errors produced while locating, reading, or writing configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("could not access {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("invalid configuration: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unknown log level `{0}` (expected error, warn, info, debug, or trace)")]
    InvalidLogLevel(String),
    #[error("cannot determine a configuration directory; set {CONFIG_PATH_ENV} to choose a path")]
    NoConfigDir,
    #[error("refusing to overwrite the existing configuration at {0}")]
    AlreadyExists(PathBuf),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ebd-config-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn defaults_are_info_level_text_logging() {
        let config = Config::default();

        assert_eq!(config.logging.level, LogLevel::Info);
        assert_eq!(config.logging.format, LogFormat::Text);
    }

    #[test]
    fn missing_files_load_as_defaults() {
        let path = temp_dir("missing").join(CONFIG_FILE_NAME);

        assert_eq!(Config::load(&path).expect("defaults"), Config::default());
    }

    #[test]
    fn config_round_trips_through_disk() {
        let dir = temp_dir("round-trip");
        let path = dir.join("nested").join(CONFIG_FILE_NAME);

        let config = Config {
            logging: LoggingConfig {
                level: LogLevel::Debug,
                format: LogFormat::Json,
            },
        };
        config.save(&path).expect("save");

        assert!(path.exists(), "save should create parents");
        assert_eq!(Config::load(&path).expect("load"), config);
    }

    #[test]
    fn invalid_json_is_an_error() {
        assert!(matches!(
            Config::from_json("{\"logging\": {"),
            Err(ConfigError::Json(_))
        ));
    }

    #[test]
    fn unknown_keys_are_rejected() {
        assert!(matches!(
            Config::from_json(r#"{"logging": {"levl": "debug"}}"#),
            Err(ConfigError::Json(_))
        ));
        assert!(matches!(
            Config::from_json(r#"{"plugins": {"enabled": true}}"#),
            Err(ConfigError::Json(_))
        ));
    }

    #[test]
    fn partial_documents_keep_defaults() {
        let config =
            Config::from_json(r#"{"logging": {"level": "trace"}}"#).expect("partial config");

        assert_eq!(config.logging.level, LogLevel::Trace);
        assert_eq!(config.logging.format, LogFormat::Text);
    }

    #[test]
    fn log_levels_parse_case_insensitively() {
        assert_eq!("DEBUG".parse::<LogLevel>().expect("debug"), LogLevel::Debug);
        assert_eq!(" warn ".parse::<LogLevel>().expect("warn"), LogLevel::Warn);
        assert!(matches!(
            "verbose".parse::<LogLevel>(),
            Err(ConfigError::InvalidLogLevel(_))
        ));
    }

    #[test]
    fn explicit_override_wins_over_platform_roots() {
        let path = default_path_from(|key| match key {
            CONFIG_PATH_ENV => Some("D:/custom/ebd.json".into()),
            "APPDATA" => Some("C:/Users/test/AppData/Roaming".into()),
            _ => None,
        })
        .expect("path");

        assert_eq!(path, PathBuf::from("D:/custom/ebd.json"));
    }

    #[test]
    fn platform_roots_and_home_are_used_in_order() {
        let appdata = default_path_from(|key| match key {
            "APPDATA" => Some("C:/Users/test/AppData/Roaming".into()),
            "HOME" => Some("C:/Users/test".into()),
            _ => None,
        })
        .expect("appdata path");
        assert_eq!(
            appdata,
            PathBuf::from("C:/Users/test/AppData/Roaming")
                .join(APP_DIR)
                .join(CONFIG_FILE_NAME)
        );

        let home = default_path_from(|key| match key {
            "HOME" => Some("/home/test".into()),
            _ => None,
        })
        .expect("home path");
        assert_eq!(
            home,
            PathBuf::from("/home/test/.config")
                .join(APP_DIR)
                .join(CONFIG_FILE_NAME)
        );
    }

    #[test]
    fn no_roots_is_an_explicit_error() {
        assert!(matches!(
            default_path_from(|_| None),
            Err(ConfigError::NoConfigDir)
        ));
    }
}
