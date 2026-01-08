use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long, alias = "cfg")]
    pub config: Option<PathBuf>,
    pub command: Vec<String>,
}
