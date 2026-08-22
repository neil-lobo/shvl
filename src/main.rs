use anyhow::{Context, Result, bail};
use clap_complete::{Shell, generate};
use dialoguer::{FuzzySelect, console::Term};
use std::{io, path::PathBuf, println};

use crate::{config::get_config, utils::CommandContext};

mod cli;
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

fn print_completion(shell: Shell) {
    let mut cmd = cli::get_cli();
    let name = cmd.get_name().to_string();

    generate(shell, &mut cmd, name, &mut io::stdout());
}

fn main() -> Result<()> {
    let matches = cli::get_cli().get_matches();

    if let Some(submatches) = matches.subcommand_matches("completion") {
        let shell = submatches.get_one::<Shell>("shell").expect("shell");
        print_completion(shell.to_owned());

        return Ok(());
    }

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
