use std::{fs, println};
use tabled::{builder::Builder, settings::Style};

use crate::{
    config::Config,
    utils::{self, default_group, parse_group_file, stringify_group},
};

pub fn create_group(config: Config, group: &String) -> Result<(), String> {
    if config.verbose {
        println!("[DEBUG] creating group '{group}'");
    }

    let dir = utils::get_base_dir(config.clone())?;

    if dir.is_file() {
        return Err("file named '.shvl' found".to_string());
    }

    if !dir.exists() {
        fs::create_dir_all(&dir).or(Err(format!("Unable to create dir: {}", &dir.display())))?;
    }

    if !dir.is_dir() {
        return Err("unable to find .shvl dir".to_string());
    }

    let path = utils::group_path(&dir, group)?;

    if path.exists() {
        return Err(format!("group '{group}' already exists"));
    }

    // create any subdirectories the group name asks for
    let parent = path.parent().ok_or("invalid group path")?;
    if !parent.exists() {
        if config.verbose {
            println!(
                "[DEBUG] parent folder of group file does not exist, creating '{}'",
                parent.to_str().unwrap_or("None")
            )
        }

        fs::create_dir_all(parent)
            .or(Err(format!("Unable to create dir: {}", parent.display())))?;
    }

    if !parent.is_dir() {
        return Err(format!("'{}' is not a directory", parent.display()));
    }

    let default_group = default_group(group.to_owned());

    let group_file_str = stringify_group(default_group)?;

    if config.verbose {
        println!("[DEBUG] writing file '{}'", path.to_str().unwrap_or("None"));
    }

    fs::write(&path, group_file_str)
        .or(Err(format!("Unable to write to file: {}", &path.display())))?;

    Ok(())
}

pub fn remove_group(config: Config, group: &String) -> Result<(), String> {
    if config.verbose {
        println!("[DEBUG] removing group '{group}'")
    }

    let dir = utils::get_base_dir(config)?;

    if dir.is_file() {
        return Err("file named '.shvl' found".to_owned());
    }

    if !dir.exists() {
        return Err(".shvl dir not found".to_owned());
    }

    if !dir.is_dir() {
        return Err("unable to find .shvl dir".to_owned());
    }

    let path = utils::group_path(&dir, group)?;

    if !path.exists() {
        return Err("group does not exist".to_owned());
    }

    fs::remove_file(&path).or(Err(format!("Unable to remove file: {}", &path.display())))?;

    // clean up any subdirectories left empty by the removal
    if let Some(parent) = path.parent() {
        utils::prune_empty_dirs(&dir, parent)?;
    }

    Ok(())
}

pub fn group_info(config: Config, group: &String) -> Result<(), String> {
    if config.verbose {
        println!("[DEBUG] getting info for group '{group}'");
    }

    let dir = utils::get_base_dir(config.clone())?;
    let path = utils::group_path(&dir, group)?;

    if config.verbose {
        println!("[DEBUG] group path: '{}'", path.to_str().unwrap_or("None"))
    }

    let exists = fs::exists(&path).or(Err("Unable to find group file"))?;

    if !exists {
        return Err("Group does not exist".to_owned());
    }

    let nix = fs::read_to_string(&path).or(Err("Unable to read file"))?;

    let group = parse_group_file(group, &nix)?;

    let mut builder = Builder::new();
    builder.push_record(["Name:", &group.name]);
    builder.push_record([""]);

    builder.push_record(["Packages:"]);
    for package in group.packages {
        builder.push_record(["", &package]);
    }

    let table = builder.build().with(Style::ascii_rounded()).to_string();
    println!("{table}");

    Ok(())
}

pub fn list_groups(config: Config) -> Result<(), String> {
    let group_names = utils::get_group_names(config)?;

    let mut builder = Builder::new();
    builder.push_record(["Groups:"]);

    for name in group_names {
        builder.push_record(["", &name]);
    }

    let table = builder.build().with(Style::ascii_rounded()).to_string();
    println!("{table}");

    Ok(())
}

pub fn add_package(config: Config, group: &String, package: &String) -> Result<(), String> {
    if config.verbose {
        println!("[DEBUG] adding package '{package}' to group '{group}'")
    }

    let dir = utils::get_base_dir(config.clone())?;
    let path = utils::group_path(&dir, group)?;

    if config.verbose {
        println!("[DEBUG] group path: '{}'", path.to_str().unwrap_or("None"))
    }

    let nix = fs::read_to_string(&path).or(Err("Unable to read file"))?;

    let mut group = parse_group_file(group, &nix)?;

    // TODO: custom insert method that errors if entry already exists
    group.packages.insert(package.to_owned());

    let serialized_group = stringify_group(group)?;

    fs::write(&path, serialized_group).or(Err("Unable to write to file"))?;

    Ok(())
}

pub fn remove_package(config: Config, group: &String, package: &String) -> Result<(), String> {
    if config.verbose {
        println!("[DEBUG] removing package '{package}' to group '{group}'")
    }

    let dir = utils::get_base_dir(config.clone())?;
    let path = utils::group_path(&dir, group)?;

    if config.verbose {
        println!("[DEBUG] group path: '{}'", path.to_str().unwrap_or("None"))
    }

    let nix = fs::read_to_string(&path).or(Err("Unable to read file"))?;

    let mut group = parse_group_file(group, &nix)?;

    // TODO: custom remove method that errors if entry does not exist
    group.packages.remove(package);

    let serialized_group = stringify_group(group)?;

    fs::write(&path, serialized_group).or(Err("Unable to write to file"))?;

    Ok(())
}
