use anyhow::{Context, Result, bail};
use rnix::ast::Expr;
use std::collections::BTreeSet;
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::config::Config;
use crate::utils::get_base_dir;

#[derive(Debug)]
pub struct Group {
    pub name: String,
    pub packages: BTreeSet<String>,
}

impl Group {
    pub fn deserialize(name: String, file_content: &String) -> Result<Self> {
        let parse = rnix::Root::parse(file_content);

        let root = parse
            .ok()
            .with_context(|| "Unable to parse nix file")?
            .expr()
            .with_context(|| "Unable to parse nix file")?;

        let Expr::Lambda(lambda) = root else {
            bail!("Root expr is not a lambda");
        };

        let _param = lambda.param().context("Lambda has no param expr")?;

        // TODO: ensure param is a single ident, and track its name

        let lambda_body = lambda.body().context("Lambda has no body expr")?;

        let Expr::Paren(paren) = lambda_body else {
            bail!("Lambda body does not start with a paren expr");
        };

        let paren_expr = paren.expr().context("Paren has no body expr")?;

        let Expr::With(with) = paren_expr else {
            bail!("Paren body expr does not start with a with expr");
        };

        // TODO: check that with ident matches the lambda param

        let with_body = with.body().context("With expr has no body expr")?;

        let Expr::List(list) = with_body else {
            bail!("With body expr is not a list expr");
        };

        let mut out = Group {
            name,
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
                    bail!("List expr contains a non ident or select expr");
                }
            }
        }

        Ok(out)
    }

    pub fn serialize(self) -> String {
        let mut out = String::new();

        out.push_str(
        "# This file is autogenerate by shvl. Do no modify unless you know what you are doing\n\n",
    );

        out.push_str("pkgs:\n");
        out.push_str("(\n");
        out.push_str("\twith pkgs;\n");
        out.push_str("\t[\n");

        for package in self.packages {
            out.push_str("\t\t");
            out.push_str(&package);
            out.push('\n');
        }

        out.push_str("\t]\n");
        out.push_str(")\n");

        out
    }

    pub fn default(name: String) -> Self {
        Group {
            name,
            packages: BTreeSet::new(),
        }
    }
}

/// Validates a group name. Group names are `/` separated paths relative to the
/// `.shvl` dir, where the final segment names the `.nix` file and any preceding
/// segments name subdirectories.
pub fn validate_group_name(group: &str) -> Result<()> {
    if group.is_empty() {
        bail!("Group name cannot be empty");
    }

    if group.starts_with('/') {
        bail!("Group name cannot start with '/'");
    }

    if group.ends_with('/') {
        bail!("Group name cannot end with '/'");
    }

    for segment in group.split('/') {
        if segment.is_empty() {
            bail!(format!("Group name '{group}' contains an empty segment"));
        }

        if segment == "." || segment == ".." {
            bail!(format!(
                "Group name '{group}' contains a '.' or '..' segment"
            ));
        }

        if segment.starts_with('.') {
            bail!(format!(
                "Group name segment '{segment}' cannot start with '.'"
            ));
        }

        if segment.contains('\\') {
            bail!(format!(
                "Group name segment '{segment}' cannot contain '\\'"
            ));
        }

        if segment.contains(|c: char| c.is_control()) {
            bail!(format!(
                "Group name segment '{segment}' cannot contain control characters"
            ));
        }
    }

    Ok(())
}

/// Resolves a group name to the path of its `.nix` file inside `base`.
pub fn group_path(base: &Path, group: &str) -> Result<PathBuf> {
    validate_group_name(group).context("Invalid group name")?;

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

pub fn get_group_names(config: Config) -> Result<Vec<String>> {
    let dir = get_base_dir(config);

    let mut out = collect_group_names(&dir, "")?;

    out.sort();

    Ok(out)
}

/// Recursively walks `dir`, returning every `.nix` file it finds as a `/`
/// separated group name relative to the `.shvl` root.
fn collect_group_names(dir: &Path, prefix: &str) -> Result<Vec<String>> {
    let mut out: Vec<String> = Vec::new();

    let entries = fs::read_dir(dir).with_context(|| "Unable to read shvl base dir")?;

    for entry in entries {
        let entry = entry.with_context(|| "Unable to read dir entry")?;

        let file_name = entry
            .file_name()
            .to_str()
            .with_context(|| "Unable to read file name")?
            .to_string();

        // skip hidden entries, they can never be valid group name segments
        if file_name.starts_with('.') {
            continue;
        }

        let file_type = entry
            .file_type()
            .with_context(|| "Unable to read file type")?;

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
