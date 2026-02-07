use std::fmt::Debug;

use anyhow::Result;
use cucumber::{World, given, then, when};
use issuecraft_core::{
    Entry, ExecutionEngine, ExecutionResult, ProjectInfo, SingleUserAuthorizationProvider,
    SingleUserUserProvider, UserInfo,
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

#[when(expr = "I update the display name of the project {string} to {string}")]
async fn update_project(
    world: &mut IssuecraftWorld,
    project_id: String,
    display_name: String,
) -> Result<ExecutionResult> {
    let query = format!("UPDATE PROJECT {project_id} SET name = '{display_name}'");
    Ok(world.execute(&query).await?)
}

#[then(expr = "a user {string} exists with the name {string} and the email {string}")]
async fn user_exists(
    world: &mut IssuecraftWorld,
    user_id: String,
    name: String,
    email: String,
) -> Result<()> {
    let query = format!("SELECT * FROM users WHERE id = '{}'", user_id);
    let result = world.execute(&query).await?;
    let result: Vec<Entry<UserId, UserInfo>> = facet_json::from_str(result.data.as_ref().unwrap())?;
    assert_eq!(result.len(), 1);
    let user = result.first().unwrap();
    assert_eq!(user.value.name, name);
    assert_eq!(user.value.email, email);
    Ok(())
}

#[then(expr = "a project {string} exists with the name {string}")]
async fn project_exists(
    world: &mut IssuecraftWorld,
    project_id: String,
    name: String,
) -> Result<()> {
    let query = format!("SELECT * FROM projects WHERE id = '{}'", project_id);
    let result = world.execute(&query).await?;
    let result: Vec<Entry<ProjectId, ProjectInfo>> =
        facet_json::from_str(result.data.as_ref().unwrap())?;
    assert_eq!(result.len(), 1);
    let user = result.first().unwrap();
    assert_eq!(user.value.name, Some(name));
    Ok(())
}

#[tokio::main]
async fn main() {
    IssuecraftWorld::run("tests/features/query.feature").await
}
