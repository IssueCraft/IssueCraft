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
impl Client for Database {
    async fn init(&mut self) {}
    async fn login(&mut self, login: LoginInfo) -> Result<(), ClientError> {
        Ok(())
    }
    async fn logout(&mut self) -> Result<(), ClientError> {
        Ok(())
    }
    async fn add_project(
        &mut self,
        name: &str,
        owner: &UserId,
        display_name: Option<String>,
    ) -> Result<(), ClientError> {
        let write_txn = self.db.begin_write().map_err(to_client_error)?;
        {
            let mut table = write_txn
                .open_table(TABLE_PROJECTS)
                .map_err(to_client_error)?;
            table
                .insert(
                    name,
                    &serde_json::to_string(&ProjectInfo {
                        display: display_name,
                        owner: owner.clone(),
                    })
                    .unwrap(),
                )
                .map_err(to_client_error)?;
        }
        write_txn.commit().map_err(to_client_error)?;
        Ok(())
    }
    async fn get_projects(&self) -> Result<Vec<ProjectId>, ClientError> {
        self.get_keys_as::<ProjectId>(TABLE_PROJECTS)
    }
    async fn get_project_info(&self, project_id: &ProjectId) -> Result<ProjectInfo, ClientError> {
        self.get(TABLE_PROJECTS, project_id.0.as_str())
    }
    async fn add_user(
        &mut self,
        name: &str,
        email: &str,
        display_name: Option<String>,
    ) -> Result<(), ClientError> {
        let write_txn = self.db.begin_write().map_err(to_client_error)?;
        {
            let mut table = write_txn.open_table(TABLE_USERS).map_err(to_client_error)?;
            table
                .insert(
                    name,
                    &serde_json::to_string(&UserInfo {
                        display: display_name,
                        email: email.to_string(),
                    })
                    .unwrap(),
                )
                .map_err(to_client_error)?;
        }
        write_txn.commit().map_err(to_client_error)?;
        Ok(())
    }
    async fn get_user(&self) -> Result<UserId, ClientError> {
        Ok(UserId("local".to_string()))
    }
    async fn get_user_info(&self, user: &UserId) -> Result<UserInfo, ClientError> {
        Err(ClientError::NotSupported)
    }
    async fn get_issues(&self) -> Result<Vec<IssueId>, ClientError> {
        self.get_keys_as::<IssueId>(TABLE_ISSUES)
    }
    async fn get_issue_info(&self, issue: &IssueId) -> Result<UserInfo, ClientError> {
        self.get(TABLE_ISSUES, issue.0.as_str())
    }
    async fn add_issue(
        &mut self,
        title: &str,
        description: &str,
        project: &ProjectId,
    ) -> Result<(), ClientError> {
        let read_txn = self.db.begin_read().map_err(to_client_error)?;
        let project_name = {
            let table = read_txn
                .open_table(TABLE_PROJECTS)
                .map_err(to_client_error)?;
            table
                .get(project.0.as_str())
                .map_err(to_client_error)?
                .ok_or_else(|| ClientError::ClientSpecific(format!("Project not found")))?
                .value()
        };
        let write_txn = self.db.begin_write().map_err(to_client_error)?;
        {
            let mut table = write_txn
                .open_table(TABLE_ISSUES)
                .map_err(to_client_error)?;
            let id = format!("{}-{}", project_name, table.len().unwrap_or_default());
            table
                .insert(
                    id.as_str(),
                    &serde_json::to_string(&IssueInfo {
                        description: description.to_string(),
                        title: title.to_string(),
                        status: IssueStatus::Open,
                        project: project.clone(),
                    })
                    .unwrap(),
                )
                .map_err(to_client_error)?;
        }
        write_txn.commit().map_err(to_client_error)?;
        Ok(())
    }
    async fn update_issue(&mut self, issue: &IssueId, content: &str) -> Result<(), ClientError> {
        Err(ClientError::NotImplemented)
    }
    async fn change_issue_status(
        &mut self,
        issue: &IssueId,
        status: IssueStatus,
    ) -> Result<(), ClientError> {
        Err(ClientError::NotImplemented)
    }
    async fn get_comments(&self, issue: &IssueId) -> Result<Vec<CommentId>, ClientError> {
        Err(ClientError::NotImplemented)
    }
    async fn get_comment_info(&self, comment: &CommentId) -> Result<CommentInfo, ClientError> {
        Err(ClientError::NotImplemented)
    }
    async fn add_comment(
        &mut self,
        issue: &IssueId,
        content: &str,
    ) -> Result<CommentId, ClientError> {
        Err(ClientError::NotImplemented)
    }
    async fn update_comment(
        &mut self,
        comment: &CommentId,
        content: &str,
    ) -> Result<(), ClientError> {
        Err(ClientError::NotImplemented)
    }
    async fn delete_comment(&mut self, comment: &CommentId) -> Result<(), ClientError> {
        Err(ClientError::NotImplemented)
    }
}
