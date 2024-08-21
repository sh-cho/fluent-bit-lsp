use std::{env, path::PathBuf};

use clap::Parser;
use xshell::Shell;

use crate::cli::{Cli, Commands};

mod cli;
mod codegen;
mod dist;
mod schema;

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    let sh = &Shell::new()?;
    sh.change_dir(project_root());

    match args.command {
        Commands::Dist {
            client_patch_version,
        } => dist::run_dist(sh, client_patch_version),
        Commands::Generate { flb_version } => schema::generate(sh, flb_version),
    }
}

/// Returns the path to the root directory of project.
fn project_root() -> PathBuf {
    let dir =
        env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| env!("CARGO_MANIFEST_DIR").to_owned());
    PathBuf::from(dir).parent().unwrap().to_owned()
}
