use std::println;

use clap::{Arg, Command};
use dialoguer::{FuzzySelect, console::Term};

mod commands;
mod utils;

/// dialoguer hides the terminal cursor while a prompt is on screen and only
/// restores it on the paths that return normally. On Ctrl+C the `console` crate
/// re-raises SIGINT at itself, which by default kills us before the cursor is
/// ever restored, leaving the user's terminal without a cursor.
///
/// Installing a SIGINT handler lets us show the cursor before exiting. ctrlc
/// runs the handler on its own thread rather than in a real signal context, so
/// calling into `console` here is safe.
fn restore_cursor_on_sigint() {
    let result = ctrlc::set_handler(|| {
        let _ = Term::stderr().show_cursor();
        let _ = Term::stdout().show_cursor();

        // 130 is the conventional shell exit code for "terminated by SIGINT".
        std::process::exit(130);
    });

    // A missing handler only costs us a stray cursor, so don't fail the command
    // over it.
    if result.is_err() {
        eprintln!("warning: unable to install SIGINT handler");
    }
}

/// Prompts the user to pick a group from `group_names`.
fn select_group(group_names: &[String]) -> Result<&String, String> {
    restore_cursor_on_sigint();

    let selection = FuzzySelect::new()
        .with_prompt("Type to filter groups")
        .items(group_names)
        .interact()
        .or(Err("unable to create fuzzy menu"))?;

    group_names
        .get(selection)
        .ok_or("invalid group selection".to_owned())
}

fn main() -> Result<(), String> {
    let group_arg = Arg::new("group").short('g').long("group");
    let package_arg = Arg::new("package")
        .help("The name of the package to add")
        .required(true);

    let matches = Command::new("np")
        .disable_help_subcommand(true)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .about("A package management tool")
        .subcommand(
            Command::new("add")
                .about("Add a package to a group")
                .arg(package_arg.clone())
                .arg(group_arg.clone()),
        )
        .subcommand(
            Command::new("remove")
                .alias("rm")
                .about("Remove a package from a group")
                .arg(package_arg.clone())
                .arg(group_arg.clone()),
        )
        .subcommand(
            Command::new("group")
                .disable_help_subcommand(true)
                .subcommand_required(true)
                .arg_required_else_help(true)
                .subcommand(Command::new("create").arg(Arg::new("group").required(true)))
                .subcommand(Command::new("info").arg(Arg::new("group").required(true)))
                .subcommand(Command::new("remove").arg(Arg::new("group").required(true)))
                .subcommand(Command::new("list")),
        )
        .arg(Arg::new("dir").short('d').long("dir"))
        .get_matches();

    let dir = matches.get_one::<String>("dir");

    if matches.subcommand().is_none() {
        return Err("Unreachable".to_owned());
    }

    let (name, submatches) = matches.subcommand().unwrap();

    match name {
        "add" => {
            let package = submatches
                .get_one::<String>("package")
                .expect("package name");
            let group_names = utils::get_group_names(dir.clone())?;

            let group = if let Some(group) = submatches.get_one::<String>("group") {
                group
            } else {
                select_group(&group_names)?
            };

            println!("add -g {group} {package}");
            commands::add_package(dir.clone(), group, package)
        }
        "remove" => {
            let package = submatches
                .get_one::<String>("package")
                .expect("pacakge name");
            let group_names = utils::get_group_names(dir.clone())?;

            let group = if let Some(group) = submatches.get_one::<String>("group") {
                group
            } else {
                select_group(&group_names)?
            };

            println!("remove -g {group} {package}");
            commands::remove_package(dir.clone(), group, package)
        }
        "group" => {
            if submatches.subcommand().is_none() {
                return Err("Unreachale".to_owned());
            }

            let (name, submatches) = submatches.subcommand().unwrap();

            match name {
                "create" => {
                    let group = submatches.get_one::<String>("group").unwrap();

                    commands::create_group(dir.clone(), group)
                }
                "info" => {
                    let group = submatches.get_one::<String>("group").unwrap();
                    commands::group_info(dir.clone(), group)
                }
                "remove" => {
                    let group = submatches.get_one::<String>("group").unwrap();
                    commands::remove_group(dir.clone(), group)
                }
                "list" => commands::list_groups(dir.clone()),
                _ => Err("Unreachable".to_owned()),
            }?;

            Ok(())
        }
        _ => Err("Unreachable".to_owned()),
    }?;

    Ok(())
}
