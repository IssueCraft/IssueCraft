use std::{fmt::Display, ops::Deref};

use async_trait::async_trait;
use bon::Builder;
use facet::Facet;
use facet_json::{DeserializeError, JsonError};
use facet_pretty::FacetPretty;
use facet_value::Value as FacetValue;
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
    pub name: Option<String>,
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
    #[must_use]
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

#[derive(Debug, Clone, Facet)]
#[repr(C)]
#[facet(transparent)]
pub enum Action {
    Create,
    Delete,
    Update,
}

#[derive(Debug, Clone, Facet)]
#[repr(C)]
#[facet(transparent)]
pub enum Resource {
    User,
    Project,
    Issue,
    Comment,
}

#[derive(Debug, Clone, Facet, PartialEq)]
#[repr(C)]
#[facet(transparent)]
pub enum AuthorizationStatus {
    Authorized,
    Denied,
}

#[derive(Debug, Clone, Facet)]
pub struct AuthorizationResult {
    pub user: UserId,
    pub action: Action,
    pub resource: Resource,
    pub status: AuthorizationStatus,
}

#[async_trait]
pub trait AuthorizationProvider {
    async fn check_authorization(
        &self,
        principal: &UserId,
        action: &Action,
        resource: &Resource,
        context: Option<FacetValue>,
    ) -> Result<AuthorizationResult, BackendError>;
}

#[async_trait]
pub trait UserProvider {
    async fn get_user(&self, token: &str) -> Result<Option<UserId>, BackendError>;
}

pub struct SingleUserUserProvider;

#[async_trait]
impl UserProvider for SingleUserUserProvider {
    async fn get_user(&self, token: &str) -> Result<Option<UserId>, BackendError> {
        match token {
            "<default>" | "default" => Ok(Some(UserId::new("default"))),
            _ => Ok(None),
        }
    }
}

pub struct SingleUserAuthorizationProvider;

#[async_trait]
impl AuthorizationProvider for SingleUserAuthorizationProvider {
    async fn check_authorization(
        &self,
        principal: &UserId,
        action: &Action,
        resource: &Resource,
        _context: Option<FacetValue>,
    ) -> Result<AuthorizationResult, BackendError> {
        if principal == &UserId::new("default") {
            Ok(AuthorizationResult {
                user: principal.clone(),
                action: action.clone(),
                resource: resource.clone(),
                status: AuthorizationStatus::Authorized,
            })
        } else {
            Err(BackendError::PermissionDenied(format!(
                "User '{}' is not authorized",
                principal
            )))
        }
    }
}

#[async_trait]
pub trait ExecutionEngine {
    async fn execute<UP: UserProvider + Sync, AP: AuthorizationProvider + Sync>(
        &mut self,
        user_provider: &UP,
        authorization_provider: &AP,
        query: &IqlQuery,
    ) -> Result<ExecutionResult, BackendError>;
}

#[derive(Debug, Facet)]
pub struct Entry<K, V> {
    pub key: K,
    pub value: V,
}

#[derive(Debug, Clone, Builder)]
pub struct ExecutionResult {
    #[builder(start_fn)]
    pub rows: u128,
    pub info: Option<String>,
    pub data: Option<String>,
}

impl Display for ExecutionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Affected Rows: {}", self.rows)?;
        if let Some(info) = &self.info {
            write!(f, "\nInfo: {info}")?;
        }
        if let Some(data) = &self.data {
            let data: Vec<Entry<UserId, <UserId as EntityId>::EntityType>> =
                facet_json::from_str(&facet_json::from_str::<String>(data).unwrap()).unwrap();
            write!(f, "\nData: {}", data.pretty())?;
        }
        Ok(())
    }
}

impl From<String> for ExecutionResult {
    fn from(s: String) -> Self {
        Self {
            rows: 0,
            info: Some(s),
            data: None,
        }
    }
}

impl From<&str> for ExecutionResult {
    fn from(s: &str) -> Self {
        Self {
            rows: 0,
            info: Some(s.to_string()),
            data: None,
        }
    }
}

impl ExecutionResult {
    #[must_use]
    pub fn new(rows: u128) -> Self {
        Self {
            rows: rows,
            info: None,
            data: None,
        }
    }

    #[must_use]
    pub fn one() -> ExecutionResultBuilder {
        Self::builder(1)
    }

    #[must_use]
    pub fn zero() -> ExecutionResultBuilder {
        Self::builder(0)
    }

    pub fn inc(&mut self) {
        self.rows += 1;
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
        Self::new(s)
    }
    fn kind() -> EntityType {
        EntityType::Users
    }
}

impl EntityId for ProjectId {
    type EntityType = ProjectInfo;
    fn from_str(s: &str) -> Self {
        Self::new(s)
    }
    fn kind() -> EntityType {
        EntityType::Projects
    }
}

impl EntityId for IssueId {
    type EntityType = IssueInfo;
    fn from_str(s: &str) -> Self {
        Self::new(s)
    }
    fn kind() -> EntityType {
        EntityType::Issues
    }
}

impl EntityId for CommentId {
    type EntityType = CommentInfo;
    fn from_str(s: &str) -> Self {
        Self::new(s)
    }
    fn kind() -> EntityType {
        EntityType::Comments
    }
}
