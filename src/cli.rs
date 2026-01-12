use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long, alias = "db", env = "ISSUECRAFT_DB")]
    pub database: Option<PathBuf>,
    pub query: String,
}
