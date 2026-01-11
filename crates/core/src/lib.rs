use async_trait::async_trait;
use facet::Facet;
use facet_json::{DeserializeError, JsonError};
use issuecraft_ql::{CloseReason, ExecutionEngine, ExecutionResult, ProjectId, UserId};

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

#[derive(Debug, Clone, Facet)]
pub struct UserInfo {
    pub name: String,
    pub display: Option<String>,
    pub email: String,
}

#[derive(Debug, Clone, Facet)]
pub struct ProjectInfo {
    pub description: Option<String>,
    pub owner: UserId,
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
    pub description: Option<String>,
    pub status: IssueStatus,
    pub project: ProjectId,
    pub priority: Option<Priority>,
    pub assignee: Option<UserId>,
}

impl IssueInfo {
    pub fn is_closed(&self) -> bool {
        matches!(self.status, IssueStatus::Closed { .. })
    }
}

#[derive(Debug, Clone, Facet)]
pub struct CommentInfo {
    pub created_at: time::UtcDateTime,
    pub content: String,
    pub author: UserId,
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
        Err(ClientError::NotSupported)
    }
    async fn logout(&mut self) -> Result<(), ClientError> {
        Err(ClientError::NotSupported)
    }
    async fn query(&mut self, query: &str) -> Result<ExecutionResult, ClientError>;
}

#[async_trait]
impl<E: ExecutionEngine + Send> Client for E {
    async fn query(&mut self, query: &str) -> Result<ExecutionResult, ClientError> {
        let result = self.execute(query).await?;
        Ok(result)
    }
}

pub trait Backend {
    fn init(&mut self) {}
}
