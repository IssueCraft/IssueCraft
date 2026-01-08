use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(thiserror::Error, Debug)]
pub enum ClientError {
    #[error("Client specific: {0}")]
    ClientSpecific(String),
    #[error("Not implemented")]
    NotImplemented,
    #[error("Not supported")]
    NotSupported,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UserId(pub String);
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub display: Option<String>,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectId(pub String);
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub owner: UserId,
    pub display: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CloseReason {
    Duplicate,
    WontFix { reason: String },
    Fixed { link: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueStatus {
    Open,
    Assigned,
    Blocked,
    Closed { reason: CloseReason },
}

pub struct IssueId(pub String);
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueInfo {
    pub title: String,
    pub description: String,
    pub project: ProjectId,
    pub status: IssueStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommentId(pub String);
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentInfo {
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub content: String,
    pub author: UserId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthenticationInfo {
    Password { password: String },
    Token { token: String },
    Certificate { path: Vec<u8> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginInfo {
    pub user: String,
    pub auth: AuthenticationInfo,
}

#[async_trait]
pub trait Client {
    async fn init(&mut self) {}
    async fn login(&mut self, login: LoginInfo) -> Result<(), ClientError>;
    async fn logout(&mut self) -> Result<(), ClientError>;
    async fn add_project(
        &mut self,
        name: &str,
        owner: &UserId,
        display_name: Option<String>,
    ) -> Result<(), ClientError>;
    async fn get_projects(&self) -> Result<Vec<ProjectId>, ClientError>;
    async fn get_project_info(&self, project_id: &ProjectId) -> Result<ProjectInfo, ClientError>;
    async fn add_user(
        &mut self,
        name: &str,
        email: &str,
        display_name: Option<String>,
    ) -> Result<(), ClientError>;
    async fn get_user(&self) -> Result<UserId, ClientError>;
    async fn get_user_info(&self, user: &UserId) -> Result<UserInfo, ClientError>;
    async fn get_issues(&self) -> Result<Vec<IssueId>, ClientError>;
    async fn get_issue_info(&self, issue: &IssueId) -> Result<UserInfo, ClientError>;
    async fn add_issue(
        &mut self,
        title: &str,
        description: &str,
        project: &ProjectId,
    ) -> Result<(), ClientError>;
    async fn update_issue(&mut self, issue: &IssueId, content: &str) -> Result<(), ClientError>;
    async fn change_issue_status(
        &mut self,
        issue: &IssueId,
        status: IssueStatus,
    ) -> Result<(), ClientError>;
    async fn get_comments(&self, issue: &IssueId) -> Result<Vec<CommentId>, ClientError>;
    async fn get_comment_info(&self, comment: &CommentId) -> Result<CommentInfo, ClientError>;
    async fn add_comment(
        &mut self,
        issue: &IssueId,
        content: &str,
    ) -> Result<CommentId, ClientError>;
    async fn update_comment(
        &mut self,
        comment: &CommentId,
        content: &str,
    ) -> Result<(), ClientError>;
    async fn delete_comment(&mut self, comment: &CommentId) -> Result<(), ClientError>;
}

pub trait Backend {
    fn init(&mut self) {}
}
