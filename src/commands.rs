use anyhow::{Context, Result, bail};
use std::{fs, println};
use tabled::{builder::Builder, settings::Style};

use crate::{
    config::Config,
    group::{self, Group},
    utils,
};

pub fn create_group(config: Config, group: &String) -> Result<()> {
    if config.verbose {
        println!("[DEBUG] creating group '{group}'");
    }

    let dir = utils::get_base_dir(config.clone());

    if dir.is_file() {
        bail!("Conflicting file named '.shvl' found");
    }

    if !dir.exists() {
        fs::create_dir_all(&dir)
            .with_context(|| format!("Unable to create dir: {}", &dir.display()))?;
    }

    if !dir.is_dir() {
        bail!("Base path '{}' is not a dir", &dir.display());
    }

    let path = group::group_path(&dir, group)?;

    if path.exists() {
        bail!(format!("Group '{group}' already exists"));
    }

    // create any subdirectories the group name asks for
    let parent = path.parent().with_context(|| "Invalid group path")?;
    if !parent.exists() {
        if config.verbose {
            println!(
                "[DEBUG] parent folder of group file does not exist, creating '{}'",
                parent.to_str().unwrap_or("None")
            )
        }

        fs::create_dir_all(parent)
            .with_context(|| format!("Unable to create dir: {}", parent.display()))?;
    }

    if !parent.is_dir() {
        bail!(format!("'{}' is not a directory", parent.display()));
    }

    let default_group = Group::default(group.to_owned());

    let group_file_str = default_group.serialize();

    if config.verbose {
        println!("[DEBUG] writing file '{}'", path.to_str().unwrap_or("None"));
    }

    fs::write(&path, group_file_str)
        .with_context(|| format!("Unable to write to file: {}", &path.display()))?;

    Ok(())
}

pub fn remove_group(config: Config, group: &String) -> Result<()> {
    if config.verbose {
        println!("[DEBUG] removing group '{group}'")
    }

    let dir = utils::get_base_dir(config);

    if dir.is_file() {
        bail!("Conflicting file named '.shvl' found");
    }

    if !dir.is_dir() {
        bail!("Base path '{}' is not a dir", &dir.display());
    }

    let path = group::group_path(&dir, group)?;

    if !path.exists() {
        bail!("Group does not exist");
    }

    fs::remove_file(&path)
        .with_context(|| format!("Unable to remove file: {}", &path.display()))?;

    // clean up any subdirectories left empty by the removal
    if let Some(parent) = path.parent() {
        utils::prune_empty_dirs(&dir, parent)?;
    }

    Ok(())
}

