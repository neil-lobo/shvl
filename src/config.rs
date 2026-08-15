use std::{env::var_os, fs, path::PathBuf};

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub base_dir: String,
}

pub fn get_config() -> Result<Config, String> {
    let mut local_config = var_os("HOME")
        .map(PathBuf::from)
        .ok_or("Could not read $HOME environment variable. Please use --dir instead.")?;
    local_config.push(".local/share/shvl/config.json");

    let exists = match fs::exists(&local_config) {
        Ok(exists) => exists,
        Err(_) => {
            println!("error checking for local config, skipping.");
            false
        }
    };

    if !exists {
        return Err("config file does not exist".to_owned());
    }

    let config_json = fs::read_to_string(&local_config).or(Err("cannot read config file"))?;

    let config: Config =
        serde_json::from_str(&config_json).or(Err("Unable to parse config file"))?;

    Ok(config)
}
