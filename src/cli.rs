use clap::{Arg, ArgAction, Command, builder::EnumValueParser};
use clap_complete::Shell;

pub fn get_cli() -> Command {
    let group_arg = Arg::new("group")
        .short('g')
        .long("group")
        .help("Group name");
    let package_arg = Arg::new("package").help("Package name").required(true);

    Command::new("shvl")
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
        .subcommand(
            Command::new("completion")
                .about("Print a shell completion script to stdout")
                .arg(
                    Arg::new("shell")
                        .help("Shell to generate a completion script for")
                        .required(true)
                        .value_parser(EnumValueParser::<Shell>::new()),
                ),
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
}
