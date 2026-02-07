use std::fmt::Debug;

use anyhow::Result;
use cucumber::{World, given, then, when};
use issuecraft_core::{
    Entry, ExecutionEngine, ExecutionResult, SingleUserAuthorizationProvider,
    SingleUserUserProvider,
};
use issuecraft_ql::*;
use issuecraft_redb::{Database, DatabaseType};

#[derive(World)]
pub struct IssuecraftWorld {
    pub user_provider: Option<SingleUserUserProvider>,
    pub authorization_provider: Option<SingleUserAuthorizationProvider>,
    pub engine: Option<Database>,
}

impl Debug for IssuecraftWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IssuecraftWorld").finish()
    }
}

impl IssuecraftWorld {
    async fn execute(&mut self, query: &str) -> Result<ExecutionResult> {
        let query = parse_query(query)?;
        Ok(self
            .engine
            .as_mut()
            .unwrap()
            .execute(
                self.user_provider.as_ref().unwrap(),
                self.authorization_provider.as_ref().unwrap(),
                &query,
            )
            .await?)
    }
}

impl Default for IssuecraftWorld {
    fn default() -> Self {
        Self {
            user_provider: None,
            authorization_provider: None,
            engine: None,
        }
    }
}

#[given("a fresh database")]
fn fresh_db(world: &mut IssuecraftWorld) -> Result<()> {
    world.engine = Some(Database::new(DatabaseType::InMemory)?);
    Ok(())
}

#[given("a single user provider")]
fn single_user_provider(world: &mut IssuecraftWorld) {
    world.user_provider = Some(SingleUserUserProvider);
}

#[given("a single user authorization provider")]
fn single_user_authorization_provider(world: &mut IssuecraftWorld) {
    world.authorization_provider = Some(SingleUserAuthorizationProvider);
}

#[when(expr = "I execute the query {string}")]
async fn execute_query(world: &mut IssuecraftWorld, query: String) -> Result<ExecutionResult> {
    Ok(world.execute(&query).await?)
}

#[when(expr = "I create a project {string} with the display name {string}")]
async fn create_project(
    world: &mut IssuecraftWorld,
    project_id: String,
    display_name: String,
) -> Result<ExecutionResult> {
    let query = format!("CREATE PROJECT {project_id} WITH name '{display_name}'");
    Ok(world.execute(&query).await?)
}

#[when(expr = "I create an issue of kind {string} with the title {string} in project {string}")]
async fn create_issue(
    world: &mut IssuecraftWorld,
    kind: String,
    title: String,
    project_id: String,
) -> Result<ExecutionResult> {
    let query = format!("CREATE ISSUE OF KIND {kind} IN {project_id} WITH TITLE '{title}'");
    Ok(world.execute(&query).await?)
}

#[when(expr = "I comment {string} on issue {string}")]
async fn create_comment(
    world: &mut IssuecraftWorld,
    comment: String,
    issue_id: String,
) -> Result<ExecutionResult> {
    let query = format!("COMMENT ON ISSUE {issue_id} WITH '{comment}'");
    Ok(world.execute(&query).await?)
}

#[when(expr = "I update the display name of the project {string} to {string}")]
async fn update_project(
    world: &mut IssuecraftWorld,
    project_id: String,
    display_name: String,
) -> Result<ExecutionResult> {
    let query = format!("UPDATE PROJECT {project_id} SET name = '{display_name}'");
    Ok(world.execute(&query).await?)
}

#[then(expr = "a user {string} exists with the name {string}")]
async fn user_exists(world: &mut IssuecraftWorld, user_id: String, name: String) -> Result<()> {
    let query = format!("SELECT * FROM users WHERE id = '{user_id}'");
    let result = world.execute(&query).await?;
    let result: Vec<Entry<UserId>> = facet_json::from_str(result.data.as_ref().unwrap())?;
    assert_eq!(result.len(), 1);
    let user = result.first().unwrap();
    assert_eq!(user.value.name, name);
    Ok(())
}

#[then(expr = "a project {string} exists with the name {string}")]
async fn project_exists(
    world: &mut IssuecraftWorld,
    project_id: String,
    name: String,
) -> Result<()> {
    let query = format!("SELECT * FROM projects WHERE id = '{project_id}'");
    let result = world.execute(&query).await?;
    let result: Vec<Entry<ProjectId>> = facet_json::from_str(result.data.as_ref().unwrap())?;
    assert_eq!(result.len(), 1);
    let user = result.first().unwrap();
    assert_eq!(user.value.name, Some(name));
    Ok(())
}

#[then(expr = "an issue {string} exists with the kind {string} and title {string}")]
async fn issue_exists(
    world: &mut IssuecraftWorld,
    issue_id: String,
    kind: String,
    title: String,
) -> Result<()> {
    let query = format!("SELECT * FROM issues WHERE id = '{issue_id}'");
    let result = world.execute(&query).await?;
    let result: Vec<Entry<IssueId>> = facet_json::from_str(result.data.as_ref().unwrap())?;
    assert_eq!(result.len(), 1);
    let issue = result.first().unwrap();
    assert_eq!(issue.value.title, title);
    assert_eq!(
        issue.value.kind,
        match kind.to_lowercase().as_str() {
            "epic" => IssueKind::Epic,
            "improvement" => IssueKind::Improvement,
            "bug" => IssueKind::Bug,
            "task" => IssueKind::Task,
            _ => panic!("Invalid issue kind"),
        }
    );
    Ok(())
}

#[then(expr = "a comment exists with author {string}, issue id {string} and content {string}")]
async fn comment_exists(
    world: &mut IssuecraftWorld,
    author: String,
    issue_id: String,
    comment: String,
) -> Result<()> {
    let query = format!("SELECT * FROM comments WHERE issue = '{issue_id}'");
    let result = world.execute(&query).await?;
    let result: Vec<Entry<CommentId>> = facet_json::from_str(result.data.as_ref().unwrap())?;
    assert_eq!(result.len(), 1);
    let user = result.first().unwrap();
    assert_eq!(user.value.author, UserId::new(&author));
    assert_eq!(user.value.content, comment);
    Ok(())
}

#[tokio::main]
async fn main() {
    IssuecraftWorld::run("tests/features/query.feature").await
}
