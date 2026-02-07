#![allow(unused)]

use std::path::{Path, PathBuf};

use clap::Parser;
use issuecraft_core::{
    AuthorizationProvider, Client, ExecutionEngine, ExecutionResult, UserProvider,
};
use issuecraft_ql::{IqlQuery, UserId};

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

    let user_provider = issuecraft_core::SingleUserUserProvider;
    let authorization_provider = issuecraft_core::SingleUserAuthorizationProvider;
    let mut db = issuecraft_redb::Database::new(issuecraft_redb::DatabaseType::File(db_path))?;
    let query = issuecraft_ql::parse_query(&query)?;
    println!(
        "{}",
        run_query(&user_provider, &authorization_provider, &mut db, &query).await?
    );

    Ok(())
}

async fn run_query<
    UP: UserProvider + Sync,
    AP: AuthorizationProvider + Sync,
    T: ExecutionEngine,
>(
    user_provider: &UP,
    authorization_provider: &AP,
    engine: &mut T,
    query: &IqlQuery,
) -> anyhow::Result<ExecutionResult> {
    Ok(engine
        .execute(user_provider, authorization_provider, query)
        .await?)
}
