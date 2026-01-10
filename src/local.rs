use std::{fmt::Display, path::PathBuf};

use async_trait::async_trait;
use facet::{Facet, Shape, shape_of};
use facet_value::{Value, from_value};
use issuecraft_core::{
    Client, ClientError, CommentId, CommentInfo, IssueId, IssueInfo, IssueStatus, LoginInfo,
    ProjectId, ProjectInfo, UserId, UserInfo,
};
use issuecraft_ql::{ComparisonOp, FilterExpression, SelectStatement, parse_query};
use redb::{
    Key, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition, TableHandle,
    TransactionError, backends::InMemoryBackend,
};

const REDB_DEFAULT_USER: &str = "redb_local";

const TABLE_META: TableDefinition<&str, String> = TableDefinition::new("meta");
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

    fn create<V: Facet<'static>>(
        &mut self,
        table_definition: TableDefinition<'_, &str, String>,
        id: &str,
        info: &V,
    ) -> Result<(), ClientError> {
        let write_txn = self.db.begin_write().map_err(to_client_error)?;
        {
            let mut table = write_txn
                .open_table(table_definition)
                .map_err(to_client_error)?;
            let info_str = facet_json::to_string(info).map_err(to_client_error)?;
            table.insert(id, &info_str).map_err(to_client_error)?;
        }
        write_txn.commit().map_err(to_client_error)
    }

    fn get_all<K: IdFromStr, V: Facet<'static>>(
        &self,
        table_definition: TableDefinition<'_, &str, String>,
        SelectStatement {
            columns,
            from,
            filter,
            order_by,
            limit,
            offset,
        }: &SelectStatement,
    ) -> Result<Vec<(K, V)>, ClientError> {
        let read_txn = self.db.begin_read().map_err(to_client_error)?;
        {
            if !read_txn
                .list_tables()
                .unwrap()
                .any(|table| table.name() == table_definition.name())
            {
                return Ok(vec![]);
            }
            let mut table = read_txn
                .open_table(table_definition)
                .map_err(to_client_error)?;
            let mut values = table
                .iter()
                .map_err(to_client_error)?
                .map(|entry| {
                    entry.map_err(to_client_error).map(|entry| {
                        facet_json::from_str::<Value>(&entry.1.value())
                            .map(|v| (K::from_str(entry.0.value()), v))
                    })
                })
                .skip(offset.unwrap_or(0) as usize)
                .take(limit.unwrap_or(u32::MAX) as usize)
                .collect::<Result<Result<Vec<_>, _>, _>>()??;
            if let Some(order_by) = order_by {
                values.sort_by(|a, b| {
                    let o1 = a.1.as_object().unwrap();
                    let o2 = b.1.as_object().unwrap();
                    match (
                        o1.get(&order_by.field.clone()),
                        o2.get(&order_by.field.to_owned()),
                    ) {
                        (None, None) => return std::cmp::Ordering::Equal,
                        (Some(_), None) => return std::cmp::Ordering::Greater,
                        (None, Some(_)) => return std::cmp::Ordering::Less,
                        (Some(v1), Some(v2)) => v1.partial_cmp(v2).unwrap(),
                    }
                });
            }

            Ok(values
                .into_iter()
                .filter(|(k, v)| match filter {
                    None => true,
                    Some(filter_expr) => filter_expr.matches(v),
                })
                .map(|(k, v)| from_value::<V>(v).map_err(to_client_error).map(|v| (k, v)))
                .collect::<Result<Vec<_>, _>>()?)
        }
    }

    fn get<T: Facet<'static>>(
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
            facet_json::from_str(&info).map_err(|e| to_client_error(e))
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

fn stringify<T: Facet<'static>>(value: &T) -> Result<String, ClientError> {
    facet_json::to_string_pretty(value).map_err(to_client_error)
}

fn to_client_error<E: Display>(err: E) -> ClientError {
    ClientError::ClientSpecific(format!("{err}"))
}

#[async_trait]
impl Client for Database {
    async fn login(&mut self, login: LoginInfo) -> Result<(), ClientError> {
        Ok(())
    }
    async fn logout(&mut self) -> Result<(), ClientError> {
        Ok(())
    }
    async fn execute(&mut self, query: &str) -> Result<String, ClientError> {
        match parse_query(query)? {
            issuecraft_ql::Statement::Select(select_statement) => {
                println!("Select Statement: {select_statement:#?}");
                match select_statement.from {
                    issuecraft_ql::EntityType::Users => {
                        Ok("The redb backend only supports one user".to_string())
                    }
                    issuecraft_ql::EntityType::Projects => stringify(
                        &self
                            .get_all::<ProjectId, ProjectInfo>(TABLE_PROJECTS, &select_statement)?,
                    ),
                    issuecraft_ql::EntityType::Issues => stringify(
                        &self.get_all::<IssueId, IssueInfo>(TABLE_ISSUES, &select_statement)?,
                    ),
                    issuecraft_ql::EntityType::Comments => stringify(
                        &self
                            .get_all::<ProjectId, ProjectInfo>(TABLE_PROJECTS, &select_statement)?,
                    ),
                }
            }
            issuecraft_ql::Statement::Create(create_statement) => match create_statement {
                issuecraft_ql::CreateStatement::User {
                    username,
                    email,
                    name,
                } => Err(ClientError::NotSupported),
                issuecraft_ql::CreateStatement::Project {
                    project_id,
                    name,
                    description,
                    owner,
                } => {
                    let project_info = ProjectInfo {
                        owner: UserId(REDB_DEFAULT_USER.to_string()),
                        display: name,
                    };
                    self.create(TABLE_PROJECTS, &project_id, &project_info)?;
                    Ok("SUCCESS".to_string())
                }
                issuecraft_ql::CreateStatement::Issue {
                    project,
                    title,
                    description,
                    priority,
                    assignee,
                    labels,
                } => todo!(),
                issuecraft_ql::CreateStatement::Comment {
                    issue_id,
                    content,
                    author,
                } => todo!(),
            },
            issuecraft_ql::Statement::Update(update_statement) => todo!(),
            issuecraft_ql::Statement::Delete(delete_statement) => todo!(),
            issuecraft_ql::Statement::Assign(assign_statement) => todo!(),
            issuecraft_ql::Statement::Close(close_statement) => todo!(),
            issuecraft_ql::Statement::Comment(comment_statement) => todo!(),
        }
    }
}
