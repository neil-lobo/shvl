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
            base_dir: ctx.dir_flag,
            verbose: Some(ctx.verbose_log),
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
    let local_config = if let Some(mut local_config) = var_os("HOME").map(PathBuf::from) {
        local_config.push(".local/share/shvl/config.json");
        Some(local_config)
    } else {
        println!("[WARN] Could not read $HOME environment variable. Please use --dir instead.");
        None
    }?;

    if ctx.verbose_log {
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
        if ctx.verbose_log {
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

    if ctx.verbose_log {
        println!("[DEBUG] config loaded from file: {config:?}")
    }

    Some(config)
}
