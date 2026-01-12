#![allow(unused)]

use std::path::{Path, PathBuf};

use issuecraft_core::Client;

use clap::Parser;
use issuecraft_ql::{ExecutionEngine, ExecutionResult};

use crate::{cli::Cli, config::Config};

mod cli;
mod config;
mod local;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Cli { database, query } = Cli::parse();

    let db_path = database.unwrap_or_else(|| Config::default().db_path);

    let db_path = format!("{}", db_path.display());
    let db_path = PathBuf::from(shellexpand::full(&db_path)?.to_string());
    if let Some(db_folder) = db_path.parent() {
        tokio::fs::create_dir_all(db_folder).await?;
    }

    let mut db = local::Database::new(&local::DatabaseType::File(db_path))?;
    println!("{}", run_query(&mut db, &query).await?);

    Ok(())
}

async fn run_query<T: ExecutionEngine>(
    engine: &mut T,
    query: &str,
) -> anyhow::Result<ExecutionResult> {
    Ok(engine.execute(query).await?)
}
