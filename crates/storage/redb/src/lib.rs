use std::{fmt::Display, path::PathBuf};

use async_trait::async_trait;
use facet::Facet;
use facet_pretty::FacetPretty;
use facet_value::{Value, from_value, value};
use issuecraft_core::{
    AuthorizationProvider, BackendError, CommentInfo, EntityId, ExecutionEngine, ExecutionResult,
    IssueInfo, IssueStatus, Priority, ProjectInfo, UserProvider,
};
use issuecraft_ql::{
    AssignStatement, CloseStatement, CommentId, CommentStatement, DeleteStatement, DeleteTarget,
    EntityType, FieldUpdate, IqlQuery, IssueId, ProjectId, ReopenStatement, SelectStatement,
    UpdateStatement, UserId,
};
use nanoid::nanoid;
use redb::{
    DatabaseError, ReadableDatabase, ReadableTable, TableDefinition, TableHandle,
    backends::InMemoryBackend,
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
    pub fn new(typ: &DatabaseType) -> Result<Self, DatabaseError> {
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

    fn table_exists(&self, table_name: &str) -> Result<bool, BackendError> {
        let read_txn = self.db.begin_read().map_err(to_iql_error)?;
        Ok(read_txn
            .list_tables()
            .map_err(to_iql_error)?
            .any(|table| table.name() == table_name))
    }

    fn exists<ID: EntityId>(&self, id: &ID) -> Result<bool, BackendError> {
        let read_txn = self.db.begin_read().map_err(to_iql_error)?;
        {
            let table_definition = get_table(ID::kind());
            if !self.table_exists(table_definition.name())? {
                return Ok(false);
            }
            let table = read_txn
                .open_table(table_definition)
                .map_err(to_iql_error)?;
            Ok(table
                .iter()
                .map_err(to_iql_error)?
                .any(|entry| match entry {
                    Ok(e) => e.0.value() == &**id,
                    Err(_) => false,
                }))
        }
    }

    fn get_next_issue_id(&self, project: &ProjectId) -> Result<u64, BackendError> {
        if !self.table_exists(TABLE_ISSUES.name())? {
            return Ok(1);
        }
        let read_txn = self.db.begin_read().map_err(to_iql_error)?;
        let min = format!("{project}#");
        let max = format!("{project}#{}", u64::MAX);
        let next = read_txn
            .open_table(TABLE_ISSUES)
            .map_err(to_iql_error)?
            .range(min.as_str()..max.as_str())
            .map_err(to_iql_error)?
            .count()
            + 1;
        Ok(u64::try_from(next).expect("Maximum issue count exceeded"))
    }

    fn delete<ID: EntityId>(&mut self, id: &ID) -> Result<(), BackendError> {
        let write_txn = self.db.begin_write().map_err(to_iql_error)?;
        {
            let table_definition = get_table(ID::kind());
            let mut table = write_txn
                .open_table(table_definition)
                .map_err(to_iql_error)?;
            table.remove(&**id).map_err(to_iql_error)?;
        }
        write_txn.commit().map_err(to_iql_error)
    }

    fn delete_comment(
        &mut self,
        id: &CommentId,
        result: &mut ExecutionResult,
    ) -> Result<(), BackendError> {
        self.delete(id)?;
        result.inc();
        Ok(())
    }

    fn delete_issue(
        &mut self,
        id: &IssueId,
        result: &mut ExecutionResult,
    ) -> Result<(), BackendError> {
        self.delete(id)?;
        result.inc();

        for comment in self.get_all::<CommentId>(&SelectStatement {
            columns: issuecraft_ql::Columns::All,
            from: EntityType::Comments,
            filter: Some(issuecraft_ql::FilterExpression::Comparison {
                field: "issue".to_string(),
                op: issuecraft_ql::ComparisonOp::Equal,
                value: issuecraft_ql::IqlValue::String(id.to_string()),
            }),
            order_by: None,
            limit: None,
            offset: None,
        })? {
            self.delete_comment(&comment.key, result)?;
        }
        Ok(())
    }

    fn delete_project(
        &mut self,
        id: &ProjectId,
        result: &mut ExecutionResult,
    ) -> Result<(), BackendError> {
        self.delete(id)?;
        result.inc();

        for issue in self.get_all::<IssueId>(&SelectStatement {
            columns: issuecraft_ql::Columns::All,
            from: EntityType::Comments,
            filter: Some(issuecraft_ql::FilterExpression::Comparison {
                field: "issue".to_string(),
                op: issuecraft_ql::ComparisonOp::Equal,
                value: issuecraft_ql::IqlValue::String(id.to_string()),
            }),
            order_by: None,
            limit: None,
            offset: None,
        })? {
            self.delete_issue(&issue.key, result)?;
        }
        Ok(())
    }

    fn update<ID: EntityId>(
        &mut self,
        id: &ID,
        updates: &[FieldUpdate],
    ) -> Result<(), BackendError> {
        let mut item_info: Value = self.get_as(id)?;
        for update in updates {
            update.apply_to::<ID::EntityType>(&mut item_info)?;
        }
        self.set_from_value(id, &item_info)?;
        Ok(())
    }

    fn set_from_value<ID: EntityId, V: Facet<'static>>(
        &mut self,
        id: &ID,
        info: &V,
    ) -> Result<(), BackendError> {
        let write_txn = self.db.begin_write().map_err(to_iql_error)?;
        {
            let table_definition = get_table(ID::kind());
            let mut table = write_txn
                .open_table(table_definition)
                .map_err(to_iql_error)?;
            let info_str = facet_json::to_string(info).map_err(to_iql_error)?;
            table.insert(&**id, &info_str).map_err(to_iql_error)?;
        }
        write_txn.commit().map_err(to_iql_error)
    }

    fn set<ID: EntityId>(&mut self, id: &ID, info: &ID::EntityType) -> Result<(), BackendError> {
        self.set_from_value(id, info)
    }

    fn get_all<K: EntityId>(
        &self,
        SelectStatement {
            columns: _,
            from,
            filter,
            order_by,
            limit,
            offset,
        }: &SelectStatement,
    ) -> Result<Vec<Entry<K, K::EntityType>>, BackendError> {
        let read_txn = self.db.begin_read().map_err(to_iql_error)?;
        {
            let table_definition = get_table(*from);
            if !read_txn
                .list_tables()
                .unwrap()
                .any(|table| table.name() == table_definition.name())
            {
                return Ok(vec![]);
            }
            let table = read_txn
                .open_table(table_definition)
                .map_err(to_iql_error)?;
            let mut values = table
                .iter()
                .map_err(to_iql_error)?
                .map(|entry| {
                    entry.map_err(to_iql_error).map(|entry| {
                        facet_json::from_str::<Value>(&entry.1.value())
                            .map(|v| (K::from_str(entry.0.value()), v))
                    })
                })
                .skip(
                    usize::try_from(offset.unwrap_or(0))
                        .expect("Number exceeds max supported value"),
                )
                .take(
                    usize::try_from(limit.unwrap_or(u64::MAX))
                        .expect("Number exceeds max supported value"),
                )
                .collect::<Result<Result<Vec<_>, _>, _>>()?
                .map_err(to_iql_error)?;
            if let Some(order_by) = order_by {
                values.sort_by(|a, b| {
                    let o1 = a.1.as_object().unwrap();
                    let o2 = b.1.as_object().unwrap();
                    match (
                        o1.get(&order_by.field.clone()),
                        o2.get(&order_by.field.clone()),
                    ) {
                        (None, None) => std::cmp::Ordering::Equal,
                        (Some(_), None) => std::cmp::Ordering::Greater,
                        (None, Some(_)) => std::cmp::Ordering::Less,
                        (Some(v1), Some(v2)) => v1.partial_cmp(v2).unwrap(),
                    }
                });
            }

            values
                .into_iter()
                .filter(|(k, v)| match filter {
                    None => true,
                    Some(filter_expr) => filter_expr.matches(k, v),
                })
                .map(|(k, v)| {
                    from_value::<K::EntityType>(v)
                        .map_err(to_iql_error)
                        .map(|v| Entry { key: k, value: v })
                })
                .collect::<Result<Vec<_>, _>>()
        }
    }

    fn get<ID: EntityId>(&self, key: &ID) -> Result<ID::EntityType, BackendError> {
        self.get_as(key)
    }

    fn get_as<ID: EntityId, T: Facet<'static>>(&self, key: &ID) -> Result<T, BackendError> {
        let read_txn = self.db.begin_read().map_err(to_iql_error)?;
        {
            let table_definition = get_table(ID::kind());
            let table = read_txn
                .open_table(table_definition)
                .map_err(to_iql_error)?;
            let info = table
                .get(&**key)
                .map_err(to_iql_error)?
                .ok_or_else(|| BackendError::ItemNotFound {
                    id: key.to_string(),
                    kind: ID::kind().to_string(),
                })?
                .value();
            facet_json::from_str(&info).map_err(to_iql_error)
        }
    }
}

