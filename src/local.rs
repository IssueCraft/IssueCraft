use std::{fmt::Display, path::PathBuf};

use async_trait::async_trait;
use facet::{Facet, Shape, shape_of};
use facet_pretty::FacetPretty;
use facet_value::{Value, from_value};
use issuecraft_core::{
    Client, CommentInfo, IssueInfo, IssueStatus, LoginInfo, Priority, ProjectInfo, UserInfo,
};
use issuecraft_ql::{
    Columns, CommentId, ComparisonOp, ExecutionEngine, ExecutionResult, FilterExpression, IdHelper,
    IqlError, IssueId, ProjectId, SelectStatement, UserId, parse_query,
};
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

#[derive(Facet)]
struct Entry<K, V> {
    pub key: K,
    pub value: V,
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

    fn table_exists(&self, table_name: &str) -> Result<bool, IqlError> {
        let read_txn = self.db.begin_read().map_err(to_iql_error)?;
        Ok(read_txn
            .list_tables()
            .map_err(to_iql_error)?
            .any(|table| table.name() == table_name))
    }

    fn exists(
        &self,
        table_definition: TableDefinition<'_, &str, String>,
        key: &str,
    ) -> Result<bool, IqlError> {
        let read_txn = self.db.begin_read().map_err(to_iql_error)?;
        {
            if !self.table_exists(table_definition.name())? {
                return Ok(false);
            }
            let table = read_txn
                .open_table(table_definition)
                .map_err(to_iql_error)?;
            Ok(table
                .iter()
                .map_err(to_iql_error)?
                .find(|entry| match entry {
                    Ok(e) => e.0.value() == key,
                    Err(e) => false,
                })
                .is_some())
        }
    }

    fn get_next_issue_id(&self, project: &str) -> Result<u32, IqlError> {
        if !self.table_exists(TABLE_ISSUES.name())? {
            return Ok(1);
        }
        let read_txn = self.db.begin_read().map_err(to_iql_error)?;
        let min = format!("{project}#");
        let max = format!("{project}#{}", u32::MAX);
        let next = read_txn
            .open_table(TABLE_ISSUES)
            .map_err(to_iql_error)?
            .range(min.as_str()..max.as_str())
            .map_err(to_iql_error)?
            .count()
            + 1;
        Ok(next as u32)
    }

    fn create<V: Facet<'static>>(
        &mut self,
        table_definition: TableDefinition<'_, &str, String>,
        id: &str,
        info: &V,
    ) -> Result<(), IqlError> {
        let write_txn = self.db.begin_write().map_err(to_iql_error)?;
        {
            let mut table = write_txn
                .open_table(table_definition)
                .map_err(to_iql_error)?;
            let info_str = facet_json::to_string(info).map_err(to_iql_error)?;
            table.insert(id, &info_str).map_err(to_iql_error)?;
        }
        write_txn.commit().map_err(to_iql_error)
    }

    fn get_all<K: IdHelper, V: Facet<'static>>(
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
    ) -> Result<Vec<Entry<K, V>>, IqlError> {
        let read_txn = self.db.begin_read().map_err(to_iql_error)?;
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
                .map_err(to_iql_error)?;
            let mut values = table
                .iter()
                .map_err(to_iql_error)?
                .map(|entry| {
                    entry.map_err(to_iql_error).map(|entry| {
                        facet_json::from_str::<Value>(&entry.1.value())
                            .map(|v| (K::id_from_str(entry.0.value()), v))
                    })
                })
                .skip(offset.unwrap_or(0) as usize)
                .take(limit.unwrap_or(u32::MAX) as usize)
                .collect::<Result<Result<Vec<_>, _>, _>>()?
                .map_err(to_iql_error)?;
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
                    Some(filter_expr) => filter_expr.matches(k.str_from_id(), v),
                })
                .map(|(k, v)| {
                    from_value::<V>(v)
                        .map_err(to_iql_error)
                        .map(|v| Entry { key: k, value: v })
                })
                .collect::<Result<Vec<_>, _>>()?)
        }
    }

    fn get<T: Facet<'static>>(
        &self,
        table_definition: TableDefinition<'_, &str, String>,
        key: &str,
    ) -> Result<T, IqlError> {
        let read_txn = self.db.begin_read().map_err(to_iql_error)?;
        {
            let table = read_txn
                .open_table(table_definition)
                .map_err(to_iql_error)?;
            let info = table
                .get(key)
                .map_err(to_iql_error)?
                .ok_or_else(|| IqlError::ProjectNotFound(key.to_string()))?
                .value();
            facet_json::from_str(&info).map_err(|e| to_iql_error(e))
        }
    }

    fn get_keys(
        &self,
        table_definition: TableDefinition<'_, &str, String>,
    ) -> Result<Vec<String>, IqlError> {
        self.get_keys_as::<String>(table_definition)
    }

    fn get_keys_as<T: IdHelper>(
        &self,
        table_definition: TableDefinition<'_, &str, String>,
    ) -> Result<Vec<T>, IqlError> {
        let read_txn = self.db.begin_read().map_err(to_iql_error)?;
        {
            let table = read_txn
                .open_table(table_definition)
                .map_err(to_iql_error)?;
            table
                .iter()
                .map_err(to_iql_error)?
                .map(|entry| entry.map(|k| T::id_from_str(k.0.value())))
                .collect::<Result<Vec<_>, _>>()
                .map_err(to_iql_error)
        }
    }
}

