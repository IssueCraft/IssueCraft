#![allow(unused)]

use std::path::{Path, PathBuf};

use issuecraft_core::Client;

use clap::Parser;
use issuecraft_ql::{ExecutionEngine, ExecutionResult};

use crate::{
    cli::{Cli, Command},
    config::Config,
};

mod cli;
mod config;
mod local;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Cli { config, command } = Cli::parse();

    let default_config_path = Path::new(".ic.toml");

    let config = if let Some(config_path) = config {
        let config = tokio::fs::read_to_string(config_path).await?;
        facet_toml::from_str(&config)?
    } else {
        if default_config_path.exists() {
            let config = tokio::fs::read_to_string(default_config_path).await?;
            facet_toml::from_str(&config)?
        } else {
            Config::default()
        }
    };

    match command {
        Command::Query { query } => println!("{}", run_query(&config, &query).await?),
    }

    Ok(())
}

async fn run_query(config: &Config, query: &str) -> anyhow::Result<ExecutionResult> {
    let db_path = format!("{}", config.db_path.display());
    let db_path = PathBuf::from(shellexpand::full(&db_path)?.to_string());
    if let Some(db_folder) = db_path.parent() {
        tokio::fs::create_dir_all(db_folder).await?;
    }
    let mut db = local::Database::new(&local::DatabaseType::File(db_path))?;
    Ok(db.execute(query).await?)
}
