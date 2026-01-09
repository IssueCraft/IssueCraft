#![allow(unused)]

use std::path::Path;

use issuecraft_common::{Client, ProjectId, UserId};
use issuecraft_ql::parse;

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
        toml::from_str(&config)?
    } else {
        if default_config_path.exists() {
            let config = tokio::fs::read_to_string(default_config_path).await?;
            toml::from_str(&config)?
        } else {
            Config::default()
        }
    };

    let mut db = local::Database::new(&local::DatabaseType::File(config.db_path.clone().into()))?;
    db.execute(&query.join(" ")?).await?;
    match parse(&query.join(" "))? {
        issuecraft_ql::Statement::Create(create_statement) => todo!(),
        issuecraft_ql::Statement::Select(select_statement) => todo!(),
        issuecraft_ql::Statement::Update(update_statement) => todo!(),
        issuecraft_ql::Statement::Delete(delete_statement) => todo!(),
        issuecraft_ql::Statement::Assign(assign_statement) => todo!(),
        issuecraft_ql::Statement::Close(close_statement) => todo!(),
        issuecraft_ql::Statement::Comment(comment_statement) => todo!(),
    }

    println!("Config: {config:?}");

    Ok(())
}
