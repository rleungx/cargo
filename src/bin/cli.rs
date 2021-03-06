extern crate clap;

use clap::{AppSettings, Arg, ArgMatches};

use cargo::{self, CliResult, Config};

use super::list_commands;
use super::commands;
use command_prelude::*;

pub fn main(config: &mut Config) -> CliResult {
    let args = cli().get_matches_safe()?;
    let is_verbose = args.occurrences_of("verbose") > 0;
    if args.is_present("version") {
        let version = cargo::version();
        println!("{}", version);
        if is_verbose {
            println!(
                "release: {}.{}.{}",
                version.major, version.minor, version.patch
            );
            if let Some(ref cfg) = version.cfg_info {
                if let Some(ref ci) = cfg.commit_info {
                    println!("commit-hash: {}", ci.commit_hash);
                    println!("commit-date: {}", ci.commit_date);
                }
            }
        }
        return Ok(());
    }

    if let Some(ref code) = args.value_of("explain") {
        let mut procss = config.rustc()?.process();
        procss.arg("--explain").arg(code).exec()?;
        return Ok(());
    }

    if args.is_present("list") {
        println!("Installed Commands:");
        for command in list_commands(config) {
            let (command, path) = command;
            if is_verbose {
                match path {
                    Some(p) => println!("    {:<20} {}", command, p),
                    None => println!("    {:<20}", command),
                }
            } else {
                println!("    {}", command);
            }
        }
        return Ok(());
    }

    if args.subcommand_name().is_none() {}

    execute_subcommand(config, args)
}

fn execute_subcommand(config: &mut Config, args: ArgMatches) -> CliResult {
    config.configure(
        args.occurrences_of("verbose") as u32,
        if args.is_present("quiet") {
            Some(true)
        } else {
            None
        },
        &args.value_of("color").map(|s| s.to_string()),
        args.is_present("frozen"),
        args.is_present("locked"),
        &args.values_of_lossy("unstable-features")
            .unwrap_or_default(),
    )?;

    let (cmd, args) = match args.subcommand() {
        (cmd, Some(args)) => (cmd, args),
        _ => {
            cli().print_help()?;
            return Ok(());
        }
    };

    if let Some(exec) = commands::builtin_exec(cmd) {
        return exec(config, args);
    }

    if let Some(mut alias) = super::aliased_command(config, cmd)? {
        alias.extend(
            args.values_of("")
                .unwrap_or_default()
                .map(|s| s.to_string()),
        );
        let args = cli()
            .setting(AppSettings::NoBinaryName)
            .get_matches_from_safe(alias)?;
        return execute_subcommand(config, args);
    }
    let mut ext_args: Vec<&str> = vec![cmd];
    ext_args.extend(args.values_of("").unwrap_or_default());
    super::execute_external_subcommand(config, cmd, &ext_args)
}

fn cli() -> App {
    let app = App::new("cargo")
        .settings(&[
            AppSettings::UnifiedHelpMessage,
            AppSettings::DeriveDisplayOrder,
            AppSettings::VersionlessSubcommands,
            AppSettings::AllowExternalSubcommands,
        ])
        .about("")
        .template(
            "\
Rust's package manager

USAGE:
    {usage}

OPTIONS:
{unified}

Some common cargo commands are (see all commands with --list):
    build       Compile the current project
    check       Analyze the current project and report errors, but don't build object files
    clean       Remove the target directory
    doc         Build this project's and its dependencies' documentation
    new         Create a new cargo project
    init        Create a new cargo project in an existing directory
    run         Build and execute src/main.rs
    test        Run the tests
    bench       Run the benchmarks
    update      Update dependencies listed in Cargo.lock
    search      Search registry for crates
    publish     Package and upload this project to the registry
    install     Install a Rust binary
    uninstall   Uninstall a Rust binary

See 'cargo help <command>' for more information on a specific command.\n",
        )
        .arg(opt("version", "Print version info and exit").short("V"))
        .arg(opt("list", "List installed commands"))
        .arg(opt("explain", "Run `rustc --explain CODE`").value_name("CODE"))
        .arg(
            opt(
                "verbose",
                "Use verbose output (-vv very verbose/build.rs output)",
            ).short("v")
                .multiple(true)
                .global(true),
        )
        .arg(
            opt("quiet", "No output printed to stdout")
                .short("q")
                .global(true),
        )
        .arg(
            opt("color", "Coloring: auto, always, never")
                .value_name("WHEN")
                .global(true),
        )
        .arg(opt("frozen", "Require Cargo.lock and cache are up to date").global(true))
        .arg(opt("locked", "Require Cargo.lock is up to date").global(true))
        .arg(
            Arg::with_name("unstable-features")
                .help("Unstable (nightly-only) flags to Cargo")
                .short("Z")
                .value_name("FLAG")
                .multiple(true)
                .number_of_values(1)
                .global(true),
        )
        .subcommands(commands::builtin());
    app
}
