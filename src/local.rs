use std::{fmt::Display, path::PathBuf};

use async_trait::async_trait;
use facet::{Facet, Shape, shape_of};
use facet_pretty::FacetPretty;
use facet_value::{Value, from_value};
use issuecraft_core::{
    Client, CommentInfo, IssueInfo, IssueStatus, LoginInfo, Priority, ProjectInfo, UserInfo,
};
use issuecraft_ql::{
    CloseStatement, Columns, CommentId, CommentStatement, ComparisonOp, EntityType,
    ExecutionEngine, ExecutionResult, FieldUpdate, FilterExpression, IdHelper, IqlError, IssueId,
    ProjectId, ReopenStatement, SelectStatement, UpdateStatement, UserId, parse_query,
};
use nanoid::nanoid;
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

fn get_table<'a>(kind: EntityType) -> TableDefinition<'a, &'a str, String> {
    match kind {
        EntityType::Users => TABLE_META,
        EntityType::Projects => TABLE_PROJECTS,
        EntityType::Issues => TABLE_ISSUES,
        EntityType::Comments => TABLE_COMMENTS,
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

    fn table_exists(&self, table_name: &str) -> Result<bool, IqlError> {
        let read_txn = self.db.begin_read().map_err(to_iql_error)?;
        Ok(read_txn
            .list_tables()
            .map_err(to_iql_error)?
            .any(|table| table.name() == table_name))
    }

    fn exists(&self, kind: EntityType, key: &str) -> Result<bool, IqlError> {
        let read_txn = self.db.begin_read().map_err(to_iql_error)?;
        {
            let table_definition = get_table(kind);
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

    fn update<'a, S: Facet<'a>>(
        &mut self,
        kind: EntityType,
        id: &str,
        updates: Vec<FieldUpdate>,
    ) -> Result<(), IqlError> {
        let mut item_info: Value = self.get(kind, &id)?;
        for update in updates {
            update.apply_to::<S>(&mut item_info)?;
        }
        self.set(kind, &id, &item_info)?;
        Ok(())
    }

    fn set<V: Facet<'static>>(
        &mut self,
        kind: EntityType,
        id: &str,
        info: &V,
    ) -> Result<(), IqlError> {
        let write_txn = self.db.begin_write().map_err(to_iql_error)?;
        {
            let table_definition = get_table(kind);
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
        kind: EntityType,
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
            let table_definition = get_table(kind);
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

    fn get<T: Facet<'static>>(&self, kind: EntityType, key: &str) -> Result<T, IqlError> {
        let read_txn = self.db.begin_read().map_err(to_iql_error)?;
        {
            let table_definition = get_table(kind);
            let table = read_txn
                .open_table(table_definition)
                .map_err(to_iql_error)?;
            let info = table
                .get(key)
                .map_err(to_iql_error)?
                .ok_or_else(|| IqlError::ItemNotFound {
                    id: key.to_string(),
                    kind: kind.kind(),
                })?
                .value();
            facet_json::from_str(&info).map_err(|e| to_iql_error(e))
        }
    }

    fn get_keys(&self, kind: EntityType) -> Result<Vec<String>, IqlError> {
        self.get_keys_as::<String>(kind)
    }

    fn get_keys_as<T: IdHelper>(&self, kind: EntityType) -> Result<Vec<T>, IqlError> {
        let read_txn = self.db.begin_read().map_err(to_iql_error)?;
        {
            let table_definition = get_table(kind);
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
    let value: Value = facet_json::from_str(&facet_json::to_string(value).unwrap()).unwrap();
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
                let info = match select_statement.from {
                    issuecraft_ql::EntityType::Users => return Err(IqlError::NotSupported),
                    issuecraft_ql::EntityType::Projects => {
                        stringify(&self.get_all::<ProjectId, ProjectInfo>(
                            EntityType::Projects,
                            &select_statement,
                        )?)
                    }
                    issuecraft_ql::EntityType::Issues => stringify(
                        &self
                            .get_all::<IssueId, IssueInfo>(EntityType::Issues, &select_statement)?,
                    ),
                    issuecraft_ql::EntityType::Comments => {
                        stringify(&self.get_all::<CommentId, CommentInfo>(
                            EntityType::Comments,
                            &select_statement,
                        )?)
                    }
                };
                Ok(ExecutionResult::zero().with_info(&info))
            }
            issuecraft_ql::Statement::Create(create_statement) => match create_statement {
                issuecraft_ql::CreateStatement::User { .. } => Err(IqlError::NotSupported),
                issuecraft_ql::CreateStatement::Project {
                    project_id,
                    name,
                    description,
                    owner,
                } => {
                    if self.exists(EntityType::Projects, &project_id)? {
                        return Err(IqlError::ProjectAlreadyExists(project_id));
                    }
                    let project_info = ProjectInfo {
                        owner: UserId(REDB_DEFAULT_USER.to_string()),
                        description,
                        display: name,
                    };
                    self.set(EntityType::Projects, &project_id, &project_info)?;
                    Ok(ExecutionResult::one())
                }
                issuecraft_ql::CreateStatement::Issue {
                    project,
                    title,
                    description,
                    priority,
                    assignee,
                    labels,
                } => {
                    if !self.exists(EntityType::Projects, &project)? {
                        return Err(IqlError::ItemNotFound {
                            kind: EntityType::Projects.kind(),
                            id: project.to_string(),
                        });
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
                    self.set(
                        EntityType::Issues,
                        &format!("{project}#{issue_number}"),
                        &issue_info,
                    )?;

                    Ok(ExecutionResult::one())
                }
            },
            issuecraft_ql::Statement::Update(UpdateStatement { entity, updates }) => match entity {
                issuecraft_ql::UpdateTarget::User(id) => Err(IqlError::NotSupported),
                issuecraft_ql::UpdateTarget::Project(ProjectId(id)) => {
                    self.update::<ProjectInfo>(EntityType::Projects, &id, updates)?;
                    Ok(ExecutionResult::one())
                }
                issuecraft_ql::UpdateTarget::Issue(IssueId(id)) => {
                    self.update::<IssueInfo>(EntityType::Issues, &id, updates)?;
                    Ok(ExecutionResult::one())
                }
                issuecraft_ql::UpdateTarget::Comment(CommentId(id)) => {
                    self.update::<CommentInfo>(EntityType::Comments, &id, updates)?;
                    Ok(ExecutionResult::one())
                }
            },
            issuecraft_ql::Statement::Delete(_) => Err(IqlError::NotSupported),
            issuecraft_ql::Statement::Assign(_) => Err(IqlError::NotSupported),
            issuecraft_ql::Statement::Close(CloseStatement { issue_id, reason }) => {
                let mut issue_info: IssueInfo =
                    self.get(EntityType::Issues, &issue_id.str_from_id())?;
                if let IssueStatus::Closed { reason } = issue_info.status {
                    return Err(IqlError::IssueAlreadyClosed(
                        issue_id.str_from_id().to_string(),
                        reason,
                    ));
                }
                self.set(
                    EntityType::Issues,
                    &issue_id.str_from_id(),
                    &IssueInfo {
                        status: IssueStatus::Closed {
                            reason: reason.unwrap_or_default(),
                        },
                        ..issue_info
                    },
                )?;

                Ok(ExecutionResult::one())
            }
            issuecraft_ql::Statement::Reopen(ReopenStatement { issue_id }) => {
                let mut issue_info: IssueInfo =
                    self.get(EntityType::Issues, &issue_id.str_from_id())?;
                if let IssueStatus::Closed { reason } = issue_info.status {
                    return Ok(ExecutionResult::zero());
                }
                self.set(
                    EntityType::Issues,
                    &issue_id.str_from_id(),
                    &IssueInfo {
                        status: IssueStatus::Open,
                        ..issue_info
                    },
                )?;

                Ok(ExecutionResult::one())
            }
            issuecraft_ql::Statement::Comment(CommentStatement { issue_id, content }) => {
                if !self.exists(EntityType::Issues, &issue_id.str_from_id())? {
                    return Err(IqlError::ItemNotFound {
                        kind: EntityType::Issues.kind(),
                        id: issue_id.str_from_id().to_string(),
                    });
                }
                let comment_info = CommentInfo {
                    issue: issue_id,
                    author: UserId(REDB_DEFAULT_USER.to_string()),
                    content,
                    created_at: time::UtcDateTime::now(),
                };
                self.set(
                    EntityType::Comments,
                    &format!("C{}", nanoid!()),
                    &comment_info,
                )?;
                Ok(ExecutionResult::one())
            }
        }
    }
}
