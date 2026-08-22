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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn group_from(nix: &str) -> Result<Group> {
        Group::deserialize("test".to_owned(), &nix.to_owned())
    }

    fn packages_of(nix: &str) -> Vec<String> {
        group_from(nix).unwrap().packages.into_iter().collect()
    }

    fn group_of(name: &str, packages: &[&str]) -> Group {
        Group {
            name: name.to_owned(),
            packages: packages.iter().map(|p| (*p).to_owned()).collect(),
        }
    }

    fn setup() -> (TempDir, Config) {
        let temp_dir = tempfile::tempdir().unwrap();

        let config = Config {
            base_dir: temp_dir.path().to_str().unwrap().to_owned(),
            verbose: false,
        };

        (temp_dir, config)
    }

    fn touch(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }

        fs::write(path, "").unwrap();
    }

    mod deserialize {
        use super::*;

        #[test]
        fn basic() {
            let packages =
                packages_of("pkgs:\n(\n\twith pkgs;\n\t[\n\t\tripgrep\n\t\tfd\n\t]\n)\n");

            assert!(packages == vec!["fd".to_owned(), "ripgrep".to_owned()]);
        }

        #[test]
        fn name_is_taken_from_argument() {
            let group = group_from("pkgs:\n(\n\twith pkgs;\n\t[\n\t]\n)\n").unwrap();

            assert!(group.name == "test");
        }

        #[test]
        fn empty_list() {
            let group = group_from("pkgs:\n(\n\twith pkgs;\n\t[\n\t]\n)\n").unwrap();

            assert!(group.packages.is_empty());
        }

        #[test]
        fn select_expr() {
            let packages =
                packages_of("pkgs:\n(\n\twith pkgs;\n\t[\n\t\tnodePackages.prettier\n\t]\n)\n");

            assert!(packages == vec!["nodePackages.prettier".to_owned()]);
        }

        #[test]
        fn nested_select_expr() {
            let packages = packages_of("pkgs:\n(\n\twith pkgs;\n\t[\n\t\tfoo.bar.baz\n\t]\n)\n");

            assert!(packages == vec!["foo.bar.baz".to_owned()]);
        }

        #[test]
        fn duplicates_collapse() {
            let packages = packages_of("pkgs:\n(\n\twith pkgs;\n\t[\n\t\tfoo\n\t\tfoo\n\t]\n)\n");

            assert!(packages == vec!["foo".to_owned()]);
        }

        #[test]
        fn ignores_comments_and_layout() {
            let packages =
                packages_of("# a comment\npkgs: ( with pkgs; [ ripgrep /* inline */ fd ] )\n");

            assert!(packages == vec!["fd".to_owned(), "ripgrep".to_owned()]);
        }

        #[test]
        fn param_name_need_not_be_pkgs() {
            let packages = packages_of("p:\n(\n\twith p;\n\t[\n\t\tfoo\n\t]\n)\n");

            assert!(packages == vec!["foo".to_owned()]);
        }

        #[test]
        fn empty_file() {
            let err = group_from("").unwrap_err();

            assert!(err.to_string() == "Unable to parse nix file");
        }

        #[test]
        fn invalid_syntax() {
            let err = group_from("pkgs: ( with pkgs; [").unwrap_err();

            assert!(err.to_string() == "Unable to parse nix file");
        }

        #[test]
        fn root_not_lambda() {
            let err = group_from("[ foo ]").unwrap_err();

            assert!(err.to_string() == "Root expr is not a lambda");
        }

        #[test]
        fn body_not_paren() {
            let err = group_from("pkgs: [ foo ]").unwrap_err();

            assert!(err.to_string() == "Lambda body does not start with a paren expr");
        }

        #[test]
        fn paren_not_with() {
            let err = group_from("pkgs: (\n[ foo ]\n)").unwrap_err();

            assert!(err.to_string() == "Paren body expr does not start with a with expr");
        }

        #[test]
        fn with_body_not_list() {
            let err = group_from("pkgs: (\nwith pkgs;\nfoo\n)").unwrap_err();

            assert!(err.to_string() == "With body expr is not a list expr");
        }

        #[test]
        fn list_item_not_ident_or_select() {
            for src in [
                "pkgs: (\nwith pkgs;\n[ 123 ]\n)",
                "pkgs: (\nwith pkgs;\n[ \"foo\" ]\n)",
            ] {
                let err = group_from(src).unwrap_err();

                assert!(err.to_string() == "List expr contains a non ident or select expr");
            }
        }
    }

    mod serialize {
        use super::*;

        #[test]
        fn basic() {
            let nix = group_of("test", &["ripgrep", "fd"]).serialize();

            assert!(nix.starts_with("# This file is autogenerate by shvl."));
            assert!(nix.contains("pkgs:\n"));
            assert!(nix.contains("\twith pkgs;\n"));
            assert!(nix.contains("\t\tfd\n\t\tripgrep\n"));
        }

        #[test]
        fn empty_group() {
            let nix = Group::default("test".to_owned()).serialize();

            assert!(nix.contains("\t[\n\t]\n"));
        }

        #[test]
        fn name_is_not_written_to_file() {
            let nix = group_of("some-unique-group-name", &[]).serialize();

            assert!(!nix.contains("some-unique-group-name"));
        }

        #[test]
        fn round_trip() {
            let packages = ["ripgrep", "fd", "nodePackages.prettier", "foo-bar", "_x'y"];

            let nix = group_of("test", &packages).serialize();

            let group = Group::deserialize("test".to_owned(), &nix).unwrap();

            let mut expected: Vec<String> = packages.iter().map(|p| (*p).to_owned()).collect();
            expected.sort();

            assert!(group.packages.into_iter().collect::<Vec<_>>() == expected);
        }

        #[test]
        fn round_trip_empty() {
            let nix = Group::default("test".to_owned()).serialize();

            let group = Group::deserialize("test".to_owned(), &nix).unwrap();

            assert!(group.packages.is_empty());
        }

        #[test]
        fn round_trip_is_lossy_for_non_ident_names() {
            let nix = group_of("test", &["foo bar"]).serialize();
            let group = Group::deserialize("test".to_owned(), &nix).unwrap();
            assert!(group.packages.into_iter().collect::<Vec<_>>() == vec!["bar", "foo"]);

            let nix = group_of("test", &[""]).serialize();
            assert!(
                Group::deserialize("test".to_owned(), &nix)
                    .unwrap()
                    .packages
                    .is_empty()
            );

            let nix = group_of("test", &["foo;"]).serialize();
            assert!(Group::deserialize("test".to_owned(), &nix).is_err());

            let nix = group_of("test", &["1abc"]).serialize();
            assert!(Group::deserialize("test".to_owned(), &nix).is_err());
        }
    }

    mod validate_group_name {
        use super::*;

        #[test]
        fn valid_names() {
            for name in [
                "foo",
                "foo/bar",
                "foo/bar/fizz",
                "foo-bar",
                "foo_bar",
                "foo.bar",
                "a",
                "foo..bar",
            ] {
                assert!(validate_group_name(name).is_ok(), "expected ok: {name}");
            }
        }

        #[test]
        fn empty() {
            let err = validate_group_name("").unwrap_err();

            assert!(err.to_string() == "Group name cannot be empty");
        }

        #[test]
        fn leading_slash() {
            let err = validate_group_name("/foo").unwrap_err();

            assert!(err.to_string() == "Group name cannot start with '/'");
        }

        #[test]
        fn trailing_slash() {
            let err = validate_group_name("foo/").unwrap_err();

            assert!(err.to_string() == "Group name cannot end with '/'");
        }

        #[test]
        fn empty_segment() {
            let err = validate_group_name("foo//bar").unwrap_err();

            assert!(err.to_string() == "Group name 'foo//bar' contains an empty segment");
        }

        #[test]
        fn dot_segments() {
            for name in [
                ".",
                "..",
                "../foo",
                "foo/..",
                "foo/../bar",
                "./foo",
                "foo/.",
            ] {
                assert!(validate_group_name(name).is_err(), "expected err: {name}");
            }
        }

        #[test]
        fn hidden_segment() {
            let err = validate_group_name("foo/.bar").unwrap_err();

            assert!(err.to_string() == "Group name segment '.bar' cannot start with '.'");
        }

        #[test]
        fn backslash() {
            let err = validate_group_name("foo\\bar").unwrap_err();

            assert!(err.to_string() == "Group name segment 'foo\\bar' cannot contain '\\'");
        }

        #[test]
        fn control_characters() {
            for name in ["foo\nbar", "foo\tbar", "foo\0bar"] {
                assert!(validate_group_name(name).is_err(), "expected err: {name:?}");
            }
        }
    }

    mod group_path {
        use super::*;

        #[test]
        fn basic() {
            let path = group_path(Path::new("/base"), "foo").unwrap();

            assert!(path == PathBuf::from("/base/foo.nix"));
        }

        #[test]
        fn nested() {
            let path = group_path(Path::new("/base"), "foo/bar/fizz").unwrap();

            assert!(path == PathBuf::from("/base/foo/bar/fizz.nix"));
        }

        #[test]
        fn invalid_name_is_rejected() {
            for name in ["", "/foo", "foo/", "foo//bar", "../foo", ".foo"] {
                assert!(
                    group_path(Path::new("/base"), name).is_err(),
                    "expected err: {name}"
                );
            }
        }

        #[test]
        fn stays_within_base() {
            let path = group_path(Path::new("/base"), "foo/bar").unwrap();

            assert!(path.starts_with("/base"));
        }
    }

    mod get_group_names {
        use super::*;

        #[test]
        fn empty_dir() {
            let (_temp_dir, config) = setup();

            assert!(get_group_names(config).unwrap().is_empty());
        }

        #[test]
        fn sorted_and_nested() {
            let (temp_dir, config) = setup();
            let base = temp_dir.path();

            touch(&base.join("foo.nix"));
            touch(&base.join("bar.nix"));
            touch(&base.join("nested/fizz.nix"));
            touch(&base.join("nested/deeper/buzz.nix"));

            let names = get_group_names(config).unwrap();

            assert!(
                names
                    == vec![
                        "bar".to_owned(),
                        "foo".to_owned(),
                        "nested/deeper/buzz".to_owned(),
                        "nested/fizz".to_owned(),
                    ]
            );
        }

        #[test]
        fn names_round_trip_through_group_path() {
            let (temp_dir, config) = setup();
            let base = temp_dir.path();

            touch(&base.join("foo.nix"));
            touch(&base.join("nested/deeper/buzz.nix"));

            for name in get_group_names(config.clone()).unwrap() {
                let path = group_path(base, &name).unwrap();

                assert!(path.is_file(), "expected file for group: {name}");
            }
        }

        #[test]
        fn skips_non_nix_files() {
            let (temp_dir, config) = setup();
            let base = temp_dir.path();

            touch(&base.join("foo.nix"));
            touch(&base.join("README.md"));
            touch(&base.join("notnix"));
            touch(&base.join("foo.nix.bak"));

            assert!(get_group_names(config).unwrap() == vec!["foo".to_owned()]);
        }

        #[test]
        fn skips_hidden_entries() {
            let (temp_dir, config) = setup();
            let base = temp_dir.path();

            touch(&base.join("foo.nix"));
            touch(&base.join(".hidden.nix"));
            touch(&base.join(".git/config.nix"));

            assert!(get_group_names(config).unwrap() == vec!["foo".to_owned()]);
        }

        #[test]
        fn skips_bare_nix_suffix_file() {
            let (temp_dir, config) = setup();

            touch(&temp_dir.path().join(".nix"));

            assert!(get_group_names(config).unwrap().is_empty());
        }

        #[test]
        fn empty_subdirs_contribute_nothing() {
            let (temp_dir, config) = setup();

            fs::create_dir_all(temp_dir.path().join("empty/deeper")).unwrap();

            assert!(get_group_names(config).unwrap().is_empty());
        }

        #[test]
        fn missing_base_dir_errors() {
            let temp_dir = tempfile::tempdir().unwrap();

            let config = Config {
                base_dir: temp_dir
                    .path()
                    .join("does-not-exist")
                    .to_str()
                    .unwrap()
                    .to_owned(),
                verbose: false,
            };

            let err = get_group_names(config).unwrap_err();

            assert!(err.to_string() == "Unable to read shvl base dir");
        }
    }
}
