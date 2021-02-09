// Max clippy pedanticness.
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
    missing_debug_implementations
)]
#![allow(
    clippy::module_name_repetitions,
    clippy::implicit_return,
    clippy::missing_inline_in_public_items,
    clippy::missing_docs_in_private_items,
    clippy::missing_errors_doc
)]
use anyhow::Result;
use args::GenerateLib;
use log::trace;
use update::update_self::update_self;

use crate::{
    args::{Args, SubCommand},
    config::UpConfig,
    tasks::{git, link::LinkConfig},
};

pub mod args;
mod config;
mod env;
mod generate;
pub mod tasks;
pub mod update;

/// Run `up_rs` with provided [Args][] struct.
///
/// # Errors
///
/// Errors if the relevant subcommand fails.
///
/// # Panics
///
/// Panics for unimplemented commands.
///
/// [Args]: crate::args::Args
pub fn run(args: Args) -> Result<()> {
    match args.cmd {
        // TODO(gib): Handle multiple link directories both as args and in config.
        // TODO(gib): Add option to warn instead of failing if there are conflicts.
        // TODO(gib): Check for conflicts before doing any linking.
        Some(SubCommand::Link {
            from_dir,
            to_dir,
            backup_dir,
        }) => {
            // Expand ~, this is only used for the default options, if the user passes them
            // as explicit args then they will be expanded by the shell.
            tasks::link::run(LinkConfig {
                from_dir: shellexpand::tilde(&from_dir).into_owned(),
                to_dir: shellexpand::tilde(&to_dir).into_owned(),
                backup_dir: shellexpand::tilde(&backup_dir).into_owned(),
            })?;
        }
        Some(SubCommand::Git(git_options)) => {
            git::update::update(&git_options.into())?;
        }
        Some(SubCommand::Defaults {}) => {
            // TODO(gib): implement defaults setting.
            unimplemented!("Not yet implemented.");
        }
        Some(SubCommand::Self_(opts)) => {
            update_self(&opts)?;
        }
        Some(SubCommand::Generate(ref opts)) => match opts.lib {
            Some(GenerateLib::Git(ref git_opts)) => {
                generate::git::run_single(git_opts)?;
            }
            Some(GenerateLib::Defaults(ref defaults_opts)) => {
                trace!("Options: {:?}", defaults_opts);
                // TODO(gib): implement defaults generation.
                unimplemented!("Allow generating defaults toml.");
            }
            None => {
                let config = UpConfig::from(args)?;
                generate::run(&config)?;
            }
        },
        Some(SubCommand::Run(ref _opts)) => {
            // TODO(gib): Store and fetch config in config module.
            let config = UpConfig::from(args)?;
            update::update(&config)?;
        }
        None => {
            // TODO(gib): Store and fetch config in config module.
            let config = UpConfig::from(args)?;
            update::update(&config)?;
        }
    }
    Ok(())
}
