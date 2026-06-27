use std::fs;
use std::path::{Path, PathBuf};

use commitchi_core::DiffOptions;
use commitchi_pet::{MoodConfig, PetScope};
use serde::Deserialize;
use thiserror::Error;

use crate::animation::AnimationConfig;

const CONFIG_FILE_NAME: &str = "commitchi.toml";

pub type Result<T> = std::result::Result<T, ConfigError>;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to parse config at {path}: {source}")]
    Toml {
        path: PathBuf,
        source: toml::de::Error,
    },

    #[error("invalid config value {field} in {path}: {message}")]
    Invalid {
        path: PathBuf,
        field: &'static str,
        message: String,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RuntimeConfig {
    pub diff_options: DiffOptions,
    pub animation_config: AnimationConfig,
    pub pet_scope: PetScope,
    pub mood_config: MoodConfig,
}

impl RuntimeConfig {
    pub fn load(repo_root: &Path, explicit_path: Option<&Path>) -> Result<Self> {
        let Some(path) = config_path(repo_root, explicit_path) else {
            return Ok(Self::default());
        };

        let contents = fs::read_to_string(&path).map_err(|source| ConfigError::Io {
            path: path.clone(),
            source,
        })?;

        Self::from_toml_str(&contents, &path)
    }

    pub fn apply_overrides(&mut self, overrides: ConfigOverrides) -> Result<()> {
        let path = Path::new("<cli>");

        if let Some(line_limit) = overrides.large_diff_line_limit {
            validate_positive_usize(line_limit, "git.large_diff_line_limit", path)?;
            self.diff_options.line_limit = line_limit;
        }

        if let Some(file_limit) = overrides.large_diff_file_limit {
            validate_positive_usize(file_limit, "git.large_diff_file_limit", path)?;
            self.diff_options.file_limit = file_limit;
        }

        if let Some(lines_per_second) = overrides.lines_per_second {
            validate_positive_f64(lines_per_second, "animation.lines_per_second", path)?;
            self.animation_config =
                AnimationConfig::new(lines_per_second, self.animation_config.commits_per_second());
        }

        if let Some(commits_per_second) = overrides.commits_per_second {
            validate_positive_f64(commits_per_second, "animation.commits_per_second", path)?;
            self.animation_config =
                AnimationConfig::new(self.animation_config.lines_per_second(), commits_per_second);
        }

        if let Some(pet_scope) = overrides.pet_scope {
            self.pet_scope = pet_scope;
        }

        Ok(())
    }

    fn from_toml_str(contents: &str, path: &Path) -> Result<Self> {
        let file_config =
            toml::from_str::<FileConfig>(contents).map_err(|source| ConfigError::Toml {
                path: path.to_path_buf(),
                source,
            })?;

        let mut config = Self::default();
        config.apply_file_config(file_config, path)?;
        Ok(config)
    }

    fn apply_file_config(&mut self, file_config: FileConfig, path: &Path) -> Result<()> {
        if let Some(pet) = file_config.pet {
            if let Some(scope) = pet.scope {
                self.pet_scope = scope;
            }

            if let Some(thresholds) = pet.thresholds {
                let mut mood_config = self.mood_config;
                if let Some(value) = thresholds.thriving_hours {
                    mood_config.thriving_hours = value;
                }
                if let Some(value) = thresholds.content_hours {
                    mood_config.content_hours = value;
                }
                if let Some(value) = thresholds.neutral_hours {
                    mood_config.neutral_hours = value;
                }
                if let Some(value) = thresholds.anxious_hours {
                    mood_config.anxious_hours = value;
                }
                validate_mood_config(mood_config, path)?;
                self.mood_config = mood_config;
            }
        }

        if let Some(animation) = file_config.animation {
            let lines_per_second = animation
                .lines_per_second
                .unwrap_or_else(|| self.animation_config.lines_per_second());
            let commits_per_second = animation
                .commits_per_second
                .unwrap_or_else(|| self.animation_config.commits_per_second());

            validate_positive_f64(lines_per_second, "animation.lines_per_second", path)?;
            validate_positive_f64(commits_per_second, "animation.commits_per_second", path)?;
            self.animation_config = AnimationConfig::new(lines_per_second, commits_per_second);
        }

        if let Some(git) = file_config.git {
            if let Some(line_limit) = git.large_diff_line_limit {
                validate_positive_usize(line_limit, "git.large_diff_line_limit", path)?;
                self.diff_options.line_limit = line_limit;
            }
            if let Some(file_limit) = git.large_diff_file_limit {
                validate_positive_usize(file_limit, "git.large_diff_file_limit", path)?;
                self.diff_options.file_limit = file_limit;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct ConfigOverrides {
    pub large_diff_line_limit: Option<usize>,
    pub large_diff_file_limit: Option<usize>,
    pub lines_per_second: Option<f64>,
    pub commits_per_second: Option<f64>,
    pub pet_scope: Option<PetScope>,
}

#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    pet: Option<PetConfig>,
    animation: Option<AnimationFileConfig>,
    git: Option<GitFileConfig>,
}

#[derive(Debug, Default, Deserialize)]
struct PetConfig {
    scope: Option<PetScope>,
    thresholds: Option<MoodThresholdsFileConfig>,
}

#[derive(Debug, Default, Deserialize)]
struct MoodThresholdsFileConfig {
    thriving_hours: Option<i64>,
    content_hours: Option<i64>,
    neutral_hours: Option<i64>,
    anxious_hours: Option<i64>,
}

#[derive(Debug, Default, Deserialize)]
struct AnimationFileConfig {
    lines_per_second: Option<f64>,
    commits_per_second: Option<f64>,
}

#[derive(Debug, Default, Deserialize)]
struct GitFileConfig {
    large_diff_line_limit: Option<usize>,
    large_diff_file_limit: Option<usize>,
}

fn config_path(repo_root: &Path, explicit_path: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = explicit_path {
        return Some(path.to_path_buf());
    }

    let repo_config = repo_root.join(CONFIG_FILE_NAME);
    repo_config.is_file().then_some(repo_config)
}

fn validate_positive_usize(value: usize, field: &'static str, path: &Path) -> Result<()> {
    if value > 0 {
        Ok(())
    } else {
        Err(invalid(path, field, "must be greater than 0"))
    }
}

fn validate_positive_f64(value: f64, field: &'static str, path: &Path) -> Result<()> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(invalid(
            path,
            field,
            "must be a finite number greater than 0",
        ))
    }
}

fn validate_mood_config(config: MoodConfig, path: &Path) -> Result<()> {
    let thresholds = [
        ("pet.thresholds.thriving_hours", config.thriving_hours),
        ("pet.thresholds.content_hours", config.content_hours),
        ("pet.thresholds.neutral_hours", config.neutral_hours),
        ("pet.thresholds.anxious_hours", config.anxious_hours),
    ];

    for (field, value) in thresholds {
        if value <= 0 {
            return Err(invalid(path, field, "must be greater than 0"));
        }
    }

    if config.thriving_hours > config.content_hours
        || config.content_hours > config.neutral_hours
        || config.neutral_hours > config.anxious_hours
    {
        return Err(invalid(
            path,
            "pet.thresholds",
            "hours must be ordered thriving <= content <= neutral <= anxious",
        ));
    }

    Ok(())
}

fn invalid(path: &Path, field: &'static str, message: impl Into<String>) -> ConfigError {
    ConfigError::Invalid {
        path: path.to_path_buf(),
        field,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_repo_config_uses_defaults() {
        let tmp = tempfile::tempdir().expect("tempdir");

        let config = RuntimeConfig::load(tmp.path(), None).expect("load config");

        assert_eq!(config, RuntimeConfig::default());
    }

    #[test]
    fn toml_config_overrides_runtime_defaults() {
        let path = Path::new("commitchi.toml");
        let config = RuntimeConfig::from_toml_str(
            r#"
                [pet]
                scope = "both"

                [pet.thresholds]
                thriving_hours = 6
                content_hours = 12
                neutral_hours = 48
                anxious_hours = 96

                [animation]
                lines_per_second = 60
                commits_per_second = 2.5

                [git]
                large_diff_line_limit = 500
                large_diff_file_limit = 20
            "#,
            path,
        )
        .expect("parse config");

        assert_eq!(config.pet_scope, PetScope::Both);
        assert_eq!(config.mood_config.thriving_hours, 6);
        assert_eq!(config.animation_config.lines_per_second(), 60.0);
        assert_eq!(config.animation_config.commits_per_second(), 2.5);
        assert_eq!(config.diff_options.line_limit, 500);
        assert_eq!(config.diff_options.file_limit, 20);
    }

    #[test]
    fn cli_overrides_win_over_file_config() {
        let mut config = RuntimeConfig::from_toml_str(
            r#"
                [pet]
                scope = "repo"

                [animation]
                lines_per_second = 20
            "#,
            Path::new("commitchi.toml"),
        )
        .expect("parse config");

        config
            .apply_overrides(ConfigOverrides {
                lines_per_second: Some(90.0),
                pet_scope: Some(PetScope::Global),
                ..ConfigOverrides::default()
            })
            .expect("apply overrides");

        assert_eq!(config.animation_config.lines_per_second(), 90.0);
        assert_eq!(config.pet_scope, PetScope::Global);
    }

    #[test]
    fn invalid_threshold_order_is_rejected() {
        let err = RuntimeConfig::from_toml_str(
            r#"
                [pet.thresholds]
                thriving_hours = 48
                content_hours = 24
            "#,
            Path::new("commitchi.toml"),
        )
        .expect_err("invalid thresholds");

        assert!(err.to_string().contains("pet.thresholds"));
    }

    #[test]
    fn explicit_missing_config_is_an_error() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("missing.toml");

        let err = RuntimeConfig::load(tmp.path(), Some(&path)).expect_err("missing config");

        assert!(matches!(err, ConfigError::Io { .. }));
    }
}