pub fn group_info(config: Config, group: &String) -> Result<()> {
    if config.verbose {
        println!("[DEBUG] getting info for group '{group}'");
    }

    let dir = utils::get_base_dir(config.clone());
    let path = group::group_path(&dir, group)?;

    if config.verbose {
        println!("[DEBUG] group path: '{}'", path.to_str().unwrap_or("None"))
    }

    let exists = fs::exists(&path).with_context(|| "Unable to find group file")?;

    if !exists {
        bail!("Group does not exist");
    }

    let nix = fs::read_to_string(&path).with_context(|| "Unable to read file")?;

    let group = Group::deserialize(group.to_owned(), &nix)?;

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

pub fn list_groups(config: Config) -> Result<()> {
    let group_names = group::get_group_names(config)?;

    let mut builder = Builder::new();
    builder.push_record(["Groups:"]);

    for name in group_names {
        builder.push_record(["", &name]);
    }

    let table = builder.build().with(Style::ascii_rounded()).to_string();
    println!("{table}");

    Ok(())
}

pub fn add_package(config: Config, group: &String, package: &String) -> Result<()> {
    if config.verbose {
        println!("[DEBUG] adding package '{package}' to group '{group}'")
    }

    let dir = utils::get_base_dir(config.clone());
    let path = group::group_path(&dir, group)?;

    if config.verbose {
        println!("[DEBUG] group path: '{}'", path.to_str().unwrap_or("None"))
    }

    let nix = fs::read_to_string(&path).with_context(|| "Unable to read file")?;

    let mut group = Group::deserialize(group.to_owned(), &nix)?;

    if group.packages.contains(package) {
        bail!(format!(
            "Package '{package}' already exists in group '{}'",
            group.name
        ))
    }

    group.packages.insert(package.to_owned());

    let serialized_group = group.serialize();

    fs::write(&path, serialized_group).with_context(|| "Unable to write to file")?;

    Ok(())
}

pub fn remove_package(config: Config, group: &String, package: &String) -> Result<()> {
    if config.verbose {
        println!("[DEBUG] removing package '{package}' to group '{group}'")
    }

    let dir = utils::get_base_dir(config.clone());
    let path = group::group_path(&dir, group)?;

    if config.verbose {
        println!("[DEBUG] group path: '{}'", path.to_str().unwrap_or("None"))
    }

    let nix = fs::read_to_string(&path).with_context(|| "Unable to read file")?;

    let mut group = Group::deserialize(group.to_owned(), &nix)?;

    if !group.packages.contains(package) {
        bail!(format!(
            "Package '{package}' does not exist in group '{}'",
            group.name
        ));
    }

    group.packages.remove(package);

    let serialized_group = group.serialize();

    fs::write(&path, serialized_group).with_context(|| "Unable to write to file")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, Config) {
        let temp_dir = tempfile::tempdir().unwrap();

        let config = Config {
            base_dir: temp_dir.path().to_str().unwrap().to_owned(),
            verbose: false,
        };

        (temp_dir, config)
    }

    fn group_file(config: &Config, group: &str) -> std::path::PathBuf {
        group::group_path(&utils::get_base_dir(config.clone()), group).unwrap()
    }
    fn read_group(config: &Config, group: &str) -> Group {
        let nix = fs::read_to_string(group_file(config, group)).unwrap();

        Group::deserialize(group.to_owned(), &nix).unwrap()
    }

    fn packages(config: &Config, group: &str) -> Vec<String> {
        read_group(config, group).packages.into_iter().collect()
    }

    mod create_group {
        use super::*;

        #[test]
        fn basic() {
            let (_temp_dir, config) = setup();

            create_group(config.clone(), &"foo".to_owned()).unwrap();

            let path = group_file(&config, "foo");

            assert!(path.is_file());
            assert!(path.file_name().unwrap() == "foo.nix");

            let group = read_group(&config, "foo");
            assert!(group.name == "foo");
            assert!(group.packages.is_empty());
        }

        #[test]
        fn nested() {
            let (_temp_dir, config) = setup();

            create_group(config.clone(), &"foo/bar/fizz".to_owned()).unwrap();

            let path = group_file(&config, "foo/bar/fizz");

            assert!(path.is_file());
            assert!(path.parent().unwrap().is_dir());

            assert!(read_group(&config, "foo/bar/fizz").packages.is_empty());
        }

        #[test]
        fn creates_base_dir() {
            let temp_dir = tempfile::tempdir().unwrap();

            let base_dir = temp_dir.path().join(".shvl");

            let config = Config {
                base_dir: base_dir.to_str().unwrap().to_owned(),
                verbose: false,
            };

            assert!(!base_dir.exists());

            create_group(config.clone(), &"foo".to_owned()).unwrap();

            assert!(base_dir.is_dir());
            assert!(group_file(&config, "foo").is_file());
        }

        #[test]
        fn already_exists() {
            let (_temp_dir, config) = setup();

            create_group(config.clone(), &"foo".to_owned()).unwrap();

            let err = create_group(config.clone(), &"foo".to_owned()).unwrap_err();

            assert!(err.to_string() == "Group 'foo' already exists");
        }

        #[test]
        fn invalid_name() {
            let (_temp_dir, config) = setup();

            for name in ["", "/foo", "foo/", "foo//bar", "../foo", ".foo"] {
                assert!(create_group(config.clone(), &name.to_owned()).is_err());
            }
        }

        #[test]
        fn base_dir_is_file() {
            let temp_dir = tempfile::tempdir().unwrap();

            let base_dir = temp_dir.path().join(".shvl");
            fs::write(&base_dir, "not a dir").unwrap();

            let config = Config {
                base_dir: base_dir.to_str().unwrap().to_owned(),
                verbose: false,
            };

            let err = create_group(config, &"foo".to_owned()).unwrap_err();

            assert!(err.to_string() == "Conflicting file named '.shvl' found");
        }
    }

    mod remove_group {
        use super::*;

        #[test]
        fn basic() {
            let (_temp_dir, config) = setup();

            create_group(config.clone(), &"foo".to_owned()).unwrap();

            let path = group_file(&config, "foo");
            assert!(path.exists());

            remove_group(config.clone(), &"foo".to_owned()).unwrap();

            assert!(!path.exists());
            assert!(utils::get_base_dir(config).is_dir());
        }

        #[test]
        fn prunes_empty_dirs() {
            let (_temp_dir, config) = setup();

            create_group(config.clone(), &"foo/bar/fizz".to_owned()).unwrap();
            create_group(config.clone(), &"foo/buzz".to_owned()).unwrap();

            remove_group(config.clone(), &"foo/bar/fizz".to_owned()).unwrap();

            let base_dir = utils::get_base_dir(config.clone());

            assert!(!base_dir.join("foo/bar").exists());
            assert!(base_dir.join("foo").is_dir());
            assert!(group_file(&config, "foo/buzz").is_file());
        }

        #[test]
        fn missing() {
            let (_temp_dir, config) = setup();

            let err = remove_group(config, &"foo".to_owned()).unwrap_err();

            assert!(err.to_string() == "Group does not exist");
        }
    }

    mod group_info {
        use super::*;

        #[test]
        fn basic() {
            let (_temp_dir, config) = setup();

            create_group(config.clone(), &"foo".to_owned()).unwrap();
            add_package(config.clone(), &"foo".to_owned(), &"ripgrep".to_owned()).unwrap();

            group_info(config.clone(), &"foo".to_owned()).unwrap();

            assert!(packages(&config, "foo") == vec!["ripgrep".to_owned()]);
        }

        #[test]
        fn missing() {
            let (_temp_dir, config) = setup();

            let err = group_info(config, &"foo".to_owned()).unwrap_err();

            assert!(err.to_string() == "Group does not exist");
        }
    }

    #[test]
    fn test_list_groups() {
        let (_temp_dir, config) = setup();

        list_groups(config.clone()).unwrap();

        create_group(config.clone(), &"foo".to_owned()).unwrap();
        create_group(config.clone(), &"bar/fizz".to_owned()).unwrap();

        list_groups(config.clone()).unwrap();

        let names = group::get_group_names(config).unwrap();

        assert!(names == vec!["bar/fizz".to_owned(), "foo".to_owned()]);
    }

    mod add_package {
        use super::*;

        #[test]
        fn basic() {
            let (_temp_dir, config) = setup();

            create_group(config.clone(), &"foo".to_owned()).unwrap();

            add_package(config.clone(), &"foo".to_owned(), &"ripgrep".to_owned()).unwrap();
            add_package(config.clone(), &"foo".to_owned(), &"fd".to_owned()).unwrap();

            assert!(packages(&config, "foo") == vec!["fd".to_owned(), "ripgrep".to_owned()]);
        }

        #[test]
        fn attr_path() {
            let (_temp_dir, config) = setup();

            create_group(config.clone(), &"foo".to_owned()).unwrap();

            add_package(
                config.clone(),
                &"foo".to_owned(),
                &"nodePackages.prettier".to_owned(),
            )
            .unwrap();

            assert!(packages(&config, "foo") == vec!["nodePackages.prettier".to_owned()]);
        }

        #[test]
        fn exists() {
            let (_temp_dir, config) = setup();

            create_group(config.clone(), &"foo".to_owned()).unwrap();
            add_package(config.clone(), &"foo".to_owned(), &"ripgrep".to_owned()).unwrap();

            let err =
                add_package(config.clone(), &"foo".to_owned(), &"ripgrep".to_owned()).unwrap_err();

            assert!(err.to_string() == "Package 'ripgrep' already exists in group 'foo'");

            assert!(packages(&config, "foo") == vec!["ripgrep".to_owned()]);
        }

        #[test]
        fn missing_group() {
            let (_temp_dir, config) = setup();

            assert!(add_package(config, &"foo".to_owned(), &"ripgrep".to_owned()).is_err());
        }
    }

    mod remove_package {
        use super::*;

        #[test]
        fn basic() {
            let (_temp_dir, config) = setup();

            create_group(config.clone(), &"foo".to_owned()).unwrap();
            add_package(config.clone(), &"foo".to_owned(), &"ripgrep".to_owned()).unwrap();
            add_package(config.clone(), &"foo".to_owned(), &"fd".to_owned()).unwrap();

            remove_package(config.clone(), &"foo".to_owned(), &"ripgrep".to_owned()).unwrap();

            assert!(packages(&config, "foo") == vec!["fd".to_owned()]);
        }

        #[test]
        fn last_package() {
            let (_temp_dir, config) = setup();

            create_group(config.clone(), &"foo".to_owned()).unwrap();
            add_package(config.clone(), &"foo".to_owned(), &"ripgrep".to_owned()).unwrap();

            remove_package(config.clone(), &"foo".to_owned(), &"ripgrep".to_owned()).unwrap();

            assert!(read_group(&config, "foo").packages.is_empty());
        }

        #[test]
        fn does_not_exists() {
            let (_temp_dir, config) = setup();

            create_group(config.clone(), &"foo".to_owned()).unwrap();
            add_package(config.clone(), &"foo".to_owned(), &"fd".to_owned()).unwrap();

            let err = remove_package(config.clone(), &"foo".to_owned(), &"ripgrep".to_owned())
                .unwrap_err();

            assert!(err.to_string() == "Package 'ripgrep' does not exist in group 'foo'");

            assert!(packages(&config, "foo") == vec!["fd".to_owned()]);
        }

        #[test]
        fn missing_group() {
            let (_temp_dir, config) = setup();

            assert!(remove_package(config, &"foo".to_owned(), &"ripgrep".to_owned()).is_err());
        }
    }
}
