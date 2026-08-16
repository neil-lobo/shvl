use serde::Deserialize;
use std::{env::var_os, fs, path::PathBuf};

use crate::utils::CommandContext;

#[derive(Deserialize)]
pub struct PartialConfig {
    pub base_dir: Option<String>,
}

#[derive(Clone)]
pub struct Config {
    pub base_dir: String,
}

impl PartialConfig {
    /// `self` takes precedent in merge
    pub fn merge(self, other: PartialConfig) -> Self {
        PartialConfig {
            base_dir: other.base_dir.or(self.base_dir),
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
        }
    }
}

impl From<CommandContext> for PartialConfig {
    fn from(ctx: CommandContext) -> Self {
        PartialConfig {
            base_dir: ctx.dir_flag,
        }
    }
}

pub fn get_config(ctx: CommandContext) -> Config {
    let config = PartialConfig::from(ctx)
        .merge_option(get_config_from_file())
        .merge_default();

    config
}

fn get_config_from_file() -> Option<PartialConfig> {
    let local_config = if let Some(mut local_config) = var_os("HOME").map(PathBuf::from) {
        local_config.push(".local/share/shvl/config.json");
        Some(local_config)
    } else {
        // TODO: print wanring
        // .ok_or("Could not read $HOME environment variable. Please use --dir instead.")?;
        None
    }?;

    let exists = if let Ok(exists) = fs::exists(&local_config) {
        Some(exists)
    } else {
        // TODO: print warning
        //         println!("error checking for local config, skipping.");
        None
    }?;

    if !exists {
        // TODO: print debug
        return None;
        // return Err(ConfigError::MissingFile);
        // return Err("config file does not exist".to_owned());
    }

    let config_json = if let Ok(config_json) = fs::read_to_string(&local_config) {
        Some(config_json)
    } else {
        // TODO: print warning
        // .or(Err("cannot read config file"))?;
        None
    }?;

    let config = if let Ok(config) = serde_json::from_str::<PartialConfig>(&config_json) {
        Some(config)
    } else {
        // TODO: print warning
        // .or(Err("Unable to parse config file"))?;
        None
    }?;

    Some(config)
}
