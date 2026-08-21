use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::config::Config;

#[derive(Clone, Debug)]
pub struct CommandContext {
    pub dir_flag: Option<String>,
    pub verbose_log: bool,
}

pub fn get_base_dir(config: Config) -> PathBuf {
    PathBuf::from(config.base_dir)
}

/// Removes now empty directories starting at `dir` and walking up towards
/// `base`. `base` itself is never removed.
pub fn prune_empty_dirs(base: &Path, dir: &Path) -> Result<()> {
    let mut current = dir.to_path_buf();

    while current.starts_with(base) && current != *base {
        let is_empty = fs::read_dir(&current)
            .with_context(|| format!("Unable to read dir: {}", current.display()))?
            .next()
            .is_none();

        if !is_empty {
            break;
        }

        fs::remove_dir(&current)
            .with_context(|| format!("Unable to remove empty dir: {}", current.display()))?;

        if !current.pop() {
            break;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod get_base_dir {
        use super::*;

        #[test]
        fn basic() {
            let temp_dir = tempfile::tempdir().unwrap();

            let config = Config {
                base_dir: temp_dir.path().to_str().unwrap().to_owned(),
                verbose: false,
            };

            let path = get_base_dir(config);

            assert!(path.exists());
            assert!(path.is_dir());
        }
    }

    mod prune_empty_dirs {
        use super::*;

        #[test]
        fn basic() {
            let base = tempfile::tempdir().unwrap();
            let mut path = base.path().to_path_buf();

            let dirs = ["foo", "bar", "fizz", "buzz"];

            for dir in dirs {
                path.push(dir);
                fs::create_dir(path.join(path.to_owned())).unwrap();
                assert!(path.exists());
                assert!(path.is_dir());
            }

            prune_empty_dirs(base.path(), path.as_path()).unwrap();

            for _ in 0..dirs.len() - 1 {
                path.pop();

                assert!(!path.exists());
            }
        }
    }
}
