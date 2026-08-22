use serde::Deserialize;
use std::{env::var_os, fs, path::PathBuf};

use crate::utils::CommandContext;

#[derive(Deserialize, Debug)]
pub struct PartialConfig {
    pub base_dir: Option<String>,
    pub verbose: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub base_dir: String,
    pub verbose: bool,
}

impl PartialConfig {
    /// `self` takes precedent in merge
    pub fn merge(self, other: PartialConfig) -> Self {
        PartialConfig {
            base_dir: self.base_dir.or(other.base_dir),
            verbose: self.verbose.or(other.verbose),
        }
    }

    /// `self` takes precedent in merge
    pub fn merge_option(self, other: Option<Self>) -> Self {
        if let Some(other) = other {
            self.merge(other)
        } else {
            self
        }
    }

    pub fn merge_default(self) -> Config {
        Config {
            base_dir: self.base_dir.unwrap_or("/etc/nixos/.shvl".to_owned()),
            verbose: self.verbose.unwrap_or(false),
        }
    }
}

impl From<CommandContext> for PartialConfig {
    fn from(ctx: CommandContext) -> Self {
        PartialConfig {
            base_dir: ctx.base_dir,
            verbose: ctx.verbose_log,
        }
    }
}

pub fn get_config(ctx: CommandContext) -> Config {
    let config = PartialConfig::from(ctx.clone())
        .merge_option(get_config_from_file(ctx))
        .merge_default();

    config
}

fn get_config_from_file(ctx: CommandContext) -> Option<PartialConfig> {
    let local_config = if let Some(config_path) = ctx.config_path {
        Some(PathBuf::from(config_path))
    } else if let Some(mut local_config) = var_os("HOME").map(PathBuf::from) {
        local_config.push(".local/share/shvl/config.json");
        Some(local_config)
    } else {
        if ctx.verbose_log == Some(true) {
            println!("[DEBUG] could not read $HOME environment variable")
        }

        println!("[WARN] Could not locate config. Use --config instead.");
        None
    }?;

    if ctx.verbose_log == Some(true) {
        println!(
            "[DEBUG] loading config file from '{}'",
            local_config.to_str().unwrap_or("None")
        )
    }

    let exists = if let Ok(exists) = fs::exists(&local_config) {
        Some(exists)
    } else {
        println!("[WARN] Could not determine if local config exists or not, skipping.");
        None
    }?;

    if !exists {
        if ctx.verbose_log == Some(true) {
            println!(
                "[DEBUG] config file '{}' does not exist",
                local_config.to_str().unwrap_or("None")
            )
        }

        return None;
    }

    let config_json = if let Ok(config_json) = fs::read_to_string(&local_config) {
        Some(config_json)
    } else {
        println!("[WARN] Unable to read config file, skipping.");
        None
    }?;

    let config = if let Ok(config) = serde_json::from_str::<PartialConfig>(&config_json) {
        Some(config)
    } else {
        println!("[WARN] Unable to parse config file, skipping.");
        None
    }?;

    if ctx.verbose_log == Some(true) {
        println!("[DEBUG] config loaded from file: {config:?}")
    }

    Some(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod partial_config_struct {
        use super::*;

        mod from_context {
            use super::*;

            #[test]
            fn with_dir_flag() {
                let base_dir = tempfile::tempdir()
                    .unwrap()
                    .path()
                    .to_str()
                    .unwrap()
                    .to_owned();

                let ctx = CommandContext {
                    base_dir: Some(base_dir.clone()),
                    verbose_log: None,
                    config_path: None,
                };

                let config = PartialConfig::from(ctx);

                assert!(config.base_dir == Some(base_dir));
                assert!(config.verbose == None);
            }

            #[test]
            fn without_dir_flag() {
                let ctx = CommandContext {
                    base_dir: None,
                    verbose_log: Some(false),
                    config_path: None,
                };

                let config = PartialConfig::from(ctx);

                assert!(config.base_dir == None);
                assert!(config.verbose == Some(false));
            }
        }

        #[test]
        fn merge() {
            let c1 = PartialConfig {
                base_dir: None,
                verbose: Some(true),
            };

            let c2 = PartialConfig {
                base_dir: Some("foo".to_owned()),
                verbose: Some(false),
            };

            let config = c1.merge(c2);

            assert!(config.base_dir == Some("foo".to_owned()));
            assert!(config.verbose == Some(true));
        }

        #[test]
        fn merge_option() {
            let c1 = PartialConfig {
                base_dir: None,
                verbose: Some(true),
            };

            let c2 = Some(PartialConfig {
                base_dir: Some("foo".to_owned()),
                verbose: Some(false),
            });

            let config = c1.merge_option(c2);

            assert!(config.base_dir == Some("foo".to_owned()));
            assert!(config.verbose == Some(true));
        }
    }

    #[test]
    fn test_get_config_from_file() {
        let base_dir = tempfile::tempdir().unwrap();

        let mut config_path = base_dir.path().to_path_buf();
        config_path.push("config.json");

        fs::write(
            &config_path,
            "
{
    \"base_dir\":\"foobar\",
    \"verbose\": true
}
",
        )
        .unwrap();

        println!("wrote to {}", config_path.to_str().unwrap());

        println!(
            "config file:\n{}",
            fs::read_to_string(&config_path).unwrap()
        );

        let ctx = CommandContext {
            base_dir: None,
            verbose_log: None,
            config_path: Some(config_path.to_str().unwrap().to_owned()),
        };

        let config = get_config_from_file(ctx).unwrap();

        assert!(config.base_dir == Some("foobar".to_owned()));
        assert!(config.verbose == Some(true));
    }

    #[test]
    fn test_get_config() {
        let base_dir = tempfile::tempdir().unwrap();

        let mut config_path = base_dir.path().to_path_buf();
        config_path.push("config.json");

        fs::write(
            &config_path,
            "
{
    \"base_dir\":\"foobar\",
    \"verbose\": false
}
",
        )
        .unwrap();

        println!("wrote to {}", config_path.to_str().unwrap());

        println!(
            "config file:\n{}",
            fs::read_to_string(&config_path).unwrap()
        );

        let ctx = CommandContext {
            base_dir: None,
            verbose_log: None,
            config_path: Some(config_path.to_str().unwrap().to_owned()),
        };

        let config = get_config(ctx);

        assert!(config.base_dir == "foobar");
        assert!(config.verbose == false);
    }
}