fn stringify<'a, T: Facet<'a>>(value: &'a T) -> String {
    let value: Value = facet_json::from_str(&facet_json::to_string(value).unwrap()).unwrap();
    format!("{}", value.pretty())
}

fn to_iql_error<E: Display>(err: E) -> BackendError {
    BackendError::ImplementationSpecific(format!("{err}"))
}

#[async_trait]
#[allow(clippy::too_many_lines)]
impl ExecutionEngine for Database {
    async fn execute<UP: UserProvider + Sync, AP: AuthorizationProvider + Sync>(
        &mut self,
        user_provider: &UP,
        authorization_provider: &AP,
        query: &IqlQuery,
    ) -> Result<ExecutionResult, BackendError> {
        match query {
            issuecraft_ql::IqlQuery::Select(select_statement) => {
                let info = match select_statement.from {
                    issuecraft_ql::EntityType::Users => return Err(BackendError::NotSupported),
                    issuecraft_ql::EntityType::Projects => {
                        stringify(&self.get_all::<ProjectId>(select_statement)?)
                    }
                    issuecraft_ql::EntityType::Issues => {
                        stringify(&self.get_all::<IssueId>(select_statement)?)
                    }
                    issuecraft_ql::EntityType::Comments => {
                        stringify(&self.get_all::<CommentId>(select_statement)?)
                    }
                };
                Ok(ExecutionResult::zero().with_info(&info))
            }
            issuecraft_ql::IqlQuery::Create(create_statement) => match create_statement {
                issuecraft_ql::CreateStatement::User { .. } => Err(BackendError::NotSupported),
                issuecraft_ql::CreateStatement::Project {
                    project_id,
                    name,
                    description,
                    owner,
                } => {
                    if self.exists(project_id)? {
                        return Err(BackendError::ProjectAlreadyExists(project_id.to_string()));
                    }
                    let owner = match owner {
                        Some(owner) => owner.clone(),
                        None => user_provider.get_user("").await?,
                    };

                    if !self.exists(&owner)? {
                        return Err(BackendError::UserNotFound {
                            id: owner.to_string(),
                        });
                    }
                    let project_info = ProjectInfo {
                        owner,
                        description: description.clone(),
                        display: name.clone(),
                    };
                    self.set(project_id, &project_info)?;
                    Ok(ExecutionResult::one())
                }
                issuecraft_ql::CreateStatement::Issue {
                    project,
                    kind,
                    title,
                    description,
                    priority,
                    assignee,
                } => {
                    if !self.exists(project)? {
                        return Err(BackendError::ItemNotFound {
                            kind: EntityType::Projects.to_string(),
                            id: project.to_string(),
                        });
                    }
                    let assignee = match assignee {
                        Some(assignee) => assignee.clone(),
                        None => user_provider.get_user("").await?,
                    };
                    let issue_number = self.get_next_issue_id(project)?;
                    let issue_info = IssueInfo {
                        title: title.clone(),
                        kind: kind.clone(),
                        description: description.clone(),
                        status: IssueStatus::Open,
                        project: project.clone(),
                        assignee,
                        priority: priority.clone().map(|p| match p {
                            issuecraft_ql::Priority::Critical => Priority::Critical,
                            issuecraft_ql::Priority::High => Priority::High,
                            issuecraft_ql::Priority::Medium => Priority::Medium,
                            issuecraft_ql::Priority::Low => Priority::Low,
                        }),
                    };
                    self.set(
                        &IssueId::new(&format!("{project}#{issue_number}")),
                        &issue_info,
                    )?;

                    Ok(ExecutionResult::one())
                }
            },
            issuecraft_ql::IqlQuery::Update(UpdateStatement { entity, updates }) => match entity {
                issuecraft_ql::UpdateTarget::User(_) => Err(BackendError::NotSupported),
                issuecraft_ql::UpdateTarget::Project(id) => {
                    self.update(id, updates)?;
                    Ok(ExecutionResult::one())
                }
                issuecraft_ql::UpdateTarget::Issue(id) => {
                    self.update(id, updates)?;
                    Ok(ExecutionResult::one())
                }
                issuecraft_ql::UpdateTarget::Comment(id) => {
                    let user = user_provider.get_user("").await?;
                    let author: Value = self.get(id)?.author.into();
                    let context = value!({
                        "owner": author
                    });
                    if authorization_provider
                        .check_authorization(
                            &user,
                            &issuecraft_core::Action::Update,
                            &issuecraft_core::Resource::Comment,
                            Some(context),
                        )
                        .await?
                        .status
                        != issuecraft_core::AuthorizationStatus::Authorized
                    {
                        return Err(BackendError::PermissionDenied(
                            "User is not authorized to edit comments".to_string(),
                        ));
                    }

                    if self.get(id)?.author != user {
                        return Err(BackendError::PermissionDenied(
                            "Cannot edit comments authored by other users".to_string(),
                        ));
                    }
                    self.update(id, updates)?;
                    Ok(ExecutionResult::one())
                }
            },
            issuecraft_ql::IqlQuery::Delete(DeleteStatement { entity }) => {
                let mut result = ExecutionResult::zero();
                match entity {
                    DeleteTarget::User(_) => return Err(BackendError::NotSupported),
                    DeleteTarget::Project(project_id) => {
                        self.delete_project(project_id, &mut result)?;
                    }
                    DeleteTarget::Issue(issue_id) => self.delete_issue(issue_id, &mut result)?,
                    DeleteTarget::Comment(comment_id) => {
                        self.delete_comment(comment_id, &mut result)?;
                    }
                }
                Ok(result)
            }
            issuecraft_ql::IqlQuery::Assign(AssignStatement { issue_id, assignee }) => {
                let mut issue_info: IssueInfo = self.get(issue_id)?;
                issue_info.assignee = assignee.clone();
                self.set(issue_id, &issue_info)?;
                Ok(ExecutionResult::one())
            }
            issuecraft_ql::IqlQuery::Close(CloseStatement { issue_id, reason }) => {
                let issue_info: IssueInfo = self.get(issue_id)?;
                if let IssueStatus::Closed { reason } = issue_info.status {
                    return Err(BackendError::IssueAlreadyClosed(
                        issue_id.to_string(),
                        reason,
                    ));
                }
                self.set(
                    issue_id,
                    &IssueInfo {
                        status: IssueStatus::Closed {
                            reason: reason.clone().unwrap_or_default(),
                        },
                        ..issue_info
                    },
                )?;

                Ok(ExecutionResult::one())
            }
            issuecraft_ql::IqlQuery::Reopen(ReopenStatement { issue_id }) => {
                let issue_info: IssueInfo = self.get(issue_id)?;
                if !matches!(issue_info.status, IssueStatus::Closed { .. }) {
                    return Ok(ExecutionResult::zero());
                }
                self.set(
                    issue_id,
                    &IssueInfo {
                        status: IssueStatus::Open,
                        ..issue_info
                    },
                )?;

                Ok(ExecutionResult::one())
            }
            issuecraft_ql::IqlQuery::Comment(CommentStatement { issue_id, content }) => {
                if !self.exists(issue_id)? {
                    return Err(BackendError::ItemNotFound {
                        kind: EntityType::Issues.to_string(),
                        id: issue_id.to_string(),
                    });
                }
                let comment_info = CommentInfo {
                    issue: issue_id.clone(),
                    author: UserId::from_str(REDB_DEFAULT_USER),
                    content: content.clone(),
                    created_at: time::UtcDateTime::now(),
                };
                self.set(
                    &CommentId::from_str(&format!("C{}", nanoid!())),
                    &comment_info,
                )?;
                Ok(ExecutionResult::one())
            }
        }
    }
}
