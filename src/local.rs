use std::{fmt::Display, path::PathBuf};

use async_trait::async_trait;
use issuecraft_common::{
    Client, ClientError, CommentId, CommentInfo, IssueId, IssueInfo, IssueStatus, LoginInfo,
    ProjectId, ProjectInfo, UserId, UserInfo,
};
use redb::{
    Key, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition, TransactionError,
    Value, backends::InMemoryBackend,
};
use serde::de::DeserializeOwned;

const TABLE_META: TableDefinition<&str, String> = TableDefinition::new("meta");
const TABLE_USERS: TableDefinition<&str, String> = TableDefinition::new("users");
const TABLE_PROJECTS: TableDefinition<&str, String> = TableDefinition::new("projects");
const TABLE_ISSUES: TableDefinition<&str, String> = TableDefinition::new("issues");
const TABLE_COMMENTS: TableDefinition<&str, String> = TableDefinition::new("comments");

pub struct Database {
    db: redb::Database,
}

pub enum DatabaseType {
    InMemory,
    File(PathBuf),
}

trait IdFromStr {
    fn from_str(val: &str) -> Self;
}

impl IdFromStr for String {
    fn from_str(val: &str) -> Self {
        val.to_string()
    }
}

impl IdFromStr for ProjectId {
    fn from_str(val: &str) -> Self {
        ProjectId(val.to_string())
    }
}

impl IdFromStr for IssueId {
    fn from_str(val: &str) -> Self {
        IssueId(val.to_string())
    }
}

impl IdFromStr for CommentId {
    fn from_str(val: &str) -> Self {
        CommentId(val.to_string())
    }
}

impl Database {
    pub fn new(typ: &DatabaseType) -> anyhow::Result<Self> {
        match typ {
            DatabaseType::InMemory => {
                let db = redb::Database::builder().create_with_backend(InMemoryBackend::new())?;
                Ok(Self { db })
            }
            DatabaseType::File(path) => {
                let db = redb::Database::create(path)?;
                Ok(Self { db })
            }
        }
    }

    fn get<T: DeserializeOwned>(
        &self,
        table_definition: TableDefinition<'_, &str, String>,
        key: &str,
    ) -> Result<T, ClientError> {
        let read_txn = self.db.begin_read().map_err(to_client_error)?;
        {
            let table = read_txn
                .open_table(table_definition)
                .map_err(to_client_error)?;
            let info = table
                .get(key)
                .map_err(to_client_error)?
                .ok_or_else(|| ClientError::ClientSpecific(format!("Project not found")))?
                .value();
            serde_json::from_str(&info).map_err(|e| to_client_error(e))
        }
    }

    fn get_keys(
        &self,
        table_definition: TableDefinition<'_, &str, String>,
    ) -> Result<Vec<String>, ClientError> {
        self.get_keys_as::<String>(table_definition)
    }

    fn get_keys_as<T: IdFromStr>(
        &self,
        table_definition: TableDefinition<'_, &str, String>,
    ) -> Result<Vec<T>, ClientError> {
        let read_txn = self.db.begin_read().map_err(to_client_error)?;
        {
            let table = read_txn
                .open_table(table_definition)
                .map_err(to_client_error)?;
            table
                .iter()
                .map_err(to_client_error)?
                .map(|entry| entry.map(|k| T::from_str(k.0.value())))
                .collect::<Result<Vec<_>, _>>()
                .map_err(to_client_error)
        }
    }
}

fn to_client_error<E: Display>(err: E) -> ClientError {
    ClientError::ClientSpecific(format!("{err}"))
}

#[async_trait]
impl Store for Database {
    async fn fetch_schema(&self, table_name: &str) -> Result<Option<Schema>> {}

    async fn fetch_all_schemas(&self) -> Result<Vec<Schema>> {}

    async fn fetch_data(&self, table_name: &str, key: &Key) -> Result<Option<DataRow>> {}

    async fn scan_data(&self, table_name: &str) -> Result<RowIter> {}
}
