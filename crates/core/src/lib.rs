use std::{fmt::Display, ops::Deref};

use async_trait::async_trait;
use facet::Facet;
use facet_json::{DeserializeError, JsonError};
use issuecraft_ql::{
    CloseReason, CommentId, EntityType, IqlError, IqlQuery, IssueId, IssueKind, ProjectId, UserId,
};

#[derive(thiserror::Error, Debug)]
pub enum ClientError {
    #[error("Not implemented")]
    NotImplemented,
    #[error("This action is not supported by the chosen backend")]
    NotSupported,
    #[error("IQL error: {0}")]
    IqlError(#[from] issuecraft_ql::IqlError),
    #[error("Deserialization error: {0}")]
    DeserializationError(#[from] DeserializeError<JsonError>),
    #[error("Client specific: {0}")]
    ClientSpecific(String),
}

#[derive(thiserror::Error, Debug)]
pub enum BackendError {
    #[error("IQL error: {0}")]
    IqlError(#[from] IqlError),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("A project with the name '{0}' already exists")]
    ProjectAlreadyExists(String),
    #[error("User with id '{id}' not found")]
    UserNotFound { id: String },
    #[error("No item of type '{kind}' with the id '{id}' exists")]
    ItemNotFound { kind: String, id: String },
    #[error("The issue withe the name '{0}' was already closed. Reason '{1}'")]
    IssueAlreadyClosed(String, CloseReason),
    #[error("Field not found: {0}")]
    FieldNotFound(String),
    #[error("IQL impl {0}")]
    ImplementationSpecific(String),
    #[error("Could not parse id: {0}")]
    InvalidId(String),
    #[error("Not implemented")]
    NotImplemented,
    #[error("This action is not supported by the chosen backend")]
    NotSupported,
}

#[derive(Debug, Clone, Facet)]
pub struct UserInfo {
    pub name: String,
    #[facet( skip_serializing_if = Option::is_none)]
    pub display: Option<String>,
    pub email: String,
}

#[derive(Debug, Clone, Facet)]
pub struct ProjectInfo {
    #[facet(skip_serializing_if = Option::is_none)]
    pub description: Option<String>,
    pub owner: UserId,
    #[facet(skip_serializing_if = Option::is_none)]
    pub display: Option<String>,
}

#[derive(Debug, Clone, Facet)]
#[repr(C)]
pub enum IssueStatus {
    Open,
    Assigned,
    Blocked,
    Closed { reason: CloseReason },
}

#[derive(Debug, Clone, Facet)]
#[repr(C)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Facet)]
pub struct IssueInfo {
    pub title: String,
    pub kind: IssueKind,
    #[facet(skip_serializing_if = Option::is_none)]
    pub description: Option<String>,
    pub status: IssueStatus,
    pub project: ProjectId,
    #[facet(skip_serializing_if = Option::is_none)]
    pub priority: Option<Priority>,
    pub assignee: UserId,
}

impl IssueInfo {
    pub fn is_closed(&self) -> bool {
        matches!(self.status, IssueStatus::Closed { .. })
    }
}

#[derive(Debug, Clone, Facet)]
pub struct CommentInfo {
    pub issue: IssueId,
    pub created_at: time::UtcDateTime,
    pub content: String,
    pub author: UserId,
}

#[async_trait]
pub trait UserProvider {
    fn get_user(&self, token: &str) -> Result<UserId, BackendError>;
}

pub struct SingleUserProvider {
    pub user: String,
}

impl SingleUserProvider {
    pub fn new(user: &str) -> Self {
        Self {
            user: user.to_string(),
        }
    }
}

impl UserProvider for SingleUserProvider {
    fn get_user(&self, _token: &str) -> Result<UserId, BackendError> {
        Ok(UserId::from(UserId(self.user.clone())))
    }
}

#[async_trait]
pub trait ExecutionEngine {
    async fn execute<UP: UserProvider + Sync>(
        &mut self,
        user_provider: &UP,
        query: &IqlQuery,
    ) -> Result<ExecutionResult, BackendError>;
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub affected_rows: u128,
    pub info: Option<String>,
}

impl Display for ExecutionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Affected Rows: {}", self.affected_rows)?;
        if let Some(info) = &self.info {
            write!(f, "\nInfo: {}", info)?;
        }
        Ok(())
    }
}

impl From<String> for ExecutionResult {
    fn from(s: String) -> Self {
        Self {
            affected_rows: 0,
            info: Some(s),
        }
    }
}

impl From<&str> for ExecutionResult {
    fn from(s: &str) -> Self {
        Self {
            affected_rows: 0,
            info: Some(s.to_string()),
        }
    }
}

impl ExecutionResult {
    pub fn new(rows: u128) -> Self {
        Self {
            affected_rows: rows,
            info: None,
        }
    }

    pub fn one() -> Self {
        Self {
            affected_rows: 1,
            info: None,
        }
    }

    pub fn zero() -> Self {
        Self {
            affected_rows: 0,
            info: None,
        }
    }

    pub fn inc(&mut self) {
        self.affected_rows += 1;
    }

    pub fn with_info(mut self, info: &str) -> Self {
        self.info = Some(info.to_string());
        self
    }
}

#[derive(Debug, Clone)]
pub enum AuthenticationInfo {
    Password { password: String },
    Token { token: String },
    Certificate { path: Vec<u8> },
}

#[derive(Debug, Clone)]
pub struct LoginInfo {
    pub user: String,
    pub auth: AuthenticationInfo,
}

#[async_trait]
pub trait Client {
    async fn login(&mut self, _login: LoginInfo) -> Result<(), ClientError> {
        Err(ClientError::NotImplemented)
    }
    async fn logout(&mut self) -> Result<(), ClientError> {
        Err(ClientError::NotImplemented)
    }
    async fn query(&mut self, query: &IqlQuery) -> Result<ExecutionResult, ClientError>;
}

pub trait Backend {
    fn init(&mut self) {}
    fn run_migrations(&mut self) {}
}

pub trait EntityId: Deref<Target = str> + Sized {
    type EntityType: Facet<'static> + Clone;
    fn from_str(s: &str) -> Self;
    fn kind() -> EntityType;
}

impl EntityId for UserId {
    type EntityType = UserInfo;
    fn from_str(s: &str) -> Self {
        Self(s.to_string())
    }
    fn kind() -> EntityType {
        EntityType::Users
    }
}

impl EntityId for ProjectId {
    type EntityType = ProjectInfo;
    fn from_str(s: &str) -> Self {
        Self(s.to_string())
    }
    fn kind() -> EntityType {
        EntityType::Projects
    }
}

impl EntityId for IssueId {
    type EntityType = IssueInfo;
    fn from_str(s: &str) -> Self {
        Self(s.to_string())
    }
    fn kind() -> EntityType {
        EntityType::Issues
    }
}

impl EntityId for CommentId {
    type EntityType = CommentInfo;
    fn from_str(s: &str) -> Self {
        Self(s.to_string())
    }
    fn kind() -> EntityType {
        EntityType::Comments
    }
}
