use async_trait::async_trait;
use facet::Facet;
use facet_json::{DeserializeError, JsonError};

#[derive(thiserror::Error, Debug)]
pub enum ClientError {
    #[error("Database error: {0}")]
    MalformedIql(#[from] issuecraft_ql::ParseError),
    #[error("Deserialization error: {0}")]
    DeserializationError(#[from] DeserializeError<JsonError>),
    #[error("Client specific: {0}")]
    ClientSpecific(String),
    #[error("Not implemented")]
    NotImplemented,
    #[error("This action is not supported by the chosen backend")]
    NotSupported,
}

#[derive(Debug, Clone, Facet)]
#[facet(transparent)]
pub struct UserId(pub String);
#[derive(Debug, Clone, Facet)]
pub struct UserInfo {
    pub display: Option<String>,
    pub email: String,
}

#[derive(Debug, Clone, Facet)]
#[facet(transparent)]
pub struct ProjectId(pub String);
#[derive(Debug, Clone, Facet)]
pub struct ProjectInfo {
    pub owner: UserId,
    pub display: Option<String>,
}

#[derive(Debug, Clone, Facet)]
#[repr(C)]
pub enum CloseReason {
    Duplicate,
    WontFix,
    Fixed,
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
#[facet(transparent)]
pub struct IssueId(pub String);
#[derive(Debug, Clone, Facet)]
pub struct IssueInfo {
    pub title: String,
    pub description: String,
    pub status: IssueStatus,
    pub project: ProjectId,
}

#[derive(Debug, Clone, Facet)]
#[facet(transparent)]
pub struct CommentId(pub String);
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
    async fn login(&mut self, login: LoginInfo) -> Result<(), ClientError>;
    async fn logout(&mut self) -> Result<(), ClientError>;
    async fn execute(&mut self, query: &str) -> Result<String, ClientError>;
}

pub trait Backend {
    fn init(&mut self) {}
}
