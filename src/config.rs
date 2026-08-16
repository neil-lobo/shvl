use serde::Deserialize;
use std::{env::var_os, fs, path::PathBuf};

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub base_dir: Option<String>,
}

pub fn get_config() -> Option<Config> {
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

    let Ok(config_json) = fs::read_to_string(&local_config) else {
        // TODO: print warning
        // .or(Err("cannot read config file"))?;
        return None;
    };

    let Ok(config) = serde_json::from_str(&config_json) else {
        // TODO: print warning
        // .or(Err("Unable to parse config file"))?;
        return None;
    };

    config
}