fn stringify<'a, T: Facet<'a>>(value: &'a T) -> String {
    format!("{}", value.pretty())
}

fn to_iql_error<E: Display>(err: E) -> IqlError {
    IqlError::ImplementationSpecific(format!("{err}"))
}

#[async_trait]
impl ExecutionEngine for Database {
    async fn execute(&mut self, query: &str) -> Result<ExecutionResult, IqlError> {
        match parse_query(query)? {
            issuecraft_ql::Statement::Select(select_statement) => {
                println!("Select Statement: {select_statement:#?}");
                let info = match select_statement.from {
                    issuecraft_ql::EntityType::Users => return Err(IqlError::NotSupported),
                    issuecraft_ql::EntityType::Projects => stringify(
                        &self
                            .get_all::<ProjectId, ProjectInfo>(TABLE_PROJECTS, &select_statement)?,
                    ),
                    issuecraft_ql::EntityType::Issues => stringify(
                        &self.get_all::<IssueId, IssueInfo>(TABLE_ISSUES, &select_statement)?,
                    ),
                    issuecraft_ql::EntityType::Comments => stringify(
                        &self
                            .get_all::<CommentId, CommentInfo>(TABLE_PROJECTS, &select_statement)?,
                    ),
                };
                Ok(ExecutionResult::zero().with_info(&info))
            }
            issuecraft_ql::Statement::Create(create_statement) => match create_statement {
                issuecraft_ql::CreateStatement::User {
                    username,
                    email,
                    name,
                } => Err(IqlError::NotSupported),
                issuecraft_ql::CreateStatement::Project {
                    project_id,
                    name,
                    description,
                    owner,
                } => {
                    if self.exists(TABLE_PROJECTS, &project_id)? {
                        return Err(IqlError::ProjectAlreadyExists(project_id));
                    }
                    let project_info = ProjectInfo {
                        owner: UserId(REDB_DEFAULT_USER.to_string()),
                        description,
                        display: name,
                    };
                    self.create(TABLE_PROJECTS, &project_id, &project_info)?;
                    Ok(ExecutionResult::zero())
                }
                issuecraft_ql::CreateStatement::Issue {
                    project,
                    title,
                    description,
                    priority,
                    assignee,
                    labels,
                } => {
                    if !self.exists(TABLE_PROJECTS, &project)? {
                        return Err(IqlError::ProjectNotFound(project));
                    }
                    let issue_number = self.get_next_issue_id(&project)?;
                    let issue_info = IssueInfo {
                        title,
                        description,
                        status: IssueStatus::Open,
                        project: ProjectId(project.clone()),
                        assignee: assignee.or(Some(UserId(REDB_DEFAULT_USER.to_string()))),
                        priority: priority.map(|p| match p {
                            issuecraft_ql::Priority::Critical => Priority::Critical,
                            issuecraft_ql::Priority::High => Priority::High,
                            issuecraft_ql::Priority::Medium => Priority::Medium,
                            issuecraft_ql::Priority::Low => Priority::Low,
                        }),
                    };
                    self.create(
                        TABLE_ISSUES,
                        &format!("{project}#{issue_number}"),
                        &issue_info,
                    )?;

                    Ok(ExecutionResult::zero())
                }
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
