#![allow(unused)]

use std::path::{Path, PathBuf};

use clap::Parser;
use issuecraft_core::{Client, ExecutionEngine, ExecutionResult, UserProvider};
use issuecraft_ql::UserId;

use crate::{cli::Cli, config::Config};

mod cli;
mod config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Cli { database, query } = Cli::parse();

    let db_path = database.unwrap_or_else(|| Config::default().db_path);

    let db_path = format!("{}", db_path.display());
    let db_path = PathBuf::from(shellexpand::full(&db_path)?.to_string());
    if let Some(db_folder) = db_path.parent() {
        tokio::fs::create_dir_all(db_folder).await?;
    }

    let user_provider = issuecraft_core::SingleUserProvider::new(UserId::new("default"));
    let mut db = issuecraft_redb::Database::new(&issuecraft_redb::DatabaseType::File(db_path))?;
    println!("{}", run_query(&user_provider, &mut db, &query).await?);

    Ok(())
}

async fn run_query<UP: UserProvider + Sync, T: ExecutionEngine>(
    user_provider: &UP,
    engine: &mut T,
    query: &str,
) -> anyhow::Result<ExecutionResult> {
    let query = issuecraft_ql::parse_query(query)?;
    Ok(engine.execute(user_provider, &query).await?)
}
