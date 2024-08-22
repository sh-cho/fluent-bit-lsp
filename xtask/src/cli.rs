use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(about = "Build and package the language server and client")]
    Dist {
        #[arg(long)]
        client_patch_version: Option<String>,
    },

    #[command(about = "Generate the schema")]
    Generate {
        #[arg(long)]
        flb_version: Option<String>,
    },
}
