#![allow(unused)]

use std::path::Path;

use issuecraft_common::{Client, ProjectId, UserId};

use clap::Parser;

use crate::{cli::Cli, config::Config};

mod cli;
mod config;
mod local;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Cli { config, query } = Cli::parse();

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

    let mut db = local::Database::new(&local::DatabaseType::File(config.db_path.clone().into()))?;
    println!("{}", db.execute(&query).await?);

    println!("Config: {config:?}");

    Ok(())
}
