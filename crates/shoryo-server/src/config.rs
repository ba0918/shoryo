//! The per-user config file: the screen's language and theme, kept apart from every topic's
//! data (`docs/spec/server.md`, "設定ファイル").

use std::fmt;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    #[default]
    En,
    Ja,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    /// Follows the OS's light or dark setting.
    #[default]
    System,
    Light,
    Dark,
}

/// The settings; an absent key is its default. A key shoryo does not know makes the file
/// unreadable, so a file written by something else is never rewritten without it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub language: Language,
    #[serde(default)]
    pub theme: Theme,
}

/// A switch made on the screen: the keys it sets.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigChange {
    pub language: Option<Language>,
    pub theme: Option<Theme>,
}

#[derive(Debug)]
pub enum ConfigError {
    /// No per-user config directory could be found.
    NoConfigDirectory,
    /// The file is there but cannot be read or understood; it is left as it is.
    Unreadable { path: PathBuf, reason: String },
    Write {
        path: PathBuf,
        error: std::io::Error,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoConfigDirectory => write!(f, "the per-user config directory cannot be found"),
            Self::Unreadable { path, reason } => write!(
                f,
                "the config file {} cannot be read ({reason}); using the defaults and leaving \
                 the file unchanged",
                path.display()
            ),
            Self::Write { path, error } => write!(f, "{}: {error}", path.display()),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Where the config file is: `<config dir>/shoryo/config.toml`.
#[derive(Debug, Clone)]
pub struct ConfigFile {
    path: Option<PathBuf>,
}

impl ConfigFile {
    /// The per-user config file, honouring `XDG_CONFIG_HOME`.
    pub fn per_user() -> Self {
        Self {
            path: dirs::config_dir().map(|dir| dir.join("shoryo").join("config.toml")),
        }
    }

    /// Reads the file; a missing file is the defaults.
    pub fn read(&self) -> Result<Config, ConfigError> {
        let path = self.path.as_deref().ok_or(ConfigError::NoConfigDirectory)?;
        match fs::read_to_string(path) {
            Ok(text) => toml::from_str(&text).map_err(|error| unreadable(path, error)),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(Config::default()),
            Err(error) => Err(unreadable(path, error)),
        }
    }

    /// Applies a switch and writes the whole file through a temporary file and a rename, so a
    /// crash leaves the old file or the new one. An unreadable file is refused, untouched.
    pub fn change(&self, change: ConfigChange) -> Result<Config, ConfigError> {
        let mut config = self.read()?;
        let path = self.path.as_deref().ok_or(ConfigError::NoConfigDirectory)?;
        config.language = change.language.unwrap_or(config.language);
        config.theme = change.theme.unwrap_or(config.theme);
        let text = toml::to_string(&config)
            .expect("the config holds only two plain keys, which always serialise");
        let write = |target: &Path, error: std::io::Error| ConfigError::Write {
            path: target.to_path_buf(),
            error,
        };
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|error| write(dir, error))?;
        }
        let temporary = path.with_extension("toml.tmp");
        fs::write(&temporary, text).map_err(|error| write(&temporary, error))?;
        fs::rename(&temporary, path).map_err(|error| write(path, error))?;
        Ok(config)
    }
}

fn unreadable(path: &Path, reason: impl fmt::Display) -> ConfigError {
    ConfigError::Unreadable {
        path: path.to_path_buf(),
        reason: reason.to_string(),
    }
}
