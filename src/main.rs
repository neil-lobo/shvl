use anyhow::{Context, Result, bail};
use clap::{Arg, ArgAction, Command};
use dialoguer::{FuzzySelect, console::Term};
use std::{path::PathBuf, println};

use crate::{config::get_config, utils::CommandContext};

mod commands;
mod config;
mod group;
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
fn select_group(group_names: &[String]) -> Result<&String> {
    restore_cursor_on_sigint();

    let selection = FuzzySelect::new()
        .with_prompt("Type to filter groups")
        .items(group_names)
        .interact()
        .with_context(|| "Unable to create fuzzy menu")?;

    group_names
        .get(selection)
        .with_context(|| "Invalid group selection")
}

fn main() -> Result<()> {
    let group_arg = Arg::new("group")
        .short('g')
        .long("group")
        .help("Group name");
    let package_arg = Arg::new("package").help("Package name").required(true);

    let matches = Command::new("shvl")
        .disable_help_subcommand(true)
        .subcommand_required(true)
        .arg_required_else_help(true)
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
                .about("Group commands")
                .disable_help_subcommand(true)
                .subcommand_required(true)
                .arg_required_else_help(true)
                .subcommand(
                    Command::new("create")
                        .about("Create a group")
                        .arg(Arg::new("group").help("Group name").required(true)),
                )
                .subcommand(
                    Command::new("info")
                        .about("Get a info about a group")
                        .arg(Arg::new("group").help("Group name").required(true)),
                )
                .subcommand(
                    Command::new("remove")
                        .about("Remove a group")
                        .arg(Arg::new("group").help("Group name").required(true)),
                )
                .subcommand(Command::new("list").about("List groups")),
        )
        .arg(Arg::new("dir").help("Set base dir").short('d').long("dir"))
        .arg(
            Arg::new("config_path")
                .help("Set config file path")
                .long("config"),
        )
        .arg(
            Arg::new("verbose")
                .help("Enable verbose logs")
                .short('v')
                .long("verbose")
                .action(ArgAction::SetTrue),
        )
        .get_matches();

    let base_dir = matches.get_one::<String>("dir").cloned();
    let config_path = matches.get_one::<String>("config").cloned();
    let verbose_log = if matches.get_flag("verbose") == false {
        None
    } else {
        Some(true)
    };

    let ctx: CommandContext = CommandContext {
        base_dir,
        config_path,
        verbose_log,
    };

    if ctx.verbose_log == Some(true) {
        println!("[DEBUG] command context: {ctx:?}");
    }

    let config = get_config(ctx);

    if !PathBuf::from(&config.base_dir).is_dir() {
        bail!(format!("{} is not a dir", config.base_dir))
    }

    if config.verbose {
        println!("[DEBUG] final merged config: {config:?}")
    }

    let (name, submatches) = matches
        .subcommand()
        .with_context(|| "Error getting submatches")?;

    match name {
        "add" => {
            let package = submatches
                .get_one::<String>("package")
                .expect("package name");
            let group_names = group::get_group_names(config.clone())?;

            let group = if let Some(group) = submatches.get_one::<String>("group") {
                group
            } else {
                select_group(&group_names)?
            };

            commands::add_package(config, group, package)?;
            println!("Package '{package}' added to group '{group}'");

            Ok(())
        }
        "remove" => {
            let package = submatches
                .get_one::<String>("package")
                .expect("pacakge name");
            let group_names = group::get_group_names(config.clone())?;

            let group = if let Some(group) = submatches.get_one::<String>("group") {
                group
            } else {
                select_group(&group_names)?
            };

            commands::remove_package(config, group, package)?;
            println!("Package '{package}' removed from group '{group}'");

            Ok(())
        }
        "group" => {
            let (name, submatches) = submatches
                .subcommand()
                .with_context(|| "Error getting submatches")?;

            let group_res: Result<()> = match name {
                "create" => {
                    let group = submatches.get_one::<String>("group").unwrap();

                    commands::create_group(config.clone(), group)?;
                    println!("Group '{group}' created");
                    Ok(())
                }
                "info" => {
                    let group = submatches.get_one::<String>("group").unwrap();
                    commands::group_info(config.clone(), group)
                }
                "remove" => {
                    let group = submatches.get_one::<String>("group").unwrap();
                    commands::remove_group(config.clone(), group)?;
                    println!("Group '{group}' removed");
                    Ok(())
                }
                "list" => commands::list_groups(config.clone()),
                _ => bail!("Invalid group command"),
            };

            group_res
        }
        _ => bail!("Invalid command"),
    }
}
