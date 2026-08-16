use rnix::ast::Expr;
use std::collections::BTreeSet;
use std::println;
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::config::Config;

#[derive(Debug)]
pub struct Group {
    pub name: String,
    pub packages: BTreeSet<String>,
}

#[derive(Clone, Debug)]
pub struct CommandContext {
    pub dir_flag: Option<String>,
    pub verbose_log: bool,
}

/// Validates a group name. Group names are `/` separated paths relative to the
/// `.shvl` dir, where the final segment names the `.nix` file and any preceding
/// segments name subdirectories.
pub fn validate_group_name(group: &str) -> Result<(), String> {
    if group.is_empty() {
        return Err("group name cannot be empty".to_owned());
    }

    if group.starts_with('/') {
        return Err("group name cannot start with '/'".to_owned());
    }

    if group.ends_with('/') {
        return Err("group name cannot end with '/'".to_owned());
    }

    for segment in group.split('/') {
        if segment.is_empty() {
            return Err(format!("group name '{group}' contains an empty segment"));
        }

        if segment == "." || segment == ".." {
            return Err(format!(
                "group name '{group}' contains a '.' or '..' segment"
            ));
        }

        if segment.starts_with('.') {
            return Err(format!(
                "group name segment '{segment}' cannot start with '.'"
            ));
        }

        if segment.contains('\\') {
            return Err(format!(
                "group name segment '{segment}' cannot contain '\\'"
            ));
        }

        if segment.contains(|c: char| c.is_control()) {
            return Err(format!(
                "group name segment '{segment}' cannot contain control characters"
            ));
        }
    }

    Ok(())
}

/// Resolves a group name to the path of its `.nix` file inside `base`.
pub fn group_path(base: &Path, group: &str) -> Result<PathBuf, String> {
    validate_group_name(group)?;

    let mut path = base.to_path_buf();

    let mut segments = group.split('/').peekable();
    while let Some(segment) = segments.next() {
        if segments.peek().is_none() {
            path.push(format!("{segment}.nix"));
        } else {
            path.push(segment);
        }
    }

    Ok(path)
}

pub fn get_base_dir(config: Config) -> Result<PathBuf, String> {
    Ok(PathBuf::from(config.base_dir))
}

// TODO: move to group file? rename to deserialize?
pub fn parse_group_file(group: &String, file_content: &String) -> Result<Group, String> {
    let parse = rnix::Root::parse(file_content);

    let root = parse
        .ok()
        .or(Err("Error parsing nix file"))?
        .expr()
        .ok_or("Error parsing nix file")?;

    let lambda = match root {
        Expr::Lambda(l) => Ok(l),
        _ => Err("not a lambda"),
    }?;

    let _param = lambda.param().ok_or("lambda has no param")?;

    // TODO: ensure param is a single ident, and track its name

    let lambda_body = lambda.body().ok_or("lambda has no body")?;

    let paren = match lambda_body {
        Expr::Paren(p) => Ok(p),
        _ => Err("not a paren"),
    }?;

    let paren_expr = paren.expr().ok_or("paren has no body expr")?;

    let with = match paren_expr {
        Expr::With(w) => Ok(w),
        _ => Err("not a with"),
    }?;

    // TODO: check that with ident matches the lambda param

    let with_body = with.body().ok_or("with has no body expr")?;

    let list = match with_body {
        Expr::List(l) => Ok(l),
        _ => Err("with body is not a list"),
    }?;

    let mut out = Group {
        name: group.to_owned(),
        packages: BTreeSet::new(),
    };

    for item in list.items() {
        match item {
            Expr::Ident(i) => {
                let ident_token = i.ident_token().unwrap();

                let text = ident_token.text();

                out.packages.insert(text.to_owned());
            }
            Expr::Select(s) => {
                let expr = s.expr().unwrap().to_string();
                let attr_path = s.attrpath().unwrap().to_string();

                out.packages.insert(format!("{}.{}", expr, attr_path));
            }
            _ => {
                println!("not ident or select");
            }
        }
    }

    Ok(out)
}

// TODO: same comment as `parse_group_file`
// TODO: tabs vs 4 spaces for indents? (configurable)
pub fn stringify_group(group: Group) -> Result<String, String> {
    let mut out = String::new();

    out.push_str(
        "# This file is autogenerate by shvl. Do no modify unless you know what you are doing\n\n",
    );

    out.push_str("pkgs:\n");
    out.push_str("(\n");
    out.push_str("\twith pkgs;\n");
    out.push_str("\t[\n");

    for package in group.packages {
        out.push_str("\t\t");
        out.push_str(&package);
        out.push('\n');
    }

    out.push_str("\t]\n");
    out.push_str(")\n");

    Ok(out)
}

// TODO: move to Group impl of Default trait?
pub fn default_group(name: String) -> Group {
    Group {
        name,
        packages: BTreeSet::new(),
    }
}

pub fn get_group_names(config: Config) -> Result<Vec<String>, String> {
    let dir = get_base_dir(config)?;

    if !dir.is_dir() {
        return Err("no groups found".to_string());
    }

    let mut out = collect_group_names(&dir, "")?;

    out.sort();

    Ok(out)
}

/// Recursively walks `dir`, returning every `.nix` file it finds as a `/`
/// separated group name relative to the `.shvl` root.
fn collect_group_names(dir: &Path, prefix: &str) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();

    let entries = fs::read_dir(dir).or(Err("unable to read .shvl dir".to_string()))?;

    for entry in entries {
        let entry = entry.or(Err("unable to read dir entry"))?;

        let file_name = entry
            .file_name()
            .to_str()
            .ok_or("unable to read file name".to_string())?
            .to_string();

        // skip hidden entries, they can never be valid group name segments
        if file_name.starts_with('.') {
            continue;
        }

        let file_type = entry.file_type().or(Err("unable to read file type"))?;

        if file_type.is_dir() {
            let nested_prefix = if prefix.is_empty() {
                file_name
            } else {
                format!("{prefix}/{file_name}").to_owned()
            };

            out.extend(collect_group_names(&entry.path(), &nested_prefix)?);
            continue;
        }

        let Some(stem) = file_name.strip_suffix(".nix") else {
            continue;
        };

        if stem.is_empty() {
            continue;
        }

        if prefix.is_empty() {
            out.push(stem.to_owned());
        } else {
            out.push(format!("{prefix}/{stem}"));
        }
    }

    Ok(out)
}

/// Removes now empty directories starting at `dir` and walking up towards
/// `base`. `base` itself is never removed.
pub fn prune_empty_dirs(base: &Path, dir: &Path) -> Result<(), String> {
    let mut current = dir.to_path_buf();

    while current.starts_with(base) && current != *base {
        let is_empty = fs::read_dir(&current)
            .or(Err(format!("unable to read dir: {}", current.display())))?
            .next()
            .is_none();

        if !is_empty {
            break;
        }

        fs::remove_dir(&current).or(Err(format!(
            "unable to remove empty dir: {}",
            current.display()
        )))?;

        if !current.pop() {
            break;
        }
    }

    Ok(())
}
